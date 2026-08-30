use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{
        Ast, AstVisitor, ConstDeclarationStatement, Constant, EnumDeclarationAstNode, Expression, FunctionDeclarationAstNode, FunctionImplementation,
        MemberReferenceExpression, ParameterSpecifier, TypeDeclarationAstNode, VariableDeclarationStatement, walk_function_declaration,
        walk_function_implementation,
    },
    compiler::workspace::Workspace,
    executable::VariableType,
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
const CONSTANT: u32 = 12;
const DIRECTIVE: u32 = 13;

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
        SemanticTokenType::new("constant"),
        SemanticTokenType::new("directive"),
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
    preprocessor_tokens(source, &mut raw);
    let parameter_spans = parameter_spans(ast);
    reference_tokens(ast, visitor, &parameter_spans, &mut raw);

    let mut collector = AstTokens { visitor, tokens: &mut raw };
    ast.visit(&mut collector);

    encode(raw, rope)
}

fn preprocessor_tokens(source: &str, tokens: &mut BTreeMap<usize, RawToken>) {
    const DIRECTIVES: &[&str] = &["LANGVERSION", "DEFINE", "IF", "ELSEIF", "ELIF", "ELSE", "ENDIF", "USEFUNCS"];
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let line_without_lf = line.strip_suffix('\n').unwrap_or(line);
        let line_without_newline = line_without_lf.strip_suffix('\r').unwrap_or(line_without_lf);
        let leading = line_without_newline.chars().take_while(|ch| ch.is_whitespace()).count();
        let rest: String = line_without_newline.chars().skip(leading).collect();
        let marker_start = offset + leading;

        if let Some(after_marker) = rest.strip_prefix(";$") {
            let word: String = after_marker.chars().take_while(|ch| ch.is_ascii_alphabetic()).collect();
            if DIRECTIVES.iter().any(|directive| directive.eq_ignore_ascii_case(&word)) {
                let marker_end = marker_start + 2 + word.chars().count();
                remove_covering_comment(tokens, marker_start);
                insert(tokens, &(marker_start..marker_end), DIRECTIVE, 0);
                highlight_preprocessor_names(tokens, after_marker, marker_start + 2, word.chars().count());
            }
        }
        for (marker_byte, _) in line_without_newline.match_indices(";#") {
            if line_without_newline[..marker_byte].chars().filter(|ch| *ch == '"').count() % 2 != 0 {
                continue;
            }
            let after_marker = &line_without_newline[marker_byte + 2..];
            let name: String = after_marker.chars().take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_').collect();
            if name.is_empty() {
                continue;
            }
            let marker_start = offset + line_without_newline[..marker_byte].chars().count();
            let marker_end = marker_start + 2 + name.chars().count();
            remove_covering_comment(tokens, marker_start);
            insert(tokens, &(marker_start..marker_start + 2), DIRECTIVE, 0);
            insert(tokens, &(marker_start + 2..marker_end), CONSTANT, READONLY);
        }
        offset += line.chars().count();
    }
}

fn remove_covering_comment(tokens: &mut BTreeMap<usize, RawToken>, offset: usize) {
    let covering = tokens
        .range(..=offset)
        .next_back()
        .filter(|(_, token)| token.token_type == COMMENT && token.end > offset)
        .map(|(start, _)| *start);
    if let Some(start) = covering {
        tokens.remove(&start);
    }
}

fn highlight_preprocessor_names(tokens: &mut BTreeMap<usize, RawToken>, text: &str, text_start: usize, directive_len: usize) {
    let mut quoted = false;
    let mut start = None;
    for (byte, ch) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        if byte < directive_len {
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        }
        let is_name = !quoted && (ch.is_ascii_alphanumeric() || ch == '_');
        if is_name && start.is_none() && (ch.is_ascii_alphabetic() || ch == '_') {
            start = Some(byte);
        } else if !is_name
            && let Some(name_start) = start.take()
        {
            let name = &text[name_start..byte];
            if !matches!(name.to_ascii_uppercase().as_str(), "AND" | "OR" | "NOT" | "TRUE" | "FALSE") {
                let start = text_start + text[..name_start].chars().count();
                insert(tokens, &(start..start + name.chars().count()), CONSTANT, READONLY);
            }
        }
    }
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
                Token::Const(Constant::Boolean(_) | Constant::Builtin(_)) => Some((CONSTANT, READONLY)),
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

