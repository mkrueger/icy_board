use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    compiler::{CompilationErrorType, PPECompiler, workspace::Workspace},
    executable::{Executable, ExecutableError, GenericVariableData, RecordField, VariableType, VariableValue, create_record_value},
    parser::{
        CONTACT_ID, Encoding, ErrorReporter, FIRST_USER_TYPE_ID, UserTypeRegistry, board_catalog, parse_ast, parse_ast_with_predeclared_types,
        preparse_type_declarations,
    },
};

fn build(source: &str, predeclare: bool) -> (Option<Executable>, Arc<Mutex<ErrorReporter>>) {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(400));
    workspace.package.runtime = Some(400);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let file = PathBuf::from("record_fields.pps");
    let ast = if predeclare {
        preparse_type_declarations(file.clone(), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
        parse_ast_with_predeclared_types(file, errors.clone(), source, &registry, Encoding::Utf8, &workspace)
    } else {
        parse_ast(file, errors.clone(), source, &registry, Encoding::Utf8, &workspace)
    };
    if errors.lock().unwrap().has_errors() {
        return (None, errors);
    }
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    (compiler.create_executable().ok(), errors)
}

fn compile(source: &str) -> Executable {
    let (executable, errors) = build(source, false);
    assert!(
        !errors.lock().unwrap().has_errors(),
        "{source}\n{:?}",
        errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    executable.expect("source compiled")
}

fn rejects(source: &str, predicate: impl Fn(&CompilationErrorType) -> bool) {
    let (executable, errors) = build(source, false);
    assert!(executable.is_none(), "unexpectedly compiled: {source}");
    let errors = errors.lock().unwrap();
    assert!(
        errors
            .errors
            .iter()
            .any(|error| error.error.downcast_ref::<CompilationErrorType>().is_some_and(&predicate)),
        "{source}\n{:?}",
        errors.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
}

fn fields(value: &VariableValue) -> &[VariableValue] {
    let GenericVariableData::Record(fields) = &value.generic_data else {
        panic!("expected record: {value:?}")
    };
    fields
}

fn assert_var_storage_paths(executable: &Executable, expected: usize) {
    use crate::executable::{PPECommand, PPEExpr, PPEScript};

    fn check_path(expression: &PPEExpr) {
        match expression {
            PPEExpr::Value(_) | PPEExpr::Dim(_, _) => {}
            PPEExpr::Member(base, _) | PPEExpr::IndexedMember(base, _, _) => check_path(base),
            other => panic!("VAR copy-back cannot address {other:?}"),
        }
    }

    let script = PPEScript::from_ppe_file(executable).unwrap();
    assert_eq!(script.serialize(), executable.script_buffer);
    let mut count = 0;
    for statement in &script.statements {
        if let PPECommand::ProcedureCall(id, arguments) = &statement.command {
            let flags = unsafe { executable.variable_table.get_var_entry(*id).value.data.procedure_value.pass_flags };
            for (index, argument) in arguments.iter().enumerate() {
                if 1u16.checked_shl(index as u32).is_some_and(|mask| flags & mask != 0) {
                    check_path(argument);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, expected);
}

fn assert_record_redim_storage_path(executable: &Executable, rank: u8) -> (crate::executable::PPEScript, usize) {
    use crate::executable::{OpCode, PPECommand, PPEExpr, PPEScript, VARIABLE_FLAG_DYNAMIC_ARRAY};

    fn check_path(expression: &PPEExpr) {
        match expression {
            PPEExpr::Value(_) | PPEExpr::Dim(_, _) => {}
            PPEExpr::Member(base, _) | PPEExpr::IndexedMember(base, _, _) => check_path(base),
            other => panic!("REDIM write-back cannot address {other:?}"),
        }
    }

    let script = PPEScript::from_ppe_file(executable).expect("REDIM script must decode before execution");
    assert_eq!(script.serialize(), executable.script_buffer);
    let resizes: Vec<_> = script
        .statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| matches!(&statement.command, PPECommand::PredefinedCall(def, _) if def.opcode == OpCode::REDIM).then_some(index))
        .collect();
    let [resize] = resizes.as_slice() else {
        panic!("expected one REDIM, got {resizes:?}")
    };
    let resize = *resize;
    let PPECommand::PredefinedCall(_, arguments) = &script.statements[resize].command else {
        panic!()
    };
    assert_eq!(arguments.len(), rank as usize + 1);
    let PPEExpr::Value(temporary) = arguments[0] else {
        panic!("REDIM must retain its bare-ID encoding")
    };
    let header = &executable.variable_table.get_var_entry(temporary).header;
    assert_eq!(header.dim, rank);
    assert_ne!(header.flags & VARIABLE_FLAG_DYNAMIC_ARRAY, 0);
    assert_eq!((header.vector_size, header.matrix_size, header.cube_size), (0, 0, 0));
    let PPECommand::Let(captured, _) = &script.statements[resize - 1].command else {
        panic!("copy the field into the shape temporary before REDIM")
    };
    let PPECommand::Let(target, result) = &script.statements[resize + 1].command else {
        panic!("copy the resized temporary back to the field")
    };
    assert_eq!(captured.as_ref(), &PPEExpr::Value(temporary));
    assert_eq!(result.as_ref(), &PPEExpr::Value(temporary));
    check_path(target);
    (script, resize)
}

#[test]
fn all_host_types_are_source_fields_in_both_parser_passes() {
    let mut source = "TYPE Bundle\n INTEGER Tag\n".to_string();
    for (index, &(_, name, _)) in board_catalog::TYPES.iter().enumerate() {
        for (suffix, shape) in [("Scalar", ""), ("Fixed", "[0]"), ("Vector", "[]"), ("Matrix", "[,]"), ("Cube", "[,,]")] {
            source.push_str(&format!(" {name} Field{index}{suffix}{shape}\n"));
        }
    }
    source.push_str("ENDTYPE\nBundle item\nPRINTLN item.Tag\n");
    for predeclare in [false, true] {
        let (executable, errors) = build(&source, predeclare);
        assert!(
            !errors.lock().unwrap().has_errors(),
            "{:?}",
            errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
        );
        let executable = executable.unwrap();
        let layout = &executable.user_types[0];
        assert_eq!(layout.len(), 1 + board_catalog::TYPES.len() * 5);
        for (index, &(id, _, _)) in board_catalog::TYPES.iter().enumerate() {
            for (offset, rank, dynamic) in [(0, 0, false), (1, 1, false), (2, 1, true), (3, 2, true), (4, 3, true)] {
                let field = layout[1 + index * 5 + offset];
                assert_eq!(field.variable_type, VariableType::UserData(id as u8));
                assert_eq!((field.dim, field.is_dynamic), (rank, dynamic));
                assert_eq!(field.element_count(), Some(if dynamic { 0 } else { 1 }));
                assert_eq!((field.vector_size, field.matrix_size, field.cube_size), (0, 0, 0));
            }
        }
        assert!(matches!(executable.to_buffer(), Err(ExecutableError::UnsupportedRecordFieldEncoding { .. })));
    }
}

#[test]
fn recursive_source_records_are_rejected_including_dynamic_edges() {
    for source in [
        "TYPE Node\n Node Child\nENDTYPE\n",
        "TYPE Node\n Node Children[]\nENDTYPE\n",
        "TYPE Node\n Node Children[,,]\nENDTYPE\n",
        "TYPE First\n Second Child\nENDTYPE\nTYPE Second\n First Child\nENDTYPE\n",
        "TYPE First\n Second Children[,]\nENDTYPE\nTYPE Second\n First Children[]\nENDTYPE\n",
    ] {
        for predeclare in [false, true] {
            let (executable, errors) = build(source, predeclare);
            assert!(executable.is_none() && errors.lock().unwrap().has_errors(), "{source}");
        }
    }
}

#[test]
fn recursive_internal_layouts_cannot_recurse_through_empty_arrays() {
    let first = FIRST_USER_TYPE_ID as u8;
    for dynamic in [false, true] {
        let field = RecordField {
            dim: u8::from(dynamic),
            is_dynamic: dynamic,
            ..RecordField::scalar(VariableType::UserData(first))
        };
        assert!(create_record_value(first, &[vec![field]], &BTreeMap::new()).is_none());
        let other = RecordField {
            variable_type: VariableType::UserData(first + 1),
            ..field
        };
        assert!(create_record_value(first, &[vec![other], vec![field]], &BTreeMap::new()).is_none());
    }
}

#[test]
fn defaults_distinguish_empty_dynamic_arrays_fixed_zero_bounds_and_contact_values() {
    let executable = compile(
        "ENUM Choice\n First = 7\n Second = 9\nENDENUM\nTYPE Inner\n CONTACT Person\n AUDIO Sound\n Choice State\nENDTYPE\nTYPE Outer\n Inner Scalar\n Inner Fixed[0]\n Inner Vector[]\n Inner Matrix[,]\n Inner Cube[,,]\nENDTYPE\nOuter item\nPRINTLN item.Scalar.State\n",
    );
    let value = create_record_value(101, &executable.user_types, &executable.variable_table.enums).unwrap();
    let outer = fields(&value);
    let inner = fields(&outer[0]);
    assert_eq!(inner[0].vtype, VariableType::UserData(CONTACT_ID as u8));
    assert_eq!(fields(&inner[0]).len(), 2);
    assert!(fields(&inner[0]).iter().all(|value| value.as_string().is_empty()));
    assert!(matches!(inner[1].generic_data, GenericVariableData::None));
    assert_eq!(inner[2].as_int(), 7);
    let GenericVariableData::Dim1(fixed) = &outer[1].generic_data else {
        panic!("fixed vector")
    };
    assert_eq!(fixed.len(), 1);
    assert_eq!(fields(&fixed[0])[2].as_int(), 7);
    assert!(matches!(&outer[2].generic_data, GenericVariableData::Dim1(values) if values.is_empty()));
    assert!(matches!(&outer[3].generic_data, GenericVariableData::Dim2(values) if values.is_empty()));
    assert!(matches!(&outer[4].generic_data, GenericVariableData::Dim3(values) if values.is_empty()));
    assert!(executable.variable_table.record_io_unsupported_types.contains(&100));
    assert!(executable.variable_table.record_io_unsupported_types.contains(&101));
    assert!(create_record_value(99, &executable.user_types, &executable.variable_table.enums).is_none());
}

#[test]
fn nested_dynamic_values_preserve_copy_on_write() {
    let executable = compile("TYPE Inner\n INTEGER Values[]\nENDTYPE\nTYPE Outer\n Inner Child\nENDTYPE\nOuter item\nPRINTLN item.Child.Values.Len()\n");
    let mut original = create_record_value(101, &executable.user_types, &executable.variable_table.enums).unwrap();
    let GenericVariableData::Record(outer) = &mut original.generic_data else {
        panic!()
    };
    let GenericVariableData::Record(inner) = &mut Arc::make_mut(outer)[0].generic_data else {
        panic!()
    };
    Arc::make_mut(inner)[0] = VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(7)]);
    let mut copy = original.clone();
    assert_eq!(original, copy);
    let GenericVariableData::Record(outer) = &mut copy.generic_data else {
        panic!()
    };
    let GenericVariableData::Record(inner) = &mut Arc::make_mut(outer)[0].generic_data else {
        panic!()
    };
    let GenericVariableData::Dim1(values) = &mut Arc::make_mut(inner)[0].generic_data else {
        panic!()
    };
    Arc::make_mut(values)[0] = VariableValue::new_int(9);
    assert_eq!(fields(&fields(&original)[0])[0].get_array_value(0, 0, 0).as_int(), 7);
    assert_eq!(fields(&fields(&copy)[0])[0].get_array_value(0, 0, 0).as_int(), 9);
    assert_ne!(original, copy);
}

#[test]
fn dynamic_fields_support_assignment_literals_indexing_parameters_and_redim() {
    let executable = compile(
        r#"
TYPE Payload
 INTEGER Vector[]
 INTEGER Matrix[,]
 INTEGER Cube[,,]
 SURFACE Image
ENDTYPE
TYPE Wrapper
 Payload Items[0]
ENDTYPE
INTEGER vector[4], matrix[1,2], cube[1,2,3]
SURFACE image
Payload item = Payload { Vector = vector, Matrix = matrix, Cube = cube, Image = image }
Wrapper group
item.Vector = vector
item.Matrix = matrix
item.Cube = cube
item.Vector.Redim(2)
REDIM item.Matrix, 3, 4
group.Items[0].Cube.Redim(2, 3, 4)
group.Items[0].Vector = vector
item.Cube[0,0,0] = 5
Consume(item.Vector)
PRINTLN item.Cube[0,0,0], group.Items[0].Vector.Len()
PROCEDURE Consume(INTEGER values[])
 PRINTLN values.Len()
ENDPROC
"#,
    );
    let script = crate::executable::PPEScript::from_ppe_file(&executable).unwrap();
    assert_eq!(script.serialize(), executable.script_buffer);
}

#[test]
fn s1_var_nested_dynamic_record_fields_accept_all_ranks() {
    for (rank, bounds) in [("[]", "1"), ("[,]", "1, 2"), ("[,,]", "1, 2, 3")] {
        let executable = compile(&format!(
            r#"
TYPE Leaf
 INTEGER Values{rank}
ENDTYPE
TYPE Branch
 Leaf Leaves[]
 Leaf Child
ENDTYPE
TYPE Tree
 Branch Branches[1]
 Branch Child
ENDTYPE
Tree roots[1], root
Change(roots[1].Branches[1].Leaves[1].Values)
Change(root.Child.Child.Values)
Change((roots[1].Branches[1].Leaves[1].Values))
Change(roots(1).Branches(1).Leaves(1).Values)
PROCEDURE Change(VAR INTEGER values{rank})
 values.Redim({bounds})
 values[{bounds}] = 17
ENDPROC
"#
        ));
        assert_var_storage_paths(&executable, 4);
    }
}

#[test]
fn s1_var_nested_records_scalar_fields_and_indexed_elements_are_storage_paths() {
    let executable = compile(
        r#"
TYPE Leaf
 INTEGER Values[]
 INTEGER Matrix[,]
 INTEGER Cube[,,]
 INTEGER Number
ENDTYPE
TYPE Branch
 Leaf Leaves[]
 Leaf Child
ENDTYPE
TYPE Tree
 Branch Branches[1]
ENDTYPE
Tree roots[1]
ReplaceLeaf(roots[1].Branches[1].Leaves[1])
ReplaceLeaf(roots[1].Branches[1].Child)
Increment(roots[1].Branches[1].Leaves[1].Values[2])
Increment(roots[1].Branches[1].Leaves[1].Matrix[1, 2])
Increment(roots[1].Branches[1].Leaves[1].Cube[1, 2, 3])
Increment(roots[1].Branches[1].Leaves[1].Number)
ReplaceBranch(roots[1].Branches[1])
ReplaceLeaf(roots(1).Branches(1).Leaves(1))
Increment(roots(1).Branches(1).Leaves(1).Values(2))
PROCEDURE ReplaceLeaf(VAR Leaf item)
 INTEGER numbers[] = { 4, 5, 6 }
 item = Leaf { Values = numbers }
ENDPROC
PROCEDURE Increment(VAR INTEGER number)
 number += 1
ENDPROC
PROCEDURE ReplaceBranch(VAR Branch item)
 Branch replacement
 item = replacement
ENDPROC
"#,
    );
    assert_var_storage_paths(&executable, 9);
}

#[test]
fn s1_var_nested_record_fields_retain_rank_and_nominal_type_checks() {
    let header = "TYPE Leaf\n INTEGER Values[,]\n STRING Words[,]\nENDTYPE\nTYPE Branch\n Leaf Leaves[]\nENDTYPE\nBranch roots[1]\n";
    for (argument, parameter) in [
        ("roots[1].Leaves[1].Values", "INTEGER values[]"),
        ("roots[1].Leaves[1].Words", "INTEGER values[,]"),
    ] {
        rejects(&format!("{header}Change({argument})\nPROCEDURE Change(VAR {parameter})\nENDPROC\n"), |error| {
            matches!(error, CompilationErrorType::RecordArrayShapeMismatch(..))
        });
    }
    rejects(
        &format!("{header}Change(roots[1].Leaves[1].Values)\nPROCEDURE Change(VAR INTEGER number)\nENDPROC\n"),
        |error| matches!(error, CompilationErrorType::WholeArrayUsedAsScalar),
    );
    for (argument, parameter) in [("roots[1].Leaves[1]", "Branch item"), ("roots[1].Leaves[1].Values[0, 0]", "Leaf item")] {
        rejects(&format!("{header}Change({argument})\nPROCEDURE Change(VAR {parameter})\nENDPROC\n"), |error| {
            matches!(error, CompilationErrorType::ArgumentTypeMismatch(1, ..))
        });
    }
}

#[test]
fn s1_var_rejects_host_readonly_and_temporary_record_paths() {
    let header = "TYPE Leaf\n INTEGER Values[]\n INTEGER Number\n USER Owner\nENDTYPE\nTYPE Branch\n Leaf Leaves[]\nENDTYPE\nBranch roots[1]\n";
    for (argument, parameter) in [
        ("roots[1].Leaves[1].Owner.Name", "STRING value"),
        ("roots[1].Leaves[1].Owner.Notes", "STRING values[]"),
        ("roots[1].Leaves[1].Owner.Notes[0]", "STRING value"),
        ("roots[1].Leaves[1].Owner.Contacts[0]", "CONTACT value"),
        ("roots[1].Leaves[1].Owner.Contacts[0].Account", "STRING value"),
        ("(roots[1].Leaves[1].Owner.Contacts)[0].Account", "STRING value"),
        ("roots(1).Leaves(1).Owner.Name", "STRING value"),
        ("Make().Leaves[1]", "Leaf value"),
        ("Make().Leaves[1].Values", "INTEGER values[]"),
        ("Make().Leaves[1].Values[0]", "INTEGER value"),
        ("Make().Leaves[1].Number", "INTEGER value"),
        ("Make().Leaves(1).Number", "INTEGER value"),
        ("(Leaf { Number = 1 }).Number", "INTEGER value"),
        ("roots[1].Leaves[1].Number + 1", "INTEGER value"),
        ("roots[1].Leaves[1].Values.Len()", "INTEGER value"),
        ("roots[1].Leaves[1].Values.At(0)", "INTEGER value"),
    ] {
        rejects(
            &format!("{header}Change({argument})\nPROCEDURE Change(VAR {parameter})\nENDPROC\nFUNCTION Make() Branch\nRETURN roots[1]\nENDFUNC\n"),
            |error| matches!(error, CompilationErrorType::VariableExpected(1)),
        );
    }
    for source in [
        "STRING text\nChange(text[0])\nPROCEDURE Change(VAR STRING value)\nENDPROC\n",
        "CONST INTEGER number = 1\nChange(number)\nPROCEDURE Change(VAR INTEGER value)\nENDPROC\n",
    ] {
        rejects(source, |error| matches!(error, CompilationErrorType::VariableExpected(1)));
    }
}

#[test]
fn s1_var_lowering_preserves_index_calls_and_by_value_arguments() {
    use crate::executable::{FuncOpCode, PPECommand, PPEExpr, PPEScript};

    let argument = "roots[Outer()].Branches[Middle()].Leaves[Inner()].Values[Index()]";
    let executable = compile(&format!(
        r#"
TYPE Leaf
 INTEGER Values[]
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
TYPE Tree
 Branch Branches[1]
ENDTYPE
Tree roots[1]
Change({argument}, {argument})
PROCEDURE Change(INTEGER supplied, VAR INTEGER target)
 target += supplied
ENDPROC
FUNCTION Outer() INTEGER
 PRINT "O"
 RETURN 1
ENDFUNC
FUNCTION Middle() INTEGER
 PRINT "M"
 RETURN 1
ENDFUNC
FUNCTION Inner() INTEGER
 PRINT "I"
 RETURN 1
ENDFUNC
FUNCTION Index() INTEGER
 PRINT "X"
 RETURN 1
ENDFUNC
"#
    ));
    assert_var_storage_paths(&executable, 1);
    let script = PPEScript::from_ppe_file(&executable).unwrap();
    let call_index = script
        .statements
        .iter()
        .position(|statement| matches!(statement.command, PPECommand::ProcedureCall(..)))
        .unwrap();
    assert!(
        !script.statements[..call_index]
            .iter()
            .any(|statement| matches!(statement.command, PPECommand::Let(..)))
    );
    let PPECommand::ProcedureCall(_, arguments) = &script.statements[call_index].command else {
        panic!()
    };
    assert_eq!(arguments.len(), 2);
    assert!(matches!(&arguments[0], PPEExpr::PredefinedFunctionCall(def, _) if def.opcode == FuncOpCode::ArrayValueAt));
    let mut target = &arguments[1];
    for name in ["Index", "Inner", "Middle", "Outer"] {
        let indices = match target {
            PPEExpr::IndexedMember(base, field, indices) => {
                assert_eq!(*field, 0);
                target = base;
                indices
            }
            PPEExpr::Dim(_, indices) if name == "Outer" => indices,
            other => panic!("expected {name} in storage path, got {other:?}"),
        };
        let [PPEExpr::FunctionCall(id, parameters)] = indices.as_slice() else {
            panic!("index must retain its original call: {indices:?}")
        };
        assert!(parameters.is_empty());
        assert!(executable.variable_table.get_var_entry(*id).name.eq_ignore_ascii_case(name));
    }
}

#[test]
fn record_redim_lowers_to_existing_instructions_and_captures_indices() {
    use crate::executable::{FuncOpCode, PPECommand, PPEExpr, VARIABLE_FLAG_DYNAMIC_ARRAY};

    let executable = compile(
        "TYPE Item\n INTEGER Values[,]\nENDTYPE\nItem items[2]\nitems[Index()].Values.Redim(3,4)\nPRINTLN items[0].Values.Len()\nFUNCTION Index() INTEGER\nPRINTLN 1\nRETURN 0\nENDFUNC\n",
    );
    let (script, resize) = assert_record_redim_storage_path(&executable, 2);
    let PPECommand::PredefinedCall(_, arguments) = &script.statements[resize].command else {
        panic!()
    };
    let PPEExpr::Value(temporary) = arguments[0] else {
        panic!("REDIM must retain its bare-ID encoding")
    };
    let header = &executable.variable_table.get_var_entry(temporary).header;
    assert_eq!(header.dim, 2);
    assert_ne!(header.flags & VARIABLE_FLAG_DYNAMIC_ARRAY, 0);
    let PPECommand::Let(captured_array, read_target) = &script.statements[resize - 1].command else {
        panic!()
    };
    let PPECommand::Let(write_target, result) = &script.statements[resize + 1].command else {
        panic!()
    };
    assert_eq!(captured_array.as_ref(), &PPEExpr::Value(temporary));
    assert_eq!(result.as_ref(), &PPEExpr::Value(temporary));
    assert_eq!(read_target, write_target);
    let PPEExpr::Member(receiver, _) = read_target.as_ref() else { panic!() };
    let PPEExpr::Dim(_, indices) = receiver.as_ref() else {
        panic!("indexed record storage")
    };
    let PPEExpr::Value(index) = indices[0] else {
        panic!("index must be evaluated once")
    };
    assert!(script.statements[..resize - 1].iter().any(|statement| matches!(
        &statement.command,
        PPECommand::Let(target, value)
            if target.as_ref() == &PPEExpr::Value(index)
                && matches!(value.as_ref(), PPEExpr::PredefinedFunctionCall(def, args)
                    if def.opcode == FuncOpCode::TOINTEGER && matches!(args.as_slice(), [PPEExpr::FunctionCall(_, _)]))
    )));
}

#[test]
fn s1_redim_parenthesized_roots_decode_as_storage_paths() {
    use crate::executable::{FuncOpCode, PPECommand, PPEExpr};

    for (root_shape, indices, opcode, root_rank) in [
        ("[0]", "0", FuncOpCode::ArrayValueAt, 1),
        ("[0,0]", "0,0", FuncOpCode::ArrayValueAt2, 2),
        ("[0,0,0]", "0,0,0", FuncOpCode::ArrayValueAt3, 3),
    ] {
        for (field_shape, bounds, field_rank) in [("[]", "1", 1), ("[,]", "1,2", 2), ("[,,]", "1,2,3", 3)] {
            for (root, getter) in [
                (format!("roots[{indices}]"), false),
                (format!("roots({indices})"), false),
                (format!("(roots)[{indices}]"), true),
                (format!("(((roots)))[{indices}]"), true),
                (format!("((roots[{indices}]))"), false),
            ] {
                let executable = compile(&format!(
                    "TYPE Item\n INTEGER Values{field_shape}\nENDTYPE\nItem roots{root_shape}\nREDIM ({root}.Values), {bounds}\n"
                ));
                let (script, resize) = assert_record_redim_storage_path(&executable, field_rank);
                let PPECommand::Let(target, _) = &script.statements[resize + 1].command else {
                    panic!()
                };
                let PPEExpr::Member(base, 0) = target.as_ref() else { panic!("{target:?}") };
                let PPEExpr::Dim(id, subscripts) = base.as_ref() else { panic!("{base:?}") };
                assert!(executable.variable_table.get_var_entry(*id).name.eq_ignore_ascii_case("roots"));
                assert_eq!(subscripts.len(), root_rank);
                let PPECommand::Let(_, read) = &script.statements[resize - 1].command else {
                    panic!()
                };
                let PPEExpr::Member(read_base, 0) = read.as_ref() else { panic!("{read:?}") };
                if getter {
                    let PPEExpr::PredefinedFunctionCall(def, arguments) = read_base.as_ref() else {
                        panic!("{read_base:?}")
                    };
                    assert_eq!(def.opcode, opcode);
                    assert_eq!(arguments[0], PPEExpr::Value(*id));
                    assert_eq!(&arguments[1..], subscripts);
                } else {
                    assert_eq!(read, target);
                }
            }
        }
    }
}

#[test]
fn s1_redim_parenthesized_nested_fields_decode_as_storage_paths() {
    use crate::executable::{FuncOpCode, PPECommand, PPEExpr};

    for (shape, indices, rank, opcode) in [
        ("[]", "0", 1, FuncOpCode::ArrayValueAt),
        ("[,]", "0,0", 2, FuncOpCode::ArrayValueAt2),
        ("[,,]", "0,0,0", 3, FuncOpCode::ArrayValueAt3),
    ] {
        let root = format!("(((roots)))[{indices}]");
        let branch = format!("(({root}.Branches))[{indices}]");
        let target = format!("(({branch}.Child.Leaves))[{indices}].Values");
        let executable = compile(&format!(
            "TYPE Leaf\n INTEGER Tag\n INTEGER Values{shape}\nENDTYPE\n\
             TYPE Branch\n INTEGER Tag\n Leaf Leaves{shape}\nENDTYPE\n\
             TYPE Container\n INTEGER Tag\n Branch Child\nENDTYPE\n\
             TYPE Tree\n INTEGER Tag\n Container Branches[{indices}]\nENDTYPE\n\
             Tree roots[{indices}]\nREDIM ({target}), {indices}\n"
        ));
        let (script, resize) = assert_record_redim_storage_path(&executable, rank);
        let PPECommand::Let(target, _) = &script.statements[resize + 1].command else {
            panic!()
        };
        let PPEExpr::Member(base, 1) = target.as_ref() else { panic!("{target:?}") };
        let PPEExpr::IndexedMember(base, 1, leaves) = base.as_ref() else {
            panic!("{base:?}")
        };
        assert_eq!(leaves.len(), rank as usize);
        let PPEExpr::Member(base, 1) = base.as_ref() else { panic!("{base:?}") };
        let PPEExpr::IndexedMember(base, 1, branches) = base.as_ref() else {
            panic!("{base:?}")
        };
        assert_eq!(branches.len(), rank as usize);
        let PPEExpr::Dim(id, roots) = base.as_ref() else { panic!("{base:?}") };
        assert!(executable.variable_table.get_var_entry(*id).name.eq_ignore_ascii_case("roots"));
        assert_eq!(roots.len(), rank as usize);

        let PPECommand::Let(_, read) = &script.statements[resize - 1].command else {
            panic!()
        };
        let PPEExpr::Member(base, 1) = read.as_ref() else { panic!("{read:?}") };
        let mut base = base.as_ref();
        for depth in 0..3 {
            let PPEExpr::PredefinedFunctionCall(def, arguments) = base else {
                panic!("{base:?}")
            };
            assert_eq!(def.opcode, opcode);
            assert_eq!(arguments.len(), rank as usize + 1);
            if depth == 2 {
                assert_eq!(arguments[0], PPEExpr::Value(*id));
                break;
            }
            let PPEExpr::Member(receiver, 1) = &arguments[0] else {
                panic!("{:?}", arguments[0])
            };
            base = receiver.as_ref();
            if depth == 0 {
                let PPEExpr::Member(receiver, 1) = base else { panic!("{base:?}") };
                base = receiver.as_ref();
            }
        }
    }
}

#[test]
fn s1_redim_parenthesized_paths_capture_indices_once_left_to_right() {
    use crate::executable::{FuncOpCode, PPECommand, PPEExpr};

    let mut source = "TYPE Leaf\n INTEGER Values[,,]\nENDTYPE\nTYPE Tree\n Leaf Leaves[,,]\nENDTYPE\n\
        Tree roots[0,0,0]\nINTEGER cursor\n\
        REDIM ((((roots)))[RootX(), cursor, RootZ()].Leaves)[LeafX(), cursor, LeafZ()].Values, BoundX(), BoundY(), BoundZ()\n"
        .to_string();
    for name in ["RootX", "RootZ", "LeafX", "LeafZ", "BoundX", "BoundY", "BoundZ"] {
        source.push_str(&format!("FUNCTION {name}() INTEGER\ncursor += 1\nRETURN 0\nENDFUNC\n"));
    }
    let executable = compile(&source);
    let (script, resize) = assert_record_redim_storage_path(&executable, 3);
    assert_eq!(resize, 7, "six index captures, then one array copy");
    let mut captured = Vec::new();
    for (statement, name) in script.statements[..6]
        .iter()
        .zip([Some("RootX"), None, Some("RootZ"), Some("LeafX"), None, Some("LeafZ")])
    {
        let PPECommand::Let(target, value) = &statement.command else { panic!() };
        let PPEExpr::Value(id) = target.as_ref() else { panic!("{target:?}") };
        assert!(!captured.contains(&PPEExpr::Value(*id)), "each index has its own snapshot");
        captured.push(PPEExpr::Value(*id));
        let PPEExpr::PredefinedFunctionCall(def, arguments) = value.as_ref() else {
            panic!("{value:?}")
        };
        assert_eq!(def.opcode, FuncOpCode::TOINTEGER);
        assert_eq!(arguments.len(), 1);
        if let Some(name) = name {
            let PPEExpr::FunctionCall(id, arguments) = &arguments[0] else {
                panic!("{:?}", arguments[0])
            };
            assert!(arguments.is_empty());
            assert!(executable.variable_table.get_var_entry(*id).name.eq_ignore_ascii_case(name));
        } else {
            let PPEExpr::Value(id) = arguments[0] else { panic!("{:?}", arguments[0]) };
            assert!(executable.variable_table.get_var_entry(id).name.eq_ignore_ascii_case("cursor"));
        }
    }
    let PPECommand::Let(target, _) = &script.statements[resize + 1].command else {
        panic!()
    };
    let PPEExpr::Member(base, 0) = target.as_ref() else { panic!("{target:?}") };
    let PPEExpr::IndexedMember(base, 0, indices) = base.as_ref() else {
        panic!("{base:?}")
    };
    assert_eq!(indices, &captured[3..]);
    let PPEExpr::Dim(_, indices) = base.as_ref() else { panic!("{base:?}") };
    assert_eq!(indices, &captured[..3]);
    let PPECommand::Let(_, read) = &script.statements[resize - 1].command else {
        panic!()
    };
    let PPEExpr::Member(base, 0) = read.as_ref() else { panic!("{read:?}") };
    let PPEExpr::PredefinedFunctionCall(def, arguments) = base.as_ref() else {
        panic!("{base:?}")
    };
    assert_eq!(def.opcode, FuncOpCode::ArrayValueAt3);
    assert_eq!(&arguments[1..], &captured[3..]);
    let PPEExpr::Member(base, 0) = &arguments[0] else {
        panic!("{:?}", arguments[0])
    };
    let PPEExpr::PredefinedFunctionCall(def, arguments) = base.as_ref() else {
        panic!("{base:?}")
    };
    assert_eq!(def.opcode, FuncOpCode::ArrayValueAt3);
    assert_eq!(&arguments[1..], &captured[..3]);
    let PPECommand::PredefinedCall(_, arguments) = &script.statements[resize].command else {
        panic!()
    };
    for (argument, name) in arguments[1..].iter().zip(["BoundX", "BoundY", "BoundZ"]) {
        let PPEExpr::FunctionCall(id, parameters) = argument else {
            panic!("{argument:?}")
        };
        assert!(parameters.is_empty());
        assert!(executable.variable_table.get_var_entry(*id).name.eq_ignore_ascii_case(name));
    }
}

#[test]
fn s1_redim_parentheses_do_not_make_computed_values_writable() {
    for (shape, bounds) in [("[]", "1"), ("[,]", "1,2"), ("[,,]", "1,2,3")] {
        for target in [
            "((Make())).Values",
            "(MakeRoots())[0].Values",
            "(((roots))).At(0).Values",
            "(((Item { Values = values }))).Values",
        ] {
            rejects(
                &format!(
                    "TYPE Item\n INTEGER Values{shape}\nENDTYPE\nItem roots[0]\nINTEGER values{shape}\n\
                     REDIM {target}, {bounds}\n\
                     FUNCTION Make() Item\nRETURN roots[0]\nENDFUNC\n\
                     FUNCTION MakeRoots() Item[]\nRETURN roots\nENDFUNC\n"
                ),
                |error| matches!(error, CompilationErrorType::RedimArrayVariableExpected),
            );
        }
    }
    for target in ["(Session.User.Notes)", "(Board.Users)", "(String.Split(\"a,b\", \",\"))", "({1, 2})"] {
        rejects(&format!("REDIM {target}, 1\n"), |error| {
            matches!(error, CompilationErrorType::RedimArrayVariableExpected)
        });
    }
}

#[test]
fn dynamic_shapes_reject_wrong_rank_type_scalar_and_temporary_redim() {
    let header = "TYPE Item\n INTEGER Values[,]\nENDTYPE\nItem item\n";
    for body in [
        "INTEGER vector[2]\nitem.Values = vector\n",
        "INTEGER vector[2]\nitem = Item { Values = vector }\n",
        "STRING matrix[1,2]\nitem.Values = matrix\n",
    ] {
        rejects(&format!("{header}{body}"), |error| {
            matches!(error, CompilationErrorType::RecordArrayShapeMismatch(..))
        });
    }
    rejects(&format!("{header}item.Values = 1\n"), |error| {
        matches!(error, CompilationErrorType::RecordArrayValueExpected(..))
    });
    rejects(&format!("{header}item.Values.Redim(2)\n"), |error| {
        matches!(error, CompilationErrorType::RedimRankMismatch(2, 1))
    });
    let (executable, errors) = build(&format!("{header}PRINTLN item.Values[0]\n"), false);
    assert!(executable.is_none());
    assert!(errors.lock().unwrap().errors.iter().any(|error| matches!(
        error.error.downcast_ref::<crate::parser::ParserErrorType>(),
        Some(crate::parser::ParserErrorType::TooFewArguments(_, 1, 2))
    )));
    rejects(
        &format!("{header}Make().Values.Redim(2, 3)\nFUNCTION Make() Item\nRETURN item\nENDFUNC\n"),
        |error| matches!(error, CompilationErrorType::RedimArrayVariableExpected),
    );
}

#[test]
fn fixed_field_shapes_and_legacy_encoding_are_retained() {
    let source = "TYPE Item\n INTEGER Values[0]\n INTEGER Matrix[1,2]\n INTEGER Cube[1,2,3]\nENDTYPE\nItem item\nINTEGER values[0]\nitem.Values = values\nPRINTLN item.Values[0]\n";
    let executable = compile(source);
    assert_eq!(
        executable.user_types[0].iter().map(|field| field.element_count()).collect::<Vec<_>>(),
        [Some(1), Some(6), Some(24)]
    );
    assert!(executable.user_types[0].iter().all(|field| !field.is_dynamic));
    let bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes.clone(), false).unwrap();
    assert_eq!(executable.user_types, loaded.user_types);
    assert_eq!(bytes, loaded.to_buffer().unwrap());
    assert!(loaded.variable_table.record_io_unsupported_types.is_empty());
    rejects("TYPE Item\n INTEGER Values[2]\nENDTYPE\nItem item\nitem.Values.Redim(2)\n", |error| {
        matches!(error, CompilationErrorType::FixedRecordArrayCannotBeRedimmed(..))
    });
    rejects(
        "TYPE Item\n INTEGER Values[2]\nENDTYPE\nItem item\nINTEGER other[3]\nitem.Values = other\n",
        |error| matches!(error, CompilationErrorType::RecordArrayShapeMismatch(..)),
    );
}

#[test]
fn equality_is_transitive_and_only_resource_host_types_opt_in() {
    compile(
        "ENUM Choice\n First = 7\nENDENUM\nTYPE Leaf\n CONTACT Person\n SURFACE Image\n AUDIO Sounds[]\n Choice States[,]\n INTEGER Values[,,]\nENDTYPE\nTYPE Outer\n Leaf Items[]\nENDTYPE\nOuter left, right\nPRINTLN left = right, left <> right\n",
    );
    for &(id, host, _) in board_catalog::TYPES {
        if matches!(id, CONTACT_ID | crate::parser::SURFACE_ID | crate::parser::AUDIO_ID) {
            compile(&format!("{host} left, right\nPRINTLN left = right\n"));
            continue;
        }
        rejects(&format!("{host} left, right\nPRINTLN left = right\n"), |error| {
            matches!(error, CompilationErrorType::TypeNotComparable(..))
        });
        for shape in ["", "[0]", "[]", "[,]", "[,,]"] {
            rejects(
                &format!("TYPE Leaf\n {host} Value{shape}\nENDTYPE\nTYPE Outer\n Leaf Values[]\nENDTYPE\nOuter left, right\nPRINTLN left <> right\n"),
                |error| matches!(error, CompilationErrorType::TypeNotComparable(..)),
            );
        }
    }
}

#[test]
fn record_io_rejects_dynamic_and_host_fields_before_execution() {
    for statement in ["FGETREC", "FPUTREC", "FREADREC", "FWRITEREC"] {
        for field in [
            "INTEGER Values[]",
            "INTEGER Values[,]",
            "INTEGER Values[,,]",
            "SURFACE Value",
            "CONTACT Value",
            "HTTPREQUEST Value",
            "USER Values[0]",
        ] {
            rejects(
                &format!("TYPE Leaf\n {field}\nENDTYPE\nTYPE Outer\n INTEGER Prefix\n Leaf Child\nENDTYPE\nOuter item\n{statement} 1, item\n"),
                |error| matches!(error, CompilationErrorType::RecordIoFieldNotSerializable(path, _) if path.starts_with("Child.")),
            );
        }
    }
    let executable = compile(
        "ENUM Choice\n First = 7\nENDENUM\nTYPE Leaf\n Choice Kind\n INTEGER Values[2]\nENDTYPE\nTYPE Outer\n Leaf Child\nENDTYPE\nOuter item\nFPUTREC 1, item\n",
    );
    assert!(executable.variable_table.record_io_unsupported_types.is_empty());
    let mut bytes = executable.to_buffer().unwrap();
    assert!(
        Executable::from_buffer(&mut bytes, false)
            .unwrap()
            .variable_table
            .record_io_unsupported_types
            .is_empty()
    );
}

#[test]
fn new_layouts_are_refused_explicitly_even_without_variables() {
    for field in [
        RecordField {
            dim: 1,
            is_dynamic: true,
            ..RecordField::scalar(VariableType::Integer)
        },
        RecordField::scalar(VariableType::UserData(CONTACT_ID as u8)),
        RecordField::scalar(VariableType::UserData(crate::parser::AUDIO_ID as u8)),
    ] {
        let executable = Executable {
            runtime: 400,
            user_types: vec![vec![field]],
            ..Default::default()
        };
        let error = executable.to_buffer().unwrap_err();
        assert_eq!(error, ExecutableError::UnsupportedRecordFieldEncoding { type_id: 100, field_index: 0 });
        assert!(error.to_string().contains("no executable encoding"));
    }
}

#[test]
fn old_loader_still_rejects_host_field_bytes_and_legacy_has_no_type_section() {
    let executable = Executable {
        runtime: 400,
        user_types: vec![vec![RecordField::scalar(VariableType::Integer)]],
        ..Default::default()
    };
    let bytes = executable.to_buffer().unwrap();
    assert_eq!(&bytes[50..61], &[1, 1, 1, 4, 0, 0, 0, 0, 0, 0, 0]);
    for &(id, _, _) in board_catalog::TYPES {
        let mut changed = bytes.clone();
        changed[53] = id as u8;
        let error = Executable::from_buffer(&mut changed, false)
            .err()
            .expect("old encoding cannot contain host fields");
        assert!(matches!(
            error.downcast_ref::<ExecutableError>(),
            Some(ExecutableError::BoardObjectTypeField(100, _))
        ));
    }
    for runtime in [100, 200, 300, 310, 320, 330, 340] {
        let executable = Executable { runtime, ..Default::default() };
        let bytes = executable.to_buffer().unwrap();
        assert_eq!(bytes.len(), 52);
        let loaded = Executable::from_buffer(&mut bytes.clone(), false).unwrap();
        assert_eq!(loaded.runtime, runtime);
        assert!(loaded.user_types.is_empty());
        assert_eq!(loaded.to_buffer().unwrap(), bytes);
    }
}
