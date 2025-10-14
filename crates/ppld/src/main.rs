use argh::FromArgs;
use crossterm::ExecutableCommand;
use crossterm::execute;
use crossterm::style::Attribute;
use crossterm::style::Color;
use crossterm::style::Print;
use crossterm::style::ResetColor;
use crossterm::style::SetAttribute;
use crossterm::style::SetForegroundColor;
use icy_board_engine::ast::OutputFunc;
use icy_board_engine::ast::output_visitor;
use icy_board_engine::decompiler::decompile;
use icy_board_engine::executable::Executable;
use icy_board_engine::executable::PPEScript;
use semver::Version;
use std::ffi::OsStr;
use std::fs::*;
use std::io::*;
use std::path::Path;

use crate::compat_check::check_compatibility;

#[cfg(test)]
pub mod tests;

pub mod compat_check;

#[derive(FromArgs)]
/// PCBoard Programming Language Decompiler
struct Cli {
    /// raw ppe without reconstruction control structures
    #[argh(switch, short = 'r')]
    raw: bool,

    /// output the disassembly instead of ppl
    #[argh(switch, short = 'd')]
    disassemble: bool,

    /// output to console instead of writing to file
    #[argh(switch, short = 'o')]
    output: bool,

    /// checks a .ppe file for compatibility with the current runtime
    #[argh(switch)]
    check: bool,

    #[argh(option)]
    /// keyword casing style, valid values are u=upper (default), l=lower, c=camel
    style: Option<char>,

    /// file[.ppe] to decompile
    #[argh(positional)]
    file: String,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let arguments: Cli = argh::from_env();
    println!("PPLD v{} - PCBoard Programming Language v", *VERSION);
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
            if arguments.check {
                execute!(
                    stdout(),
                    SetAttribute(Attribute::Bold),
                    Print(format!("\nChecking compatibility for: {}\n", file_name)),
                    SetAttribute(Attribute::Reset),
                    Print(format!("PPE Version: {}\n\n", executable.runtime))
                )
                .unwrap();

                match check_compatibility(&executable) {
                    Ok(()) => {
                        std::process::exit(0);
                    }
                    Err(err) => {
                        execute!(
                            stdout(),
                            SetAttribute(Attribute::Bold),
                            SetForegroundColor(Color::Red),
                            Print("ERROR during compatibility check: ".to_string()),
                            SetAttribute(Attribute::Reset),
                            Print(format!("{}\n", err))
                        )
                        .unwrap();
                        std::process::exit(1);
                    }
                }
            }

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
                    let mut output_visitor: output_visitor::OutputVisitor = output_visitor::OutputVisitor::default();
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
                                .execute(Print(format!("Can't create {:?} on disk, reason: {}", &out_file_name, err)))
                                .unwrap()
                                .execute(ResetColor)
                                .unwrap()
                                .flush()
                                .unwrap();
                            std::process::exit(1);
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
                    std::process::exit(0);
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
                    std::process::exit(1);
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
            std::process::exit(1);
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
