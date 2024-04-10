use clap::Parser;
use crossterm::execute;
use crossterm::style::Attribute;
use crossterm::style::Color;
use crossterm::style::Print;
use crossterm::style::ResetColor;
use crossterm::style::SetAttribute;
use crossterm::style::SetForegroundColor;
use crossterm::ExecutableCommand;
use icy_ppe::ast::output_visitor;
use icy_ppe::ast::OutputFunc;
use icy_ppe::decompiler::decompile;
use icy_ppe::executable::Executable;
use icy_ppe::executable::PPEScript;
use semver::Version;
use std::ffi::OsStr;
use std::fs::*;
use std::io::*;
use std::path::Path;

#[cfg(test)]
pub mod tests;

#[derive(clap::Parser)]
#[command(version="", about="PCBoard Programming Language Decompiler", long_about = None)]
struct Cli {
    /// raw ppe without reconstruction control structures
    #[arg(long)]
    raw: bool,

    /// output the disassembly instead of ppl
    #[arg(long, short)]
    disassemble: bool,

    /// output to console instead of writing to file
    #[arg(long)]
    output: bool,

    #[arg(long)]
    /// keyword casing style, valid values are u=upper (default), l=lower, c=camel
    style: Option<char>,

    /// file[.ppe] to decompile
    file: String,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let arguments = Cli::parse();
    let mut output_func = OutputFunc::Upper;
    match arguments.style {
        Some('u') => output_func = OutputFunc::Upper,
        Some('l') => output_func = OutputFunc::Lower,
        Some('c') => output_func = OutputFunc::CamelCase,
        Some(x) => panic!("unsupported keyword style {}", x),
        None => {}
    }

    let mut file_name = arguments.file;

    let extension = Path::new(&file_name).extension().and_then(OsStr::to_str);
    if extension.is_none() {
        file_name.push_str(".ppe");
    }

    let out_file_name = Path::new(&file_name).with_extension("ppd");
    match Executable::read_file(&file_name, !arguments.output) {
        Ok(mut executable) => {
            if arguments.disassemble {
                executable.print_script_buffer_dump();
                println!();

                if let Ok(script) = PPEScript::from_ppe_file(&executable) {
                    executable.variable_table.analyze_usage(&script);
                    executable.variable_table.generate_names();
                }

                executable.print_variable_table();
                println!();
                executable.print_disassembler();
                println!();
                return;
            }

            match decompile(executable, arguments.raw) {
                Ok((decompilation, issues)) => {
                    let mut output_visitor = output_visitor::OutputVisitor::default();
                    output_visitor.output_func = output_func;
                    decompilation.visit(&mut output_visitor);
                    if arguments.output {
                        println!("{}", output_visitor.output);
                    } else {
                        let mut output = File::create(&out_file_name).unwrap();
                        if let Err(err) = write!(output, "{}", output_visitor.output) {
                            stdout()
                                .execute(SetForegroundColor(Color::Red))
                                .unwrap()
                                .execute(Print(format!(
                                    "Can't create {:?} on disk, reason: {}",
                                    &out_file_name, err
                                )))
                                .unwrap()
                                .execute(ResetColor)
                                .unwrap()
                                .flush()
                                .unwrap();
                            return;
                        }
                        execute!(
                            stdout(),
                            Print("\nSource decompilation complete: ".to_string()),
                            SetAttribute(Attribute::Bold),
                            Print(format!("{file_name}\n")),
                            SetAttribute(Attribute::Reset),
                            Print("decompiled to: ".to_string()),
                            SetAttribute(Attribute::Bold),
                            Print(format!("{out_file_name:?}\n")),
                            SetAttribute(Attribute::Reset),
                        )
                        .unwrap();
                    }

                    if !issues.is_empty() {
                        println!();
                    }
                    for issue in &issues {
                        execute!(
                            stdout(),
                            SetAttribute(Attribute::Bold),
                            SetForegroundColor(Color::Yellow),
                            Print("WARNING: ".to_string()),
                            SetAttribute(Attribute::Reset),
                            SetAttribute(Attribute::Bold),
                            Print(format!("[{:04X}]:", issue.byte_offset)),
                            SetAttribute(Attribute::Reset),
                            Print(format!("{}", issue.bug)),
                            SetAttribute(Attribute::Reset),
                        )
                        .unwrap();
                        println!();
                    }
                    if !issues.is_empty() {
                        println!("{0} issues found during decompilation", issues.len());
                    }
                }
                Err(err) => {
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Bold),
                        SetForegroundColor(Color::Red),
                        Print("ERROR: ".to_string()),
                        SetAttribute(Attribute::Reset),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{}", err)),
                        SetAttribute(Attribute::Reset),
                    )
                    .unwrap();
                    println!();
                }
            }
        }
        Err(err) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Red),
                Print("ERROR: ".to_string()),
                SetAttribute(Attribute::Reset),
                SetAttribute(Attribute::Bold),
                Print(format!("{}", err)),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();
            println!();
            println!();
        }
    }
}

/*
let mut res = String::new();

res.push_str(&self.block.to_string(self));

if !self.function_implementations.is_empty() || !self.procedure_implementations.is_empty() {
    res.push_str("; Function implementations\n");
}
for v in &self.function_implementations {
    res.push_str(v.print_content().as_str());
    res.push('\n');
}

for v in &self.procedure_implementations {
    res.push_str(v.print_content().as_str());
    res.push('\n');
}*/
