use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use reconstruct::strip_unused_labels;

use crate::{
    Res,
    ast::{
        Ast, AstNode, BinOp, BinaryExpression, BlockStatement, BreakStatement, CommentAstNode, Constant, ConstantExpression, ContinueStatement, Expression,
        ForEachStatement, FunctionCallExpression, FunctionDeclarationAstNode, FunctionImplementation, GosubStatement, GotoStatement, IdentifierExpression,
        IfStatement, IndexerExpression, LabelStatement, LetStatement, MemberCallStatement, MemberReferenceExpression, OnErrorMode, OnErrorStatement,
        ParameterSpecifier, ParensExpression, PredefinedCallStatement, ProcedureCallStatement, ProcedureDeclarationAstNode, ProcedureImplementation, Statement,
        TypeDeclarationAstNode, TypeFieldSpecifier, UnaryExpression, UnaryOp, VariableDeclarationStatement, VariableParameterSpecifier, VariableSpecifier,
        constant::NumberFormat,
    },
    compiler::{user_data::UserDataEntry, workspace::Workspace},
    executable::{
        DeserializationError, DeserializationErrorType, EntryType, Executable, FuncOpCode, OpCode, PPECommand, PPEExpr, PPEScript, PPEVisitor,
        StatementDefinition, TableEntry, VariableType,
    },
    parser::{
        ErrorReporter, UserTypeRegistry, is_user_declared_type,
        lexer::{Spanned, Token},
    },
    semantic::SemanticVisitor,
};

use self::evaluation_visitor::OptimizationVisitor;

pub mod evaluation_visitor;
pub mod reconstruct;
pub mod relabel_visitor;
pub mod rename_visitor;

#[cfg(test)]
pub mod test_evaluation_visitor;

pub struct DecompilerIssue {
    pub byte_offset: usize,
    pub bug: DeserializationErrorType,
}

/// The name a record gets in the output. The PPE stores field types only, so
/// every name here is made up.
fn user_type_name(index: usize) -> unicase::Ascii<String> {
    unicase::Ascii::new(format!("TYPE{:03}", index + 1))
}

fn user_field_name(index: usize) -> unicase::Ascii<String> {
    unicase::Ascii::new(format!("FIELD{:03}", index + 1))
}

/// The board objects, plus a stand-in declaration for every record the PPE carries.
fn build_type_registry(executable: &Executable) -> UserTypeRegistry {
    let registry = UserTypeRegistry::icy_board_registry();
    for (i, fields) in executable.user_types.iter().enumerate() {
        let fields = fields.iter().enumerate().map(|(j, field)| (user_field_name(j), *field)).collect();
        registry.declare_user_type(user_type_name(i), fields);
    }
    registry
}

#[derive(Default)]
pub struct Decompiler {
    executable: Executable,
    script: PPEScript,

    functions: Vec<AstNode>,

    label_lookup: HashMap<usize, usize>,
    used_labels: HashSet<usize>,

    function_lookup: HashMap<usize, usize>,
    cur_ptr: usize,
    issues: Vec<DecompilerIssue>,
    optimize_output: bool,
    type_registry: UserTypeRegistry,
}

impl Decompiler {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(executable: Executable, optimize_output: bool) -> Result<Self, DeserializationError> {
        let script = PPEScript::from_ppe_file(&executable)?;
        let type_registry = build_type_registry(&executable);
        Ok(Self {
            executable,
            script,
            label_lookup: HashMap::new(),
            function_lookup: HashMap::new(),
            used_labels: HashSet::new(),
            functions: Vec::new(),
            cur_ptr: 0,
            issues: Vec::new(),
            optimize_output,
            type_registry,
        })
    }

    fn analyze_labels(&mut self) -> HashMap<usize, usize> {
        let mut labels = HashSet::new();

        for statement in &self.script.statements {
            match statement.command {
                PPECommand::Goto(label) | PPECommand::Gosub(label) | PPECommand::IfNot(_, label) => {
                    labels.insert(label);
                }
                PPECommand::OnError(target) => {
                    if let Some(label) = target.label() {
                        labels.insert(label);
                    }
                }
                _ => {}
            }
        }
        let mut label_list = labels.into_iter().collect::<Vec<usize>>();
        label_list.sort_unstable();

        let mut label_offsets = HashMap::new();
        for (i, label) in label_list.iter().enumerate() {
            label_offsets.insert(*label, i);
        }
        label_offsets
    }

