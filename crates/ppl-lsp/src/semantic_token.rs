use icy_ppe::{
    ast::{
        walk_binary_expression, walk_block_stmt, walk_for_stmt, walk_function_call_expression,
        walk_function_implementation, walk_if_stmt, walk_if_then_stmt, walk_let_stmt,
        walk_predefined_call_statement, walk_procedure_call_statement,
        walk_procedure_implementation, walk_select_stmt, walk_while_do_stmt, walk_while_stmt, Ast,
        AstVisitor, Constant, ParameterSpecifier,
    },
    parser::lexer::{Spanned, Token},
};
use tower_lsp::lsp_types::SemanticTokenType;

use crate::ImCompleteSemanticToken;

pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
    SemanticTokenType::NUMBER,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::TYPE,
    SemanticTokenType::ENUM_MEMBER,
];

pub fn semantic_token_from_ast(ast: &Ast) -> Vec<ImCompleteSemanticToken> {
    let mut visitor = SemanticTokenVisitor {
        semantic_tokens: vec![],
    };

    ast.visit(&mut visitor);
    visitor.semantic_tokens
}

struct SemanticTokenVisitor {
    pub semantic_tokens: Vec<ImCompleteSemanticToken>,
}
impl SemanticTokenVisitor {
    fn highlight_token(&mut self, token: &Spanned<Token>, keyword: SemanticTokenType) {
        self.semantic_tokens.push(ImCompleteSemanticToken {
            start: token.span.start,
            length: token.span.len(),
            token_type: LEGEND_TYPE
                .iter()
                .position(|item| item == &keyword)
                .unwrap(),
        });
    }

    fn higlight_parameters(&mut self, parameters: &[ParameterSpecifier]) {
        for p in parameters {
            if let Some(var) = p.get_var_token() {
                self.highlight_token(var, SemanticTokenType::KEYWORD);
            }
            self.highlight_token(p.get_type_token(), SemanticTokenType::TYPE);
        }
    }
}

impl AstVisitor<()> for SemanticTokenVisitor {
    fn visit_identifier_expression(&mut self, _identifier: &icy_ppe::ast::IdentifierExpression) {}

    fn visit_constant_expression(&mut self, const_expr: &icy_ppe::ast::ConstantExpression) {
        match const_expr.get_constant_value() {
            Constant::String(_) => {
                self.highlight_token(const_expr.get_constant_token(), SemanticTokenType::STRING);
            }
            Constant::Integer(_) | Constant::Unsigned(_) | Constant::Double(_) => {
                self.highlight_token(const_expr.get_constant_token(), SemanticTokenType::NUMBER);
            }
            Constant::Builtin(_) => {
                self.highlight_token(
                    const_expr.get_constant_token(),
                    SemanticTokenType::ENUM_MEMBER,
                );
            }
            _ => {}
        }
    }

    fn visit_binary_expression(&mut self, binary: &icy_ppe::ast::BinaryExpression) {
        walk_binary_expression(self, binary);
    }

    fn visit_unary_expression(&mut self, unary: &icy_ppe::ast::UnaryExpression) {
        unary.get_expression().visit(self)
    }

    fn visit_function_call_expression(&mut self, call: &icy_ppe::ast::FunctionCallExpression) {
        walk_function_call_expression(self, call);
    }

    fn visit_parens_expression(&mut self, parens: &icy_ppe::ast::ParensExpression) {
        parens.get_expression().visit(self)
    }

    fn visit_comment(&mut self, comment: &icy_ppe::ast::CommentAstNode) {
        self.highlight_token(comment.get_comment_token(), SemanticTokenType::COMMENT);
    }

