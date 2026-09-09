use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        Ast, AstNode, AstVisitorMut, BinaryExpression, BlockStatement, CommentAstNode, ConstDeclarationStatement, Constant, ConstantExpression,
        DimensionSpecifier, Expression, ForEachStatement, ForStatement, FunctionImplementation, GotoStatement, IdentifierExpression, IfStatement,
        LabelStatement, LetStatement, MemberReferenceExpression, ParameterSpecifier, ProcedureImplementation, ReturnStatement, SelectStatement, Statement,
        VariableDeclarationStatement, VariableSpecifier, const_enum_value, const_expression, constant::NumberFormat,
    },
    decompiler::evaluation_visitor::ConstantFolder,
    executable::{FuncOpCode, VariableType, VariableValue},
    hir::CallId,
    parser::{
        EnumDefinition, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
    semantic::SemanticInfo,
};

/// Authoritative SOURCE annotations for one module-bound file. No expression is
/// semantically revisited during lowering. Span-keyed maps must belong to this file.
pub(crate) struct TransformationSemanticInput<'a> {
    pub function_type_lookup: &'a HashMap<CallId, SemanticInfo>,
    pub enum_binary_types: &'a HashMap<u64, u8>,
    pub user_type_lookup: &'a HashMap<usize, u8>,
    /// Resolved assignment target types, keyed by LetStatement's identifier span.start.
    /// Include scalar/array/function-result targets as well as member targets.
    /// MemberCall targets are resolved from user_type_lookup and the registry.
    pub compound_target_types: &'a HashMap<usize, VariableType>,
    pub type_registry: &'a UserTypeRegistry,
}

/// Only declarations/annotations introduced by this visitor, never source symbols.
/// Drain after each file and merge before structural constant collection/codegen.
#[derive(Default)]
pub(crate) struct GeneratedTransformationInfo {
    /// None is global/main scope; Some(name) is a module-bound routine name.
    pub temporaries: HashMap<Option<unicase::Ascii<String>>, Vec<(VariableType, VariableSpecifier)>>,
    pub function_type_lookup: HashMap<CallId, SemanticInfo>,
    pub enum_binary_types: HashMap<u64, u8>,
}

pub struct AstTransformationVisitor {
    continue_break_labels: Vec<(unicase::Ascii<String>, unicase::Ascii<String>)>,
    foreach_label_depths: Vec<usize>,
    cur_function: Option<unicase::Ascii<String>>,
    optimize_output: bool,
    labels: usize,
    global_constants: HashMap<unicase::Ascii<String>, (crate::executable::VariableType, VariableValue)>,
    local_constants: Option<HashMap<unicase::Ascii<String>, (crate::executable::VariableType, VariableValue)>>,
    local_bindings: Option<HashSet<unicase::Ascii<String>>>,
    enums: Vec<EnumDefinition>,
    loop_counters: HashSet<usize>,
    compound_receiver_types: HashMap<usize, u8>,
    compound_record_types: HashSet<u8>,
    compound_members: HashMap<(u8, unicase::Ascii<String>), (usize, VariableType)>,
    record_fields: HashMap<(u8, unicase::Ascii<String>), crate::executable::RecordField>,
    function_type_lookup: HashMap<CallId, SemanticInfo>,
    enum_binary_types: HashMap<u64, u8>,
    compound_target_types: HashMap<usize, VariableType>,
    routine_scope: Option<unicase::Ascii<String>>,
    generated: GeneratedTransformationInfo,
    temporaries: usize,
    language: u16,
}

impl AstTransformationVisitor {
    pub fn new(optimize_output: bool, enums: Vec<EnumDefinition>) -> Self {
        Self {
            continue_break_labels: Vec::new(),
            foreach_label_depths: Vec::new(),
            cur_function: None,
            optimize_output,
            labels: 0,
            global_constants: HashMap::new(),
            local_constants: None,
            local_bindings: None,
            enums,
            loop_counters: HashSet::new(),
            compound_receiver_types: HashMap::new(),
            compound_record_types: HashSet::new(),
            compound_members: HashMap::new(),
            record_fields: HashMap::new(),
            function_type_lookup: HashMap::new(),
            enum_binary_types: HashMap::new(),
            compound_target_types: HashMap::new(),
            routine_scope: None,
            generated: GeneratedTransformationInfo::default(),
            temporaries: 0,
            language: 400,
        }
    }

    /// Receiver types come from source semantic analysis. Records must keep
    /// their storage path; reference objects must instead keep their identity.
    pub(crate) fn set_compound_receiver_types(&mut self, types: HashMap<usize, u8>, registry: &crate::parser::UserTypeRegistry) {
        self.compound_record_types = types.values().copied().filter(|id| registry.is_record_type(*id)).collect();
        self.compound_members.clear();
        self.record_fields.clear();
        for id in types.values() {
            if let Some(members) = registry.get_type_from_id(*id) {
                for (name, variable_type) in &members.fields {
                    if let Some(field) = members.member_id_lookup.get(name) {
                        self.compound_members.insert((*id, name.clone()), (*field, *variable_type));
                    }
                }
            } else if let Some(record) = registry.get_record_type_from_id(*id) {
                for (field, (name, definition)) in record.fields.iter().enumerate() {
                    self.compound_members.insert((*id, name.clone()), (field, definition.variable_type));
                    self.record_fields.insert((*id, name.clone()), *definition);
                }
            }
        }
        self.compound_receiver_types = types;
    }

    pub(crate) fn set_semantic_input(&mut self, input: TransformationSemanticInput<'_>) {
        self.function_type_lookup.clone_from(input.function_type_lookup);
        self.enum_binary_types.clone_from(input.enum_binary_types);
        self.compound_target_types.clone_from(input.compound_target_types);
        self.set_compound_receiver_types(input.user_type_lookup.clone(), input.type_registry);
    }

    pub(crate) fn take_generated_info(&mut self) -> GeneratedTransformationInfo {
        std::mem::take(&mut self.generated)
    }

    fn register_temporary(&mut self, variable_type: VariableType, variable: &VariableSpecifier) {
        self.generated
            .temporaries
            .entry(self.routine_scope.clone())
            .or_default()
            .push((variable_type, variable.clone()));
    }

    fn compound_member(&self, target: &Expression) -> Option<(usize, VariableType)> {
        let member = match target {
            Expression::MemberReference(member) => member,
            Expression::FunctionCall(call) => return self.compound_member(call.get_expression()),
            Expression::Parens(parens) => return self.compound_member(parens.get_expression()),
            _ => return None,
        };
        let receiver = self.compound_receiver_types.get(&member.get_identifier_token().span.start)?;
        self.compound_members.get(&(*receiver, member.get_identifier().clone())).copied()
    }

    fn compound_binary(&mut self, op: crate::ast::BinOp, target: Expression, value: Expression, target_type: Option<VariableType>) -> Expression {
        let binary = BinaryExpression::empty(target, op, value);
        if matches!(op, crate::ast::BinOp::And | crate::ast::BinOp::Or)
            && let Some(VariableType::UserData(id)) = target_type
            && self.enums.iter().any(|definition| definition.id == id)
        {
            self.generated.enum_binary_types.insert(binary.id, id);
        }
        Expression::Binary(binary)
    }