    /// Returns the decompile of this [`Decompiler`].
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn decompile(&mut self) -> Res<Ast> {
        self.label_lookup = self.analyze_labels();

        {
            let mut visitor = VariableConstantVisitor {
                executable: &mut self.executable,
            };
            self.script.visit(&mut visitor);
        }

        self.executable.variable_table.analyze_usage(&self.script);
        self.executable.variable_table.analyze_locals();
        self.executable.variable_table.generate_names();

        let mut ast = Ast::default();

        self.generate_type_declarations(&mut ast);
        self.generate_function_declarations(&mut ast);
        self.generate_global_variable_declarations(&mut ast);

        let mut statements = Vec::new();
        while self.cur_ptr < self.script.statements.len() {
            let statement = &self.script.statements[self.cur_ptr];
            let byte_offset = statement.span.start * 2;

            if let Some(func) = self.function_lookup.get(&byte_offset) {
                self.parse_function(*func);
                continue;
            }

            if matches!(statement.command, PPECommand::ForEach(_, _, _)) {
                statements.push(self.decompile_foreach());
                continue;
            }

            if self.label_lookup.contains_key(&(byte_offset)) {
                let label = self.get_label_name(byte_offset);
                self.used_labels.insert(byte_offset);
                statements.push(LabelStatement::create_empty_statement(label));
            }
            if let Some(bugs) = self.script.bugged_offsets.get_mut(&statement.span.start) {
                for bug in bugs.drain(..) {
                    self.issues.push(DecompilerIssue { byte_offset, bug: bug.clone() });
                    statements.push(CommentAstNode::create_empty_statement(format!(
                        " PPLC bug use detected in next statement: {bug}"
                    )));
                }
            }
            statements.push(self.decompile_statement(&statement.command));
            self.cur_ptr += 1;
        }
        while let Some(Statement::PredifinedCall(c)) = statements.last() {
            if c.get_func().opcode != OpCode::END || statements.len() <= 1 {
                break;
            }
            statements.pop();
        }

        if !self.functions.is_empty() {
            statements.push(PredefinedCallStatement::create_empty_statement(OpCode::END.get_definition(), Vec::new()));
        }

        // Generate exit label - there is a case where this is needed
        // Gets removed if not used
        statements.push(LabelStatement::create_empty_statement(unicase::Ascii::new("EXIT_LABEL".to_string())));
        for offset in self.label_lookup.keys() {
            if !self.used_labels.contains(offset) {
                statements.push(LabelStatement::create_empty_statement(self.get_label_name(*offset)));
            }
        }
        ast.nodes.push(AstNode::Main(BlockStatement::empty(statements)));

        ast.nodes.append(&mut self.functions);

        for (k, bugs) in &self.script.bugged_offsets {
            for bug in bugs {
                ast.nodes.push(AstNode::TopLevelStatement(CommentAstNode::create_empty_statement(format!(
                    "{k:04X}: statement: {bug}"
                ))));
            }
        }

        if !self.script.bugged_offsets.is_empty() {
            ast.nodes.push(AstNode::TopLevelStatement(CommentAstNode::create_empty_statement(format!(
                " {} error(s) detected while decompiling",
                self.script.bugged_offsets.len(),
            ))));
        }

        Ok(ast)
    }

    fn generate_global_variable_declarations(&mut self, ast: &mut Ast) {
        for var in self.executable.variable_table.get_entries() {
            if let EntryType::Variable = var.entry_type {
                let var_decl = generate_variable_declaration(var, self.type_token(var.header.variable_type));
                ast.nodes.push(AstNode::TopLevelStatement(var_decl));
            }
        }
    }

    /// The records the PPE declares, under invented names.
    fn generate_type_declarations(&self, ast: &mut Ast) {
        for (i, fields) in self.executable.user_types.iter().enumerate() {
            let fields = fields
                .iter()
                .enumerate()
                .map(|(j, field)| {
                    let dimensions = [field.vector_size, field.matrix_size, field.cube_size]
                        .into_iter()
                        .take(field.dim as usize)
                        .map(usize::from)
                        .collect();
                    TypeFieldSpecifier::new(
                        self.type_token(field.variable_type),
                        field.variable_type,
                        VariableSpecifier::empty(user_field_name(j), dimensions),
                    )
                })
                .collect();
            ast.nodes.push(AstNode::TypeDeclaration(TypeDeclarationAstNode::new(
                Spanned::create_empty(Token::Type),
                Spanned::create_empty(Token::Identifier(user_type_name(i))),
                fields,
                Spanned::create_empty(Token::EndType),
            )));
        }
    }

    /// The name a type is written under. Only user types need one - everything
    /// else has a keyword.
    fn type_token(&self, variable_type: VariableType) -> Spanned<Token> {
        let name = match variable_type {
            VariableType::UserData(id) => self.type_name(id),
            _ => None,
        };
        let name = name.unwrap_or_else(|| unicase::Ascii::new(variable_type.to_string()));
        Spanned::create_empty(Token::Identifier(name))
    }

    fn type_name(&self, type_id: u8) -> Option<unicase::Ascii<String>> {
        if is_user_declared_type(type_id) {
            return self.type_registry.get_user_type_from_id(type_id).map(|def| def.name);
        }
        self.type_registry
            .registered_types
            .iter()
            .find(|(_, vt)| **vt == VariableType::UserData(type_id))
            .map(|(name, _)| name.clone())
    }

    fn static_receiver_type(&self, expr: &PPEExpr) -> Option<u8> {
        let PPEExpr::PredefinedFunctionCall(definition, arguments) = expr else {
            return None;
        };
        if definition.opcode != FuncOpCode::StaticReceiver {
            return None;
        }
        let PPEExpr::Value(constant_id) = arguments.first()? else {
            return None;
        };
        u8::try_from(self.executable.variable_table.try_get_entry(*constant_id)?.value.as_int()).ok()
    }

    /// The name and type of member `id` of whatever `base` evaluates to.
    fn resolve_member(&self, base: &PPEExpr, id: usize) -> Option<(unicase::Ascii<String>, VariableType)> {
        let VariableType::UserData(type_id) = self.expression_type(base)? else {
            return None;
        };
        if self.type_registry.is_record_type(type_id) {
            return self
                .type_registry
                .get_record_type_from_id(type_id)?
                .fields
                .get(id)
                .map(|(name, field)| (name.clone(), field.variable_type));
        }
        let registry = self.type_registry.get_type_from_id(type_id)?;
        match registry.id_table.get(id)? {
            UserDataEntry::Field(name) | UserDataEntry::Getter(name) => Some((name.clone(), *registry.fields.get(name)?)),
            UserDataEntry::Function(name) => Some((name.clone(), registry.functions.get(name)?.return_type)),
            UserDataEntry::Procedure(name) => Some((name.clone(), VariableType::None)),
        }
    }

    fn member_name(&self, base: &PPEExpr, id: usize) -> unicase::Ascii<String> {
        self.resolve_member(base, id)
            .map_or_else(|| unicase::Ascii::new(format!("MEMBER{:03}", id + 1)), |(name, _)| name)
    }

