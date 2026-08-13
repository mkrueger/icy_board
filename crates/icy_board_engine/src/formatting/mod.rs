use crate::ast::*;

pub mod options;
pub use options::*;

pub mod backend;
pub use backend::*;

/// Where a parameter starts, which is its `VAR` when it has one.
fn parameter_start(parameter: &ParameterSpecifier) -> usize {
    match parameter {
        ParameterSpecifier::Variable(variable) => variable
            .get_var_token()
            .as_ref()
            .map_or(variable.get_type_token().span.start, |var| var.span.start),
        ParameterSpecifier::Function(function) => function.get_function_token().span.start,
        ParameterSpecifier::Procedure(procedure) => procedure.get_procedure_token().span.start,
    }
}

pub struct FormattingVisitor<'a> {
    pub backend: &'a mut dyn FormattingBackend,
    pub options: &'a FormattingOptions,
    indent: usize,
    indent_str: Option<String>,
}

impl<'a> FormattingVisitor<'a> {
    pub fn new(backend: &'a mut dyn FormattingBackend, options: &'a FormattingOptions) -> Self {
        Self {
            backend,
            options,
            indent: 0,
            indent_str: None,
        }
    }

    fn ensure_text_or_newline(&mut self, start: std::ops::Range<usize>, arg: &str) {
        self.backend.ensure_text_or_newline(start, arg);
    }

    fn indent(&mut self, span: core::ops::Range<usize>) {
        self.update_indent_str();
        if let Some(indent_str) = &self.indent_str {
            self.backend.indent(indent_str, span);
        }
    }

    fn ensure_space_before(&mut self, start: usize) {
        self.backend.ensure_space_before(start);
    }

    fn ensure_no_space_after(&mut self, start: usize) {
        self.backend.ensure_no_space_after(start);
    }

    fn ensure_space_around(&mut self, range: core::ops::Range<usize>) {
        self.backend.ensure_space_around(range);
    }

    /// `a, b` for a list of nodes, where the first one follows an opening bracket.
    fn separate(&mut self, starts: &[usize]) {
        for start in starts {
            self.ensure_space_before(*start);
        }
    }

    fn format_parameters(&mut self, parameters: &[ParameterSpecifier]) {
        let starts: Vec<usize> = parameters.iter().map(parameter_start).collect();
        self.separate(&starts);
        for parameter in parameters {
            if let ParameterSpecifier::Function(function) = parameter {
                self.format_parameters(function.get_parameters());
            }
            if let ParameterSpecifier::Procedure(procedure) = parameter {
                self.format_parameters(procedure.get_parameters());
            }
        }
    }

    fn format_arguments(&mut self, get_arguments: &[Expression]) {
        for arg in get_arguments {
            self.ensure_space_before(arg.get_span().start);
            arg.visit(self);
            self.ensure_no_space_after(arg.get_span().end);
        }
    }

    fn inc_indent(&mut self) {
        self.indent += 1;
        self.indent_str = None;
    }

    fn dec_indent(&mut self) {
        self.indent -= 1;
        self.indent_str = None;
    }

    fn update_indent_str(&mut self) {
        if self.indent_str.is_none() {
            let one_indent = if self.options.use_tabs {
                "\t".to_string()
            } else {
                " ".repeat(self.options.indent_size)
            };
            self.indent_str = Some(one_indent.repeat(self.indent));
        }
    }

    /// Formats a whole file, which is where the top level starts at column zero.
    pub fn format(&mut self, ast: &Ast) {
        for node in &ast.nodes {
            let start = match node {
                AstNode::TopLevelStatement(statement) => Some(statement.get_span().start),
                AstNode::Function(function) => Some(function.get_function_token().span.start),
                AstNode::Procedure(procedure) => Some(procedure.get_procedure_token().span.start),
                AstNode::FunctionDeclaration(function) => Some(function.get_declare_token().span.start),
                AstNode::ProcedureDeclaration(procedure) => Some(procedure.get_declare_token().span.start),
                AstNode::TypeDeclaration(declaration) => Some(declaration.get_type_token().span.start),
                AstNode::Main(_) => None,
            };
            if let Some(start) = start {
                self.indent(start..start);
            }
            node.visit(self);
        }
    }
}