    fn capture_compound_value(&mut self, value: Expression, variable_type: VariableType, statements: &mut Vec<Statement>) -> Expression {
        let name = unicase::Ascii::new(format!("*(compound{})", self.temporaries));
        self.temporaries += 1;
        let variable = VariableSpecifier::empty(name.clone(), Vec::new());
        self.register_temporary(variable_type, &variable);
        statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            variable_type,
            vec![variable],
        )));
        statements.push(LetStatement::create_empty_statement(name.clone(), Token::Eq, Vec::new(), value));
        IdentifierExpression::create_empty_expression(name)
    }

    fn capture_compound_indices(&mut self, arguments: &[Expression], statements: &mut Vec<Statement>) -> Vec<Expression> {
        arguments
            .iter()
            .map(|argument| {
                if matches!(argument, Expression::Const(_)) {
                    argument.clone()
                } else {
                    // Snapshot even a plain variable: a later index or the RHS
                    // may change it. Make the VM's integer index conversion
                    // explicit so enum indices don't become illegal enum-to-int
                    // assignments merely because lowering introduced a temp.
                    let index = crate::ast::FunctionCallExpression::empty(
                        IdentifierExpression::create_empty_expression(unicase::Ascii::new("ToInteger".to_string())),
                        vec![argument.clone()],
                    );
                    self.generated
                        .function_type_lookup
                        .insert(CallId(index.id), SemanticInfo::PredefinedFunc(FuncOpCode::TOINTEGER));
                    self.capture_compound_value(Expression::FunctionCall(index), VariableType::Integer, statements)
                }
            })
            .collect()
    }

    fn compound_object_receiver_type(&self, member: &MemberReferenceExpression) -> Option<VariableType> {
        self.compound_receiver_types
            .get(&member.get_identifier_token().span.start)
            .filter(|id| !self.compound_record_types.contains(id))
            .map(|id| VariableType::UserData(*id))
    }

    fn capture_compound_target(&mut self, target: Expression, statements: &mut Vec<Statement>) -> Expression {
        match target {
            Expression::Parens(mut parens) => {
                *parens.get_expression_mut() = self.capture_compound_target(parens.get_expression().clone(), statements);
                Expression::Parens(parens)
            }
            Expression::Indexer(mut indexer) => {
                let arguments = self.capture_compound_indices(indexer.get_arguments(), statements);
                indexer.set_arguments(arguments);
                Expression::Indexer(indexer)
            }
            Expression::MemberReference(member) => {
                let base = if let Some(variable_type) = self.compound_object_receiver_type(&member) {
                    self.capture_compound_value(member.get_expression().clone(), variable_type, statements)
                } else {
                    self.capture_compound_target(member.get_expression().clone(), statements)
                };
                Expression::MemberReference(MemberReferenceExpression::new(
                    base,
                    member.get_dot_token().clone(),
                    member.get_identifier_token().clone(),
                ))
            }
            Expression::FunctionCall(call) => {
                // In an assignable record path a call is an indexed field (or
                // legacy array notation), not a record value to copy to a temp.
                let base = self.capture_compound_target(call.get_expression().clone(), statements);
                let arguments = self.capture_compound_indices(call.get_arguments(), statements);
                Expression::FunctionCall(call.preserving_id(base, arguments))
            }
            target => target,
        }
    }

    fn compound_operator(token: &Token) -> Option<crate::ast::BinOp> {
        use crate::ast::BinOp;
        match token {
            Token::AddAssign => Some(BinOp::Add),
            Token::SubAssign => Some(BinOp::Sub),
            Token::MulAssign => Some(BinOp::Mul),
            Token::DivAssign => Some(BinOp::Div),
            Token::ModAssign => Some(BinOp::Mod),
            Token::AndAssign => Some(BinOp::And),
            Token::OrAssign => Some(BinOp::Or),
            _ => None,
        }
    }

    fn lower_record_redim(&mut self, target: &Expression, bounds: &[Expression]) -> Option<Statement> {
        let mut field_target = target;
        while let Expression::Parens(parens) = field_target {
            field_target = parens.get_expression();
        }
        let Expression::MemberReference(member) = field_target else { return None };
        let receiver = self.compound_receiver_types.get(&member.get_identifier_token().span.start)?;
        let field = *self.record_fields.get(&(*receiver, member.get_identifier().clone()))?;
        if !field.is_dynamic {
            return None;
        }
        // REDIM's existing instruction encodes a bare variable id, never a field path.
        let mut statements = Vec::new();
        let target = self.capture_compound_target(target.clone(), &mut statements);
        let name = unicase::Ascii::new(format!("*(redim{})", self.temporaries));
        self.temporaries += 1;
        let variable = VariableSpecifier::new(
            Spanned::create_empty(Token::Identifier(name.clone())),
            None,
            vec![DimensionSpecifier::dynamic(); field.dim as usize],
            None,
            None,
            None,
        );
        self.register_temporary(field.variable_type, &variable);
        statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            field.variable_type,
            vec![variable],
        )));
        statements.push(LetStatement::create_empty_statement(name.clone(), Token::Eq, Vec::new(), target.clone()));
        let temporary = IdentifierExpression::create_empty_expression(name.clone());
        let mut arguments = vec![temporary.clone()];
        arguments.extend_from_slice(bounds);
        statements.push(crate::ast::PredefinedCallStatement::create_empty_statement(
            crate::executable::OpCode::REDIM.get_definition(),
            arguments,
        ));
        let target = self.record_redim_lvalue(target);
        statements.push(Statement::Let(
            LetStatement::empty(name, Token::Eq, Vec::new(), temporary).with_target_expression(target),
        ));
        Some(Statement::Block(BlockStatement::empty(statements)))
    }

    fn record_redim_lvalue(&mut self, target: Expression) -> Expression {
        match target {
            Expression::Parens(parens) => self.record_redim_lvalue(parens.get_expression().clone()),
            Expression::MemberReference(member) => Expression::MemberReference(MemberReferenceExpression::new(
                self.record_redim_lvalue(member.get_expression().clone()),
                member.get_dot_token().clone(),
                member.get_identifier_token().clone(),
            )),
            Expression::FunctionCall(call) => {
                if matches!(self.function_type_lookup.get(&CallId(call.id)), Some(SemanticInfo::ArrayValueAt))
                    && let Expression::MemberReference(indexer) = call.get_expression()
                {
                    let base = self.record_redim_lvalue(indexer.get_expression().clone());
                    // Parenthesized variable roots use the same getter syntax as indexed fields.
                    if let Expression::Identifier(identifier) = &base {
                        return Expression::Indexer(crate::ast::IndexerExpression::new(
                            identifier.get_identifier_token().clone(),
                            indexer.get_dot_token().clone(),
                            call.get_arguments().clone(),
                            call.get_rpar_token().clone(),
                        ));
                    }
                    if matches!(base, Expression::MemberReference(_))
                        && let Some((field, _)) = self.compound_member(&base)
                    {
                        // A fresh call keeps the read copy's ArrayValueAt annotation intact.
                        let indexed = crate::ast::FunctionCallExpression::new(
                            base,
                            call.get_lpar_token().clone(),
                            call.get_arguments().clone(),
                            call.get_rpar_token().clone(),
                        );
                        self.generated
                            .function_type_lookup
                            .insert(CallId(indexed.id), SemanticInfo::IndexedRecordField(field));
                        return Expression::FunctionCall(indexed);
                    }
                }
                let base = self.record_redim_lvalue(call.get_expression().clone());
                Expression::FunctionCall(call.preserving_id(base, call.get_arguments().clone()))
            }
            target => target,
        }
    }

    /// Where the FOR statements of the file just transformed keep their count.
    pub fn take_loop_counters(&mut self) -> HashSet<usize> {
        std::mem::take(&mut self.loop_counters)
    }

    pub fn next_label(&mut self) -> unicase::Ascii<String> {
        let label = unicase::Ascii::new(format!("*(label{}", self.labels));
        self.labels += 1;
        label
    }

    /// Source semantics already checked the nominal type and domain, including
    /// unnamed valid values. Like Enum.Member, a known value needs no runtime
    /// cast. Enclosing bitwise expressions retain their checked binary IDs.
    fn enum_constant_expression(&self, id: u8, value: i32, token: &Spanned<Token>) -> Option<Expression> {
        self.enums.iter().find(|definition| definition.id == id)?;
        Some(Expression::Const(ConstantExpression::new(
            token.clone(),
            Constant::Integer(value, NumberFormat::Default),
        )))
    }

    /// The generic folders rebuild children with fresh identities and implement
    /// boolean AND/OR, not enum bitwise operations. Only let them consume trees
    /// with no surviving source nodes and no enum operations.
    fn foldable_scalar(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Const(constant) => !matches!(constant.get_constant_value(), Constant::Builtin(_)),
            Expression::Parens(parens) => self.foldable_scalar(parens.get_expression()),
            Expression::Unary(unary) => self.foldable_scalar(unary.get_expression()),
            Expression::Binary(binary) => {
                !self.enum_binary_types.contains_key(&binary.id)
                    && !self.generated.enum_binary_types.contains_key(&binary.id)
                    && self.foldable_scalar(binary.get_left_expression())
                    && self.foldable_scalar(binary.get_right_expression())
            }
            _ => false,
        }
    }

    fn negate_condition(&mut self, expression: &Expression) -> Expression {
        let expression = expression.visit_mut(self);
        let negated = self.negate_checked_condition(&expression);
        negated.visit_mut(self)
    }

    /// Negate only boolean contexts, retaining the identities and spans of
    /// surviving source nodes. Never apply De Morgan to checked enum AND/OR.
    fn negate_checked_condition(&self, expression: &Expression) -> Expression {
        use crate::ast::{BinOp, ExpressionDepthVisitor, UnaryExpression, UnaryOp};
        let explicit = UnaryExpression::create_empty_expression(UnaryOp::Not, expression.clone());
        let rewritten = match expression {
            Expression::Parens(parens) => self.negate_checked_condition(parens.get_expression()),
            Expression::Unary(unary) if unary.get_op() == UnaryOp::Not => unary.get_expression().clone(),
            Expression::Const(constant) if matches!(constant.get_constant_value(), Constant::Boolean(_)) => {
                ConstantExpression::create_empty_expression(Constant::Boolean(!constant.get_constant_value().get_value().as_bool()))
            }
            Expression::Binary(binary) if !self.enum_binary_types.contains_key(&binary.id) && !self.generated.enum_binary_types.contains_key(&binary.id) => {
                let token = match binary.get_op() {
                    BinOp::Eq => Token::NotEq,
                    BinOp::NotEq => Token::Eq,
                    BinOp::Lower => Token::GreaterEq,
                    BinOp::LowerEq => Token::Greater,
                    BinOp::Greater => Token::LowerEq,
                    BinOp::GreaterEq => Token::Lower,
                    BinOp::And => Token::Or,
                    BinOp::Or => Token::And,
                    _ => return explicit,
                };
                let (left, right) = if matches!(binary.get_op(), BinOp::And | BinOp::Or) {
                    (
                        self.negate_checked_condition(binary.get_left_expression()),
                        self.negate_checked_condition(binary.get_right_expression()),
                    )
                } else {
                    (binary.get_left_expression().clone(), binary.get_right_expression().clone())
                };
                let mut rewritten = BinaryExpression::new(left, Spanned::new(token, binary.get_op_token().span.clone()), right);
                rewritten.id = binary.id;
                Expression::Binary(rewritten)
            }
            _ => return explicit,
        };
        if rewritten.visit(&mut ExpressionDepthVisitor::default()) <= explicit.visit(&mut ExpressionDepthVisitor::default()) {
            rewritten
        } else {
            explicit
        }
    }

    /// Simplify generated FOR direction guards in a boolean context only.
    /// Unlike the old general optimizer, do not discard calls, member/index
    /// access or potentially faulting arithmetic, or rebuild surviving IDs.
    fn simplify_for_condition(&self, expression: Expression) -> Expression {
        use crate::ast::BinOp;
        fn scalar(expression: &Expression) -> bool {
            match expression {
                Expression::Const(_) | Expression::Identifier(_) => true,
                Expression::Parens(parens) => scalar(parens.get_expression()),
                _ => false,
            }
        }
        fn total(expression: &Expression) -> bool {
            if scalar(expression) {
                return true;
            }
            match expression {
                Expression::Binary(binary) => match binary.get_op() {
                    BinOp::And | BinOp::Or => total(binary.get_left_expression()) && total(binary.get_right_expression()),
                    BinOp::Eq | BinOp::NotEq | BinOp::Lower | BinOp::LowerEq | BinOp::Greater | BinOp::GreaterEq => {
                        scalar(binary.get_left_expression()) && scalar(binary.get_right_expression())
                    }
                    _ => false,
                },
                _ => false,
            }
        }
        let Expression::Binary(mut binary) = expression else {
            return expression;
        };
        if !matches!(binary.get_op(), BinOp::And | BinOp::Or)
            || self.enum_binary_types.contains_key(&binary.id)
            || self.generated.enum_binary_types.contains_key(&binary.id)
        {
            return Expression::Binary(binary);
        }
        let left = self.simplify_for_condition(binary.get_left_expression().clone());
        let right = self.simplify_for_condition(binary.get_right_expression().clone());
        let absorbing = binary.get_op() == BinOp::Or;
        for (constant, other) in [(&left, &right), (&right, &left)] {
            if let Expression::Const(constant) = constant {
                let truth = constant.get_constant_value().get_value().as_bool();
                if truth != absorbing {
                    return other.clone();
                }
                if total(other) {
                    return ConstantExpression::create_empty_expression(Constant::Boolean(absorbing));
                }
            }
        }
        *binary.get_left_expression_mut() = left;
        *binary.get_right_expression_mut() = right;
        Expression::Binary(binary)
    }

    fn lookup_constant(&self, id: &unicase::Ascii<String>) -> Option<&(crate::executable::VariableType, VariableValue)> {
        if let Some(local) = &self.local_constants
            && let Some(constant) = local.get(id)
        {
            return Some(constant);
        }
        if self.local_bindings.as_ref().is_some_and(|bindings| bindings.contains(id)) {
            return None;
        }
        self.global_constants.get(id)
    }

    fn collect_local_bindings(&mut self, parameters: &[ParameterSpecifier], statements: &[Statement]) {
        let mut bindings = HashSet::new();
        for parameter in parameters {
            let identifier = match parameter {
                ParameterSpecifier::Variable(parameter) => parameter.get_variable().as_ref().map(VariableSpecifier::get_identifier),
                ParameterSpecifier::Function(parameter) => Some(parameter.get_identifier()),
                ParameterSpecifier::Procedure(parameter) => Some(parameter.get_identifier()),
            };
            if let Some(identifier) = identifier {
                bindings.insert(identifier.clone());
            }
        }
        for statement in statements {
            if let Statement::VariableDeclaration(declaration) = statement {
                bindings.extend(declaration.get_variables().iter().map(|variable| variable.get_identifier().clone()));
            }
        }
        self.local_bindings = Some(bindings);
    }

    /// A constant may be written in terms of an earlier one, so the values are worked
    /// out in the order they are declared.
    fn collect_constants(&mut self, statements: &[Statement], local: bool) {
        for statement in statements {
            let Statement::ConstDeclaration(const_decl) = statement else {
                continue;
            };
            let Some(value) = const_enum_value(
                const_decl.get_value(),
                &|id| self.lookup_constant(id).map(|(_, value)| value.clone()),
                &self.enums,
            ) else {
                continue;
            };
            let Some(value) = crate::ast::convert_const_declaration(value, const_decl.get_variable_type(), self.language) else {
                continue;
            };
            let entry = (const_decl.get_variable_type(), value);
            let name = const_decl.get_identifier().clone();
            if local {
                self.local_constants.get_or_insert_with(HashMap::new).insert(name, entry);
            } else {
                self.global_constants.insert(name, entry);
            }
        }
    }
}

