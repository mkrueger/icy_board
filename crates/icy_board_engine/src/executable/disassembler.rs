use std::{io::stdout, ops::Range};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use super::{CommandOrError, Executable, FunctionDefinition, OpCode, PPEExpr, PPEScript, PPEVisitor, StatementDefinition};

pub struct DisassembleVisitor<'a> {
    /// If true, the visitor will print the deserialized data from the statement itself, if false the data will be taken from the PPE File directly.
    pub generate_statement_data: bool,
    pub ppe_file: &'a Executable,
}

impl<'a> DisassembleVisitor<'a> {
    pub fn new(ppe_file: &'a Executable) -> Self {
        Self {
            generate_statement_data: false,
            ppe_file,
        }
    }

    fn output_op_code(end: OpCode) {
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X} ", end as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Yellow),
            Print(format!("{:<10} ", end.to_string())),
            SetAttribute(Attribute::Reset),
        );
    }

    fn print_arguments(&mut self, args: &[PPEExpr]) {
        for (i, d) in args.iter().enumerate() {
            if i > 0 {
                let _ = execute!(stdout(), Print(", "));
            }
            d.visit(self);
        }
    }

    fn dump_script_data(ppe_file: &super::Executable, range: Range<usize>) {
        let offset = range.start;
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{:05X}: ", offset * 2)),
            SetForegroundColor(Color::White),
        );

        for (i, x) in ppe_file.script_buffer[range].iter().enumerate() {
            if i > 0 && (i % 16) == 0 {
                let _ = execute!(
                    stdout(),
                    Print("\n"),
                    SetForegroundColor(Color::Cyan),
                    Print(format!("{:05X}: ", (offset + i) * 2)),
                    SetForegroundColor(Color::White),
                );
            }
            let _ = execute!(stdout(), Print(format!("{:04X} ", *x)));
        }
        let _ = execute!(stdout(), SetForegroundColor(Color::Reset),);
    }

    pub fn print_disassembler(&mut self) {
        print_disassemble_header();
        for cmt in PPEScript::step_through(self.ppe_file) {
            match cmt {
                CommandOrError::Command(stmt) => {
                    self.print_statement(&stmt);
                }
                CommandOrError::Error(e) => {
                    let _ = execute!(
                        stdout(),
                        Print("\n"),
                        SetAttribute(Attribute::Bold),
                        SetForegroundColor(Color::Red),
                        Print("ERROR: ".to_string()),
                        SetAttribute(Attribute::Reset),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{}", e.error_type)),
                        SetAttribute(Attribute::Reset),
                        Print("\n"),
                    );
                    Self::dump_script_data(self.ppe_file, e.span);
                    let _ = execute!(stdout(), Print("\n"));
                }
            }
        }
    }

    pub fn print_script_buffer_dump(ppe_file: &super::Executable) {
        let _ = execute!(
            stdout(),
            Print("Real uncompressed script buffer size: ".to_string()),
            SetAttribute(Attribute::Bold),
            Print(format!("{} bytes\n\n", ppe_file.script_buffer.len() * 2)),
            SetAttribute(Attribute::Reset)
        );
        Self::dump_script_data(ppe_file, 0..ppe_file.script_buffer.len());
    }

    fn print_statement(&mut self, stmt: &super::PPEStatement) {
        let mut vec = Vec::new();
        let data: &[i16] = if self.generate_statement_data {
            stmt.command.serialize(&mut vec);
            &vec
        } else {
            &self.ppe_file.script_buffer[stmt.span.clone()]
        };
        let _ = execute!(stdout(), Print("       ["));
        for (i, x) in data.iter().enumerate() {
            if i > 0 && (i % 16) == 0 {
                let _ = execute!(stdout(), Print("\n"));
            }
            let _ = execute!(stdout(), Print(format!("{:04X} ", *x)));
        }

        let _ = execute!(
            stdout(),
            Print("]\n"),
            SetForegroundColor(Color::Cyan),
            Print(format!("{:05X}: ", stmt.span.start * 2)),
            SetForegroundColor(Color::Reset),
        );
        stmt.command.visit(self);

        let _ = execute!(stdout(), Print("\n"));
    }
}

impl<'a> PPEVisitor<()> for DisassembleVisitor<'a> {
    fn visit_value(&mut self, id: usize) {
        let _ = execute!(
            stdout(),
            Print("["),
            SetForegroundColor(Color::Magenta),
            Print(format!(
                "{} ",
                if id == 0 {
                    "INVALID"
                } else {
                    &self.ppe_file.variable_table.get_var_entry(id).name
                }
            )),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
            Print("]"),
        );
    }

