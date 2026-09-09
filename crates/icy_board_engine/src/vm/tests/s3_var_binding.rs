use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::Executable,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn compile(source: &str, language: u16, runtime: u16, optimize: bool) -> Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("s3.pps")]);
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let source = if language >= 400 {
        source.replace("\nEND\n", "\nEXIT\n")
    } else {
        source.to_string()
    };
    let ast = parse_ast(PathBuf::from("s3.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone()).with_optimization(optimize);
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "{:?}",
        reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    drop(reporter);
    compiler.create_executable().unwrap()
}

fn output(executable: Executable) -> String {
    let executable = match executable.to_buffer() {
        Ok(mut bytes) => Executable::from_buffer(&mut bytes, false).unwrap(),
        Err(
            crate::executable::ExecutableError::UnsupportedRecordFieldEncoding { .. } | crate::executable::ExecutableError::UnsupportedShortCircuitEncoding,
        ) => executable,
        Err(error) => panic!("{error}"),
    };
    let (success, text) = super::run_executable_collecting(executable, |_| {}, &[], None, &[], false, false);
    assert!(success, "{text}");
    text.replace('\r', "")
}

#[test]
fn s3_original_var_binding_and_runtime_recursion_contract() {
    let source = include_str!("../../../../../compat/var_binding.pps");
    for (language, runtime) in [(340, 340), (350, 350), (340, 400), (400, 400)] {
        for optimize in [false, true] {
            let executable = compile(source, language, runtime, optimize);
            let loaded = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
            let recursive = if runtime < 400 { 1 } else { 12 };
            assert_eq!(
                output(loaded),
                format!("---BEGIN---\nalias=10\nchanged_index=9:2:1\nindex_calls=1:8\nparameter_index=2:7:0\nrecursive_var={recursive}\n---END---\n"),
                "{language}/{runtime}, optimize={optimize}"
            );
        }
    }
}

#[test]
fn s3_nested_record_targets_capture_indices_before_later_arguments() {
    let source = r#"
TYPE Leaf
 INTEGER Values[,]
ENDTYPE
TYPE Branch
 Leaf Leaves[]
ENDTYPE
Branch roots[1]
INTEGER cursor = 1
INTEGER visits
roots[1].Leaves.Redim(1)
roots[1].Leaves[1].Values.Redim(1, 1)
roots[1].Leaves[1].Values[1, 1] = 5
Change(roots[Mark()].Leaves[Mark()].Values[Mark(), Mark()], MoveCursor())
PRINTLN visits, ":", roots[1].Leaves[1].Values[1, 1], ":", cursor
PROCEDURE Change(VAR INTEGER value, INTEGER unused)
 value = 9
ENDPROC
FUNCTION Mark() INTEGER
 visits += 1
 RETURN cursor
ENDFUNC
FUNCTION MoveCursor() INTEGER
 cursor = 0
 RETURN 0
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!(output(compile(source, 400, 400, optimize)), "4:9:0\n");
    }
}

#[test]
fn s3_overlapping_array_and_element_targets_keep_reverse_copyout() {
    let source = r#"
INTEGER data[] = { 5 }
Change(data[0], data)
PRINTLN data.Len(), ":", data[0], ":", data[1]
PROCEDURE Change(VAR INTEGER element, VAR INTEGER values[])
 element = 10
 INTEGER replacement[] = { 20, 21 }
 values = replacement
ENDPROC
"#;
    for optimize in [false, true] {
        let executable = compile(source, 400, 400, optimize);
        let loaded = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
        assert_eq!(output(loaded), "2:10:21\n");
    }
}

#[test]
fn s3_array_returns_and_var_resize_preserve_rank_and_empty_state() {
    for (rank, bounds, indices, count) in [("[]", "1", "1", 2), ("[,]", "1, 2", "1, 2", 6), ("[,,]", "1, 2, 3", "1, 2, 3", 24)] {
        let source = format!(
            r#"
INTEGER data[{bounds}]
data[{indices}] = 99
Change(data)
PRINTLN data.Len(), ":", data[{indices}]
data = Empty()
PRINTLN data.Len()
data = Nested(2)
PRINTLN data.Len(), ":", data[{indices}]
PROCEDURE Change(VAR INTEGER values{rank})
 values.Redim({bounds})
ENDPROC
FUNCTION Empty() INTEGER{rank}
ENDFUNC
FUNCTION Nested(INTEGER depth) INTEGER{rank}
 INTEGER result[{bounds}]
 result[{indices}] = depth
 IF depth > 0 THEN
  INTEGER ignored{rank} = Nested(depth - 1)
 ENDIF
 RETURN result
ENDFUNC
"#
        );
        for optimize in [false, true] {
            let executable = compile(&source, 400, 400, optimize);
            let loaded = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
            assert_eq!(output(loaded), format!("{count}:0\n0\n{count}:2\n"), "{rank}");
        }
    }
}

#[test]
fn s3_decompiler_roundtrip_preserves_var_binding_and_array_returns() {
    let source = r#"
INTEGER cursor = 1
INTEGER data[1]
Change(data[cursor])
PRINTLN data[1], ":", data[0]
data = Empty()
PRINTLN data.Len()
PROCEDURE Change(VAR INTEGER value)
 cursor = 0
 value = 9
ENDPROC
FUNCTION Empty() INTEGER[]
ENDFUNC
"#;
    for optimize in [false, true] {
        let executable = compile(source, 400, 400, optimize);
        let loaded = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
        assert_eq!(output(loaded.clone()), "9:0\n0\n");
        for raw in [false, true] {
            let (ast, issues) = crate::decompiler::decompile(loaded.clone(), raw, 400).unwrap();
            assert!(issues.is_empty());
            let mut visitor = crate::ast::output_visitor::OutputVisitor::default();
            visitor.version = 400;
            ast.visit(&mut visitor);
            assert!(visitor.output.contains("INTEGER[]"), "{}", visitor.output);
            assert_eq!(output(compile(&visitor.output, 400, 400, optimize)), "9:0\n0\n", "{}", visitor.output);
        }
    }
}
