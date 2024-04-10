use std::{collections::HashMap, mem::transmute, ops::Range};

use thiserror::Error;

use crate::{
    ast::{BinOp, UnaryOp},
    executable::{OpCode, FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS},
};

use super::{
    Executable, FuncOpCode, PPECommand, PPEExpr, StatementSignature, VariableType, VariableValue,
    LAST_STMT,
};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DeserializationErrorType {
    #[error("Expressionstack is empty")]
    ExpressionStackEmpty,

    #[error("No expression found")]
    NoExpression,

    #[error("Invalid expression stack state")]
    InvalidExpressionStackState,

    #[error("Too few arguments for binary expression")]
    TooFewArgumentsForBinaryExpression(BinOp),

    #[error("Too few arguments for unary expression")]
    TooFewArgumentsForUnaryExpression(UnaryOp),

    #[error("Too few function arguments for {0:?}, expected {1}, got {2}")]
    TooFewFunctionArguments(FuncOpCode, i8, usize),

    #[error("Index out of bounds")]
    IndexOutOfBounds,

    #[error("Unknown statement {0}")]
    UnknownStatement(i16),

    #[error("Called non-procedure({0:04X}) in PCALL {1:?}")]
    CalledNonProcedureInPCall(usize, VariableValue),

    #[error("No variable table entry for {0}")]
    NoVTableEntry(usize),

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
}

#[derive(Default)]
pub struct PPEDeserializer {
    expr_stack: Vec<PPEExpr>,
    pub offset: usize,

    stmt_offset: usize,
    expr_offset: usize,
    pub bugged_offsets: HashMap<usize, Vec<DeserializationErrorType>>,
}

impl PPEDeserializer {
    pub fn stmt_span(&self) -> Range<usize> {
        self.stmt_offset..self.offset
    }
    pub fn expr_span(&self) -> Range<usize> {
        self.expr_offset..self.offset
    }

