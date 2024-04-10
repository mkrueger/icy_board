use crate::{
    ast::AstVisitor,
    executable::{PPEExpr, VariableType},
};

use super::PPECompiler;

pub struct ExpressionCompiler<'a> {
    pub compiler: &'a mut PPECompiler,
}

impl<'a> AstVisitor<PPEExpr> for ExpressionCompiler<'a> {
    fn visit_identifier_expression(
        &mut self,
        identifier: &crate::ast::IdentifierExpression,
    ) -> PPEExpr {
        if let Some(decl) = self
            .compiler
            .lookup_table
            .lookup_variable(identifier.get_identifier())
        {
            return PPEExpr::Value(decl.header.id);
        }
        log::error!("Variable not found: {}", identifier.get_identifier());
        PPEExpr::Value(0)
    }

    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> PPEExpr {
        let table_id = self
            .compiler
            .lookup_table
            .lookup_constant(constant.get_constant_value());
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

    fn visit_predefined_function_call_expression(
        &mut self,
        call: &crate::ast::PredefinedFunctionCallExpression,
    ) -> PPEExpr {
        let def = call.get_func();
        PPEExpr::PredefinedFunctionCall(
            def.opcode.get_definition(), // to de-alias aliases
            call.get_arguments().iter().map(|e| e.visit(self)).collect(),
        )
    }

    fn visit_function_call_expression(
        &mut self,
        func_call: &crate::ast::FunctionCallExpression,
    ) -> PPEExpr {
        let arguments = func_call
            .get_arguments()
            .iter()
            .map(|e| e.visit(self))
            .collect();

        if self
            .compiler
            .lookup_table
            .has_variable(func_call.get_identifier())
        {
            let Some(table_idx) = self
                .compiler
                .lookup_variable_index(func_call.get_identifier())
            else {
                log::error!(
                    "function not found: {}",
                    func_call.get_identifier().to_string()
                );
                return PPEExpr::Value(0);
            };

            let var = self
                .compiler
                .lookup_table
                .variable_table
                .get_var_entry(table_idx);

            if var.value.get_type() == VariableType::Function {
                return PPEExpr::FunctionCall(var.header.id, arguments);
            }
            if var.header.dim as usize != arguments.len() {
                log::error!(
                    "Invalid dimensions for function call: {}",
                    func_call.get_identifier().to_string()
                );

                return PPEExpr::Value(0);
            }
            return PPEExpr::Dim(var.header.id, arguments);
        }

        let Some(table_idx) = self
            .compiler
            .lookup_variable_index(func_call.get_identifier())
        else {
            log::error!(
                "function not found: {}",
                func_call.get_identifier().to_string()
            );
            return PPEExpr::Value(0);
        };
        let var = &self
            .compiler
            .lookup_table
            .variable_table
            .get_var_entry(table_idx);
        if var.header.dim as usize != arguments.len() {
            log::error!(
                "Invalid dimensions for function call: {}",
                func_call.get_identifier().to_string()
            );
            return PPEExpr::Value(0);
        }
        PPEExpr::Dim(table_idx, arguments)
    }

    fn visit_parens_expression(&mut self, parens: &crate::ast::ParensExpression) -> PPEExpr {
        parens.get_expression().visit(self)
    }
}
