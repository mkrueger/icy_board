use core::panic;
use std::path::PathBuf;

use crate::{
    executable::LAST_PPLC,
    parser::{parse_ast, Encoding},
};

use super::SemanticVisitor;

#[test]
fn find_label_references() {
    find_references(
        r#"
@:mylabel@
PRINT "Hello World"
goto $mylabel$
gosub $MyLabel$
"#,
    );
}

#[test]
fn find_local_references() {
    find_references(
        r#"declare procedure foo()
    :mylabel
PRINT "Hello World"

procedure foo()
goto $mylabel$
gosub $MyLabel$
@:mylabel@
endproc

"#,
    );
}

#[test]
fn find_procedure() {
    find_references(
        r"
declare procedure @foo@()
$foo$()
procedure @foo@()
endproc
",
    );
}

#[test]
fn find_function() {
    find_references(
        r"
declare function @foo@() INT
PRINTLN $foo$()
function @foo@() INT
$foo$ = 1
endproc
",
    );
}

#[test]
fn find_variables() {
    find_references(
        r"
    INTEGER @BAR@
PRINTLN $BAR$
$BAR$ = $BAR$ + 1
",
    );
}

#[test]
fn find_dims() {
    find_references(
        r"
    INTEGER @BAR@(10)
PRINTLN $BAR$(1)
$BAR$(3) = $BAR$(2) + 1
",
    );
}

#[test]
fn find_variables2() {
    find_references(
        r"
    INTEGER @BAR@
PRINTLN TOSTRING($BAR$)
",
    );
}

#[test]
fn find_user_vars() {
    find_references(
        r"
        PRINTLN $U_CMNT1$
        PRINTLN $U_CMNT1$
        PRINTLN $U_CMNT1$
        PRINTLN $U_CMNT1$
        PRINTLN $U_CMNT1$
        PRINTLN $U_CMNT1$
",
    );
}

fn find_references(arg: &str) {
    let mut txt = String::new();

    let mut ref_offset = 0;

    let mut declaration_span = 0..0;
    let mut spans = Vec::new();

    for ch in arg.chars() {
        if ch == '@' {
            if ref_offset == 0 {
                ref_offset = txt.len();
            } else {
                declaration_span = ref_offset..txt.len();
                ref_offset = 0;
            }
            continue;
        }
        if ch == '$' {
            if ref_offset == 0 {
                ref_offset = txt.len();
            } else {
                spans.push(ref_offset..txt.len());
                ref_offset = 0;
            }
            continue;
        }
        txt.push(ch);
    }
    let (ast, errors) = parse_ast(PathBuf::from("."), &txt, Encoding::Utf8, LAST_PPLC);

    let mut visitor = SemanticVisitor::new(LAST_PPLC, errors.clone());
    ast.visit(&mut visitor);

    if !errors.lock().unwrap().errors.is_empty() {
        for e in &errors.lock().unwrap().errors {
            println!("{}", e.error);
        }
        panic!("parse error");
    }
    for (_rt, refs) in &visitor.references {
        if refs.usages.len() + refs.return_types.len() == spans.len() {
            if let Some(decl) = &refs.declaration {
                if decl.span == declaration_span {
                    assert_eq!(declaration_span, decl.span);
                } else {
                    let decl = refs.implementation.as_ref().unwrap();
                    assert_eq!(declaration_span, decl.span);
                }
            }

            for r in refs.usages.iter().chain(refs.return_types.iter()) {
                assert!(spans.contains(&r.span));
            }
            return;
        }
    }
    println!("REFERENCES NOT FOUND");
    println!("Expected ({}):", spans.len());
    println!("declaration:{declaration_span:?}");
    for r in &spans {
        println!("ref:{r:?}");
    }

    println!("reference table ({})\n", visitor.references.len());
    for (rt, refs) in visitor.references {
        println!("----->{:?} ({})", rt, refs.usages.len());
        println!("decl:{:?}", refs.declaration);
        println!("impl:{:?}", refs.implementation);
        for r in &refs.usages {
            println!("ref:{r:?}");
        }
        for r in &refs.return_types {
            println!("ret:{r:?}");
        }

        println!();
    }

    panic!("not found");
}
