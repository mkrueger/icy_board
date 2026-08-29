#[test]
fn bytes_function_ids_are_compact() {
    assert_eq!(FuncOpCode::ToBytes as i16, -341);
    assert_eq!(FuncOpCode::FromBytes as i16, -342);
    assert_eq!(FuncOpCode::BytesToHex as i16, -343);
    assert_eq!(FuncOpCode::BytesGetChecksum as i16, -344);

    assert_eq!(FuncOpCode::BytesToHex.get_definition().parameter_count(), 1);
    assert_eq!(FuncOpCode::BytesGetChecksum.get_definition().parameter_count(), 2);
}

#[test]
fn string_member_function_ids_are_compact() {
    assert_eq!(FuncOpCode::StringFindFrom as i16, -314);
    assert_eq!(FuncOpCode::StringFindLastFrom as i16, -315);
    assert_eq!(FuncOpCode::StringContains as i16, -316);
    assert_eq!(FuncOpCode::StringStartsWith as i16, -317);
    assert_eq!(FuncOpCode::StringEndsWith as i16, -318);
    assert_eq!(FuncOpCode::StringCount as i16, -319);
    assert_eq!(FuncOpCode::StringTrim as i16, -320);
    assert_eq!(FuncOpCode::StringTrimStart as i16, -321);
    assert_eq!(FuncOpCode::StringTrimEnd as i16, -322);
    assert_eq!(FuncOpCode::StringJoin as i16, -323);
    assert_eq!(FuncOpCode::StringRepeat as i16, -324);
    assert_eq!(FuncOpCode::StringTrimChars as i16, -325);
    assert_eq!(FuncOpCode::StringTrimStartChars as i16, -326);
    assert_eq!(FuncOpCode::StringTrimEndChars as i16, -327);
    assert_eq!(FuncOpCode::StringCharAt as i16, -328);
    assert_eq!(FuncOpCode::StringFindComparison as i16, -329);
    assert_eq!(FuncOpCode::StringFindLastComparison as i16, -330);
    assert_eq!(FuncOpCode::StringContainsComparison as i16, -331);
    assert_eq!(FuncOpCode::StringStartsWithComparison as i16, -332);
    assert_eq!(FuncOpCode::StringEndsWithComparison as i16, -333);
    assert_eq!(FuncOpCode::StringCountComparison as i16, -334);
    assert_eq!(FuncOpCode::StringEquals as i16, -335);
    assert_eq!(FuncOpCode::StringEqualsComparison as i16, -336);
    assert_eq!(FuncOpCode::StringSplit as i16, -337);
    assert_eq!(FuncOpCode::StringSplitLimit as i16, -338);
    assert_eq!(FuncOpCode::ArrayValueAt as i16, -339);

    for (opcode, arity) in [
        (FuncOpCode::StringFindFrom, 3),
        (FuncOpCode::StringFindLastFrom, 3),
        (FuncOpCode::StringContains, 2),
        (FuncOpCode::StringStartsWith, 2),
        (FuncOpCode::StringEndsWith, 2),
        (FuncOpCode::StringCount, 2),
        (FuncOpCode::StringTrim, 1),
        (FuncOpCode::StringTrimStart, 1),
        (FuncOpCode::StringTrimEnd, 1),
        (FuncOpCode::StringJoin, 2),
        (FuncOpCode::StringRepeat, 2),
        (FuncOpCode::StringTrimChars, 2),
        (FuncOpCode::StringTrimStartChars, 2),
        (FuncOpCode::StringTrimEndChars, 2),
        (FuncOpCode::StringCharAt, 2),
        (FuncOpCode::StringFindComparison, 4),
        (FuncOpCode::StringFindLastComparison, 4),
        (FuncOpCode::StringContainsComparison, 3),
        (FuncOpCode::StringStartsWithComparison, 3),
        (FuncOpCode::StringEndsWithComparison, 3),
        (FuncOpCode::StringCountComparison, 3),
        (FuncOpCode::StringEquals, 2),
        (FuncOpCode::StringEqualsComparison, 3),
        (FuncOpCode::StringSplit, 2),
        (FuncOpCode::StringSplitLimit, 3),
        (FuncOpCode::ArrayValueAt, 2),
    ] {
        let arguments: Vec<_> = (1..=arity).map(PPEExpr::Value).collect();
        let mut expected: Vec<_> = (1..=arity).flat_map(|id| [id as i16, 0]).collect();
        expected.push(opcode as i16);
        test_serialize(&PPEExpr::PredefinedFunctionCall(opcode.get_definition(), arguments), &expected);
    }
}
use crate::executable::{EntryType, FunctionValue, VariableType, VariableValue};

use super::{DeserializationErrorType, Executable, FUNCTION_DEFINITIONS, FuncOpCode, PPEExpr, TableEntry};

#[test]
fn test_value_serialization() {
    let val = PPEExpr::Value(2);
    test_serialize(&val, &[2, 0]);
}

