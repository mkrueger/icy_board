use std::{collections::HashMap, mem::transmute, ops::Range};

use thiserror::Error;

use crate::{
    ast::{BinOp, UnaryOp},
    executable::{FUNCTION_DEFINITIONS, OpCode, STATEMENT_DEFINITIONS},
};

use super::{Executable, FuncOpCode, FunctionSignature, LAST_STMT, PPECommand, PPEExpr, StatementSignature, VariableType, VariableValue};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DeserializationErrorType {
    #[error("Expressionstack is empty")]
    ExpressionStackEmpty,

    #[error("No expression found")]
    NoExpression,

    #[error("Invalid expression stack state")]
    InvalidExpressionStackState,

    #[error("Too few arguments for unary expression ({0:04X}:{1:?})")]
    TooFewArgumentsForUnaryExpression(usize, UnaryOp),

    #[error("Too few function arguments for {0:?}, expected {1}, got {2}")]
    TooFewBuiltInFunctionArguments(FuncOpCode, usize, usize),

    #[error("Too few function arguments")]
    TooFewFunctionArguments,

    #[error("Index out of bounds")]
    IndexOutOfBounds,

    #[error("Unknown statement {0}")]
    UnknownStatement(i16),

    #[error("Called non-procedure({0:04X}) in PCALL {1:?}")]
    CalledNonProcedureInPCall(usize, VariableValue),

    #[error("No variable table entry for {0}")]
    NoVTableEntry(usize),

    #[error("{0} has no start offset, its body stays inline in the main program")]
    RoutineWithoutStartOffset(String),

    #[error("Got procedure call in expression {0}")]
    GotProcedureCallInExpression(i16),

    #[error("Binary expression stack empty for binary operation ({0})")]
    BinaryExpressionStackEmpty(BinOp),

    #[error("Only one argument for binary operation ({0})")]
    OnlyOneArgumentForBinop(BinOp),

    #[error("Invalid statement ({0:04X})")]
    InvalidStatement(i16),

    #[error("Invalid dimensonized expression ({0:04X}:{1:02X})")]
    InvalidDimensonizedExpression(i16, i16),

    #[error("Invalid indexed member dimension count ({0})")]
    InvalidIndexedMemberDimensionCount(i16),

    #[error("Invalid statement signature.")]
    InvalidStatementSignature,

    #[error("Invalid let target expression ({0:04X})")]
    LetTargetInvalid(usize),

    #[error("Invalid let value expression ({0:04X})")]
    LetValueInvalid(usize),

    #[error("Invalid if condition expression ({0:04X})")]
    IfConditionInvalid(usize),

    #[error("Expression nesting exceeds the supported limit of {0}")]
    ExpressionNestingTooDeep(usize),
}

const MAX_EXPRESSION_DEPTH: usize = 64;

#[derive(Default)]
pub struct PPEDeserializer {
    expr_stack: Vec<PPEExpr>,
    pub offset: usize,

    stmt_offset: usize,
    expr_offset: usize,
    expression_depth: usize,
    pub bugged_offsets: HashMap<usize, Vec<DeserializationErrorType>>,
}

impl PPEDeserializer {
    pub fn stmt_span(&self) -> Range<usize> {
        self.stmt_offset..self.offset
    }
    pub fn expr_span(&self) -> Range<usize> {
        self.expr_offset..self.offset
    }

    fn read_word(&mut self, executable: &Executable) -> Result<i16, DeserializationErrorType> {
        let Some(&word) = executable.script_buffer.get(self.offset) else {
            return Err(DeserializationErrorType::IndexOutOfBounds);
        };
        self.offset += 1;
        Ok(word)
    }

