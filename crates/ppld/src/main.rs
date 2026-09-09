use codepages::tables::write_cp437;
use crossterm::execute;
use crossterm::style::Attribute;
use crossterm::style::Color;
use crossterm::style::Print;
use crossterm::style::SetAttribute;
use crossterm::style::SetForegroundColor;
use icy_board_ppl::ast::OutputFunc;
use icy_board_ppl::ast::output_visitor;
use icy_board_ppl::decompiler::decompile;
use icy_board_ppl::executable::Executable;
use icy_board_ppl::executable::LAST_PPL_LANGUAGE_VERSION;
use icy_board_ppl::executable::PPEScript;
use icy_board_ppl::executable::SUPPORTED_PPL_LANGUAGE_VERSIONS;
use icy_board_ppl::executable::language_version_from_env;
use semver::Version;
use std::ffi::OsStr;
use std::fs::*;
use std::io::*;
use std::path::Path;

use crate::compat_check::check_compatibility;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
mod cli_tests;

pub mod compat_check;

fn cli_text(key: &str) -> String {
    icy_board_cli::text("ppld", key)
}

#[derive(clap::Parser)]
#[command(name = "ppld", about = cli_text("about"), disable_version_flag = true)]
struct Cli {
    #[arg(long, short = 'r', help = cli_text("raw"))]
    raw: bool,

    #[arg(long, short = 'd', help = cli_text("disassemble"))]
    disassemble: bool,

    #[arg(long, short = 'o', help = cli_text("output"))]
    output: bool,

    #[arg(long, help = cli_text("check"))]
    check: bool,

    #[arg(long, requires = "check", help = cli_text("strict"))]
    strict: bool,

    #[arg(long, help = cli_text("cp437"))]
    cp437: bool,

    #[arg(long, value_name = "style", help = cli_text("style"))]
    style: Option<char>,

    #[arg(long, value_name = "lang-version", help = cli_text("lang-version"))]
    lang_version: Option<u16>,

    #[arg(long, help = cli_text("version"))]
    version: bool,

    #[arg(value_name = "file", help = cli_text("file"))]
    file: Option<String>,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// Set by build.rs, empty when the binary was not built from a checkout.
const GIT_HASH: &str = env!("GIT_HASH");

fn main() {
    let arguments: Cli = icy_board_cli::parse();
    if arguments.version {
        println!("{}", icy_board_cli::version_line("ppld", &*VERSION, GIT_HASH));
        return;
    }
    // Keep the historical banner/report stream unless stdout was requested for source.
    let mut diagnostics: Box<dyn Write> = if arguments.output { Box::new(stderr()) } else { Box::new(stdout()) };
    let _ = writeln!(diagnostics, "PPLD v{} - PCBoard Programming Language Decompiler", *VERSION);
    if let Some(version) = arguments.lang_version
        && !SUPPORTED_PPL_LANGUAGE_VERSIONS.contains(&version)
    {
        eprintln!("Invalid language version valid values {SUPPORTED_PPL_LANGUAGE_VERSIONS:?}");
        std::process::exit(2);
    }
    let env_language_version = if arguments.lang_version.is_none() {
        match language_version_from_env() {
            Ok(version) => version,
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(2);
            }
        }
    } else {
        None
    };
    let mut output_func = OutputFunc::Upper;
    match arguments.style {
        Some('u') => output_func = OutputFunc::Upper,
        Some('l') => output_func = OutputFunc::Lower,
        Some('c') => output_func = OutputFunc::CamelCase,
        Some(x) => {
            eprintln!("Invalid keyword style '{x}', valid values are u=upper, l=lower, c=camel");
            std::process::exit(2);
        }
        None => {}
    }

    let Some(mut file_name) = arguments.file else {
        eprintln!("{}", icy_board_cli::command::<Cli>().render_help());
        std::process::exit(1);
    };

    let extension = Path::new(&file_name).extension().and_then(OsStr::to_str);
    if extension.is_none() {
        file_name.push_str(".ppe");
    }

    let out_file_name = Path::new(&file_name).with_extension("ppd");
    match Executable::read_file(&file_name, !arguments.output) {
        Ok(mut executable) => {
            if arguments.check {
                let report = match check_compatibility(&executable) {
                    Ok(report) => report,
                    Err(err) => {
                        eprintln!("ERROR during compatibility check: {err}");
                        std::process::exit(1);
                    }
                };
                let result = execute!(
                    diagnostics,
                    SetAttribute(Attribute::Bold),
                    Print(format!("\nChecking compatibility for: {}\n", file_name)),
                    SetAttribute(Attribute::Reset),
                    Print(format!("PPE Version: {}\n\n", executable.runtime))
                )
                .and_then(|()| report.write_report(&mut diagnostics));
                if let Err(err) = result {
                    eprintln!("ERROR writing compatibility report: {err}");
                    std::process::exit(1);
                }
                std::process::exit(i32::from(arguments.strict && report.summary.has_findings()));
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

            let lang_version = arguments.lang_version.or(env_language_version).unwrap_or(LAST_PPL_LANGUAGE_VERSION);
            match decompile(executable, arguments.raw, lang_version) {
                Ok((decompilation, issues)) => {
                    let mut output_visitor: output_visitor::OutputVisitor = output_visitor::OutputVisitor::default();
                    // The source is written for our own pplc, whatever runtime the PPE was built for.
                    output_visitor.version = lang_version;
                    output_visitor.output_func = output_func;
                    decompilation.visit(&mut output_visitor);
                    if arguments.output {
                        println!("{}", output_visitor.output);
                    } else {
                        let res = if arguments.cp437 {
                            write_cp437(&out_file_name, &output_visitor.output)
                        } else {
                            File::create(&out_file_name).and_then(|mut output| write!(output, "{}", output_visitor.output))
                        };
                        if let Err(err) = res {
                            eprintln!("ERROR: Can't create {}: {err}", out_file_name.display());
                            std::process::exit(1);
                        }
                        let _ = execute!(
                            diagnostics,
                            Print("\nSource decompilation complete: ".to_string()),
                            SetAttribute(Attribute::Bold),
                            Print(format!("{file_name}\n")),
                            SetAttribute(Attribute::Reset),
                            Print("decompiled to: ".to_string()),
                            SetAttribute(Attribute::Bold),
                            Print(format!("{out_file_name:?}\n")),
                            SetAttribute(Attribute::Reset),
                        );
                    }

                    if !issues.is_empty() {
                        let _ = writeln!(diagnostics);
                    }
                    for issue in &issues {
                        let _ = execute!(
                            diagnostics,
                            SetAttribute(Attribute::Bold),
                            SetForegroundColor(Color::Yellow),
                            Print("WARNING: ".to_string()),
                            SetAttribute(Attribute::Reset),
                            SetAttribute(Attribute::Bold),
                            Print(format!("[{:04X}]:", issue.byte_offset)),
                            SetAttribute(Attribute::Reset),
                            Print(format!("{}", issue.bug)),
                            SetAttribute(Attribute::Reset),
                        );
                        let _ = writeln!(diagnostics);
                    }
                    if !issues.is_empty() {
                        let _ = writeln!(diagnostics, "{0} issues found during decompilation", issues.len());
                    }
                    // The .ppd is written either way, so the exit code is all a caller has to go on.
                    std::process::exit(if issues.is_empty() { 0 } else { 1 });
                }
                Err(err) => {
                    eprintln!("ERROR: Can't decompile {file_name}: {err}");
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("ERROR: Can't read {file_name}: {err}");
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