impl AstVisitorMut for AstTransformationVisitor {
    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Expression {
        let transformed = crate::ast::UnaryExpression::new(unary.get_op_token().clone(), unary.get_expression().visit_mut(self));
        if self.optimize_output && self.foldable_scalar(transformed.get_expression()) {
            let folded = ConstantFolder::default().visit_unary_expression(&transformed);
            if matches!(folded, Expression::Const(_)) {
                return folded;
            }
        }
        Expression::Unary(transformed)
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> Expression {
        let mut transformed = binary.clone();
        *transformed.get_left_expression_mut() = binary.get_left_expression().visit_mut(self);
        *transformed.get_right_expression_mut() = binary.get_right_expression().visit_mut(self);
        if self.optimize_output
            && !self.enum_binary_types.contains_key(&binary.id)
            && !self.generated.enum_binary_types.contains_key(&binary.id)
            && self.foldable_scalar(transformed.get_left_expression())
            && self.foldable_scalar(transformed.get_right_expression())
        {
            let folded = ConstantFolder::default().visit_binary_expression(&transformed);
            if matches!(folded, Expression::Const(_)) {
                return folded;
            }
        }
        Expression::Binary(transformed)
    }

    fn visit_parens_expression(&mut self, parens: &crate::ast::ParensExpression) -> Expression {
        let mut transformed = parens.clone();
        *transformed.get_expression_mut() = parens.get_expression().visit_mut(self);
        Expression::Parens(transformed)
    }

    fn visit_array_expression(&mut self, array: &crate::ast::ArrayInitializerExpression) -> Expression {
        let mut transformed = array.clone();
        *transformed.get_expressions_mut() = array.get_expressions().iter().map(|expression| expression.visit_mut(self)).collect();
        Expression::ArrayInitializer(transformed)
    }

    fn visit_continue_statement(&mut self, _continue_stmt: &crate::ast::ContinueStatement) -> Statement {
        if self.foreach_label_depths.last().is_some_and(|depth| self.continue_break_labels.len() == *depth) {
            return Statement::Continue(_continue_stmt.clone());
        }
        if self.continue_break_labels.is_empty() {
            return CommentAstNode::create_empty_statement("no continue block");
        }
        let (continue_label, _) = self.continue_break_labels.last().unwrap();
        GotoStatement::create_empty_statement(continue_label.clone())
    }
    fn visit_break_statement(&mut self, _break_stmt: &crate::ast::BreakStatement) -> Statement {
        if self.foreach_label_depths.last().is_some_and(|depth| self.continue_break_labels.len() == *depth) {
            return Statement::Break(_break_stmt.clone());
        }
        if self.continue_break_labels.is_empty() {
            return CommentAstNode::create_empty_statement("no break block");
        }
        let (_, break_label) = self.continue_break_labels.last().unwrap();
        GotoStatement::create_empty_statement(break_label.clone())
    }

    fn visit_if_statement(&mut self, if_stmt: &IfStatement) -> Statement {
        if matches!(if_stmt.get_statement(), Statement::Goto(_)) {
            return Statement::If(IfStatement::new(
                if_stmt.get_if_token().clone(),
                if_stmt.get_lpar_token().clone(),
                if_stmt.get_condition().visit_mut(self),
                if_stmt.get_rpar_token().clone(),
                if_stmt.get_statement().visit_mut(self),
            ));
        }
        let mut statements = Vec::new();
        let if_exit_label = self.next_label();
        statements.push(IfStatement::create_empty_statement(
            self.negate_condition(if_stmt.get_condition()),
            GotoStatement::create_empty_statement(if_exit_label.clone()),
        ));
        statements.push(if_stmt.get_statement().visit_mut(self));
        statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_if_then_statement(&mut self, if_then: &crate::ast::IfThenStatement) -> Statement {
        let mut statements = Vec::new();

        let last_exit_label = self.next_label();
        let mut if_exit_label = self.next_label();

        statements.push(IfStatement::create_empty_statement(
            self.negate_condition(if_then.get_condition()),
            GotoStatement::create_empty_statement(if_exit_label.clone()),
        ));
        statements.extend(if_then.get_statements().iter().map(|s| s.visit_mut(self)));

        if !if_then.get_else_if_blocks().is_empty() || if_then.get_else_block().is_some() {
            statements.push(GotoStatement::create_empty_statement(last_exit_label.clone()));
        }

        for else_if in if_then.get_else_if_blocks() {
            statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));

            if_exit_label = self.next_label();
            statements.push(IfStatement::create_empty_statement(
                self.negate_condition(else_if.get_condition()),
                GotoStatement::create_empty_statement(if_exit_label.clone()),
            ));
            statements.extend(else_if.get_statements().iter().map(|s| s.visit_mut(self)));
            statements.push(GotoStatement::create_empty_statement(last_exit_label.clone()));
        }

        if let Some(else_block) = if_then.get_else_block() {
            statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
            if_exit_label = self.next_label();

            statements.extend(else_block.get_statements().iter().map(|s| s.visit_mut(self)));
        }

        statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
        statements.push(LabelStatement::create_empty_statement(last_exit_label.clone()));

        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_while_statement(&mut self, while_stmt: &crate::ast::WhileStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.push(IfStatement::create_empty_statement(
            self.negate_condition(while_stmt.get_condition()),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));
        statements.push(while_stmt.get_statement().visit_mut(self));
        statements.push(GotoStatement::create_empty_statement(continue_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_while_do_statement(&mut self, while_do: &crate::ast::WhileDoStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.push(IfStatement::create_empty_statement(
            self.negate_condition(while_do.get_condition()),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));
        statements.extend(while_do.get_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(GotoStatement::create_empty_statement(continue_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_repeat_until_statement(&mut self, repeat_until: &crate::ast::RepeatUntilStatement) -> Statement {
        let mut statements = Vec::new();

        let loop_label = self.next_label();
        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(loop_label.clone()));
        statements.extend(repeat_until.get_statements().iter().map(|s| s.visit_mut(self)));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));

        statements.push(IfStatement::create_empty_statement(
            self.negate_condition(repeat_until.get_condition()),
            GotoStatement::create_empty_statement(loop_label.clone()),
        ));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_loop_statement(&mut self, loop_stmt: &crate::ast::LoopStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.extend(loop_stmt.get_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(GotoStatement::create_empty_statement(continue_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_select_statement(&mut self, select_stmt: &SelectStatement) -> Statement {
        let mut statements = Vec::new();
        let expr = select_stmt.get_expression().visit_mut(self);
        let case_exit_label = self.next_label();

        for case_block in select_stmt.get_case_blocks() {
            let next_case_label = self.next_label();

            let mut condition = ConstantExpression::create_empty_expression(Constant::Boolean(false));

            for spec in case_block.get_case_specifiers() {
                let cond = match spec {
                    crate::ast::CaseSpecifier::Expression(spec_expr) => {
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::NotEq, expr.clone(), spec_expr.visit_mut(self))
                    }
                    crate::ast::CaseSpecifier::FromTo(from_expr, to_expr) => BinaryExpression::create_empty_expression(
                        crate::ast::BinOp::Or,
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, from_expr.visit_mut(self), expr.clone()),
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, expr.clone(), to_expr.visit_mut(self)),
                    ),
                };
                if matches!(condition, Expression::Const(_)) {
                    condition = cond;
                } else {
                    condition = BinaryExpression::create_empty_expression(crate::ast::BinOp::And, condition, cond);
                }
            }

            statements.push(IfStatement::create_empty_statement(
                condition,
                GotoStatement::create_empty_statement(next_case_label.clone()),
            ));

            statements.extend(case_block.get_statements().iter().map(|s| s.visit_mut(self)));
            statements.push(GotoStatement::create_empty_statement(case_exit_label.clone()));
            statements.push(LabelStatement::create_empty_statement(next_case_label.clone()));
        }
        statements.extend(select_stmt.get_default_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(LabelStatement::create_empty_statement(case_exit_label.clone()));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_for_statement(&mut self, for_stmt: &ForStatement) -> Statement {
        let mut statements = Vec::new();
        self.loop_counters.insert(for_stmt.get_identifier_token().span.start);

        let loop_label = self.next_label();
        let continue_label = self.next_label();
        let break_label = self.next_label();

        let id_expr = Expression::Identifier(IdentifierExpression::new(for_stmt.get_identifier_token().clone()));

        // init variable
        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Token::Eq,
            Vec::new(),
            for_stmt.get_start_expr().visit_mut(self),
        ));

        // create loop
        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));
        statements.push(LabelStatement::create_empty_statement(loop_label.clone()));

        let increment = if let Some(increment) = for_stmt.get_step_expr() {
            increment.visit_mut(self)
        } else {
            Expression::Const(ConstantExpression::empty(Constant::Integer(1, NumberFormat::Default)))
        };

        let end_expr = for_stmt.get_end_expr().visit_mut(self);

        let lower_bound = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::Or,
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Lower,
                ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
                increment.clone(),
            ),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Lower, id_expr.clone(), end_expr.clone()),
        );

        let upper_bound = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::Or,
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Greater,
                ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
                increment.clone(),
            ),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, id_expr.clone(), end_expr.clone()),
        );