    /// .
    /// # Errors
    ///
    /// This function will return an error if .
    /// # Panics
    ///
    pub fn deserialize_statement(
        &mut self,
        executable: &Executable,
    ) -> Result<Option<PPECommand>, DeserializationErrorType> {
        self.stmt_offset = self.offset;
        if self.offset >= executable.script_buffer.len() {
            return Ok(None);
        }
        let cur_stmt = executable.script_buffer[self.offset];
        self.offset += 1;

        if cur_stmt == 0 {
            return Ok(None);
        }
        if !(0..LAST_STMT).contains(&cur_stmt) {
            self.report_bug(DeserializationErrorType::InvalidStatement(cur_stmt));
            return Ok(None);
        }

        let op: OpCode = unsafe { transmute(cur_stmt) };

        let res = match op {
            OpCode::END => Ok(Some(PPECommand::End)),
            OpCode::RETURN => Ok(Some(PPECommand::Return)),
            OpCode::FEND => Ok(Some(PPECommand::EndFunc)),
            OpCode::FPCLR => Ok(Some(PPECommand::EndProc)),
            OpCode::STOP => Ok(Some(PPECommand::Stop)),
            OpCode::LET => {
                let Some(target) = self.read_variable_expression(executable) else {
                    return Ok(None);
                };
                let Some(value) = self.deserialize_expression(executable)? else {
                    return Ok(None);
                };

                Ok(Some(PPECommand::Let(Box::new(target), Box::new(value))))
            }
            OpCode::IFNOT => {
                let Some(expr) = self.deserialize_expression(executable)? else {
                    return Ok(None);
                };
                let label = executable.script_buffer[self.offset] as usize;
                self.offset += 1;
                Ok(Some(PPECommand::IfNot(Box::new(expr), label)))
            }
            OpCode::GOSUB => {
                let label = executable.script_buffer[self.offset] as usize;
                self.offset += 1;
                Ok(Some(PPECommand::Gosub(label)))
            }
            OpCode::GOTO => {
                let label = executable.script_buffer[self.offset] as usize;
                self.offset += 1;
                Ok(Some(PPECommand::Goto(label)))
            }
            OpCode::PCALL => {
                // TODO: implement read var correctld ?
                let proc_id = executable.script_buffer[self.offset] as usize;
                self.offset += 2;

                let Some(var) = executable.variable_table.try_get_entry(proc_id) else {
                    return Err(DeserializationErrorType::NoVTableEntry(proc_id));
                };

                if var.value.vtype != VariableType::Procedure {
                    return Err(DeserializationErrorType::CalledNonProcedureInPCall(
                        proc_id,
                        var.value.clone(),
                    ));
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
                    return Ok(None);
                }

                let (var_idx, argument_count) = match def.sig {
                    crate::executable::StatementSignature::ArgumentsWithVariable(
                        var_idx,
                        argument_count,
                    ) => (var_idx, argument_count),
                    crate::executable::StatementSignature::VariableArguments(var_idx) => {
                        let argument_count = executable.script_buffer[self.offset];
                        assert!(argument_count >= 0, "negative argument count");
                        self.offset += 1;

                        let mut arguments = Vec::new();
                        for i in 0..argument_count {
                            let expr = if i + 1 == var_idx as i16 {
                                let expr =
                                    PPEExpr::Value(executable.script_buffer[self.offset] as usize);
                                self.offset += 1;
                                expr
                            } else {
                                self.deserialize_expression(executable)?.unwrap()
                            };
                            arguments.push(expr);
                        }
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseSort => {
                        let arguments = vec![
                            PPEExpr::Value(executable.script_buffer[self.offset] as usize),
                            PPEExpr::Value(executable.script_buffer[self.offset + 1] as usize),
                        ];
                        self.offset += 2;

                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseVarSeg => {
                        let arguments = vec![
                            self.read_variable_expression(executable).unwrap(),
                            self.read_variable_expression(executable).unwrap(),
                        ];
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCaseDcreate => {
                        let arguments = vec![
                            self.deserialize_expression(executable)?.unwrap(),
                            self.deserialize_expression(executable)?.unwrap(),
                            self.deserialize_expression(executable)?.unwrap(),
                            PPEExpr::Value(executable.script_buffer[self.offset] as usize),
                        ];
                        self.offset += 1;
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    super::StatementSignature::SpecialCaseDlockg => {
                        let mut arguments = vec![
                            self.deserialize_expression(executable)?.unwrap(),
                            PPEExpr::Value(executable.script_buffer[self.offset] as usize),
                        ];
                        self.offset += 1;
                        arguments.push(self.deserialize_expression(executable)?.unwrap());
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::StatementSignature::SpecialCasePop => {
                        let count = executable.script_buffer[self.offset] as usize;
                        self.offset += 1;
                        let mut arguments = Vec::new();
                        for _ in 0..count {
                            arguments.push(self.read_variable_expression(executable).unwrap());
                        }
                        return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
                    }
                    crate::executable::smt_op_codes::StatementSignature::Invalid => {
                        panic!("unhandled statement signature {:?}", def.sig);
                    }
                };

                let mut arguments = Vec::new();
                for i in 0..argument_count {
                    let expr = if i + 1 == var_idx {
                        self.read_variable_expression(executable).unwrap()
                    } else {
                        self.deserialize_expression(executable)?.unwrap()
                    };
                    arguments.push(expr);
                }
                return Ok(Some(PPECommand::PredefinedCall(def, arguments)));
            }
        };
        res
    }

    /// .
    ///
    /// # Panics
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn deserialize_expression(
        &mut self,
        executable: &Executable,
    ) -> Result<Option<PPEExpr>, DeserializationErrorType> {
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
                    log::warn!("Potential error in expression deserialization: No variable table entry for {}, skipping.", id);
                    self.offset += 1;
                    break;
                };
                match val.vtype {
                    VariableType::Function => unsafe {
                        self.offset += 2;
                        let parameters = executable
                            .variable_table
                            .get_value(id)
                            .data
                            .function_value
                            .parameters;
                        let mut arguments = Vec::new();
                        for _ in 0..parameters {
                            if let Some(expr) = self.deserialize_expression(executable)? {
                                arguments.push(expr);
                            }
                        }
                        self.push_expr(PPEExpr::FunctionCall(id, arguments));
                        continue;
                    },
                    VariableType::Procedure => {
                        self.offset += 1;
                        return Err(DeserializationErrorType::GotProcedureCallInExpression(
                            id as i16,
                        ));
                    }
                    _ => {}
                }

                if let Some(var_expr) = self.read_variable_expression(executable) {
                    self.push_expr(var_expr);
                } else {
                    break;
                }
            } else {
                if id == FuncOpCode::CPAR as i16 {
                    self.offset += 1;
                    break;
                }
                let func = -id as usize;
                let func_def = &FUNCTION_DEFINITIONS[func];
                match func_def.args {
                    0x10 => {
                        self.offset += 1;
                        let op = UnaryOp::from_opcode(func_def.opcode);

                        if let Some(unary_expr) = self.pop_expr() {
                            self.push_expr(PPEExpr::UnaryExpression(op, Box::new(unary_expr)));
                        } else {
                            // Some obfuscators try to trick the decompiler by using invalid unary expressions with 0 arguments
                            // PCBoard will just skip these
                            self.report_bug(
                                DeserializationErrorType::TooFewArgumentsForUnaryExpression(op),
                            );
                            return self.deserialize_expression(executable);
                        }
                    }
                    0x11 => {
                        self.offset += 1;
                        let binop = BinOp::from_opcode(func_def.opcode);
                        if self.expr_stack.is_empty() {
                            self.report_bug(DeserializationErrorType::BinaryExpressionStackEmpty(
                                binop,
                            ));
                            return Ok(None);
                        }
                        let r_value = self.pop_expr().unwrap();
                        if self.expr_stack.is_empty() {
                            self.report_bug(DeserializationErrorType::OnlyOneArgumentForBinop(
                                binop,
                            ));
                            self.push_expr(r_value);
                        } else {
                            let l_value = self.pop_expr().unwrap();

                            self.push_expr(PPEExpr::BinaryExpression(
                                binop,
                                Box::new(l_value),
                                Box::new(r_value),
                            ));
                        }
                    }
                    _ => {
                        self.offset += 1;
                        if func_def.args < 0 {
                            self.push_expr(PPEExpr::PredefinedFunctionCall(func_def, vec![]));
                        } else {
                            if (self.expr_stack.len() as i8) < func_def.args {
                                return Err(DeserializationErrorType::TooFewFunctionArguments(
                                    func_def.opcode,
                                    func_def.args,
                                    self.expr_stack.len(),
                                ));
                            }
                            let arguments = self
                                .expr_stack
                                .drain(self.expr_stack.len() - func_def.args as usize..)
                                .collect();
                            self.push_expr(PPEExpr::PredefinedFunctionCall(func_def, arguments));
                        }
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

    fn read_variable_expression(&mut self, executable: &Executable) -> Option<PPEExpr> {
        let id = executable.script_buffer[self.offset];
        self.offset += 1;
        if self.offset >= executable.script_buffer.len() {
            self.report_bug(DeserializationErrorType::IndexOutOfBounds);
            return None;
        }
        let dim = executable.script_buffer[self.offset];
        if !(0..=3).contains(&dim) {
            self.report_bug(DeserializationErrorType::InvalidDimensonizedExpression(
                id, dim,
            ));
            return None;
        }
        self.offset += 1;
        if dim == 0 {
            return Some(PPEExpr::Value(id as usize));
        }
        for _ in 0..dim {
            if let Some(e) = self.deserialize_expression(executable).unwrap() {
                self.push_expr(e);
            }
        }
        if self.expr_stack.len() < dim as usize {
            self.report_bug(DeserializationErrorType::InvalidExpressionStackState);
            return None;
        }
        let dims = self
            .expr_stack
            .drain(self.expr_stack.len() - dim as usize..)
            .collect();
        Some(PPEExpr::Dim(id as usize, dims))
    }
}
