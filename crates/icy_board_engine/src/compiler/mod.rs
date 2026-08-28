pub use ast_transform::*;
use workspace::Workspace;
pub mod ast_transform;
mod enum_lowering;
pub mod optimizer;
pub mod user_data;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use thiserror::Error;

use crate::{
    ast::{Ast, AstNode, Expression, OnErrorMode, Statement},
    executable::{Executable, ExpressionNegator, OnErrorTarget, OpCode, PPECommand, PPEExpr, PPEScript, RecordField, VariableType},
    parser::{
        ErrorReporter, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
    semantic::{LookupVariabeleTable, SemanticInfo, SemanticVisitor},
};

use self::expr_compiler::ExpressionCompiler;

pub mod expr_compiler;
pub mod workspace;

use optimizer::optimize_statements;

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

    #[error("ON ERROR handler '{0}' takes no parameters or a single ERROR value passed by value")]
    InvalidErrorHandler(String),

    #[error("Function not found ({0})")]
    FunctionNotFound(String),

    #[error("SORT arguments should be one (1) dimensional arrays ({0})")]
    SortArgumentDimensionError(u8),

    #[error("Argument should be a variable ({0})")]
    VariableExpected(usize),

    #[error("Can't assign value to.")]
    InvalidLetVariable,

    #[error("'{0}' can only be read")]
    MemberIsReadOnly(String),

    #[error("'{0}' is a constant, it can only be read")]
    CannotAssignToConstant(String),

    #[error("A constant needs a value the compiler can work out")]
    ConstantValueExpected,

    #[error("Enum {0} has no member named {1}")]
    EnumMemberNotFound(String, String),

    #[error("Can't assign {1} to {0}")]
    EnumAssignmentTypeMismatch(String, String),

    #[error("Can't compare {0} with {1}")]
    EnumComparisonTypeMismatch(String, String),

    #[error("Can't assign {1} to {0}")]
    AssignmentTypeMismatch(VariableType, VariableType),

    #[error("Argument {0} expects {1}, got {2}")]
    ArgumentTypeMismatch(usize, String, String),

    #[error("Operator {0} is not defined for custom types")]
    CustomTypeOperatorNotSupported(crate::ast::BinOp),

    #[error("Can't compare {0} with {1}")]
    ComparisonTypeMismatch(VariableType, VariableType),

    #[error("Whole arrays of custom types cannot be compared")]
    CustomTypeArrayComparisonNotSupported,

    #[error("Record array field '{0}' has a fixed size and cannot be redimensioned")]
    FixedRecordArrayCannotBeRedimmed(String),

    #[error("Record array field '{0}' requires an array value with the same shape")]
    RecordArrayValueExpected(String),

    #[error("Record array field '{0}' expects {1}, got {2}")]
    RecordArrayShapeMismatch(String, String, String),

    #[error("Whole arrays cannot be used as scalar values; index an element first")]
    WholeArrayUsedAsScalar,

    #[error("Record array field '{0}' has rank {1}, but {2} {3} supplied")]
    RecordArrayIndexCount(String, u8, usize, &'static str),

    #[error("Record literal field '{0}' is listed more than once")]
    DuplicateRecordLiteralField(String),

    #[error("Record type {0} has no field '{1}'")]
    UnknownRecordLiteralField(VariableType, String),

    #[error("Record field '{0}' expects {1}, got {2}")]
    RecordLiteralFieldTypeMismatch(String, String, String),

    #[error("Record literals need runtime {0}")]
    RecordLiteralNeedsRuntime(u16),

    #[error("Record field '{0}' of type {1} cannot be stored by record I/O")]
    RecordIoFieldNotSerializable(String, VariableType),

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

    #[error("Passing a FUNCTION/PROCEDURE needs runtime {0}")]
    RoutineReferenceNeedsRuntime(u16),

    #[error("{0} needs runtime {1}")]
    BuiltinNeedsRuntime(String, u16),

    #[error("Indexer called on function or procedure ({0})")]
    IndexerCalledOnFunction(String),

    #[error("Member not found")]
    InvalidMemberReferenceExpression,

    #[error("Record type {0} has no member named {1}")]
    RecordMemberNotFound(VariableType, String),

    #[error("Type not found.")]
    TypeNotFound,

    #[error("Too few arguments ({0}:{1})")]
    TooFewArguments(String, usize),

    #[error("Too many arguments ({0}:{1})")]
    TooManyArguments(String, usize),

    #[error("Function expected")]
    FunctionExpected,

    #[error("Procedure expected")]
    ProcedureExpected,

    #[error("Function used as variable ({0})")]
    FunctionUsedAsVariable(String),

    #[error("'{0}' is a type, and this one has no value of its own to read members from")]
    TypeUsedAsValue(String),

    #[error("'{0}' belongs to the type itself, so it cannot be reached through a value")]
    StaticMemberOnValue(String),

    #[error("Internal error ({0})")]
    InternalError(String),

    #[error("Undeclared procedure was used as function.")]
    ProcedureUsedAsFunction,
}

#[derive(Error, Debug)]
pub enum CompilationWarningType {
    #[error("Unused label {0}")]
    UnusedLabel(String),

    #[error("Assigning to procdure has no effect.")]
    CannotAssignToProcedure,
}

struct LabelDescriptor {
    /// None until the label is placed, since offset 0 is a valid position.
    pub offset: Option<usize>,
}

pub struct PPECompiler {
    runtime: u16,
    lookup_table: LookupVariabeleTable,
    semantic_visitor: SemanticVisitor,

    cur_offset: usize,

    label_table: Vec<LabelDescriptor>,
    label_lookup_table: HashMap<unicase::Ascii<String>, usize>,

    commands: PPEScript,
}

impl PPECompiler {
    pub fn new(workspace: &Workspace, type_registry: UserTypeRegistry, errors: Arc<Mutex<ErrorReporter>>) -> Self {
        let semantic_visitor = SemanticVisitor::new(workspace, errors, type_registry);
        Self {
            lookup_table: LookupVariabeleTable::default(),
            semantic_visitor,
            cur_offset: 0,
            label_table: Vec::new(),
            label_lookup_table: HashMap::new(),
            runtime: workspace.runtime(),
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
        // One transformer for the whole package, so its generated labels stay unique across files.
        let mut transformer = AstTransformationVisitor::new(true, self.semantic_visitor.type_registry.enums());
        for prg in asts {
            self.semantic_visitor.errors.lock().unwrap().set_file_name(&prg.file_name);
            let prg = prg.visit_mut(&mut transformer);
            // println!("{}", prg);
            self.semantic_visitor.set_loop_counters(transformer.take_loop_counters());
            prg.visit(&mut self.semantic_visitor);
            visted.push(prg);
        }
        self.semantic_visitor.finish();

        for program in &mut visted {
            *program = program.visit_mut(&mut enum_lowering::EnumLoweringVisitor::new(&self.semantic_visitor.type_registry));
        }

        self.lookup_table = self.semantic_visitor.generate_variable_table();
        for prg in visted {
            self.semantic_visitor.errors.lock().unwrap().set_file_name(&prg.file_name);
            for d in &prg.nodes {
                match d {
                    AstNode::Function(_func) => {}
                    AstNode::Procedure(_proc) => {}
                    AstNode::FunctionDeclaration(_func) => {}
                    AstNode::ProcedureDeclaration(_proc) => {}
                    // The layout is settled while parsing, nothing is emitted for it.
                    AstNode::TypeDeclaration(_type_decl) => {}
                    AstNode::EnumDeclaration(_enum_decl) => {}
                    AstNode::TopLevelStatement(stmt) => {
                        // may get transformed by the ast transformer.
                        if let Statement::Block(block) = stmt {
                            for s in optimize_statements(block.get_statements()) {
                                self.compile_add_statement(&s);
                            }
                        }
                    }
                    AstNode::Main(block) => {
                        for s in optimize_statements(block.get_statements()) {
                            self.compile_add_statement(&s);
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
                    for s in &optimize_statements(proc.get_statements()) {
                        self.compile_add_statement(s);
                    }
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
                    for s in &optimize_statements(func.get_statements()) {
                        self.compile_add_statement(s);
                    }
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
            Statement::Return(_) => Some(PPECommand::Return),
            Statement::Gosub(gosub_stmt) => Some(PPECommand::Gosub(self.get_label_index(gosub_stmt.get_label_token()))),
            Statement::Goto(goto_stmt) => Some(PPECommand::Goto(self.get_label_index(goto_stmt.get_label_token()))),
            Statement::OnError(on_error_stmt) => {
                let target = match on_error_stmt.get_mode() {
                    OnErrorMode::Off => OnErrorTarget::Off,
                    OnErrorMode::Goto => OnErrorTarget::Goto(self.get_label_index(on_error_stmt.get_target_token())),
                    OnErrorMode::Gosub => OnErrorTarget::Gosub(self.get_label_index(on_error_stmt.get_target_token())),
                    OnErrorMode::Procedure => {
                        let name = on_error_stmt.get_target()?;
                        let Some(decl_idx) = self.lookup_variable_index(name) else {
                            log::error!("Error handler procedure not found: {name}");
                            return None;
                        };
                        OnErrorTarget::Procedure(decl_idx)
                    }
                };
                Some(PPECommand::OnError(target))
            }
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

            // The value took the place of the name before this point, so nothing is left to emit.
            Statement::Empty | Statement::Comment(_) | Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_) => None,

            Statement::Let(let_smt) => {
                let var_name = let_smt.get_identifier();
                assert!(let_smt.get_let_variant() == &Token::Eq, "Let variants allowed in output AST.");
                if let Some(target) = let_smt.get_target_expression() {
                    if let crate::ast::Expression::MemberReference(member) = target
                        && self
                            .semantic_visitor
                            .user_type_lookup
                            .get(&member.get_identifier_token().span.start)
                            .is_some_and(|type_id| !self.semantic_visitor.type_registry.is_record_type(*type_id))
                        && !matches!(
                            member.get_expression(),
                            crate::ast::Expression::FunctionCall(call)
                                if matches!(self.semantic_visitor.function_type_lookup.get(&call.id), Some(SemanticInfo::IndexedRecordField(_)))
                        )
                    {
                        let PPEExpr::Member(base, member_id) = self.comp_expr(target) else {
                            return None;
                        };
                        let value = self.comp_expr(let_smt.get_value_expression());
                        return Some(PPECommand::MemberCall(Box::new(PPEExpr::MemberFunctionCall(base, vec![value], member_id))));
                    }
                    return Some(PPECommand::Let(
                        Box::new(self.comp_expr(target)),
                        Box::new(self.comp_expr(let_smt.get_value_expression())),
                    ));
                }
                if self
                    .semantic_visitor
                    .instance_provider_lookup
                    .contains_key(&let_smt.get_identifier_token().span.start)
                {
                    let base = crate::ast::Expression::Identifier(crate::ast::IdentifierExpression::new(let_smt.get_identifier_token().clone()));
                    let mut variable = base.visit(&mut crate::compiler::expr_compiler::ExpressionCompiler { compiler: self });
                    for member_token in let_smt.get_members() {
                        let Token::Identifier(member) = &member_token.token else {
                            return None;
                        };
                        let type_id = self.semantic_visitor.user_type_lookup.get(&member_token.span.start)?;
                        let registry = self.semantic_visitor.type_registry.get_type_from_id(*type_id)?;
                        let member_id = registry.member_id_lookup.get(member).copied()?;
                        variable = PPEExpr::Member(Box::new(variable), member_id);
                    }
                    let value = self.comp_expr(let_smt.get_value_expression());
                    let PPEExpr::Member(base, member_id) = variable else {
                        return None;
                    };
                    return Some(PPECommand::MemberCall(Box::new(PPEExpr::MemberFunctionCall(base, vec![value], member_id))));
                }
                let Some(decl_idx) = self.lookup_variable_index(var_name) else {
                    log::error!("Variable not found: {var_name}");
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
                    log::error!("Invalid dimensions for variable: {var_name}");
                    return None;
                }
                let decl_id = if decl.header.variable_type == VariableType::Function {
                    unsafe { decl.value.data.function_value.return_var as usize }
                } else {
                    decl.header.id
                };
                let variable_type = decl.header.variable_type;
                let dim = decl.header.dim;
                let variable = if dim == 0 {
                    PPEExpr::Value(decl_id)
                } else {
                    let mut arguments = Vec::new();
                    for arg in let_smt.get_arguments() {
                        let expr_buffer = self.comp_expr(arg);
                        arguments.push(expr_buffer);
                    }
                    PPEExpr::Dim(decl_id, arguments)
                };
                let mut variable = variable;
                let mut member_type = variable_type;
                for member_token in let_smt.get_members() {
                    let Token::Identifier(member) = &member_token.token else {
                        return None;
                    };
                    let VariableType::UserData(type_id) = member_type else {
                        log::error!("Not a record: {var_name}");
                        return None;
                    };
                    if let Some(definition) = self.semantic_visitor.type_registry.get_record_type_from_id(type_id) {
                        let Some(field) = definition.field_index(member) else {
                            log::error!("Field not found: {var_name}.{member}");
                            return None;
                        };
                        member_type = definition.field_type(field).unwrap_or(VariableType::None);
                        variable = PPEExpr::Member(Box::new(variable), field);
                    } else {
                        let registry = self.semantic_visitor.type_registry.get_type_from_id(type_id)?;
                        let field = registry.member_id_lookup.get(member).copied()?;
                        member_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                        variable = PPEExpr::Member(Box::new(variable), field);
                    }
                }
                let value = self.comp_expr(let_smt.get_value_expression());

                Some(PPECommand::Let(Box::new(variable), Box::new(value)))
            }
            Statement::MemberCall(call_stmt) => {
                if let Expression::FunctionCall(call) = call_stmt.get_expression()
                    && let Some(SemanticInfo::StringSplitProc { static_call, default_limit }) =
                        self.semantic_visitor.function_type_lookup.get(&call.id).cloned()
                    && let Expression::MemberReference(member) = call.get_expression()
                {
                    let mut arguments = Vec::new();
                    if !static_call {
                        arguments.push(self.comp_expr(member.get_expression()));
                    }
                    for argument in call.get_arguments() {
                        arguments.push(self.comp_expr(argument));
                    }
                    if default_limit {
                        let zero = self
                            .lookup_table
                            .lookup_constant(&crate::ast::Constant::Integer(0, crate::ast::constant::NumberFormat::Default));
                        arguments.push(PPEExpr::Value(zero));
                    }
                    return Some(PPECommand::PredefinedCall(OpCode::StringSplit.get_definition(), arguments));
                }
                // `a.Redim(10)` is `REDIM a, 10`, so it compiles to the statement rather
                // than to a member call.
                if let Expression::FunctionCall(call) = call_stmt.get_expression()
                    && let Some(SemanticInfo::ArrayMemberProc(opcode)) = self.semantic_visitor.function_type_lookup.get(&call.id).cloned()
                    && let Expression::MemberReference(member) = call.get_expression()
                {
                    let mut arguments = vec![self.comp_expr(member.get_expression())];
                    for arg in call.get_arguments() {
                        arguments.push(self.comp_expr(arg));
                    }
                    return Some(PPECommand::PredefinedCall(opcode.get_definition(), arguments));
                }
                Some(PPECommand::MemberCall(Box::new(self.comp_expr(call_stmt.get_expression()))))
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
                    log::error!("Procedure not found: {}", call_stmt.get_identifier());
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
                    None
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
            Statement::ForEach(_) => panic!("foreach not allowed in output AST."),
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
    pub fn create_executable(&self) -> Result<Executable, CompilationErrorType> {
        let mut variable_table = self.lookup_table.variable_table.clone();
        variable_table.set_version(self.runtime);
        let user_types: Vec<Vec<RecordField>> = self
            .semantic_visitor
            .type_registry
            .user_types()
            .iter()
            .map(|definition| {
                definition
                    .fields
                    .iter()
                    .map(|(_, field)| {
                        let mut field = *field;
                        if self.semantic_visitor.type_registry.is_enum_type(field.variable_type) {
                            field.variable_type = VariableType::Integer;
                        }
                        field
                    })
                    .collect()
            })
            .collect();
        variable_table.fill_in_records(&user_types);
        Ok(Executable {
            runtime: self.runtime,
            variable_table,
            user_types,
            script_buffer: self.commands.serialize(),
        })
    }

    fn comp_expr(&mut self, expr: &Expression) -> PPEExpr {
        expr.visit(&mut ExpressionCompiler { compiler: self })
    }

    fn get_label_index(&mut self, label_token: &Spanned<Token>) -> usize {
        let Token::Identifier(label) = &label_token.token else {
            panic!("Invalid label token {label_token:?}");
        };

        if let Some(idx) = self.label_lookup_table.get(label) {
            *idx
        } else {
            self.define_label_at_cur_pos(label)
        }
    }

    fn define_label_at_cur_pos(&mut self, label: &unicase::Ascii<String>) -> usize {
        let idx: usize = self.label_table.len();
        self.label_lookup_table.insert(label.clone(), idx);
        self.label_table.push(LabelDescriptor { offset: None });
        idx
    }

    fn set_label_offset(&mut self, label_token: &Spanned<Token>) {
        let Token::Label(identifier) = &label_token.token else {
            log::error!("Invalid label token {label_token:?}");
            return;
        };
        if let Some(idx) = self.label_lookup_table.get_mut(identifier) {
            let label_descr = &mut self.label_table[*idx];
            if label_descr.offset.is_some() {
                log::error!("Label already defined: {identifier}");
                return;
            }
            label_descr.offset = Some(self.cur_offset);
        } else {
            let idx = self.define_label_at_cur_pos(identifier);
            self.label_table[idx].offset = Some(self.cur_offset);
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
                        if let Some(offset) = label_descr.offset {
                            *idx = offset * 2;
                        } else {
                            *idx = last;
                        }
                    } else {
                        panic!("Label {idx} not found only {} labels defined.", self.label_table.len());
                    }
                }
                PPECommand::OnError(target) => {
                    if let Some(idx) = target.label_mut() {
                        if let Some(label_descr) = self.label_table.get(*idx) {
                            if let Some(offset) = label_descr.offset {
                                *idx = offset * 2;
                            } else {
                                *idx = last;
                            }
                        } else {
                            panic!("Label {idx} not found only {} labels defined.", self.label_table.len());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