    fn visit_member(&mut self, expr: &PPEExpr, id: usize) -> () {
        expr.visit(self);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Blue),
            Print(format!(".[{:03X}]", id)),
            SetAttribute(Attribute::Reset),
        );
    }

    fn visit_proc_call(&mut self, id: usize, args: &[PPEExpr]) {
        Self::output_op_code(OpCode::PCALL);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Magenta),
            Print(format!(" {}", self.ppe_file.variable_table.get_var_entry(id).name)),
            SetAttribute(Attribute::Reset),
            Print("["),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
            Print("] ("),
        );

        self.print_arguments(args);
        let _ = execute!(stdout(), Print(")"));
    }

    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) {
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Magenta),
            Print(format!(" {}", self.ppe_file.variable_table.get_var_entry(id).name)),
            SetAttribute(Attribute::Reset),
            Print("["),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
            Print("] ("),
        );

        self.print_arguments(arguments);
        let _ = execute!(stdout(), Print(")"));
    }
    fn visit_member_function_call(&mut self, expr: &PPEExpr, arguments: &[PPEExpr], id: usize) {
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Magenta),
            Print(format!("Member:#{:02X}", id)),
            SetAttribute(Attribute::Reset),
            Print(" on ["),
            SetForegroundColor(Color::Green),
            Print(format!("{:?}", expr)),
            SetAttribute(Attribute::Reset),
            Print("] ("),
        );
        self.print_arguments(arguments);
        let _ = execute!(stdout(), Print(")"));
    }

    fn visit_unary_expression(&mut self, op: crate::ast::UnaryOp, expr: &PPEExpr) {
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(format!("{op}")),
            SetAttribute(Attribute::Reset),
        );
        let _ = execute!(stdout(), Print("("));

        expr.visit(self);
        let _ = execute!(stdout(), Print(")"));
    }

    fn visit_binary_expression(&mut self, op: crate::ast::BinOp, left: &PPEExpr, right: &PPEExpr) {
        left.visit(self);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(format!(" {op} ")),
            SetAttribute(Attribute::Reset),
        );
        right.visit(self);
    }

    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) {
        let _ = execute!(
            stdout(),
            Print("["),
            SetForegroundColor(Color::Yellow),
            Print("#"),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
        );
        let _ = execute!(stdout(), Print(", "));
        self.print_arguments(dim);
        let _ = execute!(stdout(), Print("]"));
    }

    fn visit_predefined_function_call(&mut self, def: &FunctionDefinition, arguments: &[PPEExpr]) {
        let _ = execute!(stdout(), Print("("));

        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X}", def.opcode as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkYellow),
            Print(format!("'{}'", def.name)),
            SetAttribute(Attribute::Reset),
        );
        if !arguments.is_empty() {
            let _ = execute!(stdout(), Print(" "));
            self.print_arguments(arguments);
        }
        let _ = execute!(stdout(), Print(")"));
    }

    fn visit_end(&mut self) {
        Self::output_op_code(OpCode::END);
    }

    fn visit_return(&mut self) {
        Self::output_op_code(OpCode::RETURN);
    }

    fn visit_if(&mut self, cond: &PPEExpr, label: &usize) {
        Self::output_op_code(OpCode::IFNOT);
        let _ = execute!(stdout(), Print(" ("));
        cond.visit(self);
        let _ = execute!(stdout(), Print(")"));

        let _ = execute!(stdout(), SetForegroundColor(Color::Yellow), Print(" GOTO "), SetAttribute(Attribute::Reset),);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        );
    }

    fn visit_predefined_call(&mut self, def: &StatementDefinition, args: &[PPEExpr]) {
        let name = format!("'{}'", def.name);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X} ", def.opcode as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkYellow),
            Print(format!("{name:<12}")),
            SetAttribute(Attribute::Reset),
        );

        self.print_arguments(args);
    }

    fn visit_goto(&mut self, label: &usize) {
        Self::output_op_code(OpCode::GOTO);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!(" {{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        );
    }

    fn visit_gosub(&mut self, label: &usize) {
        Self::output_op_code(OpCode::GOSUB);
        let _ = execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!(" {{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        );
    }

    fn visit_end_func(&mut self) {
        Self::output_op_code(OpCode::FEND);
    }

    fn visit_end_proc(&mut self) {
        Self::output_op_code(OpCode::FPCLR);
    }

    fn visit_stop(&mut self) {
        Self::output_op_code(OpCode::STOP);
    }

    fn visit_let(&mut self, target: &PPEExpr, value: &PPEExpr) {
        Self::output_op_code(OpCode::LET);
        let _ = execute!(stdout(), Print(" "));
        target.visit(self);
        let _ = execute!(stdout(), SetForegroundColor(Color::Yellow), Print(" <- "), SetAttribute(Attribute::Reset),);
        value.visit(self);
    }

    fn visit_script(&mut self, script: &PPEScript) {
        print_disassemble_header();
        for stmt in &script.statements {
            self.print_statement(stmt);
        }
        let _ = execute!(stdout(), Print("\n"));
    }
}

fn print_disassemble_header() {
    let _ = execute!(stdout(), Print("\n"));
    println!("Offset  # OpCode      Parameters");
    println!("---------------------------------------------------------------------------------------");
}