    fn convert_to_type(&self, expr: Expression, expected: VariableType) -> Expression {
        let VariableType::UserData(type_id) = expected else {
            return expr;
        };
        let Some(enum_definition) = self.type_registry.get_enum_from_id(type_id) else {
            return expr;
        };
        let Expression::Const(constant) = &expr else {
            return expr;
        };
        let Constant::Integer(value, _) = constant.get_constant_value() else {
            return expr;
        };
        let Some(variant) = enum_definition.variant_name(*value).cloned() else {
            return expr;
        };
        MemberReferenceExpression::create_empty_expression(IdentifierExpression::create_empty_expression(enum_definition.name), variant)
    }

    fn convert_argument(&self, expr: Expression, arg: &crate::executable::ArgumentDefinition) -> Expression {
        convert_argument(self.convert_to_type(expr, arg.arg_type), arg)
    }

    fn member_parameter_type(&self, base: &PPEExpr, member_id: usize, parameter: usize) -> Option<VariableType> {
        let base = if let PPEExpr::Member(inner, _) = base { inner.as_ref() } else { base };
        let VariableType::UserData(type_id) = self.expression_type(base)? else {
            return None;
        };
        let registry = self.type_registry.get_type_from_id(type_id)?;
        match registry.id_table.get(member_id)? {
            UserDataEntry::Function(name) => registry.functions.get(name)?.parameters.get(parameter).copied(),
            UserDataEntry::Procedure(name) => registry.procedures.get(name)?.parameters.get(parameter).copied(),
            _ => None,
        }
    }

    fn convert_member_argument(&self, expr: Expression, expected: VariableType) -> Expression {
        let expr = self.convert_to_type(expr, expected);
        if expected == VariableType::Boolean {
            Statement::try_boolean_conversion(&expr)
        } else {
            expr
        }
    }

    /// What an expression evaluates to, as far as the variable table and the type
    /// table can say. Only member access needs this.
    fn expression_type(&self, expr: &PPEExpr) -> Option<VariableType> {
        if let Some(type_id) = self.static_receiver_type(expr) {
            return Some(VariableType::UserData(type_id));
        }
        match expr {
            PPEExpr::Value(id) | PPEExpr::Dim(id, _) => Some(self.executable.variable_table.try_get_entry(*id)?.header.variable_type),
            PPEExpr::Member(base, id) => self.resolve_member(base, *id).map(|(_, t)| t),
            PPEExpr::IndexedMember(base, id, _) => self.resolve_member(base, *id).map(|(_, t)| t),
            PPEExpr::MemberFunctionCall(base, _, id) => {
                // Codegen leaves the member reference in the base, so reach past it.
                let base = if let PPEExpr::Member(inner, _) = base.as_ref() { inner } else { base };
                self.resolve_member(base, *id).map(|(_, t)| t)
            }
            PPEExpr::FunctionCall(id, _) => {
                let entry = self.executable.variable_table.try_get_entry(*id)?;
                if entry.header.variable_type != VariableType::Function {
                    return None;
                }
                let return_var = unsafe { entry.value.data.function_value.return_var } as usize;
                Some(self.executable.variable_table.try_get_entry(return_var)?.header.variable_type)
            }
            PPEExpr::PredefinedFunctionCall(def, _) => Some(def.return_type),
            _ => None,
        }
    }

    fn generate_function_declarations(&mut self, ast: &mut Ast) {
        for entry in self.executable.variable_table.get_entries() {
            match entry.entry_type {
                EntryType::Function | EntryType::Procedure => {
                    // A zero start offset is used to stop decompilers finding the body.
                    if unsafe { entry.value.data.procedure_value.start_offset } == 0 {
                        self.issues.push(DecompilerIssue {
                            byte_offset: 0,
                            bug: DeserializationErrorType::RoutineWithoutStartOffset(entry.name.clone()),
                        });
                        ast.nodes.push(AstNode::TopLevelStatement(CommentAstNode::create_empty_statement(format!(
                            " {} has no start offset, its body is left inline in the main program",
                            entry.name
                        ))));
                        continue;
                    }
                    self.function_lookup
                        .insert(unsafe { entry.value.data.procedure_value.start_offset as usize }, entry.header.id);

                    if entry.header.variable_type == VariableType::Function {
                        let parameters = self.generate_parameter_list(entry);
                        let return_value = self
                            .executable
                            .variable_table
                            .get_var_entry(unsafe { entry.value.data.function_value.return_var as usize });
                        let func_decl = FunctionDeclarationAstNode::new(
                            Spanned::create_empty(Token::Declare),
                            Spanned::create_empty(Token::Function),
                            Spanned::create_empty(Token::Identifier(unicase::Ascii::new(entry.name.clone()))),
                            Spanned::create_empty(Token::LPar),
                            parameters,
                            Spanned::create_empty(Token::RPar),
                            self.type_token(return_value.header.variable_type),
                            return_value.header.variable_type,
                            return_value.header.dim,
                        );
                        ast.nodes.push(AstNode::FunctionDeclaration(func_decl));
                    } else {
                        let parameters = self.generate_parameter_list(entry);
                        let proc_decl = ProcedureDeclarationAstNode::empty(unicase::Ascii::new(entry.name.clone()), parameters);
                        ast.nodes.push(AstNode::ProcedureDeclaration(proc_decl));
                    }
                }
                _ => {}
            }
        }
    }

