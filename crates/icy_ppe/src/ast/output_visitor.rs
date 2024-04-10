use super::{AstVisitor, BlockStatement, Statement};

#[repr(u8)]
#[derive(PartialEq, Debug, Default)]
pub enum OutputFunc {
    #[default]
    Upper,
    Lower,
    CamelCase,
}

pub static mut DEFAULT_OUTPUT_FUNC: OutputFunc = OutputFunc::Upper;

#[derive(Default)]
pub struct OutputVisitor {
    pub output_func: OutputFunc,
    pub skip_comments: bool,
    pub output: String,
    indent: i32,
}

impl OutputVisitor {
    fn output(&mut self, format: &str) {
        self.output.push_str(format);
    }

    fn output_keyword(&mut self, str: &str) {
        match self.output_func {
            OutputFunc::Upper => self.output.push_str(&str.to_uppercase()),
            OutputFunc::Lower => self.output.push_str(&str.to_lowercase()),
            OutputFunc::CamelCase => self.output.push_str(str),
        };
    }

    fn indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn eol(&mut self) {
        self.output.push('\n');
    }

    fn output_statements(&mut self, get_statements: &[Statement]) {
        for stmt in get_statements {
            if !matches!(stmt, Statement::Label(_)) {
                self.indent();
            }
            stmt.visit(self);
            self.eol();
        }
    }
}

impl AstVisitor<()> for OutputVisitor {
    fn visit_identifier_expression(&mut self, identifier: &super::IdentifierExpression) {
        self.output(identifier.get_identifier());
    }

    fn visit_constant_expression(&mut self, constant: &super::ConstantExpression) {
        match constant.get_constant_value() {
            super::Constant::Builtin(b) => {
                self.output_keyword(b.name);
            }
            super::Constant::String(s) => {
                self.output.push_str(&format!("\"{s}\""));
            }
            super::Constant::Boolean(b) => {
                if *b {
                    self.output_keyword("True");
                } else {
                    self.output_keyword("False");
                }
            }
            _ => {
                self.output
                    .push_str(&format!("{}", constant.get_constant_value().get_value()));
            }
        }
    }

    fn visit_binary_expression(&mut self, binary: &super::BinaryExpression) {
        binary.get_left_expression().visit(self);
        self.output(&format!(" {} ", binary.get_op()));
        binary.get_right_expression().visit(self);
    }

    fn visit_unary_expression(&mut self, unary: &super::UnaryExpression) {
        self.output(&format!("{}", unary.get_op()));
        unary.get_expression().visit(self);
    }

