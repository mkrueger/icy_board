use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use icy_board_engine::{
    ast::{Ast, OutputFunc, output_visitor},
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, LAST_PPE_RUNTIME, LAST_PPL_LANGUAGE_VERSION},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

use crate::decompile;

fn is_match(output: &str, original: &str) -> bool {
    let mut i = 0;
    let mut j = 0;

    let output = output.as_bytes();
    let original = original.as_bytes();

    while i < output.len() && j < original.len() {
        // skip comments - assume that ';' is not inside a string
        if output[i] == b';' {
            while output[i] != b'\n' {
                i += 1;
            }
        }

        if output[i] == original[j] {
            i += 1;
            j += 1;
            continue;
        }
        if char::is_whitespace(output[i] as char) {
            i += 1;
            continue;
        }
        if char::is_whitespace(original[j] as char) {
            j += 1;
            continue;
        }
        return false;
    }
    // skip original trailing ws.
    while j < original.len() && char::is_whitespace(original[j] as char) {
        j += 1;
    }
    if j >= original.len() {
        return true;
    }
    false
}

#[test]
fn test_decompiler() {
    use std::fs::{self};
    let mut data_path = env::current_dir().unwrap();
    data_path.push("test_data");
    //let mut success = 0;
    //let mut skipped = 0;
    for entry in fs::read_dir(data_path).expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();
        if cur_entry.extension().unwrap() != "ppe" {
            continue;
        }

        let file_name = cur_entry.as_os_str();
        /*
        if ["select_case.ppe"].contains(&cur_entry.file_name().unwrap().to_str().unwrap()) {
            //skipped += 1;
            continue;
        }
        */

        let executable = Executable::read_file(&file_name, false).unwrap();
        // The reference sources are 3.40 era, so no REPEAT/LOOP is reconstructed for them.
        let (d, _) = decompile(executable, false, 340).unwrap();
        let source_file = cur_entry.with_extension("pps");
        let orig_text = fs::read_to_string(source_file).unwrap();
        let mut output_visitor = output_visitor::OutputVisitor::default();
        output_visitor.output_func = OutputFunc::Upper;
        output_visitor.skip_comments = true;
        d.visit(&mut output_visitor);

        let are_equal = is_match(&output_visitor.output, &orig_text);

        if are_equal {
            //success += 1;
        } else {
            println!(
                "'{}' not matched…\n{}-----\n{}",
                cur_entry.file_name().unwrap().to_str().unwrap(),
                output_visitor.output,
                orig_text
            );
        }

        assert!(are_equal);
    }
}

fn decompile_to_text(executable: Executable, language_version: u16) -> String {
    let (ast, _) = decompile(executable, false, language_version).unwrap();
    let mut visitor = output_visitor::OutputVisitor::default();
    visitor.version = language_version;
    visitor.output_func = OutputFunc::Upper;
    visitor.skip_comments = true;
    ast.visit(&mut visitor);
    visitor.output
}

fn compile_source(source: &str, runtime: u16) -> Result<Executable, String> {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let reg = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("test.pps")]);
    workspace.package.runtime = Some(runtime);

    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &reg, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&[&ast] as &[&Ast]);

    let reporter = errors.lock().unwrap();
    if reporter.has_errors() {
        return Err(reporter.errors.iter().map(|e| format!("  {}", e.error)).collect::<Vec<_>>().join("\n"));
    }
    drop(reporter);

    // Only the on disk form carries the entry types the decompiler reads, so go through it.
    let executable = compiler.create_executable().map_err(|e| e.to_string())?;
    let mut bytes = executable.to_buffer().map_err(|e| e.to_string())?;
    Executable::from_buffer(&mut bytes, false).map_err(|e| e.to_string())
}