#[test]
fn test_dim_serialization() {
    let val = PPEExpr::Dim(2, vec![PPEExpr::Value(2)]);
    test_serialize(&val, &[2, 1, 2, 0, 0]);

    let val = PPEExpr::Dim(2, vec![PPEExpr::Value(2), PPEExpr::Value(3)]);
    test_serialize(&val, &[2, 2, 2, 0, 0, 3, 0, 0]);

    let val = PPEExpr::Dim(2, vec![PPEExpr::Value(2), PPEExpr::Value(3), PPEExpr::Value(4)]);
    test_serialize(&val, &[2, 3, 2, 0, 0, 3, 0, 0, 4, 0, 0]);
}

#[test]
fn test_predefined_functions_serialization() {
    let i = -(FuncOpCode::RIGHT as i32);
    let val = PPEExpr::PredefinedFunctionCall(&FUNCTION_DEFINITIONS[i as usize], vec![PPEExpr::Value(2), PPEExpr::Value(3)]);
    test_serialize(&val, &[2, 0, 3, 0, FuncOpCode::RIGHT as i16]);

    let i = -(FuncOpCode::MID as i32);
    let val = PPEExpr::PredefinedFunctionCall(&FUNCTION_DEFINITIONS[i as usize], vec![PPEExpr::Value(2), PPEExpr::Value(3), PPEExpr::Value(4)]);
    test_serialize(&val, &[2, 0, 3, 0, 4, 0, FuncOpCode::MID as i16]);
}

#[test]
fn test_predefined_functions_without_argument() {
    let i = -(FuncOpCode::HICONFNUM as i32);
    let val = PPEExpr::PredefinedFunctionCall(&FUNCTION_DEFINITIONS[i as usize], vec![]);
    test_serialize(&val, &[FuncOpCode::HICONFNUM as i16]);
}

#[test]
fn test_binary_expression_serialization() {
    let val = PPEExpr::BinaryExpression(crate::ast::BinOp::Add, Box::new(PPEExpr::Value(2)), Box::new(PPEExpr::Value(3)));
    test_serialize(&val, &[2, 0, 3, 0, FuncOpCode::PLUS as i16]);
}

#[test]
fn test_unary_expression_serialization() {
    let val = PPEExpr::UnaryExpression(crate::ast::UnaryOp::Minus, Box::new(PPEExpr::Value(2)));
    test_serialize(&val, &[2, 0, FuncOpCode::UMINUS as i16]);
}

#[test]
fn test_function_call_serialization() {
    let val = PPEExpr::FunctionCall(6, vec![]);
    test_serialize(&val, &[6, 0]);
    let val = PPEExpr::FunctionCall(7, vec![PPEExpr::Value(5)]);
    test_serialize(&val, &[7, 0, 5, 0, 0]);

    let val = PPEExpr::FunctionCall(8, vec![PPEExpr::Value(2), PPEExpr::Value(3)]);
    test_serialize(&val, &[8, 0, 2, 0, 0, 3, 0, 0]);
}

#[test]
fn test_member_reference_serialization() {
    let val = PPEExpr::Member(Box::new(PPEExpr::Value(2)), 32);
    test_serialize(&val, &[2, 0, FuncOpCode::MemberReference as i16, 32]);
}

#[test]
fn test_indexed_member_serialization() {
    let val = PPEExpr::IndexedMember(Box::new(PPEExpr::Value(2)), 3, vec![PPEExpr::Value(4), PPEExpr::Value(5)]);
    test_serialize(&val, &[2, 0, FuncOpCode::IndexedMember as i16, 3, 2, 4, 0, 0, 5, 0, 0]);
}

#[test]
fn malformed_indexed_member_dimensions_are_rejected() {
    for (dimension_count, expected) in [
        (0, DeserializationErrorType::InvalidIndexedMemberDimensionCount(0)),
        (4, DeserializationErrorType::InvalidIndexedMemberDimensionCount(4)),
    ] {
        let mut executable = malformed_expression_executable();
        executable.script_buffer = vec![1, 0, FuncOpCode::IndexedMember as i16, 3, dimension_count];
        let mut deserializer = super::PPEDeserializer::default();
        let error = deserializer.deserialize_expression(&executable).unwrap_err();
        assert_eq!(expected, error);
    }
}

#[test]
fn truncated_indexed_member_operands_are_rejected() {
    let mut executable = malformed_expression_executable();
    executable.script_buffer = vec![1, 0, FuncOpCode::IndexedMember as i16];
    let error = super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err();
    assert_eq!(DeserializationErrorType::IndexOutOfBounds, error);
}

#[test]
fn malformed_member_reference_is_rejected() {
    let missing_base = Executable {
        script_buffer: vec![FuncOpCode::MemberReference as i16, 1],
        ..Executable::default()
    };
    assert_eq!(
        DeserializationErrorType::ExpressionStackEmpty,
        super::PPEDeserializer::default().deserialize_expression(&missing_base).unwrap_err()
    );

    let mut missing_id = malformed_expression_executable();
    missing_id.script_buffer = vec![1, 0, FuncOpCode::MemberReference as i16];
    assert_eq!(
        DeserializationErrorType::IndexOutOfBounds,
        super::PPEDeserializer::default().deserialize_expression(&missing_id).unwrap_err()
    );
}