        let condition = BinaryExpression::create_empty_expression(crate::ast::BinOp::And, lower_bound, upper_bound);
        let condition = if self.optimize_output {
            let folded = condition.visit_mut(self);
            self.simplify_for_condition(folded)
        } else {
            condition
        };
        statements.push(IfStatement::create_empty_statement(
            condition,
            GotoStatement::create_empty_statement(break_label.clone()),
        ));

        statements.extend(for_stmt.get_statements().iter().map(|s| s.visit_mut(self)));

        // create step & increment

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Token::Eq,
            Vec::new(),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Add, id_expr, increment),
        ));

        // loop & exit;
        statements.push(GotoStatement::create_empty_statement(loop_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_foreach_statement(&mut self, foreach_stmt: &ForEachStatement) -> Statement {
        self.foreach_label_depths.push(self.continue_break_labels.len());
        let collection = foreach_stmt.get_collection().visit_mut(self);
        let statements = foreach_stmt.get_statements().iter().map(|statement| statement.visit_mut(self)).collect();
        self.foreach_label_depths.pop();
        Statement::ForEach(ForEachStatement::new(
            foreach_stmt.get_foreach_token().clone(),
            foreach_stmt.get_identifier_token().clone(),
            foreach_stmt.get_in_token().clone(),
            collection,
            statements,
            foreach_stmt.get_endforeach_token().clone(),
        ))
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> Statement {
        let mut val_expr = let_stmt.get_value_expression().visit_mut(self);
        let transformed_target = let_stmt.get_target_expression().map(|target| target.visit_mut(self));
        let arguments: Vec<_> = let_stmt.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect();

        // A compound assignment reads the same place it writes, including indices and members.
        let mut target = if let Some(target) = &transformed_target {
            target.clone()
        } else if let (Some(left), Some(right)) = (let_stmt.get_lpar_token(), let_stmt.get_rpar_token()) {
            Expression::Indexer(crate::ast::IndexerExpression::new(
                let_stmt.get_identifier_token().clone(),
                left.clone(),
                arguments.clone(),
                right.clone(),
            ))
        } else {
            Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone()))
        };
        if transformed_target.is_none() {
            for member in let_stmt.get_members() {
                target = Expression::MemberReference(crate::ast::MemberReferenceExpression::new(
                    target,
                    Spanned::create_empty(Token::Dot),
                    member.clone(),
                ));
            }
        }

        let mut statements = Vec::new();
        let compound = Self::compound_operator(let_stmt.get_let_variant());
        if let Some(op) = compound {
            let target_type = self
                .compound_target_types
                .get(&let_stmt.get_identifier_token().span.start)
                .copied()
                .or_else(|| self.compound_member(&target).map(|(_, variable_type)| variable_type));
            target = self.capture_compound_target(target, &mut statements);
            val_expr = self.compound_binary(op, target.clone(), val_expr, target_type);
            if let Expression::MemberReference(member) = &target
                && self.compound_object_receiver_type(member).is_some()
                && let Some((field, _)) = self.compound_member(&target)
            {
                // Source semantics already checked writability and operand types.
                // This call did not exist in the source, so attach its field now.
                let eq = Spanned::new(Token::Eq, let_stmt.get_eq_token().span.clone());
                let call = crate::ast::FunctionCallExpression::new(target, eq.clone(), vec![val_expr], eq);
                self.generated
                    .function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::MemberSetterCall(field));
                statements.push(Statement::MemberCall(crate::ast::MemberCallStatement::new(Expression::FunctionCall(call))));
                return Statement::Block(BlockStatement::empty(statements));
            }
        }

        let statement = LetStatement::new(
            let_stmt.get_let_token().clone(),
            Spanned {
                span: let_stmt.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(let_stmt.get_identifier())),
            },
            let_stmt.get_lpar_token().clone(),
            arguments,
            let_stmt.get_rpar_token().clone(),
            let_stmt.get_members().clone(),
            Spanned::new(Token::Eq, let_stmt.get_eq_token().span.clone()),
            val_expr,
        );
        let statement = Statement::Let(if compound.is_some() && !matches!(target, Expression::Identifier(_)) {
            statement.with_target_expression(target)
        } else if let Some(target) = transformed_target {
            statement.with_target_expression(target)
        } else {
            statement
        });
        if statements.is_empty() {
            statement
        } else {
            statements.push(statement);
            Statement::Block(BlockStatement::empty(statements))
        }
    }

    fn visit_member_call_statement(&mut self, statement: &crate::ast::MemberCallStatement) -> Statement {
        let expression = statement.get_expression().visit_mut(self);
        if let Expression::FunctionCall(call) = &expression
            && matches!(
                self.function_type_lookup.get(&CallId(call.id)),
                Some(SemanticInfo::ArrayMemberProc(crate::executable::OpCode::REDIM))
            )
            && let Expression::MemberReference(member) = call.get_expression()
            && let Some(lowered) = self.lower_record_redim(member.get_expression(), call.get_arguments())
        {
            return lowered;
        }
        if let Expression::FunctionCall(call) = &expression
            && let Some(op) = Self::compound_operator(&call.get_lpar_token().token)
            && let Expression::MemberReference(member) = call.get_expression()
            && !member.get_identifier().starts_with('<')
            && self.compound_object_receiver_type(member).is_some()
            && call.get_arguments().len() == 1
        {
            let mut statements = Vec::new();
            let target_type = self.compound_member(call.get_expression()).map(|(_, variable_type)| variable_type);
            let target = self.capture_compound_target(call.get_expression().clone(), &mut statements);
            let value = self.compound_binary(op, target.clone(), call.get_arguments()[0].clone(), target_type);
            let mut setter = crate::ast::FunctionCallExpression::new(
                target,
                Spanned::new(Token::Eq, call.get_lpar_token().span.clone()),
                vec![value],
                Spanned::new(Token::Eq, call.get_rpar_token().span.clone()),
            );
            // The source MemberSetterCall annotation remains authoritative.
            setter.id = call.id;
            statements.push(Statement::MemberCall(crate::ast::MemberCallStatement::new(Expression::FunctionCall(setter))));
            return Statement::Block(BlockStatement::empty(statements));
        }
        Statement::MemberCall(crate::ast::MemberCallStatement::new(expression))
    }

    fn visit_predefined_call_statement(&mut self, statement: &crate::ast::PredefinedCallStatement) -> Statement {
        let arguments: Vec<_> = statement.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect();
        if statement.get_func().opcode == crate::executable::OpCode::REDIM
            && let Some(target) = arguments.first()
            && let Some(lowered) = self.lower_record_redim(target, &arguments[1..])
        {
            return lowered;
        }
        Statement::PredifinedCall(crate::ast::PredefinedCallStatement::new(
            statement.get_identifier_token().clone(),
            statement.get_func(),
            arguments,
        ))
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> AstNode {
        let previous_scope = self.routine_scope.replace(function.get_identifier().clone());
        self.cur_function = Some(function.get_identifier().clone());
        self.local_constants = Some(HashMap::new());
        self.collect_local_bindings(function.get_parameters(), function.get_statements());
        self.collect_constants(function.get_statements(), true);
        let res = AstNode::Function(
            FunctionImplementation::new(
                function.id,
                function.get_function_token().clone(),
                Spanned {
                    span: function.get_identifier_token().span.clone(),
                    token: Token::Identifier(self.visit_identifier(function.get_identifier())),
                },
                function.get_leftpar_token().clone(),
                function.get_parameters().iter().map(|arg| arg.visit_mut(self)).collect(),
                function.get_rightpar_token().clone(),
                function.get_return_type_token().clone(),
                function.get_return_type(),
                function.get_return_rank(),
                function.get_statements().iter().map(|stmt| stmt.visit_mut(self)).collect(),
                function.get_endfunc_token().clone(),
            )
            .with_documentation(function.get_documentation()),
        );

        self.cur_function = None;
        self.routine_scope = previous_scope;
        self.local_constants = None;
        self.local_bindings = None;
        res
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> AstNode {
        let previous_scope = self.routine_scope.replace(procedure.get_identifier().clone());
        self.local_constants = Some(HashMap::new());
        self.collect_local_bindings(procedure.get_parameters(), procedure.get_statements());
        self.collect_constants(procedure.get_statements(), true);
        let res = AstNode::Procedure(
            ProcedureImplementation::new(
                procedure.id,
                procedure.get_procedure_token().clone(),
                Spanned {
                    span: procedure.get_identifier_token().span.clone(),
                    token: Token::Identifier(self.visit_identifier(procedure.get_identifier())),
                },
                procedure.get_leftpar_token().clone(),
                procedure.get_parameters().iter().map(|arg| arg.visit_mut(self)).collect(),
                procedure.get_rightpar_token().clone(),
                procedure.get_statements().iter().map(|stmt| stmt.visit_mut(self)).collect(),
                procedure.get_endproc_token().clone(),
            )
            .with_documentation(procedure.get_documentation()),
        );
        self.local_constants = None;
        self.local_bindings = None;
        self.routine_scope = previous_scope;
        res
    }

    /// The value takes the place of the name everywhere it is used. The declaration
    /// itself stays as source provenance; it has already been checked and codegen skips it.
    fn visit_const_declaration_statement(&mut self, const_decl: &ConstDeclarationStatement) -> Statement {
        Statement::ConstDeclaration(const_decl.clone())
    }

    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> Expression {
        if let Some((variable_type, value)) = self.lookup_constant(identifier.get_identifier()).cloned() {
            if let crate::executable::VariableType::UserData(id) = variable_type {
                if let Some(expr) = self.enum_constant_expression(id, value.as_int(), identifier.get_identifier_token()) {
                    return expr;
                }
            } else if let Some(expression) = const_expression(&value, variable_type) {
                match expression {
                    Expression::Const(expr) => {
                        return Expression::Const(ConstantExpression::new(
                            identifier.get_identifier_token().clone(),
                            expr.get_constant_value().clone(),
                        ));
                    }
                    Expression::FunctionCall(call) => {
                        // Typed constants use compiler-generated conversions (e.g.
                        // DOUBLE uses ToDReal(string) to avoid f32 literal rounding).
                        // These calls did not exist during source analysis, so bind
                        // their built-in opcode here, never a same-named source symbol.
                        let Expression::Identifier(conversion) = call.get_expression() else {
                            unreachable!("constant conversions have a built-in identifier")
                        };
                        let definition = crate::executable::FUNCTION_DEFINITIONS
                            .iter()
                            .find(|definition| {
                                definition.name.eq_ignore_ascii_case(conversion.get_identifier().as_str()) && definition.return_type == variable_type
                            })
                            .expect("constant conversion must have a matching built-in result type");
                        self.generated
                            .function_type_lookup
                            .insert(CallId(call.id), SemanticInfo::PredefinedFunc(definition.opcode));
                        return Expression::FunctionCall(call);
                    }
                    _ => unreachable!("scalar constants lower to literals or built-in conversions"),
                }
            }
        }
        Expression::Identifier(IdentifierExpression::new(Spanned {
            span: identifier.get_identifier_token().span.clone(),
            token: Token::Identifier(self.visit_identifier(identifier.get_identifier())),
        }))
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) -> Expression {
        let is_enum_namespace = matches!(member.get_expression(), Expression::Identifier(base)
            if self.enums.iter().any(|definition| definition.name == *base.get_identifier()));
        Expression::MemberReference(MemberReferenceExpression::new(
            if is_enum_namespace {
                member.get_expression().clone()
            } else {
                member.get_expression().visit_mut(self)
            },
            member.get_dot_token().clone(),
            member.get_identifier_token().clone(),
        ))
    }

    fn visit_function_call_expression(&mut self, call: &crate::ast::FunctionCallExpression) -> Expression {
        let is_enum_namespace = matches!(call.get_expression(), Expression::Identifier(base)
            if self.enums.iter().any(|definition| definition.name == *base.get_identifier()));
        let is_bound_name = matches!(call.get_expression(), Expression::Identifier(_))
            && (self.function_type_lookup.contains_key(&CallId(call.id)) || self.generated.function_type_lookup.contains_key(&CallId(call.id)));
        Expression::FunctionCall(call.preserving_id(
            if is_enum_namespace || is_bound_name {
                call.get_expression().clone()
            } else {
                call.get_expression().visit_mut(self)
            },
            call.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect(),
        ))
    }

    fn visit_ast(&mut self, program: &Ast) -> Ast {
        self.language = program.language_version;
        // A constant may be used before the line that declares it, so they are all
        // known before anything is rewritten.
        for node in &program.nodes {
            match node {
                AstNode::TopLevelStatement(stmt) => self.collect_constants(std::slice::from_ref(stmt), false),
                AstNode::Main(block) => self.collect_constants(block.get_statements(), false),
                _ => {}
            }
        }

        let mut new_program = Ast::new();
        new_program.file_name.clone_from(&program.file_name);
        new_program.module.clone_from(&program.module);
        new_program.imports.clone_from(&program.imports);
        new_program.language_version = program.language_version;
        new_program.require_user_variables = program.require_user_variables;
        for node in &program.nodes {
            new_program.nodes.push(node.visit_mut(self));
        }
        new_program
    }

    fn visit_return_statement(&mut self, return_stmt: &ReturnStatement) -> Statement {
        let mut statements = Vec::new();
        if let Some(expr) = return_stmt.get_expression() {
            assert!(self.cur_function.is_some(), "Return statement outside of function");
            statements.push(Statement::Let(LetStatement::new(
                None,
                Spanned {
                    span: return_stmt.get_return_token().span.clone(),
                    token: Token::Identifier(self.cur_function.clone().unwrap()), // Parser doesn't allow return expression outside of function
                },
                None,
                Vec::new(),
                None,
                Vec::new(),
                Spanned::create_empty(Token::Eq),
                expr.visit_mut(self),
            )));
        }
        statements.push(Statement::Return(ReturnStatement::new(return_stmt.get_return_token().clone(), None)));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_variable_declaration_statement(&mut self, var_decl: &VariableDeclarationStatement) -> Statement {
        let mut statements = Vec::new();
        for var in var_decl.get_variables() {
            if let Some(init) = var.get_initalizer() {
                if let Expression::ArrayInitializer(array) = init {
                    let dynamic = var.get_dimensions().first().is_some_and(DimensionSpecifier::is_dynamic);
                    let stmt = Statement::VariableDeclaration(VariableDeclarationStatement::new(
                        var_decl.get_type_token().clone(),
                        var_decl.get_variable_type(),
                        vec![VariableSpecifier::new(
                            var.get_identifier_token().clone(),
                            var.get_leftpar_token().clone(),
                            if dynamic {
                                var.get_dimensions().clone()
                            } else {
                                vec![DimensionSpecifier::empty(array.get_expressions().len().saturating_sub(1))]
                            },
                            var.get_rightpar_token().clone(),
                            None,
                            None,
                        )],
                    ));
                    statements.push(stmt);

                    if dynamic && !array.get_expressions().is_empty() {
                        statements.push(crate::ast::PredefinedCallStatement::create_empty_statement(
                            crate::executable::OpCode::REDIM.get_definition(),
                            vec![
                                Expression::Identifier(IdentifierExpression::new(var.get_identifier_token().clone())),
                                ConstantExpression::create_empty_expression(Constant::Integer(
                                    (array.get_expressions().len() - 1) as i32,
                                    NumberFormat::Default,
                                )),
                            ],
                        ));
                    } else if dynamic {
                        // A declaration may execute repeatedly (for example in a
                        // loop). Copy a never-written empty array each time, not
                        // just the storage allocated when this frame was created.
                        let empty_name = unicase::Ascii::new(format!("*(empty_array{})", self.temporaries));
                        self.temporaries += 1;
                        let variable = VariableSpecifier::new(
                            Spanned::create_empty(Token::Identifier(empty_name.clone())),
                            None,
                            var.get_dimensions().clone(),
                            None,
                            None,
                            None,
                        );
                        self.register_temporary(var_decl.get_variable_type(), &variable);
                        statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::new(
                            var_decl.get_type_token().clone(),
                            var_decl.get_variable_type(),
                            vec![variable],
                        )));
                        statements.push(LetStatement::create_empty_statement(
                            var.get_identifier().clone(),
                            Token::Eq,
                            Vec::new(),
                            IdentifierExpression::create_empty_expression(empty_name),
                        ));
                    }

                    for (idx, expr) in array.get_expressions().iter().enumerate() {
                        statements.push(Statement::Let(LetStatement::new(
                            None,
                            var.get_identifier_token().clone(),
                            None,
                            vec![Expression::Const(ConstantExpression::empty(Constant::Integer(
                                idx as i32,
                                NumberFormat::Default,
                            )))],
                            None,
                            Vec::new(),
                            Spanned::create_empty(Token::Eq),
                            expr.visit_mut(self),
                        )));
                    }
                } else {
                    let stmt = Statement::VariableDeclaration(VariableDeclarationStatement::new(
                        var_decl.get_type_token().clone(),
                        var_decl.get_variable_type(),
                        vec![VariableSpecifier::new(
                            var.get_identifier_token().clone(),
                            var.get_leftpar_token().clone(),
                            var.get_dimensions().clone(),
                            var.get_rightpar_token().clone(),
                            None,
                            None,
                        )],
                    ));
                    statements.push(stmt);

                    statements.push(Statement::Let(LetStatement::new(
                        None,
                        var.get_identifier_token().clone(),
                        None,
                        Vec::new(),
                        None,
                        Vec::new(),
                        Spanned::create_empty(Token::Eq),
                        init.visit_mut(self),
                    )));
                }
            } else {
                statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::new(
                    var_decl.get_type_token().clone(),
                    var_decl.get_variable_type(),
                    vec![var.clone()],
                )));
            }
        }
        Statement::Block(BlockStatement::empty(statements))
    }
}

