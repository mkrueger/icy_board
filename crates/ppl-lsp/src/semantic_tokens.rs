use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{
        Ast, AstVisitor, ConstDeclarationStatement, Constant, EnumDeclarationAstNode, Expression, MemberReferenceExpression, ParameterSpecifier,
        TypeDeclarationAstNode, VariableDeclarationStatement,
    },
    compiler::workspace::Workspace,
    parser::{
        Encoding, ErrorReporter,
        lexer::{KEYWORDS, Lexer, Token},
    },
    semantic::{ReferenceType, SemanticVisitor},
};
use ropey::Rope;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType};

use crate::offset_to_position;

const KEYWORD: u32 = 0;
const COMMENT: u32 = 1;
const STRING: u32 = 2;
const NUMBER: u32 = 3;
const VARIABLE: u32 = 4;
const PARAMETER: u32 = 5;
const FUNCTION: u32 = 6;
const TYPE: u32 = 7;
const ENUM: u32 = 8;
const ENUM_MEMBER: u32 = 9;
const PROPERTY: u32 = 10;
const LABEL: u32 = 11;

const DECLARATION: u32 = 1 << 0;
const DEFINITION: u32 = 1 << 1;
const READONLY: u32 = 1 << 2;

pub fn legend_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::TYPE,
        SemanticTokenType::ENUM,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::new("label"),
    ]
}

pub fn legend_modifiers() -> Vec<SemanticTokenModifier> {
    vec![
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::DEFINITION,
        SemanticTokenModifier::READONLY,
    ]
}

#[derive(Clone)]
struct RawToken {
    end: usize,
    token_type: u32,
    modifiers: u32,
}

fn insert(tokens: &mut BTreeMap<usize, RawToken>, span: &std::ops::Range<usize>, token_type: u32, modifiers: u32) {
    if span.start < span.end {
        tokens.insert(
            span.start,
            RawToken {
                end: span.end,
                token_type,
                modifiers,
            },
        );
    }
}

pub fn get_semantic_tokens(ast: &Ast, visitor: &SemanticVisitor, rope: &Rope, source: &str, workspace: &Workspace) -> Vec<SemanticToken> {
    let mut raw = lexical_tokens(ast, source, workspace);
    reference_tokens(ast, visitor, &mut raw);

    let mut collector = AstTokens { visitor, tokens: &mut raw };
    ast.visit(&mut collector);

    encode(raw, rope)
}

fn lexical_tokens(ast: &Ast, source: &str, workspace: &Workspace) -> BTreeMap<usize, RawToken> {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut lexer = Lexer::new(ast.file_name.clone(), workspace, source, Encoding::Utf8, errors);
    let mut tokens = BTreeMap::new();
    while let Some(token) = lexer.next_token() {
        let span = lexer.span();
        let kind = if KEYWORDS.iter().any(|keyword| keyword.token == token) {
            Some((KEYWORD, 0))
        } else {
            match token {
                Token::Comment(_, _) | Token::UseFuncs(_, _) | Token::Define(_, _, _) => Some((COMMENT, 0)),
                Token::Const(Constant::String(_)) => Some((STRING, 0)),
                Token::Const(Constant::Integer(_, _) | Constant::Unsigned(_, _) | Constant::Money(_) | Constant::Double(_)) => Some((NUMBER, 0)),
                Token::Const(Constant::Boolean(_) | Constant::Builtin(_)) => Some((VARIABLE, READONLY)),
                Token::Label(_) => Some((LABEL, DEFINITION)),
                _ => None,
            }
        };
        if let Some((token_type, modifiers)) = kind {
            insert(&mut tokens, &span, token_type, modifiers);
        }
    }
    tokens
}

fn reference_tokens(ast: &Ast, visitor: &SemanticVisitor, tokens: &mut BTreeMap<usize, RawToken>) {
    for (reference_type, reference) in &visitor.references {
        let token_type = match reference_type {
            ReferenceType::PredefinedFunc(_) | ReferenceType::Function(_) => FUNCTION,
            ReferenceType::PredefinedProc(_) | ReferenceType::Procedure(_) => FUNCTION,
            ReferenceType::Label(_) => LABEL,
            ReferenceType::Variable(_) => VARIABLE,
        };
        if let Some((path, declaration)) = &reference.declaration
            && same_file(path, &ast.file_name)
        {
            insert(tokens, &declaration.span, token_type, DECLARATION);
        }
        if let Some((path, implementation)) = &reference.implementation
            && same_file(path, &ast.file_name)
        {
            insert(tokens, &implementation.span, token_type, DEFINITION);
        }
        for (path, usage) in reference.usages.iter().chain(&reference.return_types) {
            if same_file(path, &ast.file_name) {
                insert(tokens, &usage.span, token_type, 0);
            }
        }
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    left == right
}

struct AstTokens<'a> {
    visitor: &'a SemanticVisitor,
    tokens: &'a mut BTreeMap<usize, RawToken>,
}

