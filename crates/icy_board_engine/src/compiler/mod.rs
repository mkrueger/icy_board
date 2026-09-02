pub use ast_transform::*;
use workspace::Workspace;
pub mod ast_transform;
mod enum_lowering;
mod hir_lowering;
mod modules;
pub use modules::lower_modules;
pub mod optimizer;
pub mod user_data;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use thiserror::Error;

use crate::{
    ast::{Ast, AstNode, Expression, OnErrorMode, Statement},
    executable::{Executable, OpCode, PPECommand, PPEScript, RecordField, VariableType},
    hir::{CallId, CodeOffset, HirCommand, HirErrorTarget, HirExpr, HirProgram, LabelId, RoutineId, VariableId},
    parser::{
        ErrorReporter, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
    semantic::{LookupVariabeleTable, SemanticInfo, SemanticVisitor},
};

use self::expr_compiler::HirExpressionResolver;

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

    #[error("Module already defined ({0})")]
    ModuleAlreadyDefined(String),

    #[error("Module not found ({0})")]
    ModuleNotFound(String),

    #[error("Import alias already defined ({0})")]
    ImportAliasAlreadyDefined(String),

    #[error("Module {0} has no member named {1}")]
    ModuleMemberNotFound(String, String),

    #[error("{1} is private to module {0}")]
    PrivateModuleMember(String, String),

    #[error("Module {0} may only declare; it has no program of its own to run")]
    StatementInModule(String),

    #[error("Source item is outside MODULE {0}")]
    ItemOutsideModule(String),

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

    #[error("Compiled program is too large ({0} bytes; maximum is {1})")]
    ProgramTooLarge(usize, usize),

    #[error("Compiled program has too many declarations ({0}; maximum is {1})")]
    TooManyDeclarations(usize, usize),

    #[error("Routine {0} has too many parameters ({1}; maximum is {2})")]
    TooManyRoutineParameters(String, usize, usize),

    #[error("Routine {0} has too many local variables ({1}; maximum is {2})")]
    TooManyRoutineLocals(String, usize, usize),

    #[error("Procedure {0} has a VAR parameter at position {1}; the maximum supported position is {2}")]
    VarParameterOutOfRange(String, usize, usize),

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
    #[error("PPL 4.00 array indexing should use '[' and ']' instead of '(' and ')'")]
    ArrayBracketsRequired,

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
    optimize: bool,
    lookup_table: LookupVariabeleTable,
    semantic_visitor: SemanticVisitor,

    cur_offset: usize,

    label_table: Vec<LabelDescriptor>,
    label_lookup_table: HashMap<unicase::Ascii<String>, usize>,
    foreach_stack: Vec<(usize, usize)>,

    hir_program: HirProgram,
    commands: PPEScript,
}

