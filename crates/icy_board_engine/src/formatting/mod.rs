use crate::ast::*;

pub mod options;
pub use options::*;

pub mod backend;
pub use backend::*;

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
}

impl<'a> AstVisitor<()> for FormattingVisitor<'a> {
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
        for p in function.get_parameters() {
            p.visit(self);
        }
        self.inc_indent();
        for stmt in function.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(function.get_endfunc_token().span.clone());
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) {
        for p in procedure.get_parameters() {
            p.visit(self);
        }
        self.inc_indent();
        for stmt in procedure.get_statements() {
            self.indent(stmt.get_span());
            stmt.visit(self);
        }
        self.dec_indent();
        self.indent(procedure.get_endproc_token().span.clone());
    }
}