    fn decompile_expression(&self, expression: &PPEExpr) -> Expression {
        match expression {
            // A PPE that cannot be read should come out marked, not bring the
            // decompiler down with it.
            PPEExpr::Invalid => ConstantExpression::create_empty_expression(Constant::String("ERROR IN EXPRESSION invalid expression".to_string())),
            PPEExpr::Value(id) => unsafe {
                let Some(entry) = self.executable.variable_table.try_get_entry(*id) else {
                    return ConstantExpression::create_empty_expression(Constant::String(format!("ERROR IN EXPRESSION can't read table index : {:04X}", *id)));
                };
                if matches!(entry.value.get_type(), VariableType::UserData(_)) {
                    IdentifierExpression::create_empty_expression(unicase::Ascii::new(entry.name.clone()))
                } else if entry.entry_type == EntryType::Constant {
                    let constant = match entry.value.get_type() {
                        VariableType::BigStr | VariableType::String => Constant::String(entry.value.as_string()),
                        VariableType::Float => Constant::Double(entry.value.data.float_value as f64),
                        VariableType::Double => Constant::Double(entry.value.data.double_value),
                        VariableType::Boolean => Constant::Boolean(entry.value.as_bool()),
                        VariableType::Unsigned => Constant::Unsigned(entry.value.data.unsigned_value, NumberFormat::Default),
                        //VariableType::Integer |
                        _ => Constant::Integer(entry.value.as_int(), NumberFormat::Default),
                    };
                    ConstantExpression::create_empty_expression(constant)
                } else {
                    IdentifierExpression::create_empty_expression(unicase::Ascii::new(entry.name.clone()))
                }
            },
            PPEExpr::RoutineReference(id) => IdentifierExpression::create_empty_expression(self.get_variable_name(*id)),
            PPEExpr::RecordLiteral(type_id, fields) => {
                let type_name = self
                    .type_name(*type_id)
                    .unwrap_or_else(|| unicase::Ascii::new(format!("TYPE{:03}", *type_id as usize - 99)));
                let literal_fields = fields
                    .iter()
                    .map(|(field_id, value)| {
                        crate::ast::RecordLiteralField::new(
                            Spanned::create_empty(Token::Identifier(user_field_name(*field_id))),
                            self.decompile_expression(value),
                        )
                    })
                    .collect();
                Expression::RecordLiteral(crate::ast::RecordLiteralExpression::new(
                    Spanned::create_empty(Token::Identifier(type_name)),
                    VariableType::UserData(*type_id),
                    Spanned::create_empty(Token::LBrace),
                    literal_fields,
                    Spanned::create_empty(Token::RBrace),
                ))
            }
            PPEExpr::Member(expr, id) => MemberReferenceExpression::create_empty_expression(self.decompile_expression(expr), self.member_name(expr, *id)),
            PPEExpr::IndexedMember(expr, id, dimensions) => FunctionCallExpression::create_empty_expression(
                MemberReferenceExpression::create_empty_expression(self.decompile_expression(expr), self.member_name(expr, *id)),
                dimensions.iter().map(|dimension| self.decompile_expression(dimension)).collect(),
            ),
            PPEExpr::MemberFunctionCall(expr, args, id) => {
                let base = self.decompile_expression(expr);
                // Codegen writes the member reference into the base as well, so only
                // build one when it is missing.
                let callee = if matches!(base, Expression::MemberReference(_)) {
                    base
                } else {
                    MemberReferenceExpression::create_empty_expression(base, self.member_name(expr, *id))
                };
                FunctionCallExpression::create_empty_expression(
                    callee,
                    args.iter()
                        .enumerate()
                        .map(|(index, argument)| {
                            let argument = self.decompile_expression(argument);
                            self.member_parameter_type(expr, *id, index)
                                .map_or(argument.clone(), |expected| self.convert_member_argument(argument, expected))
                        })
                        .collect(),
                )
            }
            PPEExpr::UnaryExpression(op, expr) => {
                let mut expr = self.decompile_expression(expr);

                if matches!(expr, Expression::Binary(_)) {
                    expr = ParensExpression::create_empty_expression(expr);
                }

                let expr = UnaryExpression::create_empty_expression(*op, expr);
                if self.optimize_output {
                    expr.visit_mut(&mut OptimizationVisitor::default())
                } else {
                    expr
                }
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left_type = self.expression_type(left);
                let right_type = self.expression_type(right);
                let left = self.decompile_expression(left);
                let right = self.decompile_expression(right);
                let left = right_type.map_or(left.clone(), |expected| self.convert_to_type(left, expected));
                let right = left_type.map_or(right.clone(), |expected| self.convert_to_type(right, expected));
                let left = add_parens_if_required(*op, left);
                let right = add_parens_if_required(*op, right);

                let expr = BinaryExpression::create_empty_expression(*op, left, right);
                if self.optimize_output {
                    expr.visit_mut(&mut OptimizationVisitor::default())
                } else {
                    expr
                }
            }
            PPEExpr::Dim(id, dims) => {
                IndexerExpression::create_empty_expression(self.get_variable_name(*id), dims.iter().map(|e| self.decompile_expression(e)).collect())
            }
            PPEExpr::PredefinedFunctionCall(f, args) => {
                if f.opcode == FuncOpCode::ArrayValueAt
                    && let [array, index] = args.as_slice()
                {
                    return FunctionCallExpression::create_empty_expression(
                        MemberReferenceExpression::create_empty_expression(self.decompile_expression(array), unicase::Ascii::new("<get>".to_string())),
                        vec![self.decompile_expression(index)],
                    );
                }
                if f.opcode == FuncOpCode::StringCharAt
                    && let [receiver, index] = args.as_slice()
                {
                    if let PPEExpr::Value(receiver) = receiver {
                        return IndexerExpression::create_empty_expression(self.get_variable_name(*receiver), vec![self.decompile_expression(index)]);
                    }
                    return FunctionCallExpression::create_empty_expression(
                        MemberReferenceExpression::create_empty_expression(self.decompile_expression(receiver), unicase::Ascii::new("<get>".to_string())),
                        vec![self.decompile_expression(index)],
                    );
                }
                if let Some(type_id) = self.static_receiver_type(expression) {
                    return IdentifierExpression::create_empty_expression(
                        self.type_name(type_id).unwrap_or_else(|| unicase::Ascii::new(format!("TYPE{type_id}"))),
                    );
                }
                let instance_member = match f.opcode {
                    FuncOpCode::StringFindFrom => Some("Find"),
                    FuncOpCode::StringFindComparison => Some("Find"),
                    FuncOpCode::StringFindLastFrom => Some("FindLast"),
                    FuncOpCode::StringFindLastComparison => Some("FindLast"),
                    FuncOpCode::StringContains => Some("Contains"),
                    FuncOpCode::StringContainsComparison => Some("Contains"),
                    FuncOpCode::StringStartsWith => Some("StartsWith"),
                    FuncOpCode::StringStartsWithComparison => Some("StartsWith"),
                    FuncOpCode::StringEndsWith => Some("EndsWith"),
                    FuncOpCode::StringEndsWithComparison => Some("EndsWith"),
                    FuncOpCode::StringCount => Some("Count"),
                    FuncOpCode::StringCountComparison => Some("Count"),
                    FuncOpCode::StringEquals | FuncOpCode::StringEqualsComparison => Some("Equals"),
                    FuncOpCode::StringSplit | FuncOpCode::StringSplitLimit => Some("Split"),
                    FuncOpCode::StringTrim => Some("Trim"),
                    FuncOpCode::StringTrimStart => Some("TrimStart"),
                    FuncOpCode::StringTrimEnd => Some("TrimEnd"),
                    FuncOpCode::StringTrimChars => Some("Trim"),
                    FuncOpCode::StringTrimStartChars => Some("TrimStart"),
                    FuncOpCode::StringTrimEndChars => Some("TrimEnd"),
                    _ => None,
                };
                if let Some(member) = instance_member
                    && let Some((receiver, arguments)) = args.split_first()
                {
                    let mut arguments: Vec<_> = arguments.iter().map(|argument| self.decompile_expression(argument)).collect();
                    if matches!(
                        f.opcode,
                        FuncOpCode::StringFindComparison
                            | FuncOpCode::StringFindLastComparison
                            | FuncOpCode::StringContainsComparison
                            | FuncOpCode::StringStartsWithComparison
                            | FuncOpCode::StringEndsWithComparison
                            | FuncOpCode::StringCountComparison
                            | FuncOpCode::StringEqualsComparison
                    ) && let Some(comparison) = arguments.pop()
                    {
                        arguments.push(self.convert_to_type(comparison, VariableType::UserData(crate::parser::STRING_COMPARISON_ENUM_ID)));
                    }
                    return FunctionCallExpression::create_empty_expression(
                        MemberReferenceExpression::create_empty_expression(self.decompile_expression(receiver), unicase::Ascii::new(member.to_string())),
                        arguments,
                    );
                }
                let static_member = match f.opcode {
                    FuncOpCode::StringJoin => Some("Join"),
                    FuncOpCode::StringRepeat => Some("Repeat"),
                    _ => None,
                };
                if let Some(member) = static_member {
                    return FunctionCallExpression::create_empty_expression(
                        MemberReferenceExpression::create_empty_expression(
                            IdentifierExpression::create_empty_expression(unicase::Ascii::new("STRING".to_string())),
                            unicase::Ascii::new(member.to_string()),
                        ),
                        args.iter().map(|argument| self.decompile_expression(argument)).collect(),
                    );
                }
                FunctionCallExpression::create_empty_expression(
                    IdentifierExpression::create_empty_expression(unicase::Ascii::new(f.name.to_string())),
                    args.iter()
                        .enumerate()
                        .map(|(i, e)| {
                            let expr = self.decompile_expression(e);
                            if let Some(args) = &f.args
                                && let Some(arg) = args.get(i)
                            {
                                return self.convert_argument(expr, arg);
                            }
                            expr
                        })
                        .collect(),
                )
            }
            PPEExpr::FunctionCall(f, args) => FunctionCallExpression::create_empty_expression(
                IdentifierExpression::create_empty_expression(self.get_variable_name(*f)),
                args.iter().map(|e| self.decompile_expression(e)).collect(),
            ),
        }
    }