#[cfg(test)]
mod semantics_before_lowering_tests {
    use super::*;
    use crate::ast::{AstVisitor, BinOp, FunctionCallExpression, MemberCallStatement, ParensExpression, UnaryExpression, UnaryOp};
    use crate::compiler::user_data::{UserDataMemberRegistry, UserDataRegistry};

    fn name(value: &str) -> unicase::Ascii<String> {
        unicase::Ascii::new(value.to_string())
    }

    fn identifier(value: &str, start: usize) -> Expression {
        Expression::Identifier(IdentifierExpression::new(Spanned::new(
            Token::Identifier(name(value)),
            start..start + value.len(),
        )))
    }

    fn integer(value: i32) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Integer(value, NumberFormat::Default))
    }

    fn flags() -> EnumDefinition {
        EnumDefinition {
            id: 200,
            name: name("Flags"),
            variants: vec![(name("One"), 1), (name("Two"), 2)],
            domain: vec![0, 1, 2, 3],
        }
    }

    #[derive(Default)]
    struct Ids {
        calls: Vec<u64>,
        binaries: Vec<u64>,
    }

    impl AstVisitor<()> for Ids {
        fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) {
            self.calls.push(call.id);
            call.get_expression().visit(self);
            for argument in call.get_arguments() {
                argument.visit(self);
            }
        }

        fn visit_binary_expression(&mut self, binary: &BinaryExpression) {
            self.binaries.push(binary.id);
            binary.get_left_expression().visit(self);
            binary.get_right_expression().visit(self);
        }
    }

    #[test]
    fn surviving_nested_expressions_keep_ids_and_tokens_with_or_without_folding() {
        let binary = BinaryExpression::new(identifier("x", 20), Spanned::new(Token::Add, 22..23), integer(1));
        let binary_id = binary.id;
        let call = FunctionCallExpression::new(
            identifier("f", 10),
            Spanned::new(Token::LPar, 11..12),
            vec![Expression::Binary(binary)],
            Spanned::new(Token::RPar, 24..25),
        );
        let call_id = call.id;
        let source = Expression::Unary(UnaryExpression::new(
            Spanned::new(Token::Sub, 8..9),
            Expression::Parens(ParensExpression::new(
                Spanned::new(Token::LPar, 9..10),
                Expression::FunctionCall(call),
                Spanned::new(Token::RPar, 25..26),
            )),
        ));
        for optimize in [false, true] {
            let lowered = source.visit_mut(&mut AstTransformationVisitor::new(optimize, Vec::new()));
            assert_eq!(lowered, source);
            let mut ids = Ids::default();
            lowered.visit(&mut ids);
            assert_eq!(ids.calls, vec![call_id]);
            assert_eq!(ids.binaries, vec![binary_id]);
        }
    }

    #[test]
    fn enum_binary_is_not_boolean_folded_or_demorgan_rewritten() {
        let binary = BinaryExpression::empty(integer(1), BinOp::Or, integer(2));
        let id = binary.id;
        let mut visitor = AstTransformationVisitor::new(true, vec![flags()]);
        visitor.set_semantic_input(TransformationSemanticInput {
            function_type_lookup: &HashMap::new(),
            enum_binary_types: &HashMap::from([(id, 200)]),
            user_type_lookup: &HashMap::new(),
            compound_target_types: &HashMap::new(),
            type_registry: &UserTypeRegistry::default(),
        });
        let source = Expression::Binary(binary);
        assert_eq!(source.visit_mut(&mut visitor), source);
        let negated = visitor.negate_condition(&source);
        let Expression::Unary(negated) = negated else {
            panic!("expected explicit NOT")
        };
        assert_eq!(negated.get_op(), UnaryOp::Not);
        let Expression::Binary(inner) = negated.get_expression() else {
            panic!("lost enum binary")
        };
        assert_eq!(inner.id, id);
        assert_eq!(inner.get_op(), BinOp::Or);
        assert!(visitor.take_generated_info().enum_binary_types.is_empty());
        let scalar = BinaryExpression::create_empty_expression(BinOp::Add, integer(1), integer(2));
        assert_eq!(scalar.visit_mut(&mut visitor), integer(3));
    }

    #[test]
    fn typed_constant_conversions_are_preserved_and_annotated_with_or_without_optimization() {
        for optimize in [false, true] {
            for (variable_type, opcode) in [
                (VariableType::Byte, FuncOpCode::TOBYTE),
                (VariableType::SByte, FuncOpCode::TOSBYTE),
                (VariableType::Word, FuncOpCode::TOWORD),
                (VariableType::SWord, FuncOpCode::TOSWORD),
                (VariableType::Float, FuncOpCode::TOREAL),
                (VariableType::Double, FuncOpCode::TODREAL),
                (VariableType::Long, FuncOpCode::TOLONG64),
                (VariableType::ULong, FuncOpCode::TOULONG64),
                (VariableType::Date, FuncOpCode::TODATE),
                (VariableType::EDate, FuncOpCode::TOEDATE),
                (VariableType::DDate, FuncOpCode::TODDATE),
                (VariableType::Time, FuncOpCode::TOTIME),
            ] {
                let mut visitor = AstTransformationVisitor::new(optimize, Vec::new());
                let value = VariableValue::new_int(42).convert_to(variable_type);
                visitor.global_constants.insert(name("answer"), (variable_type, value));
                let Expression::FunctionCall(call) = identifier("answer", 10).visit_mut(&mut visitor) else {
                    panic!("lost {variable_type} constant with optimize={optimize}")
                };
                assert_eq!(
                    visitor.take_generated_info().function_type_lookup.get(&CallId(call.id)),
                    Some(&SemanticInfo::PredefinedFunc(opcode)),
                    "{variable_type} with optimize={optimize}"
                );
                assert_eq!(call.get_arguments().len(), 1);
                assert!(matches!(call.get_arguments()[0], Expression::Const(_)));
            }
        }
    }

    #[test]
    fn checked_enum_constants_need_no_cast_and_index_conversions_are_annotated() {
        let mut visitor = AstTransformationVisitor::new(true, vec![flags()]);
        for (symbol, value) in [("named", 1), ("unnamed", 3)] {
            visitor
                .global_constants
                .insert(name(symbol), (VariableType::UserData(200), VariableValue::new_int(value)));
            for optimize in [false, true] {
                visitor.optimize_output = optimize;
                assert_eq!(
                    identifier(symbol, 10).visit_mut(&mut visitor),
                    Expression::Const(ConstantExpression::new(
                        Spanned::new(Token::Identifier(name(symbol)), 10..10 + symbol.len()),
                        Constant::Integer(value, NumberFormat::Default),
                    ))
                );
                assert!(visitor.generated.function_type_lookup.is_empty());
            }
        }
        let source_call = FunctionCallExpression::empty(identifier("nextIndex", 30), Vec::new());
        let source_id = source_call.id;
        let mut statements = Vec::new();
        visitor.capture_compound_indices(&[Expression::FunctionCall(source_call)], &mut statements);
        let Statement::Let(assignment) = &statements[1] else {
            panic!("expected index capture")
        };
        let Expression::FunctionCall(conversion) = assignment.get_value_expression() else {
            panic!("expected ToInteger")
        };
        let Expression::FunctionCall(original) = &conversion.get_arguments()[0] else {
            panic!("lost original call")
        };
        assert_eq!(original.id, source_id);
        let generated = visitor.take_generated_info();
        assert_eq!(
            generated.function_type_lookup.get(&CallId(conversion.id)),
            Some(&SemanticInfo::PredefinedFunc(FuncOpCode::TOINTEGER))
        );
        assert!(!generated.function_type_lookup.contains_key(&CallId(source_id)));
        assert_eq!(generated.temporaries[&None][0].0, VariableType::Integer);
    }

    #[test]
    fn condition_negation_keeps_comparison_and_call_metadata() {
        for optimize in [false, true] {
            for (token, inverse) in [
                (Token::Eq, BinOp::NotEq),
                (Token::NotEq, BinOp::Eq),
                (Token::Lower, BinOp::GreaterEq),
                (Token::LowerEq, BinOp::Greater),
                (Token::Greater, BinOp::LowerEq),
                (Token::GreaterEq, BinOp::Lower),
            ] {
                let call = FunctionCallExpression::empty(identifier("nextValue", 10), Vec::new());
                let call_id = call.id;
                let binary = BinaryExpression::new(Expression::FunctionCall(call), Spanned::new(token, 22..24), integer(3));
                let binary_id = binary.id;
                let mut visitor = AstTransformationVisitor::new(optimize, Vec::new());
                let Expression::Binary(negated) = visitor.negate_condition(&Expression::Binary(binary)) else {
                    panic!("comparison must invert without an extra NOT")
                };
                assert_eq!(negated.id, binary_id);
                assert_eq!(negated.get_op(), inverse);
                assert_eq!(negated.get_op_token().span, 22..24);
                let mut ids = Ids::default();
                Expression::Binary(negated).visit(&mut ids);
                assert_eq!(ids.calls, vec![call_id]);
                assert!(visitor.take_generated_info().function_type_lookup.is_empty());
            }
        }
    }

    #[test]
    fn demorgan_keeps_checked_enum_operands_and_source_ids() {
        let bits = BinaryExpression::empty(integer(1), BinOp::Or, integer(2));
        let bits_id = bits.id;
        let comparison = BinaryExpression::new(Expression::Binary(bits), Spanned::new(Token::Eq, 20..21), integer(3));
        let comparison_id = comparison.id;
        let other = BinaryExpression::empty(identifier("x", 30), BinOp::Lower, integer(4));
        let other_id = other.id;
        let condition = BinaryExpression::new(Expression::Binary(comparison), Spanned::new(Token::And, 25..26), Expression::Binary(other));
        let condition_id = condition.id;
        let mut visitor = AstTransformationVisitor::new(true, vec![flags()]);
        visitor.enum_binary_types.insert(bits_id, 200);
        let negated = visitor.negate_condition(&Expression::Binary(condition));
        let Expression::Binary(binary) = &negated else {
            panic!("expected compact De Morgan condition")
        };
        assert_eq!(binary.get_op(), BinOp::Or);
        assert_eq!(binary.get_op_token().span, 25..26);
        let Expression::Binary(comparison) = binary.get_left_expression() else {
            panic!("lost comparison")
        };
        assert_eq!(comparison.get_op(), BinOp::NotEq);
        let Expression::Binary(bits) = comparison.get_left_expression() else {
            panic!("lost enum bits")
        };
        assert_eq!(bits.get_op(), BinOp::Or);
        let mut ids = Ids::default();
        negated.visit(&mut ids);
        assert_eq!(ids.binaries, vec![condition_id, comparison_id, bits_id, other_id]);
    }

    #[test]
    fn negation_removes_double_not_but_keeps_arithmetic_signs() {
        let mut visitor = AstTransformationVisitor::new(true, Vec::new());
        let operand = identifier("x", 10);
        let double_not = UnaryExpression::create_empty_expression(UnaryOp::Not, operand.clone());
        assert_eq!(visitor.negate_condition(&double_not), operand);
        for op in [UnaryOp::Plus, UnaryOp::Minus] {
            let signed = UnaryExpression::create_empty_expression(op, operand.clone());
            let Expression::Unary(negated) = visitor.negate_condition(&signed) else {
                panic!("expected NOT")
            };
            assert_eq!(negated.get_op(), UnaryOp::Not);
            assert_eq!(negated.get_expression(), &signed);
        }
    }

    #[test]
    fn for_direction_guards_fold_without_losing_effectful_bounds() {
        for (step, op) in [(1, BinOp::Greater), (-1, BinOp::Lower)] {
            let source = ForStatement::empty(name("i"), integer(0), identifier("limit", 20), Some(Box::new(integer(step))), Vec::new());
            for optimize in [false, true] {
                let mut visitor = AstTransformationVisitor::new(optimize, Vec::new());
                let Statement::Block(block) = visitor.visit_for_statement(&source) else {
                    panic!("expected lowered FOR")
                };
                let Statement::If(condition) = &block.get_statements()[2] else {
                    panic!("expected loop guard")
                };
                let Expression::Binary(binary) = condition.get_condition() else {
                    panic!("expected comparison")
                };
                assert_eq!(binary.get_op(), if optimize { op } else { BinOp::And });
                if optimize {
                    assert_eq!(binary.get_right_expression(), &identifier("limit", 20));
                }
            }
        }
        let call = FunctionCallExpression::empty(identifier("nextLimit", 20), Vec::new());
        let call_id = call.id;
        let bound = BinaryExpression::empty(Expression::FunctionCall(call), BinOp::Add, integer(1));
        let bound_id = bound.id;
        let source = ForStatement::empty(name("i"), integer(0), Expression::Binary(bound), None, Vec::new());
        let mut visitor = AstTransformationVisitor::new(true, Vec::new());
        let Statement::Block(block) = visitor.visit_for_statement(&source) else {
            panic!("expected lowered FOR")
        };
        let Statement::If(condition) = &block.get_statements()[2] else {
            panic!("expected loop guard")
        };
        let mut ids = Ids::default();
        condition.get_condition().visit(&mut ids);
        assert_eq!(ids.calls, vec![call_id, call_id], "both eager bound evaluations must survive");
        assert_eq!(ids.binaries.iter().filter(|id| **id == bound_id).count(), 2);
    }

    fn member_fixture() -> (AstTransformationVisitor, Expression) {
        let mut registry = UserTypeRegistry::default();
        let mut object = UserDataRegistry::default();
        object.add_property(name("Flags"), VariableType::UserData(200), true);
        registry.types.insert(30, object);
        let mut visitor = AstTransformationVisitor::new(false, vec![flags()]);
        visitor.set_semantic_input(TransformationSemanticInput {
            function_type_lookup: &HashMap::new(),
            enum_binary_types: &HashMap::new(),
            user_type_lookup: &HashMap::from([(20, 30)]),
            compound_target_types: &HashMap::new(),
            type_registry: &registry,
        });
        let target = Expression::MemberReference(MemberReferenceExpression::new(
            identifier("object", 10),
            Spanned::new(Token::Dot, 19..20),
            Spanned::new(Token::Identifier(name("Flags")), 20..25),
        ));
        (visitor, target)
    }

    fn last_setter(statement: &Statement) -> &FunctionCallExpression {
        let Statement::Block(block) = statement else {
            panic!("expected compound block")
        };
        let Some(Statement::MemberCall(setter)) = block.get_statements().last() else {
            panic!("expected setter")
        };
        let Expression::FunctionCall(setter) = setter.get_expression() else {
            panic!("expected setter call")
        };
        setter
    }

    #[test]
    fn source_member_setter_keeps_call_id_and_generated_binary_gets_enum_type() {
        let (mut visitor, target) = member_fixture();
        let call = FunctionCallExpression::new(
            target,
            Spanned::new(Token::OrAssign, 26..28),
            vec![identifier("mask", 30)],
            Spanned::new(Token::Eq, 34..34),
        );
        let source_id = call.id;
        visitor.function_type_lookup.insert(CallId(source_id), SemanticInfo::MemberSetterCall(0));
        let lowered = visitor.visit_member_call_statement(&MemberCallStatement::new(Expression::FunctionCall(call)));
        let setter = last_setter(&lowered);
        assert_eq!(setter.id, source_id);
        assert_eq!(setter.get_lpar_token(), &Spanned::new(Token::Eq, 26..28));
        let Expression::Binary(binary) = &setter.get_arguments()[0] else {
            panic!("expected compound binary")
        };
        let generated = visitor.take_generated_info();
        assert_eq!(generated.enum_binary_types.get(&binary.id), Some(&200));
        assert!(!generated.function_type_lookup.contains_key(&CallId(source_id)));
        assert_eq!(generated.temporaries[&None].len(), 1);
    }

    #[test]
    fn let_member_setter_gets_registry_annotation_without_visiting_semantics() {
        let (mut visitor, target) = member_fixture();
        let source = LetStatement::new(
            None,
            Spanned::new(Token::Identifier(name("object")), 10..16),
            None,
            Vec::new(),
            None,
            Vec::new(),
            Spanned::new(Token::AndAssign, 26..28),
            identifier("mask", 30),
        )
        .with_target_expression(target);
        let lowered = visitor.visit_let_statement(&source);
        let setter = last_setter(&lowered);
        let Expression::Binary(binary) = &setter.get_arguments()[0] else {
            panic!("expected compound binary")
        };
        let generated = visitor.take_generated_info();
        assert_eq!(generated.function_type_lookup.get(&CallId(setter.id)), Some(&SemanticInfo::MemberSetterCall(0)));
        assert_eq!(generated.enum_binary_types.get(&binary.id), Some(&200));
    }

    #[test]
    fn scalar_and_array_compounds_use_resolved_source_target_type() {
        for indexed in [false, true] {
            let mut visitor = AstTransformationVisitor::new(true, vec![flags()]);
            visitor.set_semantic_input(TransformationSemanticInput {
                function_type_lookup: &HashMap::new(),
                enum_binary_types: &HashMap::new(),
                user_type_lookup: &HashMap::new(),
                compound_target_types: &HashMap::from([(10, VariableType::UserData(200))]),
                type_registry: &UserTypeRegistry::default(),
            });
            let source = LetStatement::new(
                None,
                Spanned::new(Token::Identifier(name("bits")), 10..14),
                indexed.then(|| Spanned::new(Token::LPar, 14..15)),
                if indexed { vec![integer(0)] } else { Vec::new() },
                indexed.then(|| Spanned::new(Token::RPar, 16..17)),
                Vec::new(),
                Spanned::new(Token::OrAssign, 18..20),
                identifier("mask", 21),
            );
            let Statement::Let(lowered) = visitor.visit_let_statement(&source) else {
                panic!("unexpected captures")
            };
            let Expression::Binary(binary) = lowered.get_value_expression() else {
                panic!("expected compound binary")
            };
            assert_eq!(visitor.take_generated_info().enum_binary_types.get(&binary.id), Some(&200));
        }
    }

    #[test]
    fn only_generated_declarations_are_drained_in_their_routine_scope() {
        let declaration = Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            VariableType::Integer,
            vec![VariableSpecifier::new(
                Spanned::new(Token::Identifier(name("items")), 10..15),
                None,
                vec![DimensionSpecifier::dynamic()],
                None,
                None,
                Some(Expression::ArrayInitializer(crate::ast::ArrayInitializerExpression::empty(Vec::new()))),
            )],
        ));
        let mut program = Ast::new();
        program.nodes = vec![
            AstNode::Function(FunctionImplementation::empty(
                0,
                name("__M0_f"),
                Vec::new(),
                VariableType::Integer,
                vec![declaration.clone()],
            )),
            AstNode::Procedure(ProcedureImplementation::empty(1, name("__M0_p"), Vec::new(), vec![declaration.clone()])),
            AstNode::TopLevelStatement(declaration),
        ];
        let mut visitor = AstTransformationVisitor::new(false, Vec::new());
        let _ = visitor.visit_ast(&program);
        let generated = visitor.take_generated_info();
        assert_eq!(generated.temporaries.len(), 3);
        let mut names = HashSet::new();
        for scope in [None, Some(name("__M0_f")), Some(name("__M0_p"))] {
            let declarations = &generated.temporaries[&scope];
            assert_eq!(declarations.len(), 1);
            assert_eq!(declarations[0].0, VariableType::Integer);
            assert!(declarations[0].1.get_dimensions()[0].is_dynamic());
            assert!(declarations[0].1.get_identifier().starts_with("*(empty_array"));
            names.insert(declarations[0].1.get_identifier().clone());
        }
        assert_eq!(names.len(), 3);
        let drained = visitor.take_generated_info();
        assert!(drained.temporaries.is_empty());
        assert!(drained.function_type_lookup.is_empty());
        assert!(drained.enum_binary_types.is_empty());
    }
}