    fn visit_predefined_function_call_expression(
        &mut self,
        call: &super::PredefinedFunctionCallExpression,
    ) {
        self.output_keyword(call.get_identifier());
        self.output.push('(');
        for (i, arg) in call.get_arguments().iter().enumerate() {
            arg.visit(self);
            if i < call.get_arguments().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push(')');
    }

    fn visit_function_call_expression(&mut self, call: &super::FunctionCallExpression) {
        self.output(call.get_identifier());
        self.output.push('(');
        for (i, arg) in call.get_arguments().iter().enumerate() {
            arg.visit(self);
            if i < call.get_arguments().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push(')');
    }

    fn visit_parens_expression(&mut self, parens: &super::ParensExpression) {
        self.output.push('(');
        parens.get_expression().visit(self);
        self.output.push(')');
    }

    fn visit_comment(&mut self, comment: &super::CommentAstNode) {
        if self.skip_comments {
            return;
        }
        self.output
            .push_str(&comment.get_comment_type().to_string());
        self.output.push_str(comment.get_comment());
    }

    fn visit_main(&mut self, block: &BlockStatement) {
        /*        self.output_keyword("Begin");
        self.eol();*/

        self.indent += 1;
        self.output_statements(block.get_statements());
        self.indent -= 1;

        /*        self.indent();
        self.output_keyword("End");
        self.eol();*/
    }

    fn visit_block_statement(&mut self, block: &super::BlockStatement) {
        self.output_keyword("Begin");
        self.eol();

        self.indent += 1;
        self.output_statements(block.get_statements());
        self.indent -= 1;

        self.indent();
        self.output_keyword("End");
        self.eol();
    }

    fn visit_if_statement(&mut self, if_stmt: &super::IfStatement) {
        self.output_keyword("If");
        self.output.push_str(" (");
        if_stmt.get_condition().visit(self);
        self.output.push(')');
        self.output.push(' ');

        /*
        self.eol();
        self.indent();
        */
        self.indent += 1;
        if_stmt.get_statement().visit(self);
        self.eol();
        self.indent -= 1;
    }

    fn visit_if_then_statement(&mut self, if_then: &super::IfThenStatement) {
        self.output_keyword("If");
        self.output.push_str(" (");
        if_then.get_condition().visit(self);
        self.output.push(')');
        self.output_keyword(" Then");
        self.eol();

        self.indent += 1;
        self.output_statements(if_then.get_statements());
        self.indent -= 1;

        for if_else in if_then.get_else_if_blocks() {
            self.indent();
            self.output_keyword("ElseIf");
            self.output.push_str(" (");
            if_else.get_condition().visit(self);
            self.output.push(')');
            self.output_keyword(" Then");
            self.eol();

            self.indent += 1;
            self.output_statements(if_else.get_statements());
            self.indent -= 1;
        }

        if let Some(else_block) = if_then.get_else_block() {
            self.indent();
            self.output_keyword("Else");
            self.eol();

            self.indent += 1;
            self.output_statements(else_block.get_statements());
            self.indent -= 1;
        }

        self.indent();
        self.output_keyword("EndIf");
        self.eol();
    }

    fn visit_select_statement(&mut self, select_stmt: &super::SelectStatement) {
        self.output_keyword("Select Case ");
        select_stmt.get_expression().visit(self);
        self.eol();

        for case_block in select_stmt.get_case_blocks() {
            self.indent();
            self.output_keyword("Case ");
            for (i, spec) in case_block.get_case_specifiers().iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                spec.visit(self);
            }
            self.eol();

            self.indent += 1;
            self.output_statements(case_block.get_statements());
            self.indent -= 1;
        }

        if !select_stmt.get_default_statements().is_empty() {
            self.indent();
            self.output_keyword("Default");
            self.eol();

            self.indent += 1;
            self.output_statements(select_stmt.get_default_statements());
            self.indent -= 1;
        }

        self.indent();
        self.output_keyword("EndSelect");
        self.eol();
    }

    fn visit_while_statement(&mut self, while_stmt: &super::WhileStatement) {
        self.output_keyword("While");
        self.output.push_str(" (");
        while_stmt.get_condition().visit(self);
        self.output.push(')');
        self.output.push(' ');

        /*self.eol();
        self.indent += 1;
        self.indent();
        */
        while_stmt.get_statement().visit(self);
        self.eol();
        self.indent -= 1;
    }

    fn visit_while_do_statement(&mut self, while_do_stmt: &super::WhileDoStatement) {
        self.output_keyword("While");
        self.output.push_str(" (");
        while_do_stmt.get_condition().visit(self);
        self.output.push(')');
        self.output_keyword(" Do");
        self.eol();

        self.indent += 1;
        self.output_statements(while_do_stmt.get_statements());
        self.indent -= 1;

        self.indent();
        self.output_keyword("EndWhile");
        self.eol();
    }

    fn visit_for_statement(&mut self, for_stmt: &super::ForStatement) {
        self.output_keyword("For");
        self.output.push(' ');
        self.output(for_stmt.get_identifier());
        self.output.push(' ');
        self.output.push('=');
        self.output.push(' ');
        for_stmt.get_start_expr().visit(self);
        self.output.push(' ');

        self.output_keyword("To");
        self.output.push(' ');
        for_stmt.get_end_expr().visit(self);
        self.output.push(' ');

        if let Some(step_expr) = for_stmt.get_step_expr() {
            self.output_keyword("Step");
            self.output.push(' ');
            step_expr.visit(self);
        }
        self.eol();

        self.indent += 1;
        self.output_statements(for_stmt.get_statements());
        self.indent -= 1;

        self.indent();
        self.output_keyword("Next");
        self.eol();
    }

    fn visit_break_statement(&mut self, _break_stmt: &super::BreakStatement) {
        self.output_keyword("Break");
    }

    fn visit_continue_statement(&mut self, _continue_stmt: &super::ContinueStatement) {
        self.output_keyword("Continue");
    }

    fn visit_gosub_statement(&mut self, gosub: &super::GosubStatement) {
        self.output_keyword("GoSub ");
        self.output(gosub.get_label());
    }

    fn visit_return_statement(&mut self, _return_stmt: &super::ReturnStatement) {
        self.output_keyword("Return");
    }

    fn visit_let_statement(&mut self, let_stmt: &super::LetStatement) {
        if let_stmt.get_let_token().is_some() {
            self.output_keyword("Let ");
        }

        self.output(let_stmt.get_identifier());
        if !let_stmt.get_arguments().is_empty() {
            self.output.push('(');
            for (i, arg) in let_stmt.get_arguments().iter().enumerate() {
                arg.visit(self);
                if i < let_stmt.get_arguments().len() - 1 {
                    self.output.push_str(", ");
                }
            }
            self.output.push(')');
        }
        self.output.push_str(" = ");
        let_stmt.get_value_expression().visit(self);
    }

    fn visit_goto_statement(&mut self, goto: &super::GotoStatement) {
        self.output_keyword("Goto ");
        self.output(goto.get_label());
    }

    fn visit_label_statement(&mut self, label: &super::LabelStatement) {
        self.output.push(':');
        self.output(label.get_label());
    }

    fn visit_procedure_call_statement(&mut self, call: &super::ProcedureCallStatement) {
        self.output(call.get_identifier());
        self.output.push('(');
        for (i, arg) in call.get_arguments().iter().enumerate() {
            arg.visit(self);
            if i < call.get_arguments().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push(')');
    }

    fn visit_predefined_call_statement(&mut self, call: &super::PredefinedCallStatement) {
        self.output_keyword(call.get_func().name);
        self.output.push(' ');
        for (i, arg) in call.get_arguments().iter().enumerate() {
            arg.visit(self);
            if i < call.get_arguments().len() - 1 {
                self.output.push_str(", ");
            }
        }
    }

    fn visit_variable_declaration_statement(
        &mut self,
        var_decl: &super::VariableDeclarationStatement,
    ) {
        self.output_keyword(var_decl.get_variable_type().to_string().as_str());
        self.output.push(' ');
        for (i, var) in var_decl.get_variables().iter().enumerate() {
            self.output(var.get_identifier());
            if !var.get_dimensions().is_empty() {
                self.output.push('(');
                for (j, dim) in var.get_dimensions().iter().enumerate() {
                    self.output
                        .push_str(dim.get_dimension().to_string().as_str());
                    if j < var.get_dimensions().len() - 1 {
                        self.output.push_str(", ");
                    }
                }
                self.output.push(')');
            }
            if i < var_decl.get_variables().len() - 1 {
                self.output.push_str(", ");
            }
        }
    }

    fn visit_procedure_declaration(&mut self, proc_decl: &super::ProcedureDeclarationAstNode) {
        self.output_keyword("Declare Procedure ");
        self.output(proc_decl.get_identifier());
        self.output.push('(');
        for (i, arg) in proc_decl.get_parameters().iter().enumerate() {
            if arg.is_var() {
                self.output_keyword("Var");
                self.output.push(' ');
            }
            self.output_keyword(arg.get_variable_type().to_string().as_str());
            self.output.push(' ');
            self.output(arg.get_variable().get_identifier());

            if !arg.get_variable().get_dimensions().is_empty() {
                self.output.push('(');
                for (j, dim) in arg.get_variable().get_dimensions().iter().enumerate() {
                    self.output
                        .push_str(dim.get_dimension().to_string().as_str());
                    if j < arg.get_variable().get_dimensions().len() - 1 {
                        self.output.push_str(", ");
                    }
                }
                self.output.push(')');
            }

            if i < proc_decl.get_parameters().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push(')');
    }

    fn visit_function_declaration(&mut self, func_decl: &super::FunctionDeclarationAstNode) {
        self.output_keyword("Declare Function ");
        self.output(func_decl.get_identifier());
        self.output.push('(');
        for (i, arg) in func_decl.get_parameters().iter().enumerate() {
            if arg.is_var() {
                self.output_keyword("Var");
                self.output.push(' ');
            }
            self.output_keyword(arg.get_variable_type().to_string().as_str());
            self.output.push(' ');
            self.output(arg.get_variable().get_identifier());

            if !arg.get_variable().get_dimensions().is_empty() {
                self.output.push('(');
                for (j, dim) in arg.get_variable().get_dimensions().iter().enumerate() {
                    self.output
                        .push_str(dim.get_dimension().to_string().as_str());
                    if j < arg.get_variable().get_dimensions().len() - 1 {
                        self.output.push_str(", ");
                    }
                }
                self.output.push(')');
            }

            if i < func_decl.get_parameters().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push_str(") ");
        self.output_keyword(func_decl.get_return_type().to_string().as_str());
    }

    fn visit_function_implementation(&mut self, function: &super::FunctionImplementation) {
        self.output_keyword("Function ");
        self.output(function.get_identifier());
        self.output.push('(');
        for (i, arg) in function.get_parameters().iter().enumerate() {
            if arg.is_var() {
                self.output_keyword("Var");
                self.output.push(' ');
            }
            self.output_keyword(arg.get_variable_type().to_string().as_str());
            self.output.push(' ');
            self.output(arg.get_variable().get_identifier());

            if !arg.get_variable().get_dimensions().is_empty() {
                self.output.push('(');
                for (j, dim) in arg.get_variable().get_dimensions().iter().enumerate() {
                    self.output
                        .push_str(dim.get_dimension().to_string().as_str());
                    if j < arg.get_variable().get_dimensions().len() - 1 {
                        self.output.push_str(", ");
                    }
                }
                self.output.push(')');
            }

            if i < function.get_parameters().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push_str(") ");
        self.output_keyword(function.get_return_type().to_string().as_str());
        self.eol();

        self.indent += 1;
        self.output_statements(function.get_statements());
        self.indent -= 1;

        self.indent();
        self.output_keyword("EndFunc");
        self.eol();
    }

    fn visit_procedure_implementation(&mut self, procedure: &super::ProcedureImplementation) {
        self.output_keyword("Procedure ");
        self.output(procedure.get_identifier());
        self.output.push('(');
        for (i, arg) in procedure.get_parameters().iter().enumerate() {
            if arg.is_var() {
                self.output_keyword("Var");
                self.output.push(' ');
            }
            self.output_keyword(arg.get_variable_type().to_string().as_str());
            self.output.push(' ');
            self.output(arg.get_variable().get_identifier());

            if !arg.get_variable().get_dimensions().is_empty() {
                self.output.push('(');
                for (j, dim) in arg.get_variable().get_dimensions().iter().enumerate() {
                    self.output
                        .push_str(dim.get_dimension().to_string().as_str());
                    if j < arg.get_variable().get_dimensions().len() - 1 {
                        self.output.push_str(", ");
                    }
                }
                self.output.push(')');
            }

            if i < procedure.get_parameters().len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push(')');
        self.eol();

        self.indent += 1;
        self.output_statements(procedure.get_statements());
        self.indent -= 1;

        self.indent();
        self.output_keyword("EndProc");
        self.eol();
    }

    fn visit_ast(&mut self, program: &super::Ast) {
        for stmt in &program.nodes {
            stmt.visit(self);
            self.eol();
        }
        self.eol();
    }
}