    fn decompile_statement(&self, statement: &PPECommand) -> Statement {
        match statement {
            PPECommand::EndFunc | PPECommand::EndProc | PPECommand::End => {
                PredefinedCallStatement::create_empty_statement(OpCode::END.get_definition(), Vec::new())
            }
            PPECommand::Return => PredefinedCallStatement::create_empty_statement(OpCode::RETURN.get_definition(), Vec::new()),
            PPECommand::Stop => PredefinedCallStatement::create_empty_statement(OpCode::STOP.get_definition(), Vec::new()),
            PPECommand::Goto(label) => GotoStatement::create_empty_statement(self.get_label_name(*label)),
            PPECommand::Gosub(label) => GosubStatement::create_empty_statement(self.get_label_name(*label)),
            PPECommand::OnError(target) => match target {
                crate::executable::OnErrorTarget::Off => OnErrorStatement::create_empty_statement(OnErrorMode::Off, unicase::Ascii::new("OFF".to_string())),
                crate::executable::OnErrorTarget::Goto(label) => OnErrorStatement::create_empty_statement(OnErrorMode::Goto, self.get_label_name(*label)),
                crate::executable::OnErrorTarget::Gosub(label) => OnErrorStatement::create_empty_statement(OnErrorMode::Gosub, self.get_label_name(*label)),
                crate::executable::OnErrorTarget::Procedure(id) => {
                    OnErrorStatement::create_empty_statement(OnErrorMode::Procedure, self.get_variable_name(*id))
                }
            },

            PPECommand::IfNot(expr, label) => {
                let expr = self.decompile_expression(expr);
                let expr = Statement::try_boolean_conversion(&expr);
                IfStatement::create_empty_statement(expr.negate_expression(), GotoStatement::create_empty_statement(self.get_label_name(*label)))
            }
            PPECommand::ForEach(variable, collection, end) => CommentAstNode::create_empty_statement(format!(
                " FOREACH {} IN {} END {end:04X}",
                self.get_variable_name(*variable),
                self.decompile_expression(collection)
            )),
            PPECommand::NextForEach(start) => CommentAstNode::create_empty_statement(format!(" NEXT FOREACH {start:04X}")),
            PPECommand::ProcedureCall(p, args) => {
                ProcedureCallStatement::create_empty_statement(self.get_variable_name(*p), args.iter().map(|e| self.decompile_expression(e)).collect())
            }
            PPECommand::MemberCall(expr) => MemberCallStatement::create_empty_statement(self.decompile_expression(expr)),
            PPECommand::PredefinedCall(p, args) => {
                if matches!(p.opcode, OpCode::StringSplit | OpCode::RegexSplit) && args.len() == 4 {
                    return MemberCallStatement::create_empty_statement(FunctionCallExpression::create_empty_expression(
                        MemberReferenceExpression::create_empty_expression(self.decompile_expression(&args[0]), unicase::Ascii::new("Split".to_string())),
                        args[1..].iter().map(|argument| self.decompile_expression(argument)).collect(),
                    ));
                }
                PredefinedCallStatement::create_empty_statement(
                    p,
                    args.iter()
                        .enumerate()
                        .map(|(i, e)| {
                            let expr = self.decompile_expression(e);
                            if let Some(args) = &p.args
                                && let Some(arg) = args.get(i)
                            {
                                return self.convert_argument(expr, arg);
                            }
                            expr
                        })
                        .collect(),
                )
            }
            PPECommand::Let(left, expr) => {
                // A member assignment keeps the fields it walks through beside the base.
                let mut members = Vec::new();
                let mut base: &PPEExpr = left;
                while let PPEExpr::Member(inner, id) = base {
                    members.push(Spanned::create_empty(Token::Identifier(self.member_name(inner, *id))));
                    base = inner.as_ref();
                }
                members.reverse();

                let base_expression = self.decompile_expression(base);
                let indexed_target = matches!(base_expression, Expression::Indexer(_)).then(|| base_expression.clone());
                let (identifier, arguments) = match base_expression {
                    Expression::FunctionCall(f) => (unicase::Ascii::new(f.get_expression().to_string()), f.get_arguments().clone()),
                    Expression::Identifier(id) => (id.get_identifier().clone(), Vec::new()),
                    Expression::Indexer(f) => (unicase::Ascii::new(f.get_identifier().to_string()), f.get_arguments().clone()),

                    x => panic!("Invalid expression {x:?}"),
                };
                let mut value_expr = self.decompile_expression(expr);

                if self.expression_type(left) == Some(VariableType::Boolean) {
                    value_expr = Statement::try_boolean_conversion(&value_expr);
                }

                if members.is_empty() {
                    if let Some(target) = indexed_target {
                        return Statement::Let(LetStatement::empty(identifier, Token::Eq, Vec::new(), value_expr).with_target_expression(target));
                    }
                    return LetStatement::create_empty_statement(identifier, Token::Eq, arguments, value_expr);
                }
                Statement::Let(LetStatement::new(
                    None,
                    Spanned::create_empty(Token::Identifier(identifier)),
                    None,
                    arguments,
                    None,
                    members,
                    Spanned::create_empty(Token::Eq),
                    value_expr,
                ))
            }
        }
    }

