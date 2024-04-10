use std::{io::stdout, ops::Range};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use super::{
    CommandOrError, Executable, FunctionDefinition, OpCode, PPEExpr, PPEScript, PPEVisitor,
    StatementDefinition,
};

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
        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X} ", end as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Yellow),
            Print(format!("{:<10} ", end.to_string())),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
    }

    fn print_arguments(&mut self, args: &[PPEExpr]) {
        for (i, d) in args.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            d.visit(self);
        }
    }

    fn dump_script_data(ppe_file: &super::Executable, range: Range<usize>) {
        let offset = range.start;
        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{:05X}: ", offset * 2)),
            SetForegroundColor(Color::White),
        )
        .unwrap();

        for (i, x) in ppe_file.script_buffer[range].iter().enumerate() {
            if i > 0 && (i % 16) == 0 {
                println!();
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Cyan),
                    Print(format!("{:05X}: ", (offset + i) * 2)),
                    SetForegroundColor(Color::White),
                )
                .unwrap();
            }
            print!("{:04X} ", *x);
        }
        execute!(stdout(), SetForegroundColor(Color::Reset),).unwrap();
    }

    pub fn print_disassembler(&mut self) {
        print_disassemble_header();
        for cmt in PPEScript::step_through(self.ppe_file) {
            match cmt {
                CommandOrError::Command(stmt) => {
                    self.print_statement(&stmt);
                }
                CommandOrError::Error(e) => {
                    println!();
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Bold),
                        SetForegroundColor(Color::Red),
                        Print("ERROR: ".to_string()),
                        SetAttribute(Attribute::Reset),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{}", e.error_type)),
                        SetAttribute(Attribute::Reset),
                    )
                    .unwrap();
                    println!();
                    Self::dump_script_data(self.ppe_file, e.span);
                    println!();
                }
            }
        }
    }

    pub fn print_script_buffer_dump(ppe_file: &super::Executable) {
        execute!(
            stdout(),
            Print("Real uncompressed script buffer size: ".to_string()),
            SetAttribute(Attribute::Bold),
            Print(format!("{} bytes\n\n", ppe_file.script_buffer.len() * 2)),
            SetAttribute(Attribute::Reset)
        )
        .unwrap();
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
        print!("       [");
        for (i, x) in data.iter().enumerate() {
            if i > 0 && (i % 16) == 0 {
                println!();
            }
            print!("{:04X} ", *x);
        }
        println!("]");

        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{:05X}: ", stmt.span.start * 2)),
            SetForegroundColor(Color::Reset),
        )
        .unwrap();
        stmt.command.visit(self);
        println!();
    }
}

impl<'a> PPEVisitor<()> for DisassembleVisitor<'a> {
    fn visit_value(&mut self, id: usize) {
        execute!(
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
        )
        .unwrap();
    }

    fn visit_proc_call(&mut self, id: usize, args: &[PPEExpr]) {
        Self::output_op_code(OpCode::PCALL);
        execute!(
            stdout(),
            SetForegroundColor(Color::Magenta),
            Print(format!(
                " {}",
                self.ppe_file.variable_table.get_var_entry(id).name
            )),
            SetAttribute(Attribute::Reset),
            Print("["),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
            Print("] ("),
        )
        .unwrap();

        self.print_arguments(args);
        print!(")");
    }

    fn visit_function_call(&mut self, id: usize, arguments: &[PPEExpr]) {
        execute!(
            stdout(),
            SetForegroundColor(Color::Magenta),
            Print(format!(
                " {}",
                self.ppe_file.variable_table.get_var_entry(id).name
            )),
            SetAttribute(Attribute::Reset),
            Print("["),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
            Print("] ("),
        )
        .unwrap();

        self.print_arguments(arguments);
        print!(")");
    }

    fn visit_unary_expression(&mut self, op: crate::ast::UnaryOp, expr: &PPEExpr) {
        execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(format!("{op}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        print!("(");
        expr.visit(self);
        print!(")");
    }

    fn visit_binary_expression(&mut self, op: crate::ast::BinOp, left: &PPEExpr, right: &PPEExpr) {
        left.visit(self);
        execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(format!(" {op} ")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        right.visit(self);
    }

    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) {
        execute!(
            stdout(),
            Print("["),
            SetForegroundColor(Color::Yellow),
            Print("#"),
            SetForegroundColor(Color::Green),
            Print(format!("{id:04X}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        print!(", ");
        self.print_arguments(dim);
        print!("]");
    }

    fn visit_predefined_function_call(&mut self, def: &FunctionDefinition, arguments: &[PPEExpr]) {
        print!("(");
        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X}", def.opcode as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkYellow),
            Print(format!("'{}'", def.name)),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        if !arguments.is_empty() {
            print!(" ");
            self.print_arguments(arguments);
        }
        print!(")");
    }

    fn visit_end(&mut self) {
        Self::output_op_code(OpCode::END);
    }

    fn visit_return(&mut self) {
        Self::output_op_code(OpCode::RETURN);
    }

    fn visit_if(&mut self, cond: &PPEExpr, label: &usize) {
        Self::output_op_code(OpCode::IFNOT);
        print!(" (");
        cond.visit(self);
        print!(")");

        execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(" GOTO "),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();

        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
    }

    fn visit_predefined_call(&mut self, def: &StatementDefinition, args: &[PPEExpr]) {
        let name = format!("'{}'", def.name);
        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(format!("{:02X} ", def.opcode as i16)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkYellow),
            Print(format!("{name:<12}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();

        self.print_arguments(args);
    }

    fn visit_goto(&mut self, label: &usize) {
        Self::output_op_code(OpCode::GOTO);
        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!(" {{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
    }

    fn visit_gosub(&mut self, label: &usize) {
        Self::output_op_code(OpCode::GOSUB);
        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!(" {{{label:04X}}}")),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
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
        print!(" ");
        target.visit(self);
        execute!(
            stdout(),
            SetForegroundColor(Color::Yellow),
            Print(" <- "),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        value.visit(self);
    }

    fn visit_script(&mut self, script: &PPEScript) {
        print_disassemble_header();
        for stmt in &script.statements {
            self.print_statement(stmt);
        }
        println!();
    }
}

fn print_disassemble_header() {
    println!();
    println!("Offset  # OpCode      Parameters");
    println!(
        "---------------------------------------------------------------------------------------"
    );
}
