use crate::executable::{EntryType, FunctionValue, VariableType, VariableValue};

use super::{Executable, FuncOpCode, PPEExpr, TableEntry, FUNCTION_DEFINITIONS};

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

    let val = PPEExpr::Dim(
        2,
        vec![PPEExpr::Value(2), PPEExpr::Value(3), PPEExpr::Value(4)],
    );
    test_serialize(&val, &[2, 3, 2, 0, 0, 3, 0, 0, 4, 0, 0]);
}

#[test]
fn test_predefined_functions_serialization() {
    let i = -(FuncOpCode::RIGHT as i32);
    let val = PPEExpr::PredefinedFunctionCall(
        &FUNCTION_DEFINITIONS[i as usize],
        vec![PPEExpr::Value(2), PPEExpr::Value(3)],
    );
    test_serialize(&val, &[2, 0, 3, 0, FuncOpCode::RIGHT as i16]);

    let i = -(FuncOpCode::MID as i32);
    let val = PPEExpr::PredefinedFunctionCall(
        &FUNCTION_DEFINITIONS[i as usize],
        vec![PPEExpr::Value(2), PPEExpr::Value(3), PPEExpr::Value(4)],
    );
    test_serialize(&val, &[2, 0, 3, 0, 4, 0, FuncOpCode::MID as i16]);
}

#[test]
fn test_binary_expression_serialization() {
    let val = PPEExpr::BinaryExpression(
        crate::ast::BinOp::Add,
        Box::new(PPEExpr::Value(2)),
        Box::new(PPEExpr::Value(3)),
    );
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

fn test_serialize(val: &PPEExpr, expected: &[i16]) {
    assert_eq!(
        val.get_size(),
        expected.len(),
        "Serialization size mismatch for {val:?}"
    );
    let mut result = Vec::new();
    val.serialize(&mut result);
    assert_eq!(result, expected);

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

    assert_eq!(expr, *expected);
    assert_eq!(expr.get_size(), exe.script_buffer.len());

    assert_eq!(deserializer.offset, exe.script_buffer.len());
}