impl<'a> AstVisitor<()> for FormattingVisitor<'a> {
    fn visit_main(&mut self, block: &BlockStatement) {
        for stmt in block.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) {
        // `p . X = 1` names its members with tokens rather than an expression.
        let mut left = let_stmt.get_identifier_token().span.end;
        if let Some(rpar) = let_stmt.get_rpar_token() {
            left = rpar.span.end;
        }
        for member in let_stmt.get_members() {
            self.backend.ensure_no_space_after(left);
            self.backend.ensure_no_space_before(member.span.start);
            left = member.span.end;
        }

        let eq = &let_stmt.get_eq_token().span;
        self.ensure_space_before(eq.start);
        self.ensure_text_or_newline(eq.end..let_stmt.get_value_expression().get_span().start, " ");
        for argument in let_stmt.get_arguments() {
            argument.visit(self);
        }
        let_stmt.get_value_expression().visit(self);
    }

    fn visit_type_declaration(&mut self, type_declaration: &TypeDeclarationAstNode) {
        self.inc_indent();
        let mut indented_line = None;
        for field in type_declaration.get_fields() {
            let type_span = field.get_type_token().span.clone();
            // Several fields may share one type, and that line is indented once.
            if indented_line == Some(type_span.start) {
                self.ensure_space_before(field.get_specifier().get_identifier_token().span.start);
            } else {
                self.indent(type_span.clone());
                indented_line = Some(type_span.start);
            }
        }
        self.dec_indent();
        self.indent(type_declaration.get_endtype_token().span.clone());
    }

    fn visit_record_literal_expression(&mut self, record: &RecordLiteralExpression) {
        self.ensure_space_before(record.get_lbrace_token().span.start);
        for field in record.get_fields() {
            let name = &field.get_identifier_token().span;
            self.ensure_space_before(name.start);
            self.ensure_space_around(name.end..field.get_value().get_span().start);
            field.get_value().visit(self);
        }
        if record.get_fields().is_empty() {
            self.ensure_text_or_newline(record.get_lbrace_token().span.end..record.get_rbrace_token().span.start, "");
        } else {
            self.ensure_space_before(record.get_rbrace_token().span.start);
        }
    }

    fn visit_array_expression(&mut self, array: &ArrayInitializerExpression) {
        for expression in array.get_expressions() {
            self.ensure_space_before(expression.get_span().start);
            expression.visit(self);
        }
        if array.get_expressions().is_empty() {
            self.ensure_text_or_newline(array.get_lbrace_token().span.end..array.get_rbrace_token().span.start, "");
        } else {
            self.ensure_space_before(array.get_rbrace_token().span.start);
        }
    }