impl PPECompiler {
    pub fn new(workspace: &Workspace, type_registry: UserTypeRegistry, errors: Arc<Mutex<ErrorReporter>>) -> Self {
        let semantic_visitor = SemanticVisitor::new(workspace, errors, type_registry);
        Self {
            lookup_table: LookupVariabeleTable::default(),
            semantic_visitor,
            optimize: true,
            cur_offset: 0,
            label_table: Vec::new(),
            label_lookup_table: HashMap::new(),
            foreach_stack: Vec::new(),
            runtime: workspace.runtime(),
            hir_program: HirProgram::default(),
            commands: PPEScript::default(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_optimization(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self.semantic_visitor.set_control_flow_liveness(optimize);
        self
    }

    pub fn get_script(&self) -> &PPEScript {
        &self.commands
    }

    pub fn get_hir_program(&self) -> &HirProgram {
        &self.hir_program
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn compile(&mut self, asts: &[&Ast]) {
        self.semantic_visitor.set_modules(asts);
        let lowered = modules::lower_modules(asts, self.semantic_visitor.errors.clone());
        let asts = lowered.iter().collect::<Vec<_>>();
        let mut visted = Vec::new();
        // One transformer for the whole package, so its generated labels stay unique across files.
        let mut transformer = AstTransformationVisitor::new(self.optimize, self.semantic_visitor.type_registry.enums());
        for prg in asts {
            self.semantic_visitor.set_file_name(&prg.file_name);
            let prg = prg.visit_mut(&mut transformer);
            visted.push((prg, transformer.take_loop_counters()));
        }
        // Imported module declarations must be known before the root program is
        // checked, but the root file must remain first when code is emitted.
        for (prg, loop_counters) in visted
            .iter()
            .filter(|(program, _)| program.module.is_some())
            .chain(visted.iter().filter(|(program, _)| program.module.is_none()))
        {
            self.semantic_visitor.set_file_name(&prg.file_name);
            self.semantic_visitor.set_loop_counters(loop_counters.clone());
            prg.visit(&mut self.semantic_visitor);
        }
        self.semantic_visitor.finish();

        for (program, _) in &mut visted {
            *program = program.visit_mut(&mut enum_lowering::EnumLoweringVisitor::new(&self.semantic_visitor.type_registry));
        }

        self.lookup_table = self.semantic_visitor.generate_variable_table();
        for (program, _) in visted.iter().filter(|(program, _)| program.module.is_some()) {
            self.compile_program_statements(program);
        }
        for (prg, _) in visted {
            self.semantic_visitor.set_file_name(&prg.file_name);
            if prg.module.is_none() {
                self.compile_program_statements(&prg);
            }

            if !matches!(self.hir_program.commands.last(), Some(HirCommand::End)) {
                self.add_hir_command(HirCommand::End);
            }

            self.compile_functions(&prg);
        }
        self.fill_labels();
    }

    fn compile_program_statements(&mut self, program: &Ast) {
        for node in &program.nodes {
            match node {
                AstNode::TopLevelStatement(Statement::Block(block)) | AstNode::Main(block) => {
                    self.compile_statement_sequence(block.get_statements());
                }
                AstNode::TopLevelStatement(statement) => self.compile_add_statement(statement),
                AstNode::Function(_)
                | AstNode::Procedure(_)
                | AstNode::FunctionDeclaration(_)
                | AstNode::ProcedureDeclaration(_)
                | AstNode::TypeDeclaration(_)
                | AstNode::EnumDeclaration(_) => {}
            }
        }
    }

    fn compile_functions(&mut self, prg: &Ast) {
        for imp in &prg.nodes {
            match imp {
                AstNode::Procedure(proc) => {
                    let Some(idx) = self.lookup_table.lookup_variable_index(proc.get_identifier()) else {
                        // unused procedure
                        continue;
                    };
                    self.lookup_table.variable_table.get_var_entry_mut(idx).value.data.procedure_value.start_offset =
                        u16::try_from(self.cur_offset.saturating_mul(2)).unwrap_or_default();

                    self.lookup_table.start_compile_function_body(proc.get_identifier());
                    self.compile_statement_sequence(proc.get_statements());
                    self.lookup_table.end_compile_function_body();

                    self.add_hir_command(HirCommand::EndProcedure);
                    self.add_hir_command(HirCommand::End);
                }
                AstNode::Function(func) => {
                    let Some(idx) = self.lookup_table.lookup_variable_index(func.get_identifier()) else {
                        // unused function
                        continue;
                    };
                    self.lookup_table.variable_table.get_var_entry_mut(idx).value.data.function_value.start_offset =
                        u16::try_from(self.cur_offset.saturating_mul(2)).unwrap_or_default();
                    self.lookup_table.start_compile_function_body(func.get_identifier());
                    self.compile_statement_sequence(func.get_statements());
                    self.lookup_table.end_compile_function_body();

                    self.add_hir_command(HirCommand::EndFunction);
                    self.add_hir_command(HirCommand::End);
                }
                _ => {}
            }
        }
    }

    fn compile_statement_sequence(&mut self, statements: &[Statement]) {
        if self.optimize {
            for statement in optimize_statements(statements) {
                self.compile_add_statement(&statement);
            }
        } else {
            for statement in statements {
                self.compile_add_statement(statement);
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
        if let Statement::ForEach(foreach_stmt) = stmt {
            if self.runtime < 400 {
                self.semantic_visitor.errors.lock().unwrap().report_error(
                    foreach_stmt.get_foreach_token().span.clone(),
                    CompilationErrorType::BuiltinNeedsRuntime("FOREACH".to_string(), 400),
                );
                return;
            }
            let Some(variable_index) = self.lookup_variable_index(foreach_stmt.get_identifier()) else {
                log::error!("FOREACH variable not found: {}", foreach_stmt.get_identifier());
                return;
            };
            let variable = self.lookup_table.variable_table.get_var_entry(variable_index).header.id;
            let collection = self.resolve_expr(foreach_stmt.get_collection());
            let end_label = self.label_table.len();
            self.label_table.push(LabelDescriptor { offset: None });
            let command = HirCommand::ForEach(VariableId(variable), collection, LabelId(end_label));
            let body_start = (self.cur_offset + hir_lowering::lower_command(&command).get_size()) * 2;
            self.add_hir_command(command);
            self.foreach_stack.push((body_start, end_label));
            for statement in foreach_stmt.get_statements() {
                self.compile_add_statement(statement);
            }
            self.foreach_stack.pop();
            self.add_hir_command(HirCommand::NextForEach(CodeOffset(body_start)));
            self.label_table[end_label].offset = Some(self.cur_offset);
            return;
        }
        if let Some(command) = self.compile_statement(stmt) {
            self.add_hir_command(command);
        }
    }

    fn add_hir_command(&mut self, command: HirCommand) {
        self.commands.add_statement(&mut self.cur_offset, hir_lowering::lower_command(&command));
        self.hir_program.commands.push(command);
    }

    fn compile_statement(&mut self, s: &Statement) -> Option<HirCommand> {
        match s {
            Statement::Return(_) => Some(HirCommand::Return),
            Statement::Gosub(gosub_stmt) => Some(HirCommand::Gosub(LabelId(self.get_label_index(gosub_stmt.get_label_token())))),
            Statement::Goto(goto_stmt) => Some(HirCommand::Goto(LabelId(self.get_label_index(goto_stmt.get_label_token())))),
            Statement::OnError(on_error_stmt) => {
                let target = match on_error_stmt.get_mode() {
                    OnErrorMode::Off => HirErrorTarget::Off,
                    OnErrorMode::Goto => HirErrorTarget::Goto(LabelId(self.get_label_index(on_error_stmt.get_target_token()))),
                    OnErrorMode::Gosub => HirErrorTarget::Gosub(LabelId(self.get_label_index(on_error_stmt.get_target_token()))),
                    OnErrorMode::Procedure => {
                        let name = on_error_stmt.get_target()?;
                        let Some(decl_idx) = self.lookup_variable_index(name) else {
                            log::error!("Error handler procedure not found: {name}");
                            return None;
                        };
                        HirErrorTarget::Procedure(RoutineId(decl_idx))
                    }
                };
                Some(HirCommand::OnError(target))
            }
            Statement::Label(label) => {
                self.set_label_offset(label.get_label_token());
                None
            }
            Statement::If(if_stmt) => {
                let Statement::Goto(goto_stmt) = if_stmt.get_statement() else {
                    panic!("Invalid if statement without goto.");
                };

                let condition = self.resolve_expr(if_stmt.get_condition());
                Some(HirCommand::ConditionalGoto(
                    condition,
                    LabelId(self.get_label_index(goto_stmt.get_label_token())),
                ))
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
                                if matches!(self.semantic_visitor.function_type_lookup.get(&CallId(call.id)), Some(SemanticInfo::IndexedRecordField(_)))
                        )
                    {
                        let HirExpr::Member(base, member_id) = self.resolve_expr(target) else {
                            return None;
                        };
                        let value = self.resolve_expr(let_smt.get_value_expression());
                        return Some(HirCommand::MemberCall(HirExpr::MemberCall(base, vec![value], member_id)));
                    }
                    return Some(HirCommand::Let(self.resolve_expr(target), self.resolve_expr(let_smt.get_value_expression())));
                }
                if self
                    .semantic_visitor
                    .instance_provider_lookup
                    .contains_key(&let_smt.get_identifier_token().span.start)
                {
                    let base = crate::ast::Expression::Identifier(crate::ast::IdentifierExpression::new(let_smt.get_identifier_token().clone()));
                    let mut variable = self.resolve_expr(&base);
                    for member_token in let_smt.get_members() {
                        let Token::Identifier(member) = &member_token.token else {
                            return None;
                        };
                        let type_id = self.semantic_visitor.user_type_lookup.get(&member_token.span.start)?;
                        let registry = self.semantic_visitor.type_registry.get_type_from_id(*type_id)?;
                        let member_id = registry.member_id_lookup.get(member).copied()?;
                        variable = HirExpr::member(variable, member_id);
                    }
                    let value = self.resolve_expr(let_smt.get_value_expression());
                    let HirExpr::Member(base, member_id) = variable else {
                        return None;
                    };
                    return Some(HirCommand::MemberCall(HirExpr::MemberCall(base, vec![value], member_id)));
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

                let whole_dynamic_array =
                    decl.header.flags & crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY != 0 && let_smt.get_arguments().is_empty();
                if decl.header.dim != let_smt.get_arguments().len() as u8 && !whole_dynamic_array {
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
                let variable = if dim == 0 || whole_dynamic_array {
                    HirExpr::variable(decl_id)
                } else {
                    let mut arguments = Vec::new();
                    for arg in let_smt.get_arguments() {
                        let expr_buffer = self.resolve_expr(arg);
                        arguments.push(expr_buffer);
                    }
                    HirExpr::dim(decl_id, arguments)
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
                        variable = HirExpr::member(variable, field);
                    } else {
                        let registry = self.semantic_visitor.type_registry.get_type_from_id(type_id)?;
                        let field = registry.member_id_lookup.get(member).copied()?;
                        member_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                        variable = HirExpr::member(variable, field);
                    }
                }
                let value = self.resolve_expr(let_smt.get_value_expression());

                Some(HirCommand::Let(variable, value))
            }
            Statement::MemberCall(call_stmt) => {
                // `a.Redim(10)` is `REDIM a, 10`, so it compiles to the statement rather
                // than to a member call.
                if let Expression::FunctionCall(call) = call_stmt.get_expression()
                    && let Some(SemanticInfo::ArrayMemberProc(opcode)) = self.semantic_visitor.function_type_lookup.get(&CallId(call.id)).cloned()
                    && let Expression::MemberReference(member) = call.get_expression()
                {
                    let mut arguments = vec![self.resolve_expr(member.get_expression())];
                    for arg in call.get_arguments() {
                        arguments.push(self.resolve_expr(arg));
                    }
                    return Some(HirCommand::PredefinedCall(opcode, arguments));
                }
                Some(HirCommand::MemberCall(self.resolve_expr(call_stmt.get_expression())))
            }
            Statement::PredifinedCall(call_stmt) => {
                let def = call_stmt.get_func();
                let mut arguments = Vec::new();
                for arg in call_stmt.get_arguments() {
                    let expr_buffer = self.resolve_expr(arg);
                    arguments.push(expr_buffer);
                }

                Some(HirCommand::PredefinedCall(def.opcode, arguments))
            }
            Statement::Call(call_stmt) => {
                let Some(decl_idx) = self.lookup_variable_index(call_stmt.get_identifier()) else {
                    log::error!("Procedure not found: {}", call_stmt.get_identifier());
                    return None;
                };
                let mut arguments = Vec::new();
                for arg in call_stmt.get_arguments() {
                    let expr_buffer = self.resolve_expr(arg);
                    arguments.push(expr_buffer);
                }

                let decl = self.lookup_table.variable_table.get_var_entry(decl_idx).clone();
                if decl.header.variable_type == VariableType::Procedure {
                    let len = unsafe { decl.value.data.procedure_value.parameters as usize };
                    if !Self::check_arg_count(len, arguments.len(), call_stmt.get_identifier_token()) {
                        return None;
                    }
                    Some(HirCommand::ProcedureCall(RoutineId(decl.header.id), arguments))
                } else if decl.header.variable_type == VariableType::Function {
                    let len = unsafe { decl.value.data.function_value.parameters as usize };
                    if !Self::check_arg_count(len, arguments.len(), call_stmt.get_identifier_token()) {
                        return None;
                    }

                    Some(HirCommand::PredefinedCall(OpCode::EVAL, vec![HirExpr::function(decl.header.id, arguments)]))
                } else {
                    log::error!("Invalid call to variable: {}", call_stmt.get_identifier());
                    None
                }
            }
            Statement::While(_) => panic!("While not allowed in output AST."),
            Statement::Block(_) => panic!("Block not handled by compile statement."),
            Statement::Continue(_) => self.foreach_stack.last().map(|(start, _)| HirCommand::NextForEach(CodeOffset(*start))),
            Statement::Break(_) => self.foreach_stack.last().map(|(_, end)| HirCommand::Goto(LabelId(*end))),
            Statement::IfThen(_) => panic!("if then not allowed in output AST."),
            Statement::WhileDo(_) => panic!("do while not allowed in output AST."),
            Statement::RepeatUntil(_) => panic!("repeat until not allowed in output AST."),
            Statement::Loop(_) => panic!("loop not allowed in output AST."),
            Statement::For(_) => panic!("for not allowed in output AST."),
            Statement::ForEach(_) => panic!("foreach is handled by compile_add_statement."),
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
        let declaration_count = self.lookup_table.variable_table.len();
        if declaration_count > i16::MAX as usize {
            return Err(CompilationErrorType::TooManyDeclarations(declaration_count, i16::MAX as usize));
        }
        let script_size = self
            .commands
            .statements
            .iter()
            .map(|statement| statement.command.get_size())
            .sum::<usize>()
            .saturating_mul(2);
        if script_size > i16::MAX as usize {
            return Err(CompilationErrorType::ProgramTooLarge(script_size, i16::MAX as usize));
        }
        let mut variable_table = self.lookup_table.variable_table.clone();
        variable_table.set_version(self.runtime);
        let definitions = self.semantic_visitor.type_registry.user_types();
        let mut used_types = HashSet::new();
        for entry in variable_table.get_entries() {
            if let VariableType::UserData(type_id) = entry.header.variable_type
                && self.semantic_visitor.type_registry.get_user_type_from_id(type_id).is_some()
            {
                used_types.insert(type_id);
            }
        }
        for statement in &self.commands.statements {
            statement.command.collect_user_types(&mut used_types);
        }
        loop {
            let previous = used_types.len();
            for definition in &definitions {
                if !used_types.contains(&(definition.id as u8)) {
                    continue;
                }
                for (_, field) in &definition.fields {
                    if let VariableType::UserData(type_id) = field.variable_type
                        && self.semantic_visitor.type_registry.get_user_type_from_id(type_id).is_some()
                    {
                        used_types.insert(type_id);
                    }
                }
            }
            if used_types.len() == previous {
                break;
            }
        }
        let remap: HashMap<u8, u8> = definitions
            .iter()
            .filter(|definition| used_types.contains(&(definition.id as u8)))
            .enumerate()
            .map(|(index, definition)| (definition.id as u8, (crate::parser::FIRST_USER_TYPE_ID + index) as u8))
            .collect();
        variable_table.remap_user_types(&remap);
        let user_types: Vec<Vec<RecordField>> = definitions
            .iter()
            .filter(|definition| used_types.contains(&(definition.id as u8)))
            .map(|definition| {
                definition
                    .fields
                    .iter()
                    .map(|(_, field)| {
                        let mut field = *field;
                        field.variable_type = self.semantic_visitor.storage_type(field.variable_type);
                        if let VariableType::UserData(type_id) = field.variable_type
                            && let Some(new_id) = remap.get(&type_id)
                        {
                            field.variable_type = VariableType::UserData(*new_id);
                        }
                        field
                    })
                    .collect()
            })
            .collect();
        variable_table.fill_in_records(&user_types);
        let script_buffer = if remap.iter().all(|(old_id, new_id)| old_id == new_id) {
            self.commands.serialize()
        } else {
            let mut script_buffer = Vec::new();
            for statement in &self.commands.statements {
                let mut command = statement.command.clone();
                command.remap_user_types(&remap);
                command.serialize(&mut script_buffer);
            }
            script_buffer
        };
        Ok(Executable {
            runtime: self.runtime,
            variable_table,
            user_types,
            script_buffer,
        })
    }

    fn resolve_expr(&mut self, expr: &Expression) -> HirExpr {
        expr.visit(&mut HirExpressionResolver { compiler: self })
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
                PPECommand::IfNot(_, idx) | PPECommand::Goto(idx) | PPECommand::Gosub(idx) | PPECommand::ForEach(_, _, idx) => {
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
