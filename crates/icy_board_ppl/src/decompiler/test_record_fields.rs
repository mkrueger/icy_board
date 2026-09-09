use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{AstNode, output_visitor::OutputVisitor},
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, VariableType},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn compile_in_memory(source: &str) -> Executable {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(400));
    workspace.package.runtime = Some(400);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(
        PathBuf::from("s1_record_fields.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let errors = errors.lock().unwrap();
    assert!(
        !errors.has_errors(),
        "{source}\n{:?}",
        errors.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    drop(errors);
    compiler.create_executable().expect("in-memory executable")
}

fn assert_in_memory_roundtrip(source: &str, expected_fields: &[&str]) {
    let executable = compile_in_memory(source);
    for raw in [false, true] {
        let (ast, issues) = super::decompile(executable.clone(), raw, 400).expect("in-memory decompilation");
        assert!(issues.is_empty(), "unexpected decompiler issues");
        let declarations = ast
            .nodes
            .iter()
            .filter_map(|node| match node {
                AstNode::TypeDeclaration(declaration) => Some(declaration),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), executable.user_types.len());
        for (declaration, layout) in declarations.iter().zip(&executable.user_types) {
            assert_eq!(declaration.get_fields().len(), layout.len());
            for (field, stored) in declaration.get_fields().iter().zip(layout) {
                let source_type = if stored.variable_type == VariableType::UnboundedString {
                    VariableType::String
                } else {
                    stored.variable_type
                };
                assert_eq!(field.get_variable_type(), source_type);
                let dimensions = field.get_specifier().get_dimensions();
                assert_eq!(dimensions.len(), usize::from(stored.dim), "{field:?}");
                for (dimension, bound) in dimensions.iter().zip([stored.vector_size, stored.matrix_size, stored.cube_size]) {
                    assert_eq!(dimension.is_dynamic(), stored.is_dynamic, "{field:?}");
                    assert_eq!(dimension.get_dimension(), usize::from(bound), "{field:?}");
                }
            }
        }
        let mut output = OutputVisitor::default();
        output.version = 400;
        ast.visit(&mut output);
        for expected in expected_fields {
            assert!(
                output.output.lines().any(|line| line.trim() == *expected),
                "missing {expected}:\n{}",
                output.output
            );
        }
        // S1 layouts have no on-disk encoding; recompile the source without serializing either executable.
        let rebuilt = compile_in_memory(&output.output);
        assert_eq!(rebuilt.user_types, executable.user_types, "{}", output.output);
    }
}

#[test]
fn s1_dynamic_and_fixed_record_dimensions_survive_in_memory_decompilation() {
    assert_in_memory_roundtrip(
        r#"TYPE Payload
    INTEGER Scalar
    INTEGER Values[]
    STRING Grid[,]
    INTEGER Cube[,,]
    INTEGER Zero[0]
    INTEGER FixedGrid[2,3]
    INTEGER FixedCube[1,2,3]
ENDTYPE
Payload item
PRINTLN item.Scalar
"#,
        &[
            "INTEGER FIELD001",
            "INTEGER FIELD002[]",
            "STRING FIELD003[,]",
            "INTEGER FIELD004[,,]",
            "INTEGER FIELD005[0]",
            "INTEGER FIELD006[2,3]",
            "INTEGER FIELD007[1,2,3]",
        ],
    );
}

#[test]
fn s1_host_and_nested_record_fields_survive_in_memory_decompilation() {
    assert_in_memory_roundtrip(
        r#"TYPE Payload
    INTEGER Scalar
    SURFACE Image
    USER Owner
    AUDIO Clips[]
    USER Grid[,]
    SURFACE Cube[,,]
    CONTACT Person
    SURFACE Fixed[0]
ENDTYPE
TYPE Envelope
    Payload Value
    Payload Values[]
    Payload Grid[,]
    Payload Cube[,,]
    Payload Fixed[0]
ENDTYPE
Envelope item
PRINTLN item.Value.Scalar
"#,
        &[
            "Surface FIELD002",
            "User FIELD003",
            "Audio FIELD004[]",
            "User FIELD005[,]",
            "Surface FIELD006[,,]",
            "CONTACT FIELD007",
            "Surface FIELD008[0]",
            "TYPE001 FIELD001",
            "TYPE001 FIELD002[]",
            "TYPE001 FIELD003[,]",
            "TYPE001 FIELD004[,,]",
            "TYPE001 FIELD005[0]",
        ],
    );
}