    fn visit_indexer_expression(&mut self, indexer: &IndexerExpression) {
        self.ensure_text_or_newline(
            indexer.get_identifier_token().span.end..indexer.get_lbracket_token().span.start,
            "",
        );
        self.ensure_no_space_after(indexer.get_lbracket_token().span.end);
        for argument in indexer.get_arguments() {
            argument.visit(self);
        }
        if let Some(last) = indexer.get_arguments().last() {
            self.ensure_text_or_newline(last.get_span().end..indexer.get_rbracket_token().span.start, "");
        }
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) {
        member.get_expression().visit(self);
        let dot = &member.get_dot_token().span;
        self.ensure_text_or_newline(member.get_expression().get_span().end..dot.start, "");
        self.ensure_text_or_newline(dot.end..member.get_identifier_token().span.start, "");
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression) {
        let op_end = unary.get_op_token().span.end;
        let expr_start = unary.get_expression().get_span().start;
        self.ensure_text_or_newline(op_end..expr_start, "");

        unary.get_expression().visit(self)
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) {
        let left_end = binary.get_left_expression().get_span().end;
        let start = binary.get_op_token().span.start;

        let end = binary.get_op_token().span.end;
        let right_start = binary.get_right_expression().get_span().start;
        if self.options.space_around_binop {
            self.ensure_text_or_newline(left_end..start, " ");
            self.ensure_text_or_newline(end..right_start, " ");
        } else {
            self.ensure_text_or_newline(left_end..start, "");
            self.ensure_text_or_newline(end..right_start, "");
        }

        walk_binary_expression(self, binary);
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> () {
        call.get_expression().visit(self);
        self.format_arguments(call.get_arguments());
    }

    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) -> () {
        self.format_arguments(call.get_arguments());
    }

    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) -> () {
        self.format_arguments(call.get_arguments());
    }

    fn visit_if_then_statement(&mut self, if_then: &IfThenStatement) {
        if_then.get_condition().visit(self);
        self.inc_indent();
        for stmt in if_then.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();

        for stmt in if_then.get_else_if_blocks() {
            self.indent(stmt.get_elseif_token().span.clone());
            stmt.get_condition().visit(self);
            self.inc_indent();
            for stmt in stmt.get_statements() {
                self.indent(stmt.get_span());
                stmt.visit(self);
            }
            self.dec_indent();
        }

        if let Some(else_block) = if_then.get_else_block() {
            self.indent(else_block.get_else_token().span.clone());
            self.inc_indent();
            for stmt in else_block.get_statements() {
                self.indent(stmt.get_span());
                stmt.visit(self);
            }
            self.dec_indent();
        }
        self.indent(if_then.get_endif_token().span.clone());
    }

    fn visit_select_statement(&mut self, select_stmt: &SelectStatement) {
        select_stmt.get_expression().visit(self);

        for case_block in select_stmt.get_case_blocks() {
            self.indent(case_block.get_case_token().span.clone());
            for specifier in case_block.get_case_specifiers() {
                specifier.visit(self);
            }
            self.inc_indent();
            for stmt in case_block.get_statements() {
                self.indent(stmt.get_span());
                stmt.visit(self);
            }
            self.dec_indent();
        }
        if let Some(dt) = select_stmt.get_default_token() {
            self.indent(dt.span.clone());
        }

        self.inc_indent();
        for stmt in select_stmt.get_default_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(select_stmt.get_endselect_token().span.clone());
    }

    fn visit_for_statement(&mut self, for_stmt: &ForStatement) {
        for_stmt.get_start_expr().visit(self);
        for_stmt.get_end_expr().visit(self);
        if let Some(step) = for_stmt.get_step_expr() {
            step.visit(self);
        }
        self.inc_indent();
        for stmt in for_stmt.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(for_stmt.get_next_token().span.clone());
    }

    fn visit_while_do_statement(&mut self, while_do_stmt: &WhileDoStatement) {
        while_do_stmt.get_condition().visit(self);
        self.inc_indent();
        for stmt in while_do_stmt.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(while_do_stmt.get_endwhile_token().span.clone());
    }

    fn visit_repeat_until_statement(&mut self, repeat_until_stmt: &RepeatUntilStatement) {
        self.inc_indent();
        for stmt in repeat_until_stmt.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(repeat_until_stmt.get_until_token().span.clone());
        repeat_until_stmt.get_condition().visit(self);
    }

    fn visit_loop_statement(&mut self, loop_stmt: &LoopStatement) {
        self.inc_indent();
        for stmt in loop_stmt.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(loop_stmt.get_endloop_token().span.clone());
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) {
        self.format_parameters(function.get_parameters());
        self.inc_indent();
        for stmt in function.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(function.get_endfunc_token().span.clone());
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) {
        self.format_parameters(procedure.get_parameters());
        self.inc_indent();
        for stmt in procedure.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(procedure.get_endproc_token().span.clone());
    }

    fn visit_function_declaration(&mut self, function: &FunctionDeclarationAstNode) {
        self.format_parameters(function.get_parameters());
    }

    fn visit_procedure_declaration(&mut self, procedure: &ProcedureDeclarationAstNode) {
        self.format_parameters(procedure.get_parameters());
    }
}
