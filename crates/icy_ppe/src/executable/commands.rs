use std::{collections::HashMap, fmt::Display, ops::Range};

use thiserror::Error;

use crate::ast::{BinOp, UnaryOp};

use super::{
    DeserializationErrorType, Executable, FunctionDefinition, GenericVariableData, OpCode,
    StatementDefinition, VariableValue,
};

#[derive(Debug)]
pub struct DeserializationError {
    pub error_type: DeserializationErrorType,
    pub span: Range<usize>,
}

impl Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_type)
    }
}

pub struct PPEStatement {
    pub span: Range<usize>,
    pub command: PPECommand,
}

#[derive(Default)]
pub struct PPEScript {
    pub bugged_offsets: HashMap<usize, Vec<DeserializationErrorType>>,
    pub statements: Vec<PPEStatement>,
}

pub enum CommandOrError {
    Command(PPEStatement),
    Error(DeserializationError),
}

impl PPEScript {
    pub fn serialize(&self) -> Vec<i16> {
        let mut result = Vec::new();
        for stmt in &self.statements {
            stmt.command.serialize(&mut result);
        }
        result
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn from_ppe_file(exe: &Executable) -> Result<Self, DeserializationError> {
        let mut deserializer = super::PPEDeserializer::default();
        let mut result = PPEScript::default();
        loop {
            match deserializer.deserialize_statement(exe) {
                Ok(Some(stmt)) => result.statements.push(PPEStatement {
                    span: deserializer.stmt_span(),
                    command: stmt,
                }),
                Ok(None) => break,
                Err(err) => {
                    return Err(DeserializationError {
                        error_type: err,
                        span: deserializer.stmt_span(),
                    })
                }
            }
        }
        result.bugged_offsets = deserializer.bugged_offsets;
        Ok(result)
    }

    pub fn step_through(exe: &Executable) -> Vec<CommandOrError> {
        let mut deserializer = super::PPEDeserializer::default();
        let mut result = Vec::new();
        loop {
            match deserializer.deserialize_statement(exe) {
                Ok(Some(stmt)) => result.push(CommandOrError::Command(PPEStatement {
                    span: deserializer.stmt_span(),
                    command: stmt,
                })),
                Ok(None) => break,
                Err(err) => {
                    result.push(CommandOrError::Error(DeserializationError {
                        error_type: err,
                        span: deserializer.stmt_span(),
                    }));
                    break;
                }
            }
        }
        result
    }

    pub fn visit<T: Sized, V: PPEVisitor<T>>(&self, visitor: &mut V) {
        visitor.visit_script(self);
    }

    pub(crate) fn add_statement(&mut self, cur_offset: &mut usize, command: PPECommand) {
        let size = command.get_size();
        self.statements.push(PPEStatement {
            span: *cur_offset..*cur_offset + size,
            command,
        });
        *cur_offset += size;
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PPECommand {
    End,
    Return,
    IfNot(Box<PPEExpr>, usize),
    ProcedureCall(usize, Vec<PPEExpr>),
    PredefinedCall(&'static StatementDefinition, Vec<PPEExpr>),
    Goto(usize),
    Gosub(usize),
    EndFunc,
    EndProc,
    Stop,
    Let(Box<PPEExpr>, Box<PPEExpr>),
}

impl PPECommand {
    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn serialize(&self, vec: &mut Vec<i16>) {
        match self {
            PPECommand::End => vec.push(OpCode::END as i16),
            PPECommand::Return => vec.push(OpCode::RETURN as i16),
            PPECommand::EndFunc => vec.push(OpCode::FEND as i16),
            PPECommand::EndProc => vec.push(OpCode::FPCLR as i16),
            PPECommand::Stop => vec.push(OpCode::STOP as i16),

            PPECommand::Goto(pos) => {
                vec.push(OpCode::GOTO as i16);
                vec.push(*pos as i16);
            }
            PPECommand::Gosub(pos) => {
                vec.push(OpCode::GOSUB as i16);
                vec.push(*pos as i16);
            }
            PPECommand::IfNot(expr, label) => {
                vec.push(OpCode::IFNOT as i16);
                expr.serialize(vec);
                vec.push(0);
                vec.push(*label as i16);
            }
            PPECommand::PredefinedCall(def, args) => {
                vec.push(def.opcode as i16);
                match def.sig {
                    super::StatementSignature::ArgumentsWithVariable(var_index, arg_count) => {
                        assert!(
                            arg_count == args.len(),
                            "Invalid argument count for {} was {} should be {}",
                            def.name,
                            args.len(),
                            arg_count
                        );
                        for (i, arg) in args.iter().enumerate() {
                            arg.serialize(vec);
                            if i + 1 != var_index {
                                vec.push(0);
                            }
                        }
                    }
                    super::StatementSignature::SpecialCaseVarSeg => {
                        args[0].serialize(vec);
                        args[1].serialize(vec);
                    }
                    super::StatementSignature::SpecialCaseDlockg => {
                        args[0].serialize(vec);
                        vec.push(0);
                        let id = args[1].get_id().unwrap();
                        vec.push(id as i16);
                        args[2].serialize(vec);
                        vec.push(0);
                    }
                    super::StatementSignature::SpecialCaseDcreate => {
                        args[0].serialize(vec);
                        vec.push(0);
                        args[1].serialize(vec);
                        vec.push(0);
                        args[2].serialize(vec);
                        vec.push(0);
                        let id = args[3].get_id().unwrap();
                        vec.push(id as i16);
                    }
                    super::StatementSignature::SpecialCaseSort => {
                        let id1 = args[0].get_id().unwrap();
                        let id2 = args[1].get_id().unwrap();
                        vec.push(id1 as i16);
                        vec.push(id2 as i16);
                    }
                    super::StatementSignature::VariableArguments(var_index) => {
                        vec.push(args.len() as i16);
                        for (i, arg) in args.iter().enumerate() {
                            if i + 1 == var_index {
                                vec.push(arg.get_id().unwrap() as i16);
                            } else {
                                arg.serialize(vec);
                                vec.push(0);
                            }
                        }
                    }
                    super::StatementSignature::SpecialCasePop => {
                        vec.push(args.len() as i16);
                        for arg in args {
                            arg.serialize(vec);
                        }
                    }

                    super::StatementSignature::Invalid => {
                        panic!("Invalid signature (special case should be handled before)")
                    }
                }
            }
            PPECommand::ProcedureCall(proc_id, args) => {
                vec.push(OpCode::PCALL as i16);
                vec.push(*proc_id as i16);
                vec.push(0);
                for arg in args {
                    arg.serialize(vec);
                    vec.push(0);
                }
            }
            PPECommand::Let(target, value) => {
                vec.push(OpCode::LET as i16);
                target.serialize(vec);
                value.serialize(vec);
                vec.push(0);
            }
        }
    }

    /// Returns the get size of this [`PPECommand`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_size(&self) -> usize {
        match self {
            PPECommand::End
            | PPECommand::Return
            | PPECommand::EndFunc
            | PPECommand::EndProc
            | PPECommand::Stop => 1,

            PPECommand::Goto(_) | PPECommand::Gosub(_) => 2,
            PPECommand::IfNot(expr, _) => 1 + expr.get_size() + 2,
            PPECommand::ProcedureCall(_, args) => 3 + PPEExpr::count_size(args) + args.len(),
            PPECommand::PredefinedCall(def, args) => match def.sig {
                super::StatementSignature::ArgumentsWithVariable(var_index, _) => {
                    1 + PPEExpr::count_size(args)
                        + if var_index > 0 {
                            args.len() - 1
                        } else {
                            args.len()
                        }
                }
                super::StatementSignature::VariableArguments(var_index) => {
                    1 + 1
                        + PPEExpr::count_size(args)
                        + if var_index > 0 {
                            args.len() - 2
                        } else {
                            args.len()
                        }
                }
                super::StatementSignature::SpecialCaseSort => 3,
                super::StatementSignature::SpecialCaseVarSeg => 1 + 2 + 2,
                super::StatementSignature::SpecialCaseDcreate => {
                    1 + PPEExpr::count_size(&args[0..3]) + 3 /* args.count */  + 1
                }
                super::StatementSignature::SpecialCaseDlockg => {
                    1 + args[0].get_size() + 1 + 1 + args[1].get_size() + 1
                }
                super::StatementSignature::SpecialCasePop => 1 + 1 + PPEExpr::count_size(args),
                super::StatementSignature::Invalid => {
                    panic!("Invalid signature {:?} for function {}", def.sig, def.name)
                }
            },
            PPECommand::Let(target, value) => 2 + target.get_size() + value.get_size(),
        }
    }
    pub fn visit<T, V: PPEVisitor<T>>(&self, visitor: &mut V) -> T {
        match self {
            PPECommand::End => visitor.visit_end(),
            PPECommand::Return => visitor.visit_return(),
            PPECommand::IfNot(cond, label) => visitor.visit_if(cond, label),
            PPECommand::ProcedureCall(id, args) => visitor.visit_proc_call(*id, args),
            PPECommand::PredefinedCall(def, args) => visitor.visit_predefined_call(def, args),
            PPECommand::Goto(label) => visitor.visit_goto(label),
            PPECommand::Gosub(label) => visitor.visit_gosub(label),
            PPECommand::EndFunc => visitor.visit_end_func(),
            PPECommand::EndProc => visitor.visit_end_proc(),
            PPECommand::Stop => visitor.visit_stop(),
            PPECommand::Let(target, value) => visitor.visit_let(target, value),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub enum PPEExpr {
    #[default]
    Invalid,

    Value(usize),
    UnaryExpression(UnaryOp, Box<PPEExpr>),
    BinaryExpression(BinOp, Box<PPEExpr>, Box<PPEExpr>),
    Dim(usize, Vec<PPEExpr>),
    PredefinedFunctionCall(&'static FunctionDefinition, Vec<PPEExpr>),
    FunctionCall(usize, Vec<PPEExpr>),
}

impl PPEExpr {
    /// .
    ///
    /// # Panics
    ///
    /// Panics if expression is invalid.
    pub fn serialize(&self, vec: &mut Vec<i16>) {
        match self {
            PPEExpr::Invalid => {
                panic!("Invalid expression");
            }
            PPEExpr::Value(id) => {
                vec.push(*id as i16);
                vec.push(0);
            }
            PPEExpr::Dim(id, vars) => {
                vec.push(*id as i16);
                vec.push(vars.len() as i16);
                for var in vars {
                    var.serialize(vec);
                    vec.push(0);
                }
            }
            PPEExpr::PredefinedFunctionCall(func, args) => {
                for arg in args {
                    arg.serialize(vec);
                }
                vec.push(func.opcode as i16);
            }
            PPEExpr::FunctionCall(func, args) => {
                vec.push(*func as i16);
                vec.push(0);
                for arg in args {
                    arg.serialize(vec);
                    vec.push(0);
                }
            }
            PPEExpr::UnaryExpression(op, expr) => {
                expr.serialize(vec);
                vec.push(*op as i16);
            }
            PPEExpr::BinaryExpression(op, left_expr, right_expr) => {
                left_expr.serialize(vec);
                right_expr.serialize(vec);
                vec.push(*op as i16);
            }
        }
    }

    pub fn get_size(&self) -> usize {
        match self {
            PPEExpr::Invalid => 0,
            PPEExpr::Value(_) => 2,
            PPEExpr::Dim(_, args) => 2 + Self::count_size(args) + args.len(),
            PPEExpr::PredefinedFunctionCall(_, args) => Self::count_size(args) + 1,
            PPEExpr::FunctionCall(_, args) => Self::count_size(args) + 2 + args.len(),
            PPEExpr::UnaryExpression(_, expr) => expr.get_size() + 1,
            PPEExpr::BinaryExpression(_, left_expr, right_expr) => {
                left_expr.get_size() + right_expr.get_size() + 1
            }
        }
    }

    pub fn count_size(args: &[PPEExpr]) -> usize {
        let r = args.iter().map(PPEExpr::get_size).sum();
        r
    }

    pub fn get_id(&self) -> Option<usize> {
        match self {
            PPEExpr::Value(id) | PPEExpr::Dim(id, _) => Some(*id),
            _ => None,
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn evaluate_constant_value(
        &self,
        executable: &Executable,
    ) -> Result<VariableValue, PPEError> {
        PPEConstantValueVisitor::new(executable).visit_value(0)
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if expression is invalid.
    pub fn visit<T, V: PPEVisitor<T>>(&self, visitor: &mut V) -> T {
        match self {
            PPEExpr::Invalid => panic!("Invalid expression"),
            PPEExpr::Value(id) => visitor.visit_value(*id),
            PPEExpr::UnaryExpression(op, expr) => visitor.visit_unary_expression(*op, expr),
            PPEExpr::BinaryExpression(op, left, right) => {
                visitor.visit_binary_expression(*op, left, right)
            }
            PPEExpr::Dim(id, dim) => visitor.visit_dim_expression(*id, dim),
            PPEExpr::PredefinedFunctionCall(def, arguments) => {
                visitor.visit_predefined_function_call(def, arguments)
            }
            PPEExpr::FunctionCall(id, arguments) => visitor.visit_function_call(*id, arguments),
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if expression is invalid.
    #[must_use]
    pub fn visit_mut<V: PPEVisitorMut>(&self, visitor: &mut V) -> PPEExpr {
        match self {
            PPEExpr::Invalid => panic!("Invalid expression"),
            PPEExpr::Value(id) => visitor.visit_value(*id),
            PPEExpr::UnaryExpression(op, expr) => visitor.visit_unary_expression(*op, expr),
            PPEExpr::BinaryExpression(op, left, right) => {
                visitor.visit_binary_expression(*op, left, right)
            }
            PPEExpr::Dim(id, dim) => visitor.visit_dim_expression(*id, dim),
            PPEExpr::PredefinedFunctionCall(def, arguments) => {
                visitor.visit_predefined_function_call(def, arguments)
            }
            PPEExpr::FunctionCall(id, arguments) => visitor.visit_function_call(*id, arguments),
        }
    }
}
#[allow(unused_variables)]
pub trait PPEVisitor<T>: Sized {
    fn visit_script(&mut self, script: &PPEScript) {
        for stmt in &script.statements {
            stmt.command.visit(self);
        }
    }
    fn visit_value(&mut self, id: usize) -> T;
    fn visit_unary_expression(&mut self, op: UnaryOp, expr: &PPEExpr) -> T;
    fn visit_binary_expression(&mut self, op: BinOp, left: &PPEExpr, right: &PPEExpr) -> T;
    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) -> T;
    fn visit_predefined_function_call(
        &mut self,
        def: &FunctionDefinition,
        arguments: &[PPEExpr],
    ) -> T;
    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) -> T;

    fn visit_end(&mut self) -> T;
    fn visit_return(&mut self) -> T;
    fn visit_if(&mut self, cond: &PPEExpr, label: &usize) -> T;
    fn visit_proc_call(&mut self, id: usize, arguments: &[PPEExpr]) -> T;
    fn visit_predefined_call(&mut self, def: &StatementDefinition, arguments: &[PPEExpr]) -> T;
    fn visit_goto(&mut self, label: &usize) -> T;
    fn visit_gosub(&mut self, label: &usize) -> T;
    fn visit_end_func(&mut self) -> T;
    fn visit_end_proc(&mut self) -> T;
    fn visit_stop(&mut self) -> T;
    fn visit_let(&mut self, target: &PPEExpr, value: &PPEExpr) -> T;
}

#[allow(unused_variables)]
pub trait PPEVisitorMut: Sized {
    fn visit_value(&mut self, id: usize) -> PPEExpr {
        PPEExpr::Value(id)
    }
    fn visit_unary_expression(&mut self, op: UnaryOp, expr: &PPEExpr) -> PPEExpr {
        PPEExpr::UnaryExpression(op, Box::new(expr.visit_mut(self)))
    }
    fn visit_binary_expression(&mut self, op: BinOp, left: &PPEExpr, right: &PPEExpr) -> PPEExpr {
        PPEExpr::BinaryExpression(
            op,
            Box::new(left.visit_mut(self)),
            Box::new(right.visit_mut(self)),
        )
    }
    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) -> PPEExpr {
        PPEExpr::Dim(id, dim.iter().map(|e| e.visit_mut(self)).collect())
    }
    fn visit_predefined_function_call(
        &mut self,
        def: &'static FunctionDefinition,
        arguments: &[PPEExpr],
    ) -> PPEExpr {
        PPEExpr::PredefinedFunctionCall(def, arguments.iter().map(|e| e.visit_mut(self)).collect())
    }
    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) -> PPEExpr {
        PPEExpr::FunctionCall(id, arguments.iter().map(|e| e.visit_mut(self)).collect())
    }
}

struct PPEConstantValueVisitor<'a> {
    executable: &'a Executable,
}

impl<'a> PPEConstantValueVisitor<'a> {
    fn new(executable: &'a Executable) -> Self {
        Self { executable }
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum PPEError {
    #[error("Unsupported dimension {0} for variable {1:?}")]
    UnsupportedDimension(usize, VariableValue),

    #[error("Only constant expressions are allowed")]
    OnlyConstantsAllowed,
}

impl<'a> PPEVisitor<Result<VariableValue, PPEError>> for PPEConstantValueVisitor<'a> {
    fn visit_value(&mut self, id: usize) -> Result<VariableValue, PPEError> {
        Ok(self.executable.variable_table.get_value(id).clone())
    }

    fn visit_unary_expression(
        &mut self,
        op: UnaryOp,
        expr: &PPEExpr,
    ) -> Result<VariableValue, PPEError> {
        let val = expr.visit(self)?;
        let val = match op {
            UnaryOp::Plus => val,
            UnaryOp::Minus => -val,
            UnaryOp::Not => val.not(),
        };
        Ok(val)
    }

    fn visit_binary_expression(
        &mut self,
        op: BinOp,
        left: &PPEExpr,
        right: &PPEExpr,
    ) -> Result<VariableValue, PPEError> {
        let left_val = left.visit(self)?;
        let right_val = right.visit(self)?;
        let val = match op {
            BinOp::Add => left_val + right_val,
            BinOp::Sub => left_val - right_val,
            BinOp::Mul => left_val * right_val,
            BinOp::Div => left_val / right_val,
            BinOp::Mod => left_val % right_val,
            BinOp::PoW => left_val.pow(right_val),
            BinOp::And => VariableValue::new_bool(left_val.as_bool() && right_val.as_bool()),
            BinOp::Or => VariableValue::new_bool(left_val.as_bool() || right_val.as_bool()),
            BinOp::Eq => VariableValue::new_bool(left_val.as_bool() == right_val.as_bool()),
            BinOp::NotEq => VariableValue::new_bool(left_val.as_bool() != right_val.as_bool()),
            BinOp::Lower => VariableValue::new_bool(left_val < right_val),
            BinOp::LowerEq => VariableValue::new_bool(left_val <= right_val),
            BinOp::Greater => VariableValue::new_bool(left_val > right_val),
            BinOp::GreaterEq => VariableValue::new_bool(left_val >= right_val),
        };

        Ok(val)
    }

    fn visit_dim_expression(
        &mut self,
        id: usize,
        dim: &[PPEExpr],
    ) -> Result<VariableValue, PPEError> {
        let var_value = self.executable.variable_table.get_value(id).clone();
        match dim.len() {
            1 => {
                let vector = dim[0].visit(self)?.as_int();
                if let GenericVariableData::Dim1(data) = &var_value.generic_data {
                    if vector < 0 || vector >= data.len() as i32 {
                        return Ok(var_value.get_type().create_empty_value());
                    }
                    return Ok(data[vector as usize].clone());
                }
                Err(PPEError::UnsupportedDimension(1, var_value.clone()))
            }
            2 => {
                let vector = dim[0].visit(self)?.as_int();
                let matrix = dim[1].visit(self)?.as_int();
                if let GenericVariableData::Dim2(data) = &var_value.generic_data {
                    if vector < 0
                        || vector >= data.len() as i32
                        || matrix < 0
                        || matrix >= data[0].len() as i32
                    {
                        return Ok(var_value.get_type().create_empty_value());
                    }
                    return Ok(data[vector as usize][matrix as usize].clone());
                }
                Err(PPEError::UnsupportedDimension(2, var_value.clone()))
            }
            3 => {
                let vector = dim[0].visit(self)?.as_int();
                let matrix = dim[1].visit(self)?.as_int();
                let cube = dim[2].visit(self)?.as_int();
                if let GenericVariableData::Dim3(data) = &var_value.generic_data {
                    if vector < 0
                        || vector >= data.len() as i32
                        || matrix < 0
                        || matrix >= data[0].len() as i32
                        || cube < 0
                        || cube >= data[0][0].len() as i32
                    {
                        return Ok(var_value.get_type().create_empty_value());
                    }
                    return Ok(data[vector as usize][matrix as usize][cube as usize].clone());
                }
                Err(PPEError::UnsupportedDimension(3, var_value.clone()))
            }
            len => Err(PPEError::UnsupportedDimension(len, var_value.clone())),
        }
    }

    fn visit_predefined_function_call(
        &mut self,
        _def: &FunctionDefinition,
        _arguments: &[PPEExpr],
    ) -> Result<VariableValue, PPEError> {
        Err(PPEError::OnlyConstantsAllowed)
    }

    fn visit_function_call(
        &mut self,
        _id: usize,
        _arguments: &[PPEExpr],
    ) -> Result<VariableValue, PPEError> {
        Err(PPEError::OnlyConstantsAllowed)
    }

    fn visit_end(&mut self) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_return(&mut self) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_if(&mut self, _cond: &PPEExpr, _label: &usize) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_proc_call(
        &mut self,
        _id: usize,
        _args: &[PPEExpr],
    ) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_predefined_call(
        &mut self,
        _def: &StatementDefinition,
        _args: &[PPEExpr],
    ) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_goto(&mut self, _label: &usize) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_gosub(&mut self, _label: &usize) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_end_func(&mut self) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_end_proc(&mut self) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_stop(&mut self) -> Result<VariableValue, PPEError> {
        todo!()
    }

    fn visit_let(
        &mut self,
        _target: &PPEExpr,
        _value: &PPEExpr,
    ) -> Result<VariableValue, PPEError> {
        todo!()
    }
}

#[derive(Default)]
pub struct ExpressionNegator {}

impl PPEVisitorMut for ExpressionNegator {
    fn visit_value(&mut self, id: usize) -> PPEExpr {
        PPEExpr::UnaryExpression(UnaryOp::Not, Box::new(PPEExpr::Value(id)))
    }

    fn visit_unary_expression(&mut self, op: UnaryOp, expr: &PPEExpr) -> PPEExpr {
        if op == UnaryOp::Not {
            expr.clone()
        } else {
            PPEExpr::UnaryExpression(
                UnaryOp::Not,
                Box::new(PPEExpr::UnaryExpression(op, Box::new(expr.clone()))),
            )
        }
    }

    fn visit_binary_expression(&mut self, op: BinOp, left: &PPEExpr, right: &PPEExpr) -> PPEExpr {
        match op {
            BinOp::PoW | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Add | BinOp::Sub => {
                PPEExpr::UnaryExpression(
                    UnaryOp::Not,
                    Box::new(PPEExpr::BinaryExpression(
                        op,
                        Box::new(left.clone()),
                        Box::new(right.clone()),
                    )),
                )
            }

            BinOp::Eq => PPEExpr::BinaryExpression(
                BinOp::NotEq,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::NotEq => PPEExpr::BinaryExpression(
                BinOp::Eq,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::Lower => PPEExpr::BinaryExpression(
                BinOp::GreaterEq,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::LowerEq => PPEExpr::BinaryExpression(
                BinOp::Greater,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::Greater => PPEExpr::BinaryExpression(
                BinOp::LowerEq,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::GreaterEq => PPEExpr::BinaryExpression(
                BinOp::Lower,
                Box::new(left.clone()),
                Box::new(right.clone()),
            ),
            BinOp::And | BinOp::Or => {
                let variant1 = PPEExpr::UnaryExpression(
                    UnaryOp::Not,
                    Box::new(PPEExpr::BinaryExpression(
                        op,
                        Box::new(left.clone()),
                        Box::new(right.clone()),
                    )),
                );

                let variant2 = PPEExpr::BinaryExpression(
                    if op == BinOp::Or {
                        BinOp::And
                    } else {
                        BinOp::Or
                    },
                    Box::new(left.visit_mut(self)),
                    Box::new(right.visit_mut(self)),
                );
                if variant1.get_size() < variant2.get_size() {
                    variant1
                } else {
                    variant2
                }
            }
        }
    }

    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) -> PPEExpr {
        PPEExpr::UnaryExpression(UnaryOp::Not, Box::new(PPEExpr::Dim(id, dim.to_vec())))
    }

    fn visit_predefined_function_call(
        &mut self,
        def: &'static FunctionDefinition,
        arguments: &[PPEExpr],
    ) -> PPEExpr {
        PPEExpr::UnaryExpression(
            UnaryOp::Not,
            Box::new(PPEExpr::PredefinedFunctionCall(def, arguments.to_vec())),
        )
    }

    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) -> PPEExpr {
        PPEExpr::UnaryExpression(
            UnaryOp::Not,
            Box::new(PPEExpr::FunctionCall(id, arguments.to_vec())),
        )
    }
}