/// The first pass rewrites whatever layout the original compiler used into the one
/// our own pplc emits, so it is the passes after that which have to agree. A drift
/// between them means a construct survives being written out but not being read back.
#[test]
fn decompiled_source_settles_after_one_pass() {
    let mut data_path = env::current_dir().unwrap();
    data_path.push("test_data");

    let mut checked = 0;
    let mut failures = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(data_path)
        .expect("Error reading test_data directory.")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ppe")))
        .collect();
    entries.sort();

    for cur_entry in entries {
        let name = cur_entry.file_name().unwrap().to_string_lossy().to_string();
        let executable = Executable::read_file(&cur_entry.as_os_str(), false).unwrap();
        let runtime = executable.runtime;
        let lang_version = runtime.min(LAST_PPL_LANGUAGE_VERSION);

        let mut text = decompile_to_text(executable, lang_version);
        let mut previous = None;
        for pass in 1..=2 {
            match compile_source(&text, runtime) {
                Ok(rebuilt) => {
                    previous = Some(text);
                    text = decompile_to_text(rebuilt, lang_version);
                }
                Err(diagnostics) => {
                    failures.push(format!("{name}: does not compile after pass {pass}\n{diagnostics}"));
                    previous = None;
                    break;
                }
            }
        }

        if let Some(previous) = previous {
            if previous != text {
                failures.push(format!("{name}: still moving on the third pass"));
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "no .ppe files found in test_data");
    assert!(
        failures.is_empty(),
        "{} of {checked} files did not settle:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// PPLD writes its source for our own toolchain, so every file in the corpus has
/// to come back through pplc's defaults - the language version where END closes a
/// block instead of ending the program.
#[test]
fn decompiled_source_compiles_with_the_current_language() {
    let mut data_path = env::current_dir().unwrap();
    data_path.push("test_data");

    let mut checked = 0;
    let mut failures = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(data_path)
        .expect("Error reading test_data directory.")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ppe")))
        .collect();
    entries.sort();

    for cur_entry in entries {
        let name = cur_entry.file_name().unwrap().to_string_lossy().to_string();
        let executable = Executable::read_file(&cur_entry.as_os_str(), false).unwrap();
        let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

        if let Err(diagnostics) = compile_source(&text, LAST_PPE_RUNTIME) {
            failures.push(format!("{name}: does not compile\n{diagnostics}"));
        }
        checked += 1;
    }

    assert!(checked > 0, "no .ppe files found in test_data");
    assert!(
        failures.is_empty(),
        "{} of {checked} files did not compile:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The program terminator is spelled END in the file and EXIT in current source.
#[test]
fn ending_the_program_decompiles_to_exit() {
    let source = "DECLARE PROCEDURE Greet()\n\
                  Greet()\n\
                  EXIT\n\
                  PROCEDURE Greet()\n\
                  PRINTLN \"Hello World!\"\n\
                  ENDPROC\n";

    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    assert!(text.lines().any(|line| line.trim().eq_ignore_ascii_case("EXIT")), "no EXIT in:\n{text}");
    assert!(
        !text.lines().any(|line| line.trim().eq_ignore_ascii_case("END")),
        "END left as a statement in:\n{text}"
    );

    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|e| panic!("does not compile again:\n{text}\n{e}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));

    let legacy = decompile_to_text(compile_source(source, LAST_PPE_RUNTIME).unwrap(), 340);
    assert!(legacy.lines().any(|line| line.trim() == "END"), "no END in:\n{legacy}");
}

/// The source names the language it is written in, so it needs no option to be
/// compiled again.
#[test]
fn decompiled_source_declares_its_language() {
    let executable = compile_source("PRINTLN \"Hello World!\"\n", LAST_PPE_RUNTIME).unwrap();

    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);
    assert_eq!(
        Some(format!(";$LANGVERSION {LAST_PPL_LANGUAGE_VERSION}")),
        text.lines().next().map(str::to_string)
    );

    let executable = compile_source("PRINTLN \"Hello World!\"\n", LAST_PPE_RUNTIME).unwrap();
    let legacy = decompile_to_text(executable, 340);
    assert_eq!(Some(";$LANGVERSION 340".to_string()), legacy.lines().next().map(str::to_string));
}

/// The loops 350 added are labels and jumps in the PPE, so they only come back as
/// loops for a language that has them.
#[test]
fn repeat_and_loop_survive_decompilation() {
    let source = "INTEGER I\n\
                  I = 0\n\
                  REPEAT\n\
                  I = I + 1\n\
                  UNTIL I > 3\n\
                  LOOP\n\
                  I = I + 1\n\
                  IF (I > 6) BREAK\n\
                  ENDLOOP\n\
                  PRINTLN I\n";

    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    assert!(text.contains("REPEAT"), "no REPEAT in:\n{text}");
    assert!(text.contains("UNTIL"), "no UNTIL in:\n{text}");
    assert!(text.contains("LOOP"), "no LOOP in:\n{text}");
    assert!(text.contains("BREAK"), "no BREAK in:\n{text}");

    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|e| panic!("does not compile again:\n{text}\n{e}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));

    let legacy = decompile_to_text(compile_source(source, LAST_PPE_RUNTIME).unwrap(), 340);
    assert!(!legacy.contains("REPEAT"), "REPEAT written for a language without it:\n{legacy}");
    assert!(!legacy.contains("ENDLOOP"), "LOOP written for a language without it:\n{legacy}");
}

/// A record keeps no name in the PPE, so the decompiler invents one. What matters
/// is that the result still describes the same layout and compiles again.
#[test]
fn records_survive_decompilation() {
    let source = "TYPE Point\n\
                  INTEGER X\n\
                  STRING Label\n\
                  ENDTYPE\n\
                  \n\
                  Point Pt\n\
                  Pt.X = 42\n\
                  Pt.Label = \"here\"\n\
                  PRINTLN Pt.X, Pt.Label\n";

    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    assert!(text.contains("TYPE TYPE001"), "no type declaration in:\n{text}");
    assert!(text.contains("INTEGER FIELD001"), "no first field in:\n{text}");
    assert!(text.contains("STRING FIELD002"), "no second field in:\n{text}");
    assert!(text.contains("ENDTYPE"), "type block not closed in:\n{text}");
    assert!(text.contains(".FIELD001 = 42"), "no member assignment in:\n{text}");

    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|e| panic!("does not compile again:\n{text}\n{e}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));
}

/// A record inside a record has to come back out as two declarations, with the
/// outer one naming the inner.
#[test]
fn nested_records_survive_decompilation() {
    let source = "TYPE Inner\n\
                  INTEGER Value\n\
                  ENDTYPE\n\
                  TYPE Outer\n\
                  Inner Part\n\
                  ENDTYPE\n\
                  \n\
                  Outer Rec\n\
                  Rec.Part.Value = 7\n\
                  PRINTLN Rec.Part.Value\n";

    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|e| panic!("does not compile again:\n{text}\n{e}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));
}

/// Board objects do carry their member names, in the registry rather than in the
/// file, so those come back as they were written.
#[test]
fn board_object_members_keep_their_names() {
    let source = "CONFERENCE Conf = CONFINFO(0)\n\
                  PRINTLN Conf.Name\n\
                  PRINTLN Conf.GetDoor(0).Name\n";

    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    assert!(text.contains("Conference VAR001"), "type name lost in:\n{text}");
    assert!(text.contains("VAR001.Name"), "member name lost in:\n{text}");
    assert!(text.contains("VAR001.GetDoor(0).Name"), "chained call lost in:\n{text}");

    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|e| panic!("does not compile again:\n{text}\n{e}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));
}

#[test]
fn record_literals_survive_decompilation() {
    let source = "TYPE Point\n  INTEGER X\n  INTEGER Y\nENDTYPE\nPoint value = Point { Y = 2, X = 1 }\nPRINTLN value.X, value.Y\n";
    let executable = compile_source(source, LAST_PPE_RUNTIME).unwrap();
    let text = decompile_to_text(executable, LAST_PPL_LANGUAGE_VERSION);

    assert!(text.contains("TYPE001 { FIELD002 = 2, FIELD001 = 1 }"), "record literal lost in:\n{text}");
    let rebuilt = compile_source(&text, LAST_PPE_RUNTIME).unwrap_or_else(|error| panic!("does not compile again:\n{text}\n{error}"));
    assert_eq!(text, decompile_to_text(rebuilt, LAST_PPL_LANGUAGE_VERSION));
}
