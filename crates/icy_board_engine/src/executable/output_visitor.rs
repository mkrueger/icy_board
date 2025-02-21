use crate::{executable::OpCode, formatting::FormattingOptions};

use super::{FunctionDefinition, PPEExpr, PPEScript, PPEVisitor, StatementDefinition};

#[derive(Default)]
pub struct PPEOutputVisitor {
    pub output: String,
    pub options: FormattingOptions,
    pub indent: usize,
}

impl PPEOutputVisitor {
    pub fn new(options: FormattingOptions) -> Self {
        Self {
            output: String::new(),
            options,
            indent: 0,
        }
    }

    fn output_keyword(&mut self, str: &str) {
        self.output.push_str(&str.to_uppercase());
    }

    fn output_function(&mut self, id: usize) {
        self.output.push_str(format!("#{:04X}", id).as_str());
    }

    fn output_op_code(&mut self, end: OpCode) {
        self.output.push_str(&end.to_string());
        self.output.push(' ');
    }

    fn print_arguments(&mut self, args: &[PPEExpr]) {
        for (i, d) in args.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            d.visit(self);
        }
    }
}

impl PPEVisitor<()> for PPEOutputVisitor {
    fn visit_value(&mut self, id: usize) {
        if id == 0 {
            self.output.push_str("INVALID");
        } else {
            self.output.push_str(format!("[{:04X}]", id).as_str());
        };
    }

    fn visit_member(&mut self, expr: &PPEExpr, id: usize) -> () {
        expr.visit(self);
        self.output.push_str(format!(".[{:03X}]", id).as_str());
    }

    fn visit_proc_call(&mut self, id: usize, args: &[PPEExpr]) {
        self.output_function(id);
        self.output.push_str("(");
        self.print_arguments(args);
        self.output.push_str(")");
    }

    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) {
        self.output_function(id);
        self.output.push_str("(");
        self.print_arguments(arguments);
        self.output.push_str(")");
    }
    fn visit_member_function_call(&mut self, expr: &PPEExpr, arguments: &[PPEExpr], _id: usize) {
        expr.visit(self);
        self.output.push_str("(");
        self.print_arguments(arguments);
        self.output.push_str(")");
    }

    fn visit_unary_expression(&mut self, op: crate::ast::UnaryOp, expr: &PPEExpr) {
        self.output.push_str(op.to_string().as_str());
        expr.visit(self);
    }

    fn visit_binary_expression(&mut self, op: crate::ast::BinOp, left: &PPEExpr, right: &PPEExpr) {
        left.visit(self);
        if self.options.space_around_binop {
            self.output.push(' ');
        }
        self.output.push_str(op.to_string().as_str());
        if self.options.space_around_binop {
            self.output.push(' ');
        }
        right.visit(self);
    }

    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) {
        self.output.push_str(format!("[{:04X}, ", id).as_str());
        self.print_arguments(dim);
        print!("]");
    }

    fn visit_predefined_function_call(&mut self, def: &FunctionDefinition, arguments: &[PPEExpr]) {
        self.output_keyword(&def.name);
        self.output.push_str("(");
        if !arguments.is_empty() {
            self.print_arguments(arguments);
        }
        self.output.push_str(")");
    }

    fn visit_end(&mut self) {
        self.output_keyword(&"END");
    }

    fn visit_return(&mut self) {
        self.output_keyword(&"RETURN");
    }

    fn visit_if(&mut self, cond: &PPEExpr, label: &usize) {
        self.output_keyword(&"IF (");
        cond.visit(self);
        self.output.push_str(")");
        self.output_keyword(&" GOTO ");
        self.output_function(*label);
    }

    fn visit_predefined_call(&mut self, def: &StatementDefinition, args: &[PPEExpr]) {
        self.output_keyword(&def.name);
        self.output.push_str("(");
        self.print_arguments(args);
        self.output.push_str(")");
    }

    fn visit_goto(&mut self, label: &usize) {
        self.output_keyword(&"GOTO ");
        self.output_function(*label);
    }

    fn visit_gosub(&mut self, label: &usize) {
        self.output_keyword(&"GOSUB ");
        self.output_function(*label);
    }

    fn visit_end_func(&mut self) {
        self.output_op_code(OpCode::FPCLR);
    }

    fn visit_end_proc(&mut self) {
        self.output_op_code(OpCode::FPCLR);
    }

    fn visit_stop(&mut self) {
        self.output_op_code(OpCode::STOP);
    }

    fn visit_let(&mut self, target: &PPEExpr, value: &PPEExpr) {
        self.output_keyword(&"LET ");
        target.visit(self);
        self.output.push_str(" = ");
        value.visit(self);
    }

    fn visit_script(&mut self, _script: &PPEScript) {
        todo!();
    }
}