impl AstVisitor<()> for AstTokens<'_> {
    fn visit_parameter_specifier(&mut self, parameter: &ParameterSpecifier) {
        match parameter {
            ParameterSpecifier::Variable(parameter) => {
                insert(self.tokens, &parameter.get_type_token().span, TYPE, 0);
                if let Some(variable) = parameter.get_variable() {
                    insert(self.tokens, &variable.get_identifier_token().span, PARAMETER, DECLARATION);
                }
            }
            ParameterSpecifier::Function(parameter) => {
                insert(self.tokens, &parameter.get_identifier_token().span, FUNCTION, DECLARATION);
                insert(self.tokens, &parameter.get_return_type_token().span, TYPE, 0);
                for nested in parameter.get_parameters() {
                    nested.visit(self);
                }
            }
            ParameterSpecifier::Procedure(parameter) => {
                insert(self.tokens, &parameter.get_identifier_token().span, FUNCTION, DECLARATION);
                for nested in parameter.get_parameters() {
                    nested.visit(self);
                }
            }
        }
    }

    fn visit_const_declaration_statement(&mut self, declaration: &ConstDeclarationStatement) {
        insert(self.tokens, &declaration.get_type_token().span, TYPE, 0);
        insert(self.tokens, &declaration.get_identifier_token().span, VARIABLE, DECLARATION | READONLY);
        declaration.get_value().visit(self);
    }

    fn visit_variable_declaration_statement(&mut self, declaration: &VariableDeclarationStatement) {
        insert(self.tokens, &declaration.get_type_token().span, TYPE, 0);
        icy_board_engine::ast::walk_variable_declaration_statement(self, declaration);
    }

    fn visit_type_declaration(&mut self, declaration: &TypeDeclarationAstNode) {
        insert(self.tokens, &declaration.get_identifier_token().span, TYPE, DECLARATION);
        for field in declaration.get_fields() {
            insert(self.tokens, &field.get_type_token().span, TYPE, 0);
            insert(self.tokens, &field.get_specifier().get_identifier_token().span, PROPERTY, DECLARATION);
        }
    }

    fn visit_enum_declaration(&mut self, declaration: &EnumDeclarationAstNode) {
        insert(self.tokens, &declaration.get_identifier_token().span, ENUM, DECLARATION);
        for variant in declaration.get_variants() {
            insert(self.tokens, &variant.get_identifier_token().span, ENUM_MEMBER, DECLARATION | READONLY);
            if let Some(value) = variant.get_explicit_value() {
                value.visit(self);
            }
        }
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) {
        let start = member.get_identifier_token().span.start;
        let token_type = if self.visitor.user_type_lookup.contains_key(&start) {
            PROPERTY
        } else if let Expression::Identifier(base) = member.get_expression() {
            if self.visitor.type_registry.get_enum(base.get_identifier()).is_some() {
                ENUM_MEMBER
            } else {
                PROPERTY
            }
        } else {
            PROPERTY
        };
        insert(self.tokens, &member.get_identifier_token().span, token_type, 0);
        member.get_expression().visit(self);
    }
}

fn encode(raw: BTreeMap<usize, RawToken>, rope: &Rope) -> Vec<SemanticToken> {
    let mut absolute = Vec::new();
    for (start, token) in raw {
        if token.end > rope.len_chars() {
            continue;
        }
        let mut segment_start = start;
        for offset in start..token.end {
            if matches!(rope.char(offset), '\n' | '\r') {
                push_segment(&mut absolute, rope, segment_start, offset, &token);
                segment_start = offset + 1;
            }
        }
        push_segment(&mut absolute, rope, segment_start, token.end, &token);
    }
    absolute.sort_by_key(|(line, start, ..)| (*line, *start));

    let mut result = Vec::new();
    let (mut previous_line, mut previous_start) = (0, 0);
    for (line, start, length, token_type, modifiers) in absolute {
        let delta_line = line - previous_line;
        let delta_start = if delta_line == 0 { start - previous_start } else { start };
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });
        previous_line = line;
        previous_start = start;
    }
    result
}

fn push_segment(output: &mut Vec<(u32, u32, u32, u32, u32)>, rope: &Rope, start: usize, end: usize, token: &RawToken) {
    if start >= end {
        return;
    }
    let (Some(start), Some(end)) = (offset_to_position(start, rope), offset_to_position(end, rope)) else {
        return;
    };
    if start.line == end.line && end.character > start.character {
        output.push((start.line, start.character, end.character - start.character, token.token_type, token.modifiers));
    }
}