fn reference_tokens(ast: &Ast, visitor: &SemanticVisitor, parameter_spans: &HashSet<usize>, tokens: &mut BTreeMap<usize, RawToken>) {
    for (reference_type, reference) in &visitor.references {
        let is_parameter = reference
            .declaration
            .as_ref()
            .into_iter()
            .chain(reference.implementation.as_ref())
            .any(|(path, identifier)| same_file(path, &ast.file_name) && parameter_spans.contains(&identifier.span.start));
        let token_type = match reference_type {
            ReferenceType::PredefinedFunc(_) | ReferenceType::Function(_) => FUNCTION,
            ReferenceType::PredefinedProc(_) | ReferenceType::Procedure(_) => FUNCTION,
            ReferenceType::Label(_) => LABEL,
            ReferenceType::Variable(_) if is_parameter => PARAMETER,
            ReferenceType::Variable(_) => VARIABLE,
            ReferenceType::Constant(_) => CONSTANT,
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

fn parameter_spans(ast: &Ast) -> HashSet<usize> {
    #[derive(Default)]
    struct ParameterSpans(HashSet<usize>);

    impl AstVisitor<()> for ParameterSpans {
        fn visit_parameter_specifier(&mut self, parameter: &ParameterSpecifier) {
            match parameter {
                ParameterSpecifier::Variable(parameter) => {
                    if let Some(variable) = parameter.get_variable() {
                        self.0.insert(variable.get_identifier_token().span.start);
                    }
                }
                ParameterSpecifier::Function(parameter) => {
                    for nested in parameter.get_parameters() {
                        nested.visit(self);
                    }
                }
                ParameterSpecifier::Procedure(parameter) => {
                    for nested in parameter.get_parameters() {
                        nested.visit(self);
                    }
                }
            }
        }
    }

    let mut spans = ParameterSpans::default();
    ast.visit(&mut spans);
    spans.0
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
        insert(self.tokens, &declaration.get_identifier_token().span, CONSTANT, DECLARATION | READONLY);
        declaration.get_value().visit(self);
    }

    fn visit_variable_declaration_statement(&mut self, declaration: &VariableDeclarationStatement) {
        insert(self.tokens, &declaration.get_type_token().span, TYPE, 0);
        icy_board_engine::ast::walk_variable_declaration_statement(self, declaration);
    }

    fn visit_function_declaration(&mut self, declaration: &FunctionDeclarationAstNode) {
        insert(self.tokens, &declaration.get_return_type_token().span, TYPE, 0);
        walk_function_declaration(self, declaration);
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) {
        insert(self.tokens, &function.get_return_type_token().span, TYPE, 0);
        walk_function_implementation(self, function);
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
        let direct_receiver = if let Expression::Identifier(base) = member.get_expression() {
            self.visitor.type_registry.get_type(base.get_identifier())
        } else {
            None
        };
        let receiver_type_id = self.visitor.user_type_lookup.get(&start).copied().or_else(|| match direct_receiver {
            Some(VariableType::UserData(type_id)) => Some(type_id),
            _ => None,
        });
        let token_type = if let Expression::Identifier(base) = member.get_expression()
            && self.visitor.type_registry.get_enum(base.get_identifier()).is_some()
        {
            ENUM_MEMBER
        } else if let Some(type_id) = receiver_type_id {
            if let Some(definition) = self.visitor.type_registry.get_type_from_id(type_id)
                && (definition.functions.contains_key(member.get_identifier()) || definition.procedures.contains_key(member.get_identifier()))
            {
                FUNCTION
            } else {
                PROPERTY
            }
        } else {
            PROPERTY
        };
        insert(self.tokens, &member.get_identifier_token().span, token_type, 0);

        if let Expression::Identifier(base) = member.get_expression() {
            let span = &base.get_identifier_token().span;
            // A referenced variable already has a semantic token. With no such
            // reference, a name used as a member receiver denotes its type.
            if !self.tokens.contains_key(&span.start) {
                if self.visitor.type_registry.get_enum(base.get_identifier()).is_some() {
                    insert(self.tokens, span, ENUM, 0);
                } else if self.visitor.type_registry.get_type(base.get_identifier()).is_some() {
                    insert(self.tokens, span, TYPE, 0);
                }
            }
        }
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