    /// .
    /// # Errors
    ///
    /// This function will return an error if .
    /// # Panics
    ///
    pub fn deserialize_statement(&mut self, executable: &Executable) -> Result<Option<PPECommand>, DeserializationErrorType> {
        self.stmt_offset = self.offset;
        if self.offset >= executable.script_buffer.len() {
            return Ok(None);
        }
        let cur_stmt = executable.script_buffer[self.offset];
        self.offset += 1;

        if cur_stmt == 0 {
            return Ok(None);
        }
        if !(0..=LAST_STMT).contains(&cur_stmt) {
            self.report_bug(DeserializationErrorType::InvalidStatement(cur_stmt));
            return Ok(None);
        }

        let op: OpCode = unsafe { transmute(cur_stmt) };

        match op {
            OpCode::END => Ok(Some(PPECommand::End)),
            OpCode::RETURN => Ok(Some(PPECommand::Return)),
            OpCode::FEND => Ok(Some(PPECommand::EndFunc)),
            OpCode::FPCLR => Ok(Some(PPECommand::EndProc)),
            OpCode::STOP => Ok(Some(PPECommand::Stop)),
            OpCode::LET => {
                let Some(target) = self.read_variable_expression(executable)? else {
                    return Err(DeserializationErrorType::LetTargetInvalid(self.offset));
                };
                let Some(value) = self.deserialize_expression(executable)? else {
                    return Err(DeserializationErrorType::LetValueInvalid(self.offset));
                };

                Ok(Some(PPECommand::Let(Box::new(target), Box::new(value))))
            }
            OpCode::MemberCall => {
                let Some(expr) = self.deserialize_expression(executable)? else {
                    return Err(DeserializationErrorType::LetValueInvalid(self.offset));
                };
                Ok(Some(PPECommand::MemberCall(Box::new(expr))))
            }
            OpCode::IFNOT => {
                let Some(expr) = self.deserialize_expression(executable)? else {
                    return Err(DeserializationErrorType::IfConditionInvalid(self.offset));
                };
                let label = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::IfNot(Box::new(expr), label)))
            }
            OpCode::ForEach => {
                let variable = self.read_word(executable)? as usize;
                let Some(collection) = self.deserialize_expression(executable)? else {
                    return Err(DeserializationErrorType::NoExpression);
                };
                let end = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::ForEach(variable, Box::new(collection), end)))
            }
            OpCode::NextForEach => {
                let start = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::NextForEach(start)))
            }
            OpCode::GOSUB => {
                let label = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::Gosub(label)))
            }
            OpCode::GOTO => {
                let label = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::Goto(label)))
            }
            OpCode::OnError => {
                let mode = self.read_word(executable)?;
                let target = self.read_word(executable)? as usize;
                Ok(Some(PPECommand::OnError(super::commands::OnErrorTarget::decode(mode, target))))
            }
            OpCode::PCALL => {
                // TODO: implement read var correctld ?
                let proc_id = self.read_word(executable)? as usize;
                let _argument_separator = self.read_word(executable)?;

                let Some(var) = executable.variable_table.try_get_entry(proc_id) else {
                    return Err(DeserializationErrorType::NoVTableEntry(proc_id));
                };

                if var.value.vtype != VariableType::Procedure {
                    return Err(DeserializationErrorType::CalledNonProcedureInPCall(proc_id, var.value.clone()));
                }

                let argument_count = unsafe { var.value.data.procedure_value.parameters };
                let mut arguments = Vec::new();
                for _ in 0..argument_count {
                    if let Some(expr) = self.deserialize_expression(executable)? {
                        arguments.push(expr);
                    }
                }
                Ok(Some(PPECommand::ProcedureCall(proc_id, arguments)))
            }
            _ => {
                let idx = op as usize;
                let Some(def) = STATEMENT_DEFINITIONS.get(idx) else {
                    return Err(DeserializationErrorType::UnknownStatement(cur_stmt));
                };

                if def.sig == StatementSignature::Invalid {
                    self.report_bug(DeserializationErrorType::InvalidStatement(cur_stmt));
                    return Err(DeserializationErrorType::InvalidStatement(cur_stmt));
                }

                let (var_idx, argument_count) = match def.sig {
                    crate::executable::StatementSignature::ArgumentsWithVariable(var_idx, argument_count) => (var_idx, argument_count),
                    crate::executable::StatementSignature::VariableArguments(var_idx, _, _) => {
                        let argument_count = self.read_word(executable)?;
                        if argument_count < 0 {
                            return Err(DeserializationErrorType::InvalidStatementSignature);
                        }

                        let mut arguments = Vec::new();
                        for i in 0..argument_count {
                            let expr = if i + 1 == var_idx as i16 {
                                PPEExpr::Value(self.read_word(executable)? as usize)
                            } else {
                                self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?
                            };
                            arguments.push(expr);
                        }
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseSort => {
                        let arguments = vec![
                            PPEExpr::Value(self.read_word(executable)? as usize),
                            PPEExpr::Value(self.read_word(executable)? as usize),
                        ];

                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseVarSeg => {
                        let arguments = vec![
                            self.read_variable_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                            self.read_variable_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                        ];
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseDcreate => {
                        let arguments = vec![
                            self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                            self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                            self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                            PPEExpr::Value(self.read_word(executable)? as usize),
                        ];
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    super::StatementSignature::SpecialCaseDlockg => {
                        let mut arguments = vec![
                            self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?,
                            PPEExpr::Value(self.read_word(executable)? as usize),
                        ];
                        arguments.push(self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?);
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCasePop => {
                        let count = self.read_word(executable)?;
                        if count < 0 {
                            return Err(DeserializationErrorType::InvalidStatementSignature);
                        }
                        let count = count as usize;
                        let mut arguments = Vec::new();
                        for _ in 0..count {
                            arguments.push(self.read_variable_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?);
                        }
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::smt_op_codes::StatementSignature::Invalid => {
                        return Err(DeserializationErrorType::InvalidStatementSignature);
                    }
                };

                let mut arguments = Vec::new();
                for i in 0..argument_count {
                    let expr = if i + 1 == var_idx {
                        self.read_variable_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?
                    } else {
                        self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?
                    };
                    arguments.push(expr);
                }
                Ok(Some(PPECommand::PredefinedCall(def, arguments)))
            }
        }
    }

    /// .
    ///
    /// # Panics
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn deserialize_expression(&mut self, executable: &Executable) -> Result<Option<PPEExpr>, DeserializationErrorType> {
        if self.expression_depth >= MAX_EXPRESSION_DEPTH {
            return Err(DeserializationErrorType::ExpressionNestingTooDeep(MAX_EXPRESSION_DEPTH));
        }
        self.expression_depth += 1;
        let result = self.deserialize_expression_inner(executable);
        self.expression_depth -= 1;
        result
    }

    fn deserialize_expression_inner(&mut self, executable: &Executable) -> Result<Option<PPEExpr>, DeserializationErrorType> {
        self.expr_offset = self.offset;

        loop {
            if self.offset >= executable.script_buffer.len() {
                break;
            }
            let id = executable.script_buffer[self.offset];
            if id == 0 {
                self.offset += 1;
                break;
            }
            if id > 0 {
                let id = id as usize;
                let Some(val) = executable.variable_table.try_get_value(id) else {
                    log::warn!("Potential error in expression deserialization: No variable table entry for {id:02X}, skipping.");
                    self.offset += 1;
                    break;
                };
                if val.vtype == VariableType::Function {
                    unsafe {
                        self.offset += 2;
                        let parameters = executable.variable_table.get_value(id).data.function_value.parameters;
                        let mut arguments = Vec::new();
                        for _ in 0..parameters {
                            if let Some(expr) = self.deserialize_expression(executable)? {
                                arguments.push(expr);
                            }
                        }
                        self.push_expr(PPEExpr::FunctionCall(id, arguments));
                        continue;
                    }
                }

                if let Some(var_expr) = self.read_variable_expression(executable)? {
                    self.push_expr(var_expr);
                } else {
                    break;
                }
            } else {
                if id == FuncOpCode::CPAR as i16 {
                    self.offset += 1;
                    break;
                }
                if id == FuncOpCode::RoutineReference as i16 {
                    self.offset += 1;
                    let Some(&routine_id) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    self.push_expr(PPEExpr::RoutineReference(routine_id as usize));
                    continue;
                }
                if id == FuncOpCode::RecordLiteral as i16 {
                    self.offset += 1;
                    let Some(&type_id) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    let Some(&field_count) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    if field_count < 0 {
                        return Err(DeserializationErrorType::InvalidExpressionStackState);
                    }
                    let field_count = field_count as usize;
                    let Some(field_ids_end) = self.offset.checked_add(field_count) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    let Some(field_ids) = executable.script_buffer.get(self.offset..field_ids_end) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    let field_ids: Vec<usize> = field_ids.iter().map(|id| *id as usize).collect();
                    self.offset = field_ids_end;
                    if self.expr_stack.len() < field_count {
                        return Err(DeserializationErrorType::ExpressionStackEmpty);
                    }
                    let mut values = Vec::with_capacity(field_count);
                    for field_id in field_ids.into_iter().rev() {
                        let Some(value) = self.pop_expr() else {
                            return Err(DeserializationErrorType::ExpressionStackEmpty);
                        };
                        values.push((field_id, value));
                    }
                    values.reverse();
                    self.push_expr(PPEExpr::RecordLiteral(type_id as u8, values));
                    continue;
                }
                if id == FuncOpCode::MemberReference as i16 {
                    let Some(expr) = self.pop_expr() else {
                        return Err(DeserializationErrorType::ExpressionStackEmpty);
                    };
                    self.offset += 1;
                    let Some(&member_id) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;

                    self.push_expr(PPEExpr::Member(Box::new(expr), member_id as usize));
                    continue;
                }

                if id == FuncOpCode::IndexedMember as i16 {
                    let Some(expr) = self.pop_expr() else {
                        return Err(DeserializationErrorType::ExpressionStackEmpty);
                    };
                    self.offset += 1;
                    let Some(&member_id) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    let Some(&dimension_count) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    if !(1..=3).contains(&dimension_count) {
                        return Err(DeserializationErrorType::InvalidIndexedMemberDimensionCount(dimension_count));
                    }
                    let dimension_count = dimension_count as usize;
                    let mut dimensions = Vec::with_capacity(dimension_count);
                    for _ in 0..dimension_count {
                        let Some(dimension) = self.deserialize_expression(executable)? else {
                            return Err(DeserializationErrorType::ExpressionStackEmpty);
                        };
                        dimensions.push(dimension);
                    }
                    self.push_expr(PPEExpr::IndexedMember(Box::new(expr), member_id as usize, dimensions));
                    continue;
                }

                if id == FuncOpCode::MemberCall as i16 {
                    self.offset += 1;
                    let Some(&arg_count) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    let Some(&member_id) = executable.script_buffer.get(self.offset) else {
                        return Err(DeserializationErrorType::IndexOutOfBounds);
                    };
                    self.offset += 1;
                    if arg_count < 0 {
                        return Err(DeserializationErrorType::InvalidExpressionStackState);
                    }
                    let arg_count = arg_count as usize;
                    if self.expr_stack.len() < arg_count + 1 {
                        return Err(DeserializationErrorType::TooFewFunctionArguments);
                    }

                    let mut arguments = Vec::new();
                    for _ in 0..arg_count {
                        if let Some(expr) = self.pop_expr() {
                            arguments.push(expr);
                        } else {
                            self.report_bug(DeserializationErrorType::TooFewFunctionArguments);
                            return self.deserialize_expression(executable);
                        }
                    }
                    // They were written left to right and come off the stack the other way.
                    arguments.reverse();

                    let Some(expr) = self.pop_expr() else {
                        return Err(DeserializationErrorType::ExpressionStackEmpty);
                    };
                    self.push_expr(PPEExpr::MemberFunctionCall(Box::new(expr), arguments, member_id as usize));
                    continue;
                }

                let Some(func) = id.checked_neg().map(|id| id as usize) else {
                    return Err(DeserializationErrorType::InvalidExpressionStackState);
                };
                let Some(func_def) = FUNCTION_DEFINITIONS.get(func) else {
                    return Err(DeserializationErrorType::InvalidExpressionStackState);
                };
                match func_def.signature {
                    FunctionSignature::UnaryOp => {
                        self.offset += 1;
                        let op = UnaryOp::from_opcode(func_def.opcode);

                        if let Some(unary_expr) = self.pop_expr() {
                            self.push_expr(PPEExpr::UnaryExpression(op, Box::new(unary_expr)));
                        } else {
                            // Some obfuscators try to trick the decompiler by using invalid unary expressions with 0 arguments
                            // PCBoard will just skip these
                            self.report_bug(DeserializationErrorType::TooFewArgumentsForUnaryExpression(self.offset, op));
                            return self.deserialize_expression(executable);
                        }
                    }
                    FunctionSignature::BinaryOp => {
                        self.offset += 1;
                        let binop = BinOp::from_opcode(func_def.opcode);
                        if self.expr_stack.is_empty() {
                            self.report_bug(DeserializationErrorType::BinaryExpressionStackEmpty(binop));
                            return Ok(None);
                        }
                        let r_value = self.pop_expr().unwrap();
                        if self.expr_stack.is_empty() {
                            self.report_bug(DeserializationErrorType::OnlyOneArgumentForBinop(binop));
                            self.push_expr(r_value);
                        } else {
                            let l_value = self.pop_expr().unwrap();

                            self.push_expr(PPEExpr::BinaryExpression(binop, Box::new(l_value), Box::new(r_value)));
                        }
                    }
                    FunctionSignature::Invalid => {
                        // Consuming the opcode keeps a corrupt PPE from repeating this word forever.
                        self.offset += 1;
                        self.push_expr(PPEExpr::PredefinedFunctionCall(func_def, vec![]));
                    }
                    FunctionSignature::FixedParameters(count) => {
                        self.offset += 1;

                        if self.expr_stack.len() < count {
                            return Err(DeserializationErrorType::TooFewBuiltInFunctionArguments(
                                func_def.opcode,
                                count,
                                self.expr_stack.len(),
                            ));
                        }
                        let arguments = self.expr_stack.drain(self.expr_stack.len() - count..).collect();
                        self.push_expr(PPEExpr::PredefinedFunctionCall(func_def, arguments));
                    }
                }
            }
        }

        match self.pop_expr() {
            Some(expr) => Ok(Some(expr)),
            None => Err(DeserializationErrorType::ExpressionStackEmpty),
        }
    }

    fn report_bug(&mut self, error: DeserializationErrorType) {
        if let Some(vec) = self.bugged_offsets.get_mut(&self.stmt_offset) {
            vec.push(error);
        } else {
            self.bugged_offsets.insert(self.stmt_offset, vec![error]);
        }
    }

    fn push_expr(&mut self, expr: PPEExpr) {
        self.expr_stack.push(expr);
    }

    fn pop_expr(&mut self) -> Option<PPEExpr> {
        self.expr_stack.pop()
    }

    fn read_variable_expression(&mut self, executable: &Executable) -> Result<Option<PPEExpr>, DeserializationErrorType> {
        let Some(&id) = executable.script_buffer.get(self.offset) else {
            return Err(DeserializationErrorType::IndexOutOfBounds);
        };
        self.offset += 1;
        if self.offset >= executable.script_buffer.len() {
            return Err(DeserializationErrorType::IndexOutOfBounds);
        }
        let dim = executable.script_buffer[self.offset];
        if !(0..=3).contains(&dim) {
            return Err(DeserializationErrorType::InvalidDimensonizedExpression(id, dim));
        }
        self.offset += 1;
        let mut expr = if dim == 0 {
            PPEExpr::Value(id as usize)
        } else {
            for _ in 0..dim {
                if let Some(e) = self.deserialize_expression(executable)? {
                    self.push_expr(e);
                }
            }
            if self.expr_stack.len() < dim as usize {
                return Err(DeserializationErrorType::InvalidExpressionStackState);
            }
            let dims = self.expr_stack.drain(self.expr_stack.len() - dim as usize..).collect();
            PPEExpr::Dim(id as usize, dims)
        };

        // A record target may alternate scalar and indexed fields at any depth.
        loop {
            if executable.script_buffer.get(self.offset) == Some(&(FuncOpCode::MemberReference as i16)) {
                if self.offset + 1 >= executable.script_buffer.len() {
                    return Err(DeserializationErrorType::IndexOutOfBounds);
                }
                self.offset += 1;
                let member_id = executable.script_buffer[self.offset];
                self.offset += 1;
                expr = PPEExpr::Member(Box::new(expr), member_id as usize);
                continue;
            }
            if executable.script_buffer.get(self.offset) == Some(&(FuncOpCode::IndexedMember as i16)) {
                if self.offset + 2 >= executable.script_buffer.len() {
                    return Err(DeserializationErrorType::IndexOutOfBounds);
                }
                self.offset += 1;
                let member_id = executable.script_buffer[self.offset] as usize;
                self.offset += 1;
                let dimension_count = executable.script_buffer[self.offset];
                self.offset += 1;
                if !(1..=3).contains(&dimension_count) {
                    return Err(DeserializationErrorType::InvalidIndexedMemberDimensionCount(dimension_count));
                }
                let dimension_count = dimension_count as usize;
                let mut dimensions = Vec::with_capacity(dimension_count);
                for _ in 0..dimension_count {
                    dimensions.push(self.deserialize_expression(executable)?.ok_or(DeserializationErrorType::NoExpression)?);
                }
                expr = PPEExpr::IndexedMember(Box::new(expr), member_id, dimensions);
                continue;
            }
            break;
        }
        Ok(Some(expr))
    }
}
