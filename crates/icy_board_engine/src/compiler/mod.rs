pub use ast_transform::*;
pub mod ast_transform;
pub mod user_data;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use thiserror::Error;

use crate::{
    ast::{Ast, AstNode, Expression, Statement},
    executable::{Executable, ExpressionNegator, OpCode, PPECommand, PPEExpr, PPEScript, VariableType},
    parser::{
        lexer::{Spanned, Token},
        ErrorReporter, UserTypeRegistry,
    },
    semantic::{LookupVariabeleTable, SemanticVisitor},
};

use self::expr_compiler::ExpressionCompiler;

pub mod expr_compiler;
pub mod workspace;

#[derive(Error, Debug)]
pub enum CompilationErrorType {
    #[error("Label already used ({0})")]
    LabelAlreadyDefined(String),

    #[error("Label not found ({0})")]
    LabelNotFound(String),

    #[error("Variable name already used ({0})")]
    VariableAlreadyDefined(String),

    #[error("Variable not found ({0})")]
    VariableNotFound(String),

    #[error("Procedure not found ({0})")]
    ProcedureNotFound(String),

    #[error("Function not found ({0})")]
    FunctionNotFound(String),

    #[error("SORT arguments should be one (1) dimensional arrays ({0})")]
    SortArgumentDimensionError(u8),

    #[error("Argument should be a variable ({0})")]
    VariableExpected(usize),

    #[error("Can't assign value to.")]
    InvalidLetVariable,

    #[error("Unused variable ({0})")]
    UnusedVariable(String),

    #[error("Unused FUNCTION/PROCEDURE ({0})")]
    UnusedFunction(String),

    #[error("Missing FUNCTION/PROCEDURE definition. ({0})")]
    MissingImplementation(String),

    #[error("FUNCTION return type does not match with declaration ({0})")]
    ReturnTypeMismatch(String),

    #[error("FUNCTION/PROCEDURE parameters not match with declaration ({0})")]
    ParameterMismatch(String),

    #[error("Indexer called on function or procedure ({0})")]
    IndexerCalledOnFunction(String),

    #[error("Member not found")]
    InvalidMemberReferenceExpression,

    #[error("Type not found.")]
    TypeNotFound,

    #[error("Too few arguments ({0}:{1})")]
    TooFewArguments(String, usize),

    #[error("Too many arguments ({0}:{1})")]
    TooManyArguments(String, usize),
}

#[derive(Error, Debug)]
pub enum CompilationWarningType {
    #[error("Unused label {0}")]
    UnusedLabel(String),

    #[error("Assigning to procdure has no effect.")]
    CannotAssignToProcedure,
}

struct LabelDescriptor {
    pub offset: usize,
}

pub struct PPECompiler {
    lookup_table: LookupVariabeleTable,
    semantic_visitor: SemanticVisitor,

    cur_offset: usize,

    label_table: Vec<LabelDescriptor>,
    label_lookup_table: HashMap<unicase::Ascii<String>, usize>,

    commands: PPEScript,
}

impl PPECompiler {
    pub fn new(language_version: u16, type_registry: UserTypeRegistry, errors: Arc<Mutex<ErrorReporter>>) -> Self {
        let semantic_visitor = SemanticVisitor::new(language_version, errors, type_registry);
        Self {
            lookup_table: LookupVariabeleTable::default(),
            semantic_visitor,
            cur_offset: 0,
            label_table: Vec::new(),
            label_lookup_table: HashMap::new(),
            commands: PPEScript::default(),
        }
    }

