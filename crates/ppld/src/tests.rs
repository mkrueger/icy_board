use std::env;

use icy_ppe::{
    ast::{output_visitor, OutputFunc},
    executable::Executable,
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
        println!(
            "File: {}...",
            cur_entry.file_name().unwrap().to_str().unwrap()
        );
        if [
            "for_next.ppe",
            "select_case.ppe",
            "while_break_continue.ppe",
            "if_else_loop_bug.ppe",
            "for_break_continue.ppe",
            "99bottles.ppe",
            "if_elseif_else_endif_end.ppe",
            "if_then_else.ppe",
            "if_then.ppe",
            "while.ppe",
            "abort.ppe",
            "loggedon.ppe",
        ]
        .contains(&cur_entry.file_name().unwrap().to_str().unwrap())
        {
            //skipped += 1;
            continue;
        }

        let executable = Executable::read_file(&file_name, false).unwrap();
        let (d, _) = decompile(executable, false).unwrap();
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