    fn decompile_foreach(&mut self) -> Statement {
        let PPECommand::ForEach(variable, collection, end) = self.script.statements[self.cur_ptr].command.clone() else {
            unreachable!("decompile_foreach called on another command")
        };
        let variable = self.get_variable_name(variable);
        let collection = self.decompile_expression(&collection);
        self.cur_ptr += 1;
        let mut body = Vec::new();
        while self.cur_ptr < self.script.statements.len() {
            let statement = &self.script.statements[self.cur_ptr];
            let byte_offset = statement.span.start * 2;
            if byte_offset >= end {
                break;
            }
            if self.label_lookup.contains_key(&byte_offset) {
                self.used_labels.insert(byte_offset);
                body.push(LabelStatement::create_empty_statement(self.get_label_name(byte_offset)));
            }
            match &statement.command {
                PPECommand::ForEach(_, _, _) => body.push(self.decompile_foreach()),
                PPECommand::NextForEach(_) if statement.span.end * 2 == end => {
                    self.cur_ptr += 1;
                    break;
                }
                PPECommand::NextForEach(_) => {
                    body.push(ContinueStatement::create_empty_statement());
                    self.cur_ptr += 1;
                }
                // A jump to the loop end leaves the iterator behind, which is what BREAK does.
                PPECommand::Goto(target) if *target == end => {
                    body.push(BreakStatement::create_empty_statement());
                    self.cur_ptr += 1;
                }
                command => {
                    body.push(self.decompile_statement(command));
                    self.cur_ptr += 1;
                }
            }
        }
        ForEachStatement::create_empty_statement(variable, collection, body)
    }

    fn get_label_name(&self, label: usize) -> unicase::Ascii<String> {
        if let Some(name) = self.label_lookup.get(&label) {
            unicase::Ascii::new(format!("LABEL{:03}", *name + 1))
        } else {
            unicase::Ascii::new("EXIT_LABEL".to_string())
        }
    }