    pub fn get_script(&self) -> &PPEScript {
        &self.commands
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn compile(&mut self, asts: &[&Ast]) {
        let mut visted = Vec::new();
        for prg in asts {
            self.semantic_visitor.errors.lock().unwrap().set_file_name(&prg.file_name);
            let prg = prg.visit_mut(&mut AstTransformationVisitor::new(true));
            // println!("{}", prg);
            prg.visit(&mut self.semantic_visitor);
            visted.push(prg);
        }
        self.semantic_visitor.finish();

        self.lookup_table = self.semantic_visitor.generate_variable_table();
        for prg in visted {
            self.semantic_visitor.errors.lock().unwrap().set_file_name(&prg.file_name);
            for d in &prg.nodes {
                match d {
                    AstNode::Function(_func) => {}
                    AstNode::Procedure(_proc) => {}
                    AstNode::FunctionDeclaration(_func) => {}
                    AstNode::ProcedureDeclaration(_proc) => {}
                    AstNode::TopLevelStatement(stmt) => {
                        // may get transformed by the ast transformer.
                        if let Statement::Block(block) = stmt {
                            for s in block.get_statements() {
                                self.compile_add_statement(s);
                            }
                        }
                    }
                    AstNode::Main(block) => {
                        for s in block.get_statements() {
                            self.compile_add_statement(s);
                        }
                    }
                }
            }

            if self.commands.statements.is_empty() || self.commands.statements.last().unwrap().command != PPECommand::End {
                self.commands.add_statement(&mut self.cur_offset, PPECommand::End);
            }

            self.compile_functions(&prg);
        }
        self.fill_labels();
    }

    fn compile_functions(&mut self, prg: &Ast) {
        for imp in &prg.nodes {
            match imp {
                AstNode::Procedure(proc) => {
                    let Some(idx) = self.lookup_table.lookup_variable_index(proc.get_identifier()) else {
                        // unused procedure
                        continue;
                    };
                    self.lookup_table.variable_table.get_var_entry_mut(idx).value.data.procedure_value.start_offset = self.cur_offset as u16 * 2;

                    self.lookup_table.start_compile_function_body(proc.get_identifier());
                    proc.get_statements().iter().for_each(|s| {
                        self.compile_add_statement(s);
                    });
                    self.lookup_table.end_compile_function_body();

                    self.commands.add_statement(&mut self.cur_offset, PPECommand::EndProc);
                    self.commands.add_statement(&mut self.cur_offset, PPECommand::End);
                }
                AstNode::Function(func) => {
                    let Some(idx) = self.lookup_table.lookup_variable_index(func.get_identifier()) else {
                        // unused function
                        continue;
                    };
                    self.lookup_table.variable_table.get_var_entry_mut(idx).value.data.function_value.start_offset = self.cur_offset as u16 * 2;
                    self.lookup_table.start_compile_function_body(func.get_identifier());
                    func.get_statements().iter().for_each(|s| {
                        self.compile_add_statement(s);
                    });
                    self.lookup_table.end_compile_function_body();

                    self.commands.add_statement(&mut self.cur_offset, PPECommand::EndFunc);
                    self.commands.add_statement(&mut self.cur_offset, PPECommand::End);
                }
                _ => {}
            }
        }
    }

    fn compile_add_statement(&mut self, stmt: &Statement) {
        if let Statement::Block(block) = stmt {
            for s in block.get_statements() {
                self.compile_add_statement(s);
            }
            return;
        }
        if let Some(stmt) = self.compile_statement(stmt) {
            self.commands.add_statement(&mut self.cur_offset, stmt);
        }
    }

    fn compile_statement(&mut self, s: &Statement) -> Option<PPECommand> {
        match s {
            Statement::Empty => None,
            Statement::Comment(_) => None,
            Statement::Return(_) => Some(PPECommand::Return),
            Statement::Gosub(gosub_stmt) => Some(PPECommand::Gosub(self.get_label_index(gosub_stmt.get_label_token()))),
            Statement::Goto(goto_stmt) => Some(PPECommand::Goto(self.get_label_index(goto_stmt.get_label_token()))),
            Statement::Label(label) => {
                self.set_label_offset(label.get_label_token());
                None
            }
            Statement::If(if_stmt) => {
                let Statement::Goto(goto_stmt) = if_stmt.get_statement() else {
                    panic!("Invalid if statement without goto.");
                };

                let cond_buffer = self.comp_expr(if_stmt.get_condition()).visit_mut(&mut ExpressionNegator::default());
                Some(PPECommand::IfNot(Box::new(cond_buffer), self.get_label_index(goto_stmt.get_label_token())))
            }

            Statement::VariableDeclaration(_) => None,

            Statement::Let(let_smt) => {
                let var_name = let_smt.get_identifier();
                if let_smt.get_let_variant() != &Token::Eq {
                    panic!("Let variants allowed in output AST.");
                }
                let Some(decl_idx) = self.lookup_variable_index(var_name) else {
                    log::error!("Variable not found: {}", var_name);
                    return None;
                };

                let mut decl = self.lookup_table.variable_table.get_var_entry(decl_idx);
                if decl.header.variable_type == VariableType::Function {
                    unsafe {
                        decl = self
                            .lookup_table
                            .variable_table
                            .get_var_entry(decl.value.data.function_value.return_var as usize);
                    }
                }

                if decl.header.dim != let_smt.get_arguments().len() as u8 {
                    log::error!("Invalid dimensions for variable: {}", var_name.to_string());
                    return None;
                }
                let decl_id = if decl.header.variable_type == VariableType::Function {
                    unsafe { decl.value.data.function_value.return_var as usize }
                } else {
                    decl.header.id
                };
                let variable = if decl.header.dim == 0 {
                    PPEExpr::Value(decl_id)
                } else {
                    let mut arguments = Vec::new();
                    for arg in let_smt.get_arguments() {
                        let expr_buffer = self.comp_expr(arg);
                        arguments.push(expr_buffer);
                    }
                    PPEExpr::Dim(decl_id, arguments)
                };
                let value = self.comp_expr(let_smt.get_value_expression());

                Some(PPECommand::Let(Box::new(variable), Box::new(value)))
            }
            Statement::PredifinedCall(call_stmt) => {
                let def = call_stmt.get_func();
                let mut arguments = Vec::new();
                for arg in call_stmt.get_arguments() {
                    let expr_buffer = self.comp_expr(arg);
                    arguments.push(expr_buffer);
                }

                Some(PPECommand::PredefinedCall(
                    def.opcode.get_definition(), // to de-alias aliases
                    arguments,
                ))
            }
            Statement::Call(call_stmt) => {
                let Some(decl_idx) = self.lookup_variable_index(call_stmt.get_identifier()) else {
                    log::error!("Procedure not found: {}", call_stmt.get_identifier().to_string());
                    return None;
                };
                let mut arguments = Vec::new();
                for arg in call_stmt.get_arguments() {
                    let expr_buffer = self.comp_expr(arg);
                    arguments.push(expr_buffer);
                }

                let decl = self.lookup_table.variable_table.get_var_entry(decl_idx).clone();
                if decl.header.variable_type == VariableType::Procedure {
                    let len = unsafe { decl.value.data.procedure_value.parameters as usize };
                    if !Self::check_arg_count(len, arguments.len(), call_stmt.get_identifier_token()) {
                        return None;
                    }
                    Some(PPECommand::ProcedureCall(decl.header.id, arguments))
                } else if decl.header.variable_type == VariableType::Function {
                    let len = unsafe { decl.value.data.function_value.parameters as usize };
                    if !Self::check_arg_count(len, arguments.len(), call_stmt.get_identifier_token()) {
                        return None;
                    }

                    Some(PPECommand::PredefinedCall(
                        OpCode::EVAL.get_definition(),
                        vec![PPEExpr::FunctionCall(decl.header.id, arguments)],
                    ))
                } else {
                    log::error!("Invalid call to variable: {}", call_stmt.get_identifier());
                    return None;
                }
            }
            Statement::While(_) => panic!("While not allowed in output AST."),
            Statement::Block(_) => panic!("Block not handled by compile statement."),
            Statement::Continue(_) => panic!("Continue not allowed in output AST."),
            Statement::Break(_) => panic!("Break not allowed in output AST."),
            Statement::IfThen(_) => panic!("if then not allowed in output AST."),
            Statement::WhileDo(_) => panic!("do while not allowed in output AST."),
            Statement::RepeatUntil(_) => panic!("repeat until not allowed in output AST."),
            Statement::Loop(_) => panic!("loop not allowed in output AST."),
            Statement::For(_) => panic!("for not allowed in output AST."),
            Statement::Select(_) => panic!("select not allowed in output AST."),
        }
    }

    fn check_arg_count(arg_count_expected: usize, arg_count: usize, identifier_token: &Spanned<Token>) -> bool {
        if arg_count_expected != arg_count {
            log::error!(
                "Invalid number of parameters for {}: expected {}, got {}",
                identifier_token.token,
                arg_count_expected,
                arg_count
            );
            return false;
        }
        true
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn create_executable(&self, version: u16) -> Result<Executable, CompilationErrorType> {
        let mut variable_table = self.lookup_table.variable_table.clone();
        variable_table.set_version(version);
        Ok(Executable {
            version,
            variable_table,
            script_buffer: self.commands.serialize(),
        })
    }

    fn comp_expr(&mut self, expr: &Expression) -> PPEExpr {
        expr.visit(&mut ExpressionCompiler { compiler: self })
    }

    fn get_label_index(&mut self, label_token: &Spanned<Token>) -> usize {
        let Token::Identifier(label) = &label_token.token else {
            panic!("Invalid label token {:?}", label_token);
        };

        if let Some(idx) = self.label_lookup_table.get(&label) {
            *idx
        } else {
            self.define_label_at_cur_pos(label)
        }
    }

    fn define_label_at_cur_pos(&mut self, label: &unicase::Ascii<String>) -> usize {
        let idx: usize = self.label_table.len();
        self.label_lookup_table.insert(label.clone(), idx);
        self.label_table.push(LabelDescriptor { offset: 0 });
        idx
    }

    fn set_label_offset(&mut self, label_token: &Spanned<Token>) {
        let Token::Label(identifier) = &label_token.token else {
            log::error!("Invalid label token {:?}", label_token);
            return;
        };
        if let Some(idx) = self.label_lookup_table.get_mut(identifier) {
            let label_descr = &mut self.label_table[*idx];
            if label_descr.offset > 0 {
                log::error!("Label already defined: {}", identifier);
                return;
            }
            label_descr.offset = self.cur_offset;
        } else {
            let idx = self.define_label_at_cur_pos(identifier);
            self.label_table[idx].offset = self.cur_offset;
        }
    }

    fn lookup_variable_index(&self, get_identifier: &unicase::Ascii<String>) -> Option<usize> {
        self.lookup_table.lookup_variable_index(get_identifier)
    }

    fn fill_labels(&mut self) {
        let last = (self.commands.statements.len() as i32 - 1) as usize;
        for stmt in &mut self.commands.statements {
            match &mut stmt.command {
                PPECommand::IfNot(_, idx) | PPECommand::Goto(idx) | PPECommand::Gosub(idx) => {
                    if let Some(label_descr) = self.label_table.get(*idx) {
                        if label_descr.offset == 0 {
                            *idx = last;
                        } else {
                            *idx = label_descr.offset as usize * 2;
                        }
                    } else {
                        panic!("Label {idx} not found only {} labels defined.", self.label_table.len());
                    }
                }
                _ => {}
            }
        }
    }
}
