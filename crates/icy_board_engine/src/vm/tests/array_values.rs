use super::{compile, run_ppl};

#[test]
fn dynamic_locals_are_fresh_on_each_call() {
    assert_eq!(
        "0 3 0 0 ",
        run_ppl(
            r#"
Work(TRUE)
Work(FALSE)
PROCEDURE Work(BOOLEAN initialize)
    INTEGER values[]
    PRINT values.Len(), " "
    IF initialize values.Redim(2)
    PRINT values.Len(), " "
ENDPROC
"#
        )
    );
}

#[test]
fn dynamic_local_frames_survive_recursion() {
    for declaration in ["INTEGER values[]\nvalues.Redim(0)", "INTEGER values[] = { 0 }"] {
        assert_eq!(
            "0 1 2 ",
            run_ppl(&format!(
                r#"
Work(2)
PROCEDURE Work(INTEGER depth)
    {declaration}
    values[0] = depth
    IF depth > 0 Work(depth - 1)
    PRINT values[0], " "
ENDPROC
"#
            ))
        );
    }
}

#[test]
fn dynamic_return_slots_are_fresh_on_each_call() {
    assert_eq!(
        "2 0",
        run_ppl(
            r#"
INTEGER values[] = Choose(TRUE)
PRINT values.Len(), " "
values = Choose(FALSE)
PRINT values.Len()
FUNCTION Choose(BOOLEAN yes) INTEGER[]
    INTEGER result[] = { 7, 8 }
    IF yes RETURN result
ENDFUNC
"#
        )
    );
}

#[test]
fn recursive_array_results_do_not_overwrite_outer_result() {
    assert_eq!(
        "2",
        run_ppl(
            r#"
INTEGER values[] = Build(2)
PRINT values[0]
FUNCTION Build(INTEGER depth) INTEGER[]
    INTEGER own[] = { depth }
    Build = own
    IF depth > 0 THEN
        INTEGER child[] = Build(depth - 1)
        PRINT ""
    ENDIF
ENDFUNC
"#
        )
    );
}

#[test]
fn whole_array_assignment_copies_all_elements_and_adopts_bounds() {
    for declaration in ["INTEGER target[1]", "INTEGER target[]", "INTEGER target[] = { 1 }"] {
        assert_eq!(
            "3 7 8 9 7",
            run_ppl(&format!(
                r#"
INTEGER source[] = {{ 7, 8, 9 }}
{declaration}
target = source
source[0] = 99
PRINT target.Len(), " ", target[0], " ", target[1], " ", target[2], " "
INTEGER copy[]
copy = (target)
target[0] = 88
PRINT copy[0]
"#
            ))
        );
    }
}

#[test]
fn whole_array_assignment_preserves_matrix_and_cube_rank() {
    assert_eq!(
        "6 42 8 77",
        run_ppl(
            r#"
INTEGER matrix[1, 2]
INTEGER matrixCopy[0, 0]
matrix[1, 2] = 42
matrixCopy = matrix
INTEGER cube[1, 1, 1]
INTEGER cubeCopy[0, 0, 0]
cube[1, 1, 1] = 77
cubeCopy = cube
PRINT matrixCopy.Len(), " ", matrixCopy[1, 2], " ", cubeCopy.Len(), " ", cubeCopy[1, 1, 1]
"#
        )
    );
}

#[test]
fn dynamic_brace_initializer_keeps_dynamic_storage() {
    let executable = compile("INTEGER values[] = { 1, 2 }\nPRINT values[0]");
    let entry = executable.variable_table.get_entries().iter().find(|entry| entry.header.dim == 1).unwrap();
    assert_ne!(0, entry.header.flags & crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY);
    assert_eq!("0", run_ppl("INTEGER values[] = {}\nPRINT values.Len()"));
}

#[test]
fn array_assignment_to_and_from_fixed_record_fields() {
    assert_eq!(
        "2 7 8 7",
        run_ppl(
            r#"
TYPE Pair
    INTEGER values[1]
ENDTYPE
Pair pair
INTEGER source[1]
source[0] = 7
source[1] = 8
pair.values = source
INTEGER copy[]
copy = pair.values
source[0] = 99
PRINT copy.Len(), " ", copy[0], " ", copy[1], " ", pair.values[0]
"#
        )
    );
}

#[test]
fn empty_dynamic_initializer_runs_each_time_the_declaration_is_reached() {
    for declaration in ["INTEGER values[] = {}", "STRING values[] = {}", "REGEXMATCH values[] = {}"] {
        assert_eq!(
            "0 0 0 ",
            run_ppl(&format!(
                r#"
INTEGER iteration
FOR iteration = 1 TO 3
    {declaration}
    PRINT values.Len(), " "
    values.Redim(2)
NEXT
"#
            ))
        );
    }
}