#[test]
fn malformed_record_literal_is_rejected() {
    for script in [
        vec![FuncOpCode::RecordLiteral as i16],
        vec![FuncOpCode::RecordLiteral as i16, 100],
        vec![FuncOpCode::RecordLiteral as i16, 100, 1],
    ] {
        let executable = Executable {
            script_buffer: script,
            ..Executable::default()
        };
        assert_eq!(
            DeserializationErrorType::IndexOutOfBounds,
            super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err()
        );
    }

    let executable = Executable {
        script_buffer: vec![FuncOpCode::RecordLiteral as i16, 100, 1, 0],
        ..Executable::default()
    };
    assert_eq!(
        DeserializationErrorType::ExpressionStackEmpty,
        super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err()
    );
}

#[test]
fn malformed_member_call_is_rejected() {
    for script in [vec![FuncOpCode::MemberCall as i16], vec![FuncOpCode::MemberCall as i16, 0]] {
        let executable = Executable {
            script_buffer: script,
            ..Executable::default()
        };
        assert_eq!(
            DeserializationErrorType::IndexOutOfBounds,
            super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err()
        );
    }

    let executable = Executable {
        script_buffer: vec![FuncOpCode::MemberCall as i16, 0, 1],
        ..Executable::default()
    };
    assert_eq!(
        DeserializationErrorType::TooFewFunctionArguments,
        super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err()
    );
}

#[test]
fn truncated_routine_reference_is_rejected() {
    let executable = Executable {
        script_buffer: vec![FuncOpCode::RoutineReference as i16],
        ..Executable::default()
    };
    assert_eq!(
        DeserializationErrorType::IndexOutOfBounds,
        super::PPEDeserializer::default().deserialize_expression(&executable).unwrap_err()
    );
}

fn malformed_expression_executable() -> Executable {
    let mut executable = Executable::default();
    for id in 0..2 {
        executable.variable_table.push(TableEntry {
            name: format!("value{id}"),
            value: VariableValue::new_int(0),
            header: super::VarHeader {
                id: id + 1,
                variable_type: VariableType::Integer,
                ..Default::default()
            },
            entry_type: EntryType::Variable,
            function_id: 0,
        });
    }
    executable
}

#[test]
fn test_member_call_serialization() {
    let val = PPEExpr::MemberFunctionCall(Box::new(PPEExpr::Value(2)), vec![PPEExpr::Value(1)], 32);
    test_serialize(&val, &[2, 0, 1, 0, FuncOpCode::MemberCall as i16, 1, 32]);
}

#[test]
fn test_member_call_keeps_the_argument_order() {
    let val = PPEExpr::MemberFunctionCall(Box::new(PPEExpr::Value(2)), vec![PPEExpr::Value(3), PPEExpr::Value(4)], 32);
    test_serialize(&val, &[2, 0, 3, 0, 4, 0, FuncOpCode::MemberCall as i16, 2, 32]);
}

fn test_serialize(val: &PPEExpr, expected: &[i16]) {
    assert_eq!(val.get_size(), expected.len(), "Serialization size mismatch for {val:?}");
    let mut result = Vec::new();
    val.serialize(&mut result);
    assert_eq!(result, expected, "Serialization mismatch for {val:?}");

    test_deserialization(&result, val);
}

fn test_deserialization(script: &[i16], expected: &PPEExpr) {
    let mut exe = Executable::default();
    for i in 0..5 {
        exe.variable_table.push(TableEntry {
            name: format!("int{i}"),
            value: VariableValue::new_int(i),
            header: super::VarHeader {
                id: i as usize + 1,
                variable_type: VariableType::Integer,
                ..Default::default()
            },
            entry_type: EntryType::Constant,
            function_id: 0,
        });
    }
    for id in 6..9 {
        let func = FunctionValue {
            parameters: id - 6,
            local_variables: 1,
            start_offset: 1,
            first_var_id: 5,
            return_var: 6,
        };

        exe.variable_table.push(TableEntry {
            name: format!("func{}", id - 5),
            value: VariableValue {
                vtype: VariableType::Function,
                data: func.to_data(),
                ..Default::default()
            },
            header: super::VarHeader {
                id: id as usize,
                variable_type: VariableType::Function,
                ..Default::default()
            },
            entry_type: super::EntryType::Constant,
            function_id: 0,
        });
    }

    exe.script_buffer = script.to_vec();
    let mut deserializer = super::PPEDeserializer::default();
    let expr = deserializer.deserialize_expression(&exe).unwrap().unwrap();

    assert_eq!(expr, *expected, "Deserialization mismatch for {expected:?}");
    assert_eq!(expr.get_size(), exe.script_buffer.len(), "Deserialization size mismatch for {expected:?}");

    assert_eq!(deserializer.offset, exe.script_buffer.len(), "Deserialization offset mismatch for {expected:?}");
}