    fn get_variable_name(&self, p: usize) -> unicase::Ascii<String> {
        unicase::Ascii::new(self.executable.variable_table.get_var_entry(p).name.clone())
    }

    fn parse_function(&mut self, func: usize) {
        let entry = self.executable.variable_table.get_var_entry(func).clone();
        let mut func_body = self.generate_local_variable_declarations(&entry);
        while self.cur_ptr < self.script.statements.len() {
            let statement = &self.script.statements[self.cur_ptr];
            let byte_offset = statement.span.start * 2;
            if self.label_lookup.contains_key(&(byte_offset)) {
                self.used_labels.insert(byte_offset);
                func_body.push(LabelStatement::create_empty_statement(self.get_label_name(byte_offset)));
            }

            if matches!(statement.command, PPECommand::EndFunc) || matches!(statement.command, PPECommand::EndProc) {
                if entry.header.variable_type == VariableType::Function {
                    let parameters = self.generate_parameter_list(&entry);
                    let return_value = self
                        .executable
                        .variable_table
                        .get_var_entry(unsafe { entry.value.data.function_value.return_var as usize });
                    let func_impl = FunctionImplementation::new(
                        func,
                        Spanned::create_empty(Token::Function),
                        Spanned::create_empty(Token::Identifier(unicase::Ascii::new(entry.name.clone()))),
                        Spanned::create_empty(Token::LPar),
                        parameters,
                        Spanned::create_empty(Token::RPar),
                        self.type_token(return_value.header.variable_type),
                        return_value.header.variable_type,
                        return_value.header.dim,
                        func_body,
                        Spanned::create_empty(Token::EndFunc),
                    );
                    self.functions.push(AstNode::Function(func_impl));
                } else {
                    let parameters = self.generate_parameter_list(&entry);
                    let proc_impl = ProcedureImplementation::empty(func, unicase::Ascii::new(entry.name.clone()), parameters, func_body);
                    self.functions.push(AstNode::Procedure(proc_impl));
                }
                self.cur_ptr += 1;
                break;
            }

            if matches!(statement.command, PPECommand::ForEach(_, _, _)) {
                func_body.push(self.decompile_foreach());
                continue;
            }

            func_body.push(self.decompile_statement(&statement.command));
            self.cur_ptr += 1;
        }

        if self.cur_ptr < self.script.statements.len() && self.script.statements[self.cur_ptr].command == PPECommand::End {
            self.cur_ptr += 1;
        }
    }

    fn generate_local_variable_declarations(&self, entry: &TableEntry) -> Vec<Statement> {
        unsafe {
            let mut decl = Vec::new();

            // The return variable sits somewhere in this block and is filtered out below.
            let start = entry.value.data.function_value.first_var_id as usize + entry.value.data.function_value.parameters as usize + 1;
            let end = start + entry.value.data.function_value.local_variables as usize;

            for i in start..end {
                let local_var = self.executable.variable_table.get_var_entry(i);
                if local_var.entry_type == EntryType::LocalVariable {
                    decl.push(generate_variable_declaration(local_var, self.type_token(local_var.header.variable_type)));
                }
            }

            decl
        }
    }

    fn generate_parameter_list(&self, entry: &TableEntry) -> Vec<ParameterSpecifier> {
        unsafe {
            let mut parameters = Vec::new();

            let to;
            let pass_flags;
            let first_var;

            if entry.header.variable_type == VariableType::Function {
                to = entry.value.data.function_value.parameters as usize;
                first_var = entry.value.data.function_value.first_var_id as usize;
                pass_flags = 0;
            } else {
                to = entry.value.data.procedure_value.parameters as usize;
                pass_flags = entry.value.data.procedure_value.pass_flags;
                first_var = entry.value.data.procedure_value.first_var_id as usize;
            }

            for i in 0..to {
                let param = self.executable.variable_table.get_var_entry(first_var + 1 + i);
                let mut dimensions = Vec::new();
                match param.header.dim {
                    1 => {
                        dimensions.push(param.header.vector_size);
                    }
                    2 => {
                        dimensions.push(param.header.vector_size);
                        dimensions.push(param.header.matrix_size);
                    }
                    3 => {
                        dimensions.push(param.header.vector_size);
                        dimensions.push(param.header.matrix_size);
                        dimensions.push(param.header.cube_size);
                    }
                    _ => {}
                }
                let is_var = pass_flags & (1 << i) != 0;
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    if is_var {
                        Some(Spanned::create_empty(Token::Identifier(unicase::Ascii::new("VAR".to_string()))))
                    } else {
                        None
                    },
                    self.type_token(param.header.variable_type),
                    param.header.variable_type,
                    Some(VariableSpecifier::empty(unicase::Ascii::new(param.name.clone()), dimensions)),
                )));
            }

            parameters
        }
    }
}

fn convert_argument(expr: Expression, arg: &crate::executable::ArgumentDefinition) -> Expression {
    if arg.arg_type == VariableType::Boolean {
        return Statement::try_boolean_conversion(&expr);
    }
    if arg.number_format == NumberFormat::Hex
        && let Expression::Const(c) = &expr
        && let Constant::Integer(i, _) = c.get_constant_value()
    {
        return ConstantExpression::create_empty_expression(Constant::Integer(*i, NumberFormat::ColorCode));
    }
    arg.flags.convert_expr(expr)
}

fn add_parens_if_required(op: BinOp, expr: Expression) -> Expression {
    let add_parens = if let Expression::Binary(bin_op) = &expr {
        bin_op.get_op().get_priority() < op.get_priority()
    } else {
        false
    };

    if add_parens { ParensExpression::create_empty_expression(expr) } else { expr }
}

