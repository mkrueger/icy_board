use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{Ast, AstNode, CommentAstNode, Statement},
    compiler::workspace::Workspace,
};

use super::{
    Encoding, ErrorReporter, Parser, ParserErrorType, UserTypeRegistry,
    lexer::{CommentType, Token},
};

/// Parses one source file into an abstract syntax tree.
pub fn parse_ast(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
) -> Ast {
    parse_ast_internal(file_name, error_reporter, input, user_types, encoding, workspace, false)
}

pub fn parse_ast_with_predeclared_types(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
) -> Ast {
    parse_ast_internal(file_name, error_reporter, input, user_types, encoding, workspace, true)
}

fn parse_ast_internal(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
    types_predeclared: bool,
) -> Ast {
    error_reporter.lock().unwrap().set_file_name(&file_name);
    let mut nodes = Vec::new();
    let mut parser = Parser::new(file_name.clone(), error_reporter, user_types, input, encoding, workspace);
    parser.types_predeclared = types_predeclared;
    parser.next_token();
    parser.skip_eol();

    while parser.cur_token.is_some() {
        if let Some(node) = parser.parse_ast_node() {
            nodes.push(node);
        }
    }

    if parser.in_module && parser.module.as_ref().is_some_and(|module| !module.is_implicit()) {
        parser
            .error_reporter
            .lock()
            .unwrap()
            .report_error(parser.lex.span(), ParserErrorType::EndModuleExpected);
    }

    attach_routine_documentation(input, &mut nodes);

    Ast {
        nodes,
        file_name,
        module: parser.module,
        imports: parser.imports,
        language_version: parser.lang_version,
        require_user_variables: parser.require_user_variables,
    }
}

fn attach_routine_documentation(input: &str, nodes: &mut [AstNode]) {
    for routine_index in 0..nodes.len() {
        let routine_start = match &nodes[routine_index] {
            AstNode::Function(node) => node.get_function_token().span.start,
            AstNode::Procedure(node) => node.get_procedure_token().span.start,
            AstNode::FunctionDeclaration(node) => node.get_declare_token().span.start,
            AstNode::ProcedureDeclaration(node) => node.get_declare_token().span.start,
            _ => continue,
        };

        let mut lines: Option<Vec<String>> = None;
        let mut next_start = routine_start;
        let mut collect_comment = |comment: &CommentAstNode| {
            let token = comment.get_comment_token();
            let Token::Comment(CommentType::SingleLineSemicolon, text) = &token.token else {
                return false;
            };
            let Some(documentation) = text.strip_prefix(";;") else {
                return false;
            };
            let gap = &input[token.span.end.min(input.len())..next_start.min(input.len())];
            if !gap.chars().all(char::is_whitespace) || gap.matches('\n').count() > 1 {
                return false;
            }
            lines
                .get_or_insert_with(Vec::new)
                .push(documentation.strip_prefix(' ').unwrap_or(documentation).to_string());
            next_start = token.span.start;
            true
        };

        if let Some(AstNode::Main(main)) = nodes.get(routine_index.wrapping_sub(1)) {
            for statement in main.get_statements().iter().rev() {
                let Statement::Comment(comment) = statement else {
                    break;
                };
                if !collect_comment(comment) {
                    break;
                }
            }
        } else {
            for previous in nodes[..routine_index].iter().rev() {
                let AstNode::TopLevelStatement(Statement::Comment(comment)) = previous else {
                    break;
                };
                if !collect_comment(comment) {
                    break;
                }
            }
        }

        let Some(mut lines) = lines else { continue };
        lines.reverse();
        let documentation = lines.join("\n");
        match &mut nodes[routine_index] {
            AstNode::Function(node) => node.set_documentation(documentation),
            AstNode::Procedure(node) => node.set_documentation(documentation),
            AstNode::FunctionDeclaration(node) => node.set_documentation(documentation),
            AstNode::ProcedureDeclaration(node) => node.set_documentation(documentation),
            _ => unreachable!(),
        }
    }
}

pub fn preparse_type_declarations(
    file_name: PathBuf,
    error_reporter: Arc<Mutex<ErrorReporter>>,
    input: &str,
    user_types: &UserTypeRegistry,
    encoding: Encoding,
    workspace: &Workspace,
) {
    error_reporter.lock().unwrap().set_file_name(&file_name);
    // The whole file is read again for the real parse, which reports everything it
    // finds. Only what the declarations themselves say is new here.
    let scratch = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser = Parser::new(file_name, scratch, user_types, input, encoding, workspace);
    parser.next_token();
    while parser.cur_token.is_some() {
        if parser.lang_version >= 400
            && matches!(parser.get_cur_token(), Some(Token::Identifier(ref name)) if name.eq_ignore_ascii_case("MODULE"))
            && matches!(parser.peek_after_current(1).as_slice(), [Some(Token::Identifier(_))])
        {
            parser.parse_module_start();
        } else if parser.lang_version >= 400
            && matches!(parser.get_cur_token(), Some(Token::Identifier(ref name)) if name.eq_ignore_ascii_case("ENDMODULE"))
            && parser.module.as_ref().is_some_and(|module| !module.is_implicit())
            && matches!(parser.peek_after_current(1).as_slice(), [Some(Token::Eol | Token::Comment(_, _)) | None])
        {
            parser.parse_module_end();
        } else if parser.lang_version >= 400
            && matches!(parser.get_cur_token(), Some(Token::Identifier(ref name)) if name.eq_ignore_ascii_case("IMPORT"))
            && matches!(parser.peek_after_current(2).as_slice(), [Some(Token::Identifier(_)), Some(Token::Identifier(as_name))] if as_name.eq_ignore_ascii_case("AS"))
        {
            parser.parse_import();
        } else if matches!(parser.get_cur_token(), Some(Token::Type | Token::Enum)) {
            let scratch = std::mem::replace(&mut parser.error_reporter, error_reporter.clone());
            if parser.get_cur_token() == Some(Token::Type) {
                parser.parse_type_declaration();
            } else {
                parser.parse_enum_declaration();
            }
            parser.error_reporter = scratch;
        } else {
            parser.next_token();
        }
    }
}
