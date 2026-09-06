use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        Ast, AstNode, AstVisitorMut, BinaryExpression, BlockStatement, CommentAstNode, ConstDeclarationStatement, Constant, ConstantExpression,
        DimensionSpecifier, Expression, ForEachStatement, ForStatement, FunctionImplementation, GotoStatement, IdentifierExpression, IfStatement,
        LabelStatement, LetStatement, MemberReferenceExpression, ParameterSpecifier, ProcedureImplementation, ReturnStatement, SelectStatement, Statement,
        VariableDeclarationStatement, VariableSpecifier, const_expression, const_value_with_members, constant::NumberFormat,
    },
    decompiler::evaluation_visitor::{ConstantFolder, OptimizationVisitor},
    executable::{VariableType, VariableValue},
    parser::{
        EnumDefinition,
        lexer::{Spanned, Token},
    },
};

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
    temporaries: usize,
}

impl AstTransformationVisitor {
    pub(crate) fn needs_compound_receiver_types(program: &Ast) -> bool {
        #[derive(Default)]
        struct FindMemberCompound(bool);
        impl crate::ast::AstVisitor<()> for FindMemberCompound {
            fn visit_let_statement(&mut self, statement: &LetStatement) {
                self.0 |= AstTransformationVisitor::compound_operator(statement.get_let_variant()).is_some()
                    && (!statement.get_members().is_empty() || statement.get_target_expression().is_some());
            }

            fn visit_member_call_statement(&mut self, statement: &crate::ast::MemberCallStatement) {
                if let Expression::FunctionCall(call) = statement.get_expression() {
                    self.0 |= AstTransformationVisitor::compound_operator(&call.get_lpar_token().token).is_some();
                }
            }
        }
        let mut visitor = FindMemberCompound::default();
        program.visit(&mut visitor);
        visitor.0
    }

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
            temporaries: 0,
        }
    }

    /// Receiver types come from a non-emitting semantic pass. Records must keep
    /// their storage path; reference objects must instead keep their identity.
    pub(crate) fn set_compound_receiver_types(&mut self, types: HashMap<usize, u8>, registry: &crate::parser::UserTypeRegistry) {
        self.compound_record_types = types.values().copied().filter(|id| registry.is_record_type(*id)).collect();
        self.compound_receiver_types = types;
    }

    fn capture_compound_value(&mut self, value: Expression, variable_type: VariableType, statements: &mut Vec<Statement>) -> Expression {
        let name = unicase::Ascii::new(format!("*(compound{})", self.temporaries));
        self.temporaries += 1;
        statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            variable_type,
            vec![VariableSpecifier::empty(name.clone(), Vec::new())],
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
                    let index = crate::ast::FunctionCallExpression::create_empty_expression(
                        IdentifierExpression::create_empty_expression(unicase::Ascii::new("ToInteger".to_string())),
                        vec![argument.clone()],
                    );
                    self.capture_compound_value(index, VariableType::Integer, statements)
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

    /// Where the FOR statements of the file just transformed keep their count.
    pub fn take_loop_counters(&mut self) -> HashSet<usize> {
        std::mem::take(&mut self.loop_counters)
    }

    pub fn next_label(&mut self) -> unicase::Ascii<String> {
        let label = unicase::Ascii::new(format!("*(label{}", self.labels));
        self.labels += 1;
        label
    }

    fn enum_member_value(&self, type_name: &unicase::Ascii<String>, member: &unicase::Ascii<String>) -> Option<VariableValue> {
        let definition = self.enums.iter().find(|definition| definition.name == *type_name)?;
        definition.value(member).map(VariableValue::new_int)
    }

    /// The `Enum.Member` a value stands for, so an enum constant keeps its type until
    /// the members are lowered.
    fn enum_member_expression(&self, id: u8, value: i32) -> Option<Expression> {
        let definition = self.enums.iter().find(|definition| definition.id == id)?;
        let member = definition.variant_name(value)?;
        Some(MemberReferenceExpression::create_empty_expression(
            IdentifierExpression::create_empty_expression(definition.name.clone()),
            member.clone(),
        ))
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
            let Some(value) = const_value_with_members(
                const_decl.get_value(),
                &|id| self.lookup_constant(id).map(|(_, value)| value.clone()),
                &|type_name, member| self.enum_member_value(type_name, member),
            ) else {
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
        let transformed = crate::ast::UnaryExpression::empty(unary.get_op(), unary.get_expression().visit_mut(self));
        if self.optimize_output {
            ConstantFolder::default().visit_unary_expression(&transformed)
        } else {
            Expression::Unary(transformed)
        }
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> Expression {
        let transformed = BinaryExpression::empty(
            binary.get_left_expression().visit_mut(self),
            binary.get_op(),
            binary.get_right_expression().visit_mut(self),
        );
        if self.optimize_output {
            ConstantFolder::default().visit_binary_expression(&transformed)
        } else {
            Expression::Binary(transformed)
        }
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
            return Statement::If(IfStatement::empty(
                if_stmt.get_condition().visit_mut(self),
                if_stmt.get_statement().visit_mut(self),
            ));
        }
        let mut statements = Vec::new();
        let if_exit_label = self.next_label();
        statements.push(IfStatement::create_empty_statement(
            if_stmt.get_condition().negate_expression().visit_mut(self),
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
            if_then.get_condition().negate_expression().visit_mut(self),
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
                else_if.get_condition().negate_expression().visit_mut(self),
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
            while_stmt.get_condition().negate_expression().visit_mut(self),
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
            while_do.get_condition().negate_expression().visit_mut(self),
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
            repeat_until.get_condition().negate_expression().visit_mut(self),
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
        let expr = select_stmt.get_expression().clone();
        let case_exit_label = self.next_label();

        for case_block in select_stmt.get_case_blocks() {
            let next_case_label = self.next_label();

            let mut condition = ConstantExpression::create_empty_expression(Constant::Boolean(false));

            for spec in case_block.get_case_specifiers() {
                let cond = match spec {
                    crate::ast::CaseSpecifier::Expression(spec_expr) => {
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::NotEq, expr.clone(), *spec_expr.clone())
                    }
                    crate::ast::CaseSpecifier::FromTo(from_expr, to_expr) => BinaryExpression::create_empty_expression(
                        crate::ast::BinOp::Or,
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, *from_expr.clone(), expr.clone()),
                        BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, expr.clone(), *to_expr.clone()),
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
        statements.push(IfStatement::create_empty_statement(
            if self.optimize_output {
                condition.visit_mut(&mut OptimizationVisitor::default())
            } else {
                condition
            },
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

        // A compound assignment reads the same place it writes, including indices and members.
        let mut target = if let Some(target) = &transformed_target {
            target.clone()
        } else if let (Some(left), Some(right)) = (let_stmt.get_lpar_token(), let_stmt.get_rpar_token()) {
            Expression::Indexer(crate::ast::IndexerExpression::new(
                let_stmt.get_identifier_token().clone(),
                left.clone(),
                let_stmt.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect(),
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
            target = self.capture_compound_target(target, &mut statements);
            val_expr = BinaryExpression::create_empty_expression(op, target.clone(), val_expr);
            if let Expression::MemberReference(member) = &target
                && self.compound_object_receiver_type(member).is_some()
            {
                // Keep object writes on the setter path, including its normal
                // writable-property/type diagnostics.
                statements.push(Statement::MemberCall(crate::ast::MemberCallStatement::new(Expression::FunctionCall(
                    crate::ast::FunctionCallExpression::new(target, Spanned::create_empty(Token::Eq), vec![val_expr], Spanned::create_empty(Token::Eq)),
                ))));
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
            let_stmt.get_arguments().iter().map(|arg| arg.visit_mut(self)).collect(),
            let_stmt.get_rpar_token().clone(),
            let_stmt.get_members().clone(),
            Spanned::create_empty(Token::Eq),
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
            && let Some(op) = Self::compound_operator(&call.get_lpar_token().token)
            && let Expression::MemberReference(member) = call.get_expression()
            && !member.get_identifier().starts_with('<')
            && self.compound_object_receiver_type(member).is_some()
            && call.get_arguments().len() == 1
        {
            let mut statements = Vec::new();
            let target = self.capture_compound_target(call.get_expression().clone(), &mut statements);
            let value = BinaryExpression::create_empty_expression(op, target.clone(), call.get_arguments()[0].clone());
            statements.push(Statement::MemberCall(crate::ast::MemberCallStatement::new(Expression::FunctionCall(
                crate::ast::FunctionCallExpression::new(target, Spanned::create_empty(Token::Eq), vec![value], Spanned::create_empty(Token::Eq)),
            ))));
            return Statement::Block(BlockStatement::empty(statements));
        }
        Statement::MemberCall(crate::ast::MemberCallStatement::new(expression))
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> AstNode {
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
        self.local_constants = None;
        self.local_bindings = None;
        res
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> AstNode {
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
        res
    }

    /// The value takes the place of the name everywhere it is used. The declaration
    /// itself stays for the checks that come after; the code generator skips it.
    fn visit_const_declaration_statement(&mut self, const_decl: &ConstDeclarationStatement) -> Statement {
        Statement::ConstDeclaration(const_decl.clone())
    }

    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> Expression {
        if let Some((variable_type, value)) = self.lookup_constant(identifier.get_identifier()) {
            if let crate::executable::VariableType::UserData(id) = variable_type {
                if let Some(expr) = self.enum_member_expression(*id, value.as_int()) {
                    return expr;
                }
            } else if let Some(expr) = const_expression(value, *variable_type) {
                return expr;
            }
        }
        Expression::Identifier(IdentifierExpression::new(Spanned {
            span: identifier.get_identifier_token().span.clone(),
            token: Token::Identifier(self.visit_identifier(identifier.get_identifier())),
        }))
    }

    fn visit_ast(&mut self, program: &Ast) -> Ast {
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
        statements.push(ReturnStatement::create_empty_statement(None));
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
                            None,
                            if dynamic {
                                var.get_dimensions().clone()
                            } else {
                                vec![DimensionSpecifier::empty(array.get_expressions().len().saturating_sub(1))]
                            },
                            None,
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
                        statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::new(
                            var_decl.get_type_token().clone(),
                            var_decl.get_variable_type(),
                            vec![VariableSpecifier::new(
                                Spanned::create_empty(Token::Identifier(empty_name.clone())),
                                None,
                                var.get_dimensions().clone(),
                                None,
                                None,
                                None,
                            )],
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
                            None,
                            var.get_dimensions().clone(),
                            None,
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
                    vec![VariableSpecifier::new(
                        var.get_identifier_token().clone(),
                        None,
                        var.get_dimensions().clone(),
                        None,
                        None,
                        None,
                    )],
                )));
            }
        }
        Statement::Block(BlockStatement::empty(statements))
    }
}