    fn visit_block_statement(&mut self, block: &icy_ppe::ast::BlockStatement) {
        self.highlight_token(block.get_begin_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(block.get_end_token(), SemanticTokenType::KEYWORD);

        walk_block_stmt(self, block);
    }

    fn visit_if_statement(&mut self, if_stmt: &icy_ppe::ast::IfStatement) {
        self.highlight_token(if_stmt.get_if_token(), SemanticTokenType::KEYWORD);

        walk_if_stmt(self, if_stmt);
    }

    fn visit_if_then_statement(&mut self, if_then: &icy_ppe::ast::IfThenStatement) {
        self.highlight_token(if_then.get_if_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(if_then.get_then_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(if_then.get_endif_token(), SemanticTokenType::KEYWORD);
        for else_if_block in if_then.get_else_if_blocks() {
            self.highlight_token(else_if_block.get_elseif_token(), SemanticTokenType::KEYWORD);
        }
        if let Some(else_block) = if_then.get_else_block() {
            self.highlight_token(else_block.get_else_token(), SemanticTokenType::KEYWORD);
        }
        walk_if_then_stmt(self, if_then);
    }

    fn visit_gosub_statement(&mut self, gosub: &icy_ppe::ast::GosubStatement) {
        self.highlight_token(gosub.get_gosub_token(), SemanticTokenType::KEYWORD);
    }

    fn visit_return_statement(&mut self, return_stmt: &icy_ppe::ast::ReturnStatement) {
        self.highlight_token(return_stmt.get_return_token(), SemanticTokenType::KEYWORD);
    }

    fn visit_let_statement(&mut self, let_stmt: &icy_ppe::ast::LetStatement) {
        if let Some(let_token) = let_stmt.get_let_token() {
            self.highlight_token(let_token, SemanticTokenType::KEYWORD);
        }
        walk_let_stmt(self, let_stmt);
    }

    fn visit_goto_statement(&mut self, goto: &icy_ppe::ast::GotoStatement) {
        self.highlight_token(goto.get_goto_token(), SemanticTokenType::KEYWORD);
    }

    fn visit_label_statement(&mut self, _label: &icy_ppe::ast::LabelStatement) {}

    fn visit_procedure_call_statement(&mut self, call: &icy_ppe::ast::ProcedureCallStatement) {
        walk_procedure_call_statement(self, call);
    }

    fn visit_predefined_call_statement(&mut self, call: &icy_ppe::ast::PredefinedCallStatement) {
        self.highlight_token(call.get_identifier_token(), SemanticTokenType::FUNCTION);

        walk_predefined_call_statement(self, call);
    }

    fn visit_variable_declaration_statement(
        &mut self,
        var_decl: &icy_ppe::ast::VariableDeclarationStatement,
    ) {
        self.highlight_token(var_decl.get_type_token(), SemanticTokenType::TYPE);
    }

    fn visit_procedure_declaration(
        &mut self,
        proc_decl: &icy_ppe::ast::ProcedureDeclarationAstNode,
    ) {
        self.highlight_token(proc_decl.get_declare_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(proc_decl.get_procedure_token(), SemanticTokenType::KEYWORD);
        self.higlight_parameters(proc_decl.get_parameters());
    }

    fn visit_function_declaration(&mut self, func_decl: &icy_ppe::ast::FunctionDeclarationAstNode) {
        self.highlight_token(func_decl.get_declare_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(func_decl.get_function_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(func_decl.get_return_type_token(), SemanticTokenType::TYPE);
        self.higlight_parameters(func_decl.get_parameters());
    }

    fn visit_function_implementation(&mut self, function: &icy_ppe::ast::FunctionImplementation) {
        self.highlight_token(function.get_endfunc_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(function.get_function_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(function.get_return_type_token(), SemanticTokenType::TYPE);
        self.higlight_parameters(function.get_parameters());
        walk_function_implementation(self, function);
    }

    fn visit_procedure_implementation(
        &mut self,
        procedure: &icy_ppe::ast::ProcedureImplementation,
    ) {
        self.highlight_token(procedure.get_endproc_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(procedure.get_procedure_token(), SemanticTokenType::KEYWORD);
        self.higlight_parameters(procedure.get_parameters());
        walk_procedure_implementation(self, procedure);
    }

    fn visit_select_statement(&mut self, select_stmt: &icy_ppe::ast::SelectStatement) {
        self.highlight_token(select_stmt.get_select_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(select_stmt.get_case_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(
            select_stmt.get_endselect_token(),
            SemanticTokenType::KEYWORD,
        );

        select_stmt.get_case_blocks().iter().for_each(|case| {
            self.highlight_token(case.get_case_token(), SemanticTokenType::KEYWORD);
        });
        if let Some(default_token) = select_stmt.get_default_token() {
            self.highlight_token(default_token, SemanticTokenType::KEYWORD);
        }
        walk_select_stmt(self, select_stmt);
    }

    fn visit_while_statement(&mut self, while_stmt: &icy_ppe::ast::WhileStatement) {
        self.highlight_token(while_stmt.get_while_token(), SemanticTokenType::KEYWORD);

        walk_while_stmt(self, while_stmt);
    }

    fn visit_while_do_statement(&mut self, while_do: &icy_ppe::ast::WhileDoStatement) {
        self.highlight_token(while_do.get_while_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(while_do.get_do_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(while_do.get_endwhile_token(), SemanticTokenType::KEYWORD);
        walk_while_do_stmt(self, while_do);
    }

    fn visit_for_statement(&mut self, for_stmt: &icy_ppe::ast::ForStatement) {
        self.highlight_token(for_stmt.get_for_token(), SemanticTokenType::KEYWORD);
        self.highlight_token(for_stmt.get_to_token(), SemanticTokenType::KEYWORD);
        if let Some(step) = for_stmt.get_step_token() {
            self.highlight_token(step, SemanticTokenType::KEYWORD);
        }
        self.highlight_token(for_stmt.get_next_token(), SemanticTokenType::KEYWORD);

        walk_for_stmt(self, for_stmt);
    }

    fn visit_break_statement(&mut self, break_stmt: &icy_ppe::ast::BreakStatement) {
        self.highlight_token(break_stmt.get_break_token(), SemanticTokenType::KEYWORD);
    }

    fn visit_continue_statement(&mut self, continue_stmt: &icy_ppe::ast::ContinueStatement) {
        self.highlight_token(
            continue_stmt.get_continue_token(),
            SemanticTokenType::KEYWORD,
        );
    }
}