#[test]
fn nonempty_dynamic_initializer_restores_its_shape_on_each_iteration() {
    assert_eq!(
        "2 7 8 2 7 8 2 7 8 ",
        run_ppl(
            r#"
INTEGER iteration
FOR iteration = 1 TO 3
    INTEGER values[] = { 7, 8 }
    PRINT values.Len(), " ", values[0], " ", values[1], " "
    values.Redim(5)
    values[0] = 99
NEXT
"#
        )
    );
}

#[test]
fn string_array_assignments_preserve_elements_and_string_capacity() {
    assert_eq!(
        "2 3000 2048 3000 3000",
        run_ppl(
            r#"
TYPE Texts
    STRING values[1]
ENDTYPE
STRING source[1]
source[0] = STRING.Repeat("x", 3000)
source[1] = "tail"
STRING target[0]
BIGSTR wide[0]
target = source
wide = source
source[0] = "changed"
Texts record
record.values = source
record.values(0) = target[0]
STRING copy[]
copy = record.values
record.values(0) = "changed"
PRINT target.Len(), " ", target[0].Len(), " ", wide[0].Len(), " ", copy[0].Len(), " ", target[0].Len()
"#
        )
    );
}

#[test]
fn record_array_assignments_copy_nested_record_and_array_values() {
    assert_eq!(
        "2 first 7 first 7",
        run_ppl(
            r#"
TYPE Item
    STRING name
    INTEGER values[1]
ENDTYPE
TYPE Items
    Item entries[1]
ENDTYPE
Item source[1]
source[0].name = "first"
source[0].values(1) = 7
Item target[0]
target = source
Items record
record.entries = source
Item copy[]
copy = record.entries
source[0].name = "source"
source[0].values(1) = 99
record.entries(0).name = "record"
record.entries(0).values(1) = 88
PRINT target.Len(), " ", target[0].name, " ", target[0].values(1), " ", copy[0].name, " ", copy[0].values(1)
"#
        )
    );
}

#[test]
fn object_array_assignments_preserve_objects_from_member_results() {
    assert_eq!(
        "2 one two one",
        run_ppl(
            r#"
REGEX expression = REGEX.Compile("[a-z]+")
REGEXMATCH source[] = expression.FindAll("one two")
REGEXMATCH target[0]
target = source
REGEXMATCH copy[]
copy = (target)
source = expression.FindAll("changed")
PRINT target.Len(), " ", target[0].Value, " ", target[1].Value, " ", copy[0].Value
"#
        )
    );
}

#[test]
fn dynamic_record_arrays_start_empty_and_empty_initializers_repeat() {
    assert_eq!(
        "0 0 0 0 0 0 ",
        run_ppl(
            r#"
TYPE Item
    STRING name
ENDTYPE
Item vector[]
Item matrix[,]
Item cube[,,]
PRINT vector.Len(), " ", matrix.Len(), " ", cube.Len(), " "
INTEGER iteration
FOR iteration = 1 TO 3
    Item values[] = {}
    PRINT values.Len(), " "
    values.Redim(2)
NEXT
"#
        )
    );
}

#[test]
fn empty_local_initializers_and_array_results_keep_their_element_types() {
    for (element_type, value) in [("STRING", "\"hello\""), ("Item", "Item { name = \"hello\" }")] {
        assert_eq!(
            "0 1 0 0 ",
            run_ppl(&format!(
                r#"
TYPE Item
    STRING name
ENDTYPE
Work(TRUE)
Work(FALSE)
PROCEDURE Work(BOOLEAN initialize)
    {element_type} values[] = {{}}
    PRINT values.Len(), " "
    values = Choose(initialize)
    PRINT values.Len(), " "
ENDPROC
FUNCTION Choose(BOOLEAN initialize) {element_type}[]
    {element_type} values[] = {{ {value} }}
    IF initialize RETURN values
ENDFUNC
"#
            ))
        );
    }
}

