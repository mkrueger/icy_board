//! Informational code lenses for user-defined routines.

use icy_board_engine::{
    ast::{Ast, AstNode},
    semantic::SemanticVisitor,
};
use ropey::Rope;
use tower_lsp::lsp_types::{CodeLens, Command, Range};

use crate::offset_to_position;

fn lens_range(rope: &Rope, span: &std::ops::Range<usize>) -> Option<Range> {
    let start = offset_to_position(span.start, rope)?;
    let end = offset_to_position(span.end, rope)?;
    Some(Range::new(start, end))
}

pub fn get_code_lenses(ast: &Ast, semantic: &SemanticVisitor, rope: &Rope) -> Vec<CodeLens> {
    let mut lenses = Vec::new();
    for node in &ast.nodes {
        let token = match node {
            AstNode::Function(function) => function.get_identifier_token(),
            AstNode::Procedure(procedure) => procedure.get_identifier_token(),
            AstNode::FunctionDeclaration(function) => function.get_identifier_token(),
            AstNode::ProcedureDeclaration(procedure) => procedure.get_identifier_token(),
            _ => continue,
        };
        let count = semantic
            .references
            .iter()
            .find(|(_, references)| references.contains_pos(&ast.file_name, token.span.start))
            .map_or(0, |(_, references)| references.usages.len());
        let Some(range) = lens_range(rope, &token.span) else {
            continue;
        };
        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: format!("{count} reference{}", if count == 1 { "" } else { "s" }),
                command: String::new(),
                arguments: None,
            }),
            data: None,
        });
    }
    lenses
}
