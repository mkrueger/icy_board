use super::run_ppl;

#[test]
fn empty_dynamic_arrays_produce_empty_indices() {
    for element_type in ["INTEGER", "STRING"] {
        for destination in ["INTEGER indices[]", "INTEGER indices[] = { 9, 8 }", "INTEGER indices[3]"] {
            assert_eq!(
                "0: done",
                run_ppl(&format!(
                    r#"
;$LANGVERSION 400
{element_type} data[]
{destination}
INTEGER index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT "unexpected "
NEXT
PRINT "done"
"#
                )),
                "{element_type}, {destination}"
            );
        }
    }
}

#[test]
fn dynamic_sort_visits_each_index_once_without_changing_the_source() {
    assert_eq!(
        "4: 1=-2 3=3 0=8 2=11 |8 -2 11 3",
        run_ppl(
            r#"
;$LANGVERSION 400
INTEGER data[] = { 8, -2, 11, 3 }
INTEGER indices[], index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT index, "=", data[index], " "
NEXT
PRINT "|", data[0], " ", data[1], " ", data[2], " ", data[3]
"#
        )
    );
}

#[test]
fn equal_values_still_produce_each_source_index_exactly_once() {
    assert_eq!(
        "4: 2 2 5 5 |1 1 1 1",
        run_ppl(
            r#"
;$LANGVERSION 400
INTEGER data[] = { 5, 2, 5, 2 }
INTEGER indices[], seen[3], index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    seen[index] += 1
    PRINT data[index], " "
NEXT
PRINT "|", seen[0], " ", seen[1], " ", seen[2], " ", seen[3]
"#
        )
    );
}

#[test]
fn singleton_sort_has_only_index_zero() {
    assert_eq!(
        "1: 0=42 ",
        run_ppl(
            r#"
;$LANGVERSION 400
INTEGER data[] = { 42 }
INTEGER indices[] = { 7, 8, 9 }
INTEGER index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT index, "=", data[index], " "
NEXT
"#
        )
    );
}

#[test]
fn bounded_arrays_use_element_count_not_an_extra_upper_bound() {
    for destination_bound in [0, 1, 5] {
        assert_eq!(
            "2: 1 0 ",
            run_ppl(&format!(
                r#"
;$LANGVERSION 400
INTEGER data[1], indices[{destination_bound}], index
data[0] = 8
data[1] = 7
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT index, " "
NEXT
"#
            ))
        );
    }
}

#[test]
fn strings_keep_the_existing_comparison_order_without_an_extra_index() {
    assert_eq!(
        "3: 1=apple 2=banana 0=pear |pear apple banana",
        run_ppl(
            r#"
;$LANGVERSION 400
STRING data[] = { "pear", "apple", "banana" }
INTEGER indices[], index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT index, "=", data[index], " "
NEXT
PRINT "|", data[0], " ", data[1], " ", data[2]
"#
        )
    );
}

#[tokio::test]
async fn result_sizing_is_gated_by_ppe_runtime_version() {
    use crate::{
        executable::{EntryType, GenericVariableData, PPEExpr, TableEntry, VarHeader, VariableTable, VariableType, VariableValue},
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        parser::UserTypeRegistry,
        vm::{VirtualMachine, io::DiskIO, statements::predefined_procedures::sort},
    };
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::{path::PathBuf, sync::Arc};

    let directory = tempfile::tempdir().unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
    let registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);

    // This pins the existing pre-400 behavior, not a newly oracle-verified contract.
    for version in [300, 340, 350, 400] {
        let mut table = VariableTable::default();
        table.set_version(version);
        for (id, name) in [(1, "data"), (2, "indices")] {
            table.push(TableEntry::new(
                name,
                VarHeader {
                    id,
                    dim: 1,
                    vector_size: 1,
                    variable_type: VariableType::Integer,
                    ..Default::default()
                },
                VariableValue::new_int(0),
                EntryType::Variable,
            ));
        }
        let mut bytes = Vec::new();
        table.serialize(&mut bytes).unwrap();
        let (_, table) = VariableTable::deserialize(version, &mut bytes).unwrap();
        let mut vm = VirtualMachine::new(PathBuf::from("sort.ppe"), &registry, &mut io, &mut state);
        vm.variable_table = table;
        vm.variable_table.get_value_mut(1).set_array_value(0, 0, 0, VariableValue::new_int(8)).unwrap();
        vm.variable_table.get_value_mut(1).set_array_value(1, 0, 0, VariableValue::new_int(7)).unwrap();

        sort(&mut vm, &[PPEExpr::Value(1), PPEExpr::Value(2)]).await.unwrap();

        let GenericVariableData::Dim1(indices) = &vm.variable_table.get_value(2).generic_data else {
            panic!("SORT must produce an index vector");
        };
        let expected = if version < 400 { vec![1, 0, 0] } else { vec![1, 0] };
        assert_eq!(expected, indices.iter().map(VariableValue::as_int).collect::<Vec<_>>(), "runtime {version}");
    }
}
