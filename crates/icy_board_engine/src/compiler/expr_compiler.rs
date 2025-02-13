use crate::{
    ast::AstVisitor,
    executable::{PPEExpr, VariableType},
    semantic::SemanticInfo,
};

use super::PPECompiler;

pub struct ExpressionCompiler<'a, 'b> {
    pub compiler: &'a mut PPECompiler<'b>,
}

impl<'a, 'b> AstVisitor<PPEExpr> for ExpressionCompiler<'a, 'b> {
    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> PPEExpr {
        if let Some(decl) = self.compiler.lookup_table.lookup_variable(identifier.get_identifier()) {
            return PPEExpr::Value(decl.header.id);
        }
        log::error!("Variable not found: {}", identifier.get_identifier());
        PPEExpr::Value(0)
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> PPEExpr {
        let base = member_reference_expression.get_expression().visit(self);
        let type_id = self
            .compiler
            .semantic_visitor
            .user_type_lookup
            .get(&member_reference_expression.get_identifier_token().span.start)
            .unwrap();
        let typ = self.compiler.semantic_visitor.type_registry.get_type_from_id(*type_id).unwrap();

        let member_id = typ.member_id_lookup.get(&member_reference_expression.get_identifier()).unwrap();

        PPEExpr::Member(Box::new(base), *member_id)
    }

    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> PPEExpr {
        let table_id = self.compiler.lookup_table.lookup_constant(constant.get_constant_value());
        PPEExpr::Value(table_id)
    }

    fn visit_binary_expression(&mut self, bin_expr: &crate::ast::BinaryExpression) -> PPEExpr {
        let left = bin_expr.get_left_expression().visit(self);
        let right = bin_expr.get_right_expression().visit(self);
        PPEExpr::BinaryExpression(bin_expr.get_op(), Box::new(left), Box::new(right))
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> PPEExpr {
        let expression = unary.get_expression().visit(self);
        PPEExpr::UnaryExpression(unary.get_op(), Box::new(expression))
    }

    fn visit_function_call_expression(&mut self, call: &crate::ast::FunctionCallExpression) -> PPEExpr {
        let arguments = call.get_arguments().iter().map(|e| e.visit(self)).collect();
        let Some(function_type) = self.compiler.semantic_visitor.function_type_lookup.get(&call.get_expression().get_span().start) else {
            log::error!("function not found at: {} ({})", call.get_expression().get_span().start, call.get_expression());
            return PPEExpr::Value(0);
        };
        match function_type {
            SemanticInfo::PredefinedFunc(op_code) => {
                return PPEExpr::PredefinedFunctionCall(
                    op_code.get_definition(), // to de-alias aliases
                    call.get_arguments().iter().map(|e| e.visit(self)).collect(),
                );
            }
            SemanticInfo::MemberFunctionCall(idx) => {
                let idx = *idx;
                let expr = call.get_expression().visit(self);
                return PPEExpr::MemberFunctionCall(Box::new(expr), arguments, idx);
            }
            SemanticInfo::FunctionReference(idx) => {
                let reference_index = self.compiler.semantic_visitor.function_containers[*idx].id;
                let table_index = self.compiler.semantic_visitor.references[reference_index].1.variable_table_index;
                return PPEExpr::FunctionCall(table_index, arguments);
            }
            SemanticInfo::VariableReference(reference_index) => {
                let table_index = self.compiler.semantic_visitor.references[*reference_index].1.variable_table_index;
                return PPEExpr::Dim(table_index, arguments);
            }
            _ => {
                log::error!("Invalid function call: {:?}", function_type);
                return PPEExpr::Value(0);
            }
        }
    }

    fn visit_indexer_expression(&mut self, indexer: &crate::ast::IndexerExpression) -> PPEExpr {
        let arguments = indexer.get_arguments().iter().map(|e| e.visit(self)).collect();

        if self.compiler.lookup_table.has_variable(indexer.get_identifier()) {
            let Some(table_idx) = self.compiler.lookup_variable_index(indexer.get_identifier()) else {
                log::error!("function not found: {}", indexer.get_identifier().to_string());
                return PPEExpr::Value(0);
            };

            let var = self.compiler.lookup_table.variable_table.get_var_entry(table_idx);
            if var.value.get_type() == VariableType::Function {
                return PPEExpr::FunctionCall(var.header.id, arguments);
            }
            if var.header.dim as usize != arguments.len() {
                log::error!("Invalid dimensions for function call: {}", indexer.get_identifier().to_string());
                return PPEExpr::Value(0);
            }
            return PPEExpr::Dim(var.header.id, arguments);
        }
        log::error!("Invalid indexer call: {}", indexer.get_identifier().to_string());
        return PPEExpr::Value(0);
    }

    fn visit_parens_expression(&mut self, parens: &crate::ast::ParensExpression) -> PPEExpr {
        parens.get_expression().visit(self)
    }
}