#[test]
fn array_storage_flags_roundtrip_without_reinterpreting_classic_bits() {
    use crate::executable::{EntryType, TableEntry, VarHeader, VariableTable, VariableType};

    for version in [100, 200, 300, 340, 350, 400] {
        for flags in [0, 1, 2, 3, 0x80, 0x83] {
            for variable_type in [VariableType::Integer, VariableType::String, VariableType::BigStr] {
                for rank in 1..=3 {
                    let mut table = VariableTable::default();
                    table.set_version(version);
                    table.push(TableEntry::new(
                        "values",
                        VarHeader {
                            id: 1,
                            dim: rank,
                            vector_size: 1,
                            matrix_size: usize::from(rank >= 2),
                            cube_size: usize::from(rank >= 3),
                            variable_type,
                            flags,
                        },
                        variable_type.create_empty_value(),
                        EntryType::Variable,
                    ));
                    let mut bytes = Vec::new();
                    table.serialize(&mut bytes).unwrap();
                    let original = bytes.clone();
                    let (consumed, loaded) = VariableTable::deserialize(version, &mut bytes).unwrap();
                    assert_eq!(original.len(), consumed);
                    assert_eq!(flags, loaded.get_var_entry(1).header.flags);
                    assert_eq!(rank, loaded.get_value(1).get_dimensions());
                    let expected_count = if version >= 400 && flags & 2 != 0 { 0 } else { 1 << rank };
                    assert_eq!(
                        expected_count,
                        crate::vm::VirtualMachine::foreach_element_count(loaded.get_value(1)),
                        "version={version}, flags={flags}, type={variable_type}, rank={rank}"
                    );
                    let mut roundtrip = Vec::new();
                    loaded.serialize(&mut roundtrip).unwrap();
                    assert_eq!(original, roundtrip, "version={version}, flags={flags}");
                }
            }
        }
    }
}

#[tokio::test]
async fn classic_static_flags_and_bare_array_decay_survive_dynamic_storage_changes() {
    use crate::{
        executable::{EntryType, GenericVariableData, PPECommand, PPEExpr, ProcedureValue, TableEntry, VarHeader, VariableTable, VariableType, VariableValue},
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        vm::{ReturnAddress, VirtualMachine, io::DiskIO},
    };
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::{path::PathBuf, sync::Arc};

    let directory = tempfile::tempdir().unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
    let array = |first, second| VariableValue {
        vtype: VariableType::Integer,
        generic_data: GenericVariableData::Dim1(Arc::new(vec![VariableValue::new_int(first), VariableValue::new_int(second)])),
        ..Default::default()
    };

    for version in [200, 300, 340, 350, 400] {
        let mut table = VariableTable::default();
        table.set_version(version);
        table.push(TableEntry::new(
            "work",
            VarHeader {
                id: 1,
                variable_type: VariableType::Procedure,
                ..Default::default()
            },
            VariableValue::new_procedure(ProcedureValue {
                local_variables: 2,
                first_var_id: 1,
                ..Default::default()
            }),
            EntryType::Procedure,
        ));
        for (id, flags) in [(2, 0x01), (3, 0x02)] {
            table.push(TableEntry::new(
                "local",
                VarHeader {
                    id,
                    dim: 1,
                    vector_size: 1,
                    flags,
                    variable_type: VariableType::Integer,
                    ..Default::default()
                },
                VariableValue::new_int(0),
                EntryType::LocalVariable,
            ));
        }
        let mut bytes = Vec::new();
        table.serialize(&mut bytes).unwrap();
        let (_, table) = VariableTable::deserialize(version, &mut bytes).unwrap();
        let mut vm = VirtualMachine::new(PathBuf::from("flags.ppe"), &registry, &mut io, &mut state);
        vm.variable_table = table;
        vm.variable_table.set_value(2, array(11, 12));
        vm.variable_table.set_value(3, array(21, 22));

        vm.execute_statement(&PPECommand::Let(Box::new(PPEExpr::Value(2)), Box::new(PPEExpr::Value(3))))
            .await
            .unwrap();
        assert_eq!(21, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
        assert_eq!(
            if version < 400 { 12 } else { 22 },
            vm.variable_table.get_value(2).get_array_value(1, 0, 0).as_int()
        );

        for depth in 1..=2 {
            vm.prepare_call(2, 0, 2, &[], &[]).await.unwrap();
            assert_eq!(depth, vm.call_local_value_stack.len());
            assert_eq!(21, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
            assert_eq!(
                if version < 400 { 2 } else { 0 },
                VirtualMachine::foreach_element_count(vm.variable_table.get_value(3))
            );
            if version < 400 {
                assert_eq!(0, vm.variable_table.get_value(3).get_array_value(0, 0, 0).as_int());
            }
            vm.variable_table.set_value(3, array(40 + depth as i32, 0));
            vm.return_addresses.push(ReturnAddress::func_call(0, 1));
        }
        vm.variable_table.set_value(2, array(55, 56));
        for restored in [41, 21] {
            vm.execute_statement(&PPECommand::EndProc).await.unwrap();
            assert_eq!(restored, vm.variable_table.get_value(3).get_array_value(0, 0, 0).as_int());
            assert_eq!(55, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
        }
        assert!(vm.call_local_value_stack.is_empty());
        assert!(vm.return_addresses.is_empty());
    }
}