fn generate_variable_declaration(var: &TableEntry, type_token: Spanned<Token>) -> Statement {
    let dims = match var.header.dim {
        1 => {
            vec![var.header.vector_size]
        }
        2 => {
            vec![var.header.vector_size, var.header.matrix_size]
        }
        3 => {
            vec![var.header.vector_size, var.header.matrix_size, var.header.cube_size]
        }
        _ => Vec::new(),
    };
    Statement::VariableDeclaration(VariableDeclarationStatement::new(
        type_token,
        var.header.variable_type,
        vec![VariableSpecifier::empty(unicase::Ascii::new(var.name.clone()), dims)],
    ))
}

/// .
/// # Errors
/// # Panics
///
/// Panics if .
pub fn decompile(executable: Executable, raw: bool, lang_version: u16) -> Res<(Ast, Vec<DecompilerIssue>)> {
    match Decompiler::new(executable, !raw) {
        Ok(mut d) => {
            let mut ast = d.decompile()?;
            ast.language_version = lang_version;

            let reg = std::mem::take(&mut d.type_registry);
            let errors: Arc<std::sync::Mutex<crate::parser::ErrorReporter>> = Arc::new(Mutex::new(ErrorReporter::default()));
            let mut visitor = SemanticVisitor::new(&Workspace::default(), errors.clone(), reg);
            ast.visit(&mut visitor);
            visitor.finish();

            if !raw {
                for node in &mut ast.nodes {
                    match node {
                        AstNode::Function(f) => {
                            reconstruct::reconstruct_block(&visitor, f.get_statements_mut(), lang_version);
                        }
                        AstNode::Procedure(p) => {
                            reconstruct::reconstruct_block(&visitor, p.get_statements_mut(), lang_version);
                        }
                        AstNode::Main(block) => {
                            reconstruct::reconstruct_block(&visitor, block.get_statements_mut(), lang_version);
                        }
                        _ => {}
                    }
                }
                ast = reconstruct::finish_ast(&mut ast);
                ast = strip_unused_labels(&mut ast);
                ast = relabel_visitor::relabel_ast(&mut ast);
            }

            Ok((ast, d.issues))
        }
        Err(err) => Err(Box::new(err.error_type)),
    }
}

struct VariableConstantVisitor<'a> {
    executable: &'a mut Executable,
}

impl PPEVisitor<()> for VariableConstantVisitor<'_> {
    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) {
        // Changes CONST[expr] to VAR[expr]
        // There are some files out there that try to change the entry typ to constant for DIM variables
        if self.executable.variable_table.get_var_entry(id).entry_type == EntryType::Constant {
            self.executable.variable_table.get_var_entry_mut(id).entry_type = EntryType::Variable;
        }
        for d in dim {
            d.visit(self);
        }
    }

    fn visit_value(&mut self, _id: usize) {}
    fn visit_record_literal(&mut self, _type_id: u8, fields: &[(usize, PPEExpr)]) {
        for (_, value) in fields {
            value.visit(self);
        }
    }
    fn visit_member(&mut self, expr: &PPEExpr, _id: usize) {
        expr.visit(self);
    }
    fn visit_unary_expression(&mut self, _op: UnaryOp, expr: &PPEExpr) {
        expr.visit(self);
    }
    fn visit_binary_expression(&mut self, _op: BinOp, left: &PPEExpr, right: &PPEExpr) {
        left.visit(self);
        right.visit(self);
    }

    fn visit_predefined_function_call(&mut self, _def: &crate::executable::FunctionDefinition, arguments: &[PPEExpr]) {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_function_call(&mut self, _id: usize, arguments: &[PPEExpr]) {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_member_function_call(&mut self, _expr: &PPEExpr, arguments: &[PPEExpr], _id: usize) {
        for arg in arguments {
            arg.visit(self);
        }
    }

    fn visit_end(&mut self) {}
    fn visit_return(&mut self) {}
    fn visit_if(&mut self, cond: &PPEExpr, _label: &usize) {
        cond.visit(self);
    }
    fn visit_proc_call(&mut self, _id: usize, arguments: &[PPEExpr]) {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_predefined_call(&mut self, def: &StatementDefinition, arguments: &[PPEExpr]) {
        match def.sig {
            crate::executable::StatementSignature::SpecialCaseSort => {
                // Ensure that #1 sort argument is a variable
                if let PPEExpr::Value(id) = &arguments[0]
                    && self.executable.variable_table.get_var_entry(*id).entry_type == EntryType::Constant
                {
                    self.executable.variable_table.get_var_entry_mut(*id).entry_type = EntryType::Variable;
                }
            }
            crate::executable::StatementSignature::Invalid
            | crate::executable::StatementSignature::ArgumentsWithVariable(_, _)
            | crate::executable::StatementSignature::VariableArguments(_, _, _)
            | crate::executable::StatementSignature::SpecialCaseDlockg
            | crate::executable::StatementSignature::SpecialCaseDcreate
            | crate::executable::StatementSignature::SpecialCaseVarSeg
            | crate::executable::StatementSignature::SpecialCasePop => {}
        }
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_goto(&mut self, _label: &usize) {}
    fn visit_gosub(&mut self, _label: &usize) {}
    fn visit_on_error(&mut self, _target: &crate::executable::OnErrorTarget) {}
    fn visit_end_func(&mut self) {}
    fn visit_end_proc(&mut self) {}
    fn visit_stop(&mut self) {}
    fn visit_let(&mut self, target: &PPEExpr, value: &PPEExpr) {
        target.visit(self);
        value.visit(self);
    }

    fn visit_foreach(&mut self, _variable: usize, collection: &PPEExpr, _end: usize) {
        collection.visit(self);
    }

    fn visit_next_foreach(&mut self, _start: usize) {}
}
