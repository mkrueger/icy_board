use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        AstNode, BlockStatement, CommentAstNode, Constant, DimensionSpecifier, EnumDeclarationAstNode, EnumVariantSpecifier, FunctionDeclarationAstNode,
        FunctionImplementation, FunctionParameterSpecifier, ImportDeclaration, ModuleDeclaration, ParameterSpecifier, ProcedureDeclarationAstNode,
        ProcedureImplementation, ProcedureParameterSpecifier, Statement, TypeDeclarationAstNode, TypeFieldSpecifier, VariableParameterSpecifier,
        VariableSpecifier, Visibility, VisibilitySection, const_value,
    },
    executable::{FunctionDefinition, StatementDefinition, VariableType},
};

use self::lexer::{Lexer, Spanned, Token};
use unicase::Ascii;

mod diagnostics;
mod errors;
mod expression;
pub mod lexer;
pub mod pre_processor_expr_visitor;
pub mod statements;
mod token_stream;
mod top_level;
mod type_registry;
mod types;

pub use diagnostics::{Encoding, ErrorContainer, ErrorReporter, load_with_encoding};
pub use errors::{ParserErrorType, ParserWarningType};
pub use top_level::{parse_ast, parse_ast_with_predeclared_types, preparse_type_declarations};
pub use type_registry::*;
pub use types::{built_in_type, built_in_type_names};

#[cfg(test)]
mod api_dump;
#[cfg(test)]
mod declaration_tests;
#[cfg(test)]
mod expr_tests;
#[cfg(test)]
mod lexer_tests;
#[cfg(test)]
mod statement_tests;

pub struct Parser<'a> {
    pub error_reporter: Arc<Mutex<ErrorReporter>>,

    pub type_registry: &'a UserTypeRegistry,
    lang_version: u16,
    pub require_user_variables: bool,

    cur_token: Option<Spanned<Token>>,
    lookahead_token: Option<Spanned<Token>>,
    lex: Lexer,

    // parser state
    use_funcs: bool,
    parsed_begin: bool,
    parsed_block: bool,
    got_statement: bool,
    got_funcs: bool,
    in_function: bool,
    types_predeclared: bool,
    module: Option<ModuleDeclaration>,
    imports: Vec<ImportDeclaration>,
    dependency_imports: HashMap<unicase::Ascii<String>, unicase::Ascii<String>>,
    in_module: bool,
    expression_depth: usize,
    statement_depth: usize,
}
const MAX_EXPRESSION_DEPTH: usize = 64;
const MAX_STATEMENT_DEPTH: usize = 64;
/// A left associative operator chain is read in a loop, so it costs the parser no recursion
/// while the tree it builds is as deep as the chain is long. Everything that later walks that
/// tree does recurse, so the chain gets a bound far above what a source realistically writes.
const MAX_OPERATOR_CHAIN: usize = 1024;

impl<'a> Parser<'a> {
    fn current_module_name(&self) -> Option<&unicase::Ascii<String>> {
        self.module.as_ref().map(ModuleDeclaration::name)
    }

    fn declared_type_name(&self, name: &unicase::Ascii<String>) -> unicase::Ascii<String> {
        self.current_module_name()
            .map_or_else(|| name.clone(), |module| UserTypeRegistry::module_type_name(module, name))
    }

    fn parse_ast_node(&mut self) -> Option<AstNode> {
        let cur_token = self.cur_token.clone()?;
        if self.lang_version >= 400
            && let Token::Identifier(keyword) = &cur_token.token
        {
            if keyword.eq_ignore_ascii_case("MODULE") && matches!(self.peek_after_current(1).as_slice(), [Some(Token::Identifier(_))]) {
                self.parse_module_start();
                return None;
            }
            if keyword.eq_ignore_ascii_case("ENDMODULE")
                && self.module.as_ref().is_some_and(|module| !module.is_implicit())
                && matches!(self.peek_after_current(1).as_slice(), [Some(Token::Eol | Token::Comment(_, _)) | None])
            {
                self.parse_module_end();
                return None;
            }
            if keyword.eq_ignore_ascii_case("IMPORT")
                && matches!(self.peek_after_current(2).as_slice(), [Some(Token::Identifier(_)), Some(Token::Identifier(as_name))] if as_name.eq_ignore_ascii_case("AS"))
            {
                self.parse_import();
                return None;
            }
            if self.in_module
                && (keyword.eq_ignore_ascii_case("PUBLIC") || keyword.eq_ignore_ascii_case("PRIVATE"))
                && matches!(self.peek_after_current(1).as_slice(), [Some(Token::Eol | Token::Comment(_, _)) | None])
            {
                self.parse_visibility_section(keyword.eq_ignore_ascii_case("PUBLIC"));
                return None;
            }
        }
        match cur_token.token {
            Token::Eol => {
                self.next_token();
            }
            Token::Function => {
                if let Some(func) = self.parse_function() {
                    self.got_funcs = true;
                    return Some(AstNode::Function(func));
                }
            }
            Token::Procedure => {
                if let Some(func) = self.parse_procedure() {
                    self.got_funcs = true;
                    return Some(AstNode::Procedure(func));
                }
            }
            Token::Declare => {
                if let Some(decl) = self.parse_declaration() {
                    return Some(decl);
                }
            }
            Token::Type => {
                let original_reporter = if self.types_predeclared {
                    Some(std::mem::replace(&mut self.error_reporter, Arc::new(Mutex::new(ErrorReporter::default()))))
                } else {
                    None
                };
                let declaration = self.parse_type_declaration();
                if let Some(original_reporter) = original_reporter {
                    self.error_reporter = original_reporter;
                }
                if let Some(decl) = declaration {
                    return Some(AstNode::TypeDeclaration(decl));
                }
            }
            Token::Enum => {
                let original_reporter = if self.types_predeclared {
                    Some(std::mem::replace(&mut self.error_reporter, Arc::new(Mutex::new(ErrorReporter::default()))))
                } else {
                    None
                };
                let declaration = self.parse_enum_declaration();
                if let Some(original_reporter) = original_reporter {
                    self.error_reporter = original_reporter;
                }
                if let Some(decl) = declaration {
                    return Some(AstNode::EnumDeclaration(decl));
                }
            }
            Token::Begin => {
                if self.parsed_block {
                    self.report_error(cur_token.span.clone(), ParserErrorType::BlockAlreadyDefined);
                    return None;
                }
                let (begin_token, statements, end_token) = self.parse_block_body()?;
                self.parsed_block = true;
                self.got_statement = true;
                return Some(AstNode::Main(BlockStatement::new(begin_token, statements, end_token)));
            }
            Token::UseFuncs(_, _) => {
                if self.use_funcs {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_warning(self.lex.span(), ParserWarningType::UsefuncsAlreadySet);
                }
                if self.got_statement {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(self.lex.span(), ParserErrorType::UsefuncAfterStatement);
                    self.next_token();
                    return None;
                }
                self.use_funcs = true;
                let cmt = self.save_spanned_token();
                self.next_token();
                return Some(AstNode::TopLevelStatement(Statement::Comment(CommentAstNode::new(cmt))));
            }
            _ => {
                let stmt = self.parse_statement();
                if let Some(stmt) = stmt {
                    if let Statement::Label(label) = &stmt
                        && *label.get_label() == *statements::BEGIN_LABEL
                    {
                        self.parsed_begin = true;
                    }

                    if self.parsed_block || (self.use_funcs && !self.parsed_begin) {
                        if matches!(stmt, Statement::Comment(_) | Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_)) {
                            return Some(AstNode::TopLevelStatement(stmt));
                        }

                        self.report_error(self.lex.span(), ParserErrorType::NoStatementsAllowedOutsideBlock);
                        return None;
                    }
                    if self.got_funcs && !self.use_funcs && !self.in_module {
                        if matches!(stmt, Statement::Comment(_)) {
                            return Some(AstNode::TopLevelStatement(stmt));
                        }
                        self.report_error(stmt.get_span(), ParserErrorType::NoStatementsAfterFunctions);
                        return None;
                    }
                    if !self.got_statement && !matches!(stmt, Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_) | Statement::Comment(_)) {
                        let mut main_block = vec![stmt];
                        while let Some(cur_token) = &self.cur_token {
                            if cur_token.token == Token::Function || cur_token.token == Token::Procedure {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                main_block.push(stmt);
                            }
                        }
                        self.got_statement = true;
                        return Some(AstNode::Main(BlockStatement::empty(main_block)));
                    }
                    return Some(AstNode::TopLevelStatement(stmt));
                }
            }
        }
        None
    }

    fn parse_module_start(&mut self) {
        let module_token = self.save_spanned_token();
        if self.module.as_ref().is_some_and(|module| !module.is_implicit()) {
            self.report_error(module_token.span, ParserErrorType::ModuleAlreadyDefined);
            return;
        }
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.save_token_span(), ParserErrorType::ModuleNameExpected);
            return;
        };
        let name_token = self.save_spanned_token();
        self.next_token();
        self.check_eol();
        self.module = Some(ModuleDeclaration {
            module_token,
            name_token,
            endmodule_token: Spanned::new(Token::Identifier(Ascii::new("ENDMODULE".to_string())), 0..0),
            visibility_sections: Vec::new(),
            implicit: false,
        });
        self.in_module = true;
    }

    fn parse_module_end(&mut self) {
        let token = self.save_spanned_token();
        if !self.in_module {
            self.report_error(token.span, ParserErrorType::EndModuleExpected);
            return;
        }
        if let Some(module) = &mut self.module {
            module.endmodule_token = token;
        }
        self.in_module = false;
        self.next_token();
        self.check_eol();
    }

    fn parse_visibility_section(&mut self, public: bool) {
        let token = self.save_spanned_token();
        if !self.in_module {
            self.report_error(
                token.span,
                ParserErrorType::VisibilityOutsideModule(if public { "PUBLIC" } else { "PRIVATE" }.to_string()),
            );
            return;
        }
        self.next_token();
        if !self.check_eol() {
            return;
        }
        self.module.as_mut().unwrap().visibility_sections.push(VisibilitySection {
            token,
            visibility: if public { Visibility::Public } else { Visibility::Private },
        });
    }

    fn parse_import(&mut self) {
        let import_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.save_token_span(), ParserErrorType::InvalidImport);
            return;
        };
        let mut module_token = self.save_spanned_token();
        if let Token::Identifier(name) = &module_token.token
            && let Some(module) = self.dependency_imports.get(name)
        {
            module_token.token = Token::Identifier(module.clone());
        }
        self.next_token();
        let Some(Token::Identifier(as_name)) = self.get_cur_token() else {
            self.report_error(self.save_token_span(), ParserErrorType::InvalidImport);
            return;
        };
        if !as_name.eq_ignore_ascii_case("AS") {
            self.report_error(self.save_token_span(), ParserErrorType::InvalidImport);
            return;
        }
        let as_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.save_token_span(), ParserErrorType::InvalidImport);
            return;
        };
        let alias_token = self.save_spanned_token();
        self.next_token();
        if self.check_eol() {
            self.imports.push(ImportDeclaration {
                import_token,
                module_token,
                as_token,
                alias_token,
            });
        }
    }

    /// Parses `TYPE <name> ... ENDTYPE` and registers the record so later
    /// declarations can name it as a type.
    fn parse_type_declaration(&mut self) -> Option<TypeDeclarationAstNode> {
        let type_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(name)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        let declared_name = self.declared_type_name(&name);
        let type_already_declared = self.type_registry.get_type(&declared_name).is_some() || built_in_type(&name, self.lang_version).is_some();
        if !self.types_predeclared && type_already_declared {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeAlreadyDeclared(name.clone()));
        }

        let mut fields: Vec<TypeFieldSpecifier> = Vec::new();
        let mut field_names: Vec<Ascii<String>> = Vec::new();

        let endtype_token = loop {
            while matches!(self.get_cur_token(), Some(Token::Eol)) || matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
                self.next_token();
            }
            match self.get_cur_token() {
                Some(Token::EndType) => {
                    let token = self.save_spanned_token();
                    self.next_token();
                    break token;
                }
                None => {
                    self.report_error(self.lex.span(), ParserErrorType::EndTypeExpected);
                    return None;
                }
                _ => {}
            }

            // A record can't contain itself, that has no finite layout.
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == name
            {
                self.report_error(self.save_token_span(), ParserErrorType::TypeUsedInItself(name.clone()));
                continue;
            }

            let Some((field_type, field_type_token)) = self.parse_variable_type() else {
                self.report_error(self.save_token_span(), ParserErrorType::InvalidToken(self.save_token()));
                continue;
            };
            if !self.types_predeclared && matches!(field_type, VariableType::UserData(id) if !is_user_declared_type(id)) {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_error(field_type_token.span.clone(), ParserErrorType::TypeFieldBoardObjectNotSupported(field_type));
            }
            while let Some(specifier) = self.parse_var_info(false) {
                let field_name = specifier.get_identifier().clone();
                if specifier.get_dimensions().iter().any(|dimension| dimension.get_dimension() > u16::MAX as usize) {
                    self.error_reporter.lock().unwrap().report_error(
                        specifier.get_identifier_token().span.clone(),
                        ParserErrorType::TypeFieldDimensionTooLarge(field_name.clone()),
                    );
                }
                if specifier.get_initalizer().is_some() {
                    self.error_reporter.lock().unwrap().report_error(
                        specifier.get_identifier_token().span.clone(),
                        ParserErrorType::TypeFieldInitializerNotSupported(field_name.clone()),
                    );
                }
                if field_names.contains(&field_name) {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(specifier.get_identifier_token().span.clone(), ParserErrorType::FieldAlreadyDeclared(field_name));
                } else {
                    field_names.push(field_name);
                }
                fields.push(TypeFieldSpecifier::new(field_type_token.clone(), field_type, specifier));

                if matches!(self.get_cur_token(), Some(Token::Comma)) {
                    self.next_token();
                    continue;
                }
                break;
            }
        };

        if fields.is_empty() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeNeedsAField);
            return None;
        }

        if fields.len() > MAX_TYPE_FIELDS {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyFields(MAX_TYPE_FIELDS));
            return None;
        }

        let field_layout = fields
            .iter()
            .map(|field| {
                let dimensions = field.get_specifier().get_dimensions();
                (
                    field.get_identifier().clone(),
                    crate::executable::RecordField {
                        variable_type: field.get_variable_type(),
                        dim: dimensions.len() as u8,
                        vector_size: field.get_specifier().get_vector_size() as u16,
                        matrix_size: field.get_specifier().get_matrix_size() as u16,
                        cube_size: field.get_specifier().get_cube_size() as u16,
                    },
                )
            })
            .collect();
        if !self.types_predeclared && !type_already_declared && self.type_registry.declare_user_type(declared_name, field_layout).is_none() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyTypes(MAX_USER_TYPES));
            return None;
        }

        Some(TypeDeclarationAstNode::new(type_token, identifier_token, fields, endtype_token))
    }

    fn parse_enum_declaration(&mut self) -> Option<EnumDeclarationAstNode> {
        let enum_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(name)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        let declared_name = self.declared_type_name(&name);
        let type_already_declared = self.type_registry.get_type(&declared_name).is_some() || built_in_type(&name, self.lang_version).is_some();
        if !self.types_predeclared && type_already_declared {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TypeAlreadyDeclared(name.clone()));
        }

        let mut variants = Vec::new();
        let mut names = Vec::new();
        let mut next_value = 0i32;
        let endenum_token = loop {
            while matches!(self.get_cur_token(), Some(Token::Eol | Token::Comma)) || matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
                self.next_token();
            }
            match self.get_cur_token() {
                Some(Token::EndEnum) => {
                    let token = self.save_spanned_token();
                    self.next_token();
                    break token;
                }
                None => {
                    self.report_error(self.lex.span(), ParserErrorType::EndEnumExpected);
                    return None;
                }
                _ => {}
            }

            let Some(Token::Identifier(variant_name)) = self.get_cur_token() else {
                self.report_error(self.save_token_span(), ParserErrorType::IdentifierExpected(self.save_token()));
                continue;
            };
            let variant_token = self.save_spanned_token();
            self.next_token();

            if names.contains(&variant_name) {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_error(variant_token.span.clone(), ParserErrorType::EnumMemberAlreadyDeclared(variant_name.clone()));
            } else {
                names.push(variant_name);
            }

            let (eq_token, explicit_value, value) = if self.get_cur_token() == Some(Token::Eq) {
                let eq = self.save_spanned_token();
                self.next_token();
                let Some(expr) = self.parse_expression() else {
                    self.report_error(self.save_token_span(), ParserErrorType::EnumValueExpected);
                    continue;
                };
                let Some(value) = const_value(&expr, &|_| None).map(|value| value.as_int()) else {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(expr.get_span(), ParserErrorType::EnumValueExpected);
                    continue;
                };
                (Some(eq), Some(expr), value)
            } else {
                (None, None, next_value)
            };
            next_value = value.saturating_add(1);
            variants.push(EnumVariantSpecifier::new(variant_token, eq_token, value, explicit_value));
        };

        if variants.is_empty() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::EnumNeedsAMember);
            return None;
        }

        let layout = variants.iter().map(|variant| (variant.get_identifier().clone(), variant.get_value())).collect();
        if !self.types_predeclared && !type_already_declared && self.type_registry.declare_enum(declared_name, layout).is_none() {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(identifier_token.span.clone(), ParserErrorType::TooManyTypes(MAX_USER_TYPES));
            return None;
        }

        Some(EnumDeclarationAstNode::new(enum_token, identifier_token, variants, endenum_token))
    }

    fn parse_function_parameter_specifier(&mut self) -> ParameterSpecifier {
        let func_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters: Vec<ParameterSpecifier> = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
            }

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }

                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                var_token = Some(self.save_spanned_token());
                self.next_token();
            }
            if let Some((var_type, type_token)) = self.parse_variable_type() {
                let info = self.parse_var_info(false);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        let Some((return_type, return_type_token)) = self.parse_variable_type() else {
            self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, func_token, VariableType::Integer, None));
        };

        ParameterSpecifier::Function(FunctionParameterSpecifier::new(
            func_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token,
            return_type,
        ))
    }
    fn parse_procedure_parameter_specifier(&mut self) -> ParameterSpecifier {
        let proc_token = self.save_spanned_token();
        self.next_token();
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters: Vec<ParameterSpecifier> = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
            }

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }

                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            let mut var_token = None;
            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                var_token = Some(self.save_spanned_token());
                self.next_token();
            }
            if let Some((var_type, type_token)) = self.parse_variable_type() {
                let info = self.parse_var_info(false);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, proc_token, VariableType::Integer, None));
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        ParameterSpecifier::Procedure(ProcedureParameterSpecifier::new(
            proc_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
        ))
    }
}

impl Parser<'_> {
    fn parse_dynamic_array_rank(&mut self) -> u8 {
        if self.lang_version < 400 || self.get_cur_token() != Some(Token::LBracket) {
            return 0;
        }
        self.next_token();
        let mut rank = 1usize;
        while self.get_cur_token() == Some(Token::Comma) {
            rank += 1;
            self.next_token();
        }
        if rank > 3 {
            self.report_error(self.lex.span(), ParserErrorType::TooManyDimensions(rank));
            return 0;
        }
        if self.get_cur_token() != Some(Token::RBracket) {
            self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
            return 0;
        }
        self.next_token();
        rank as u8
    }

    pub fn get_variable_type(&self) -> Option<VariableType> {
        if let Some(token) = &self.cur_token {
            if let Token::Identifier(id) = &token.token {
                if let Some(vt) = built_in_type(id, self.lang_version) {
                    return Some(vt);
                }
                if self.lang_version >= FIRST_BOARD_OBJECT_LANGUAGE_VERSION
                    && let Some(vt) = self.type_registry.get_board_object(id)
                {
                    return Some(vt);
                }
                // An enum is a type from 350 on, so this is not gated with the objects.
                if let Some(vt) = self.type_registry.get_module_declared_type(self.current_module_name(), id) {
                    return Some(vt);
                }
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_variable_type(&mut self) -> Option<(VariableType, Spanned<Token>)> {
        let lex = self.lex.clone();
        let cur_token = self.cur_token.clone();
        let lookahead_token = self.lookahead_token.clone();
        let result = self.parse_variable_type_inner();
        if result.is_none() {
            self.lex = lex;
            self.cur_token = cur_token;
            self.lookahead_token = lookahead_token;
        }
        result
    }

    fn parse_variable_type_inner(&mut self) -> Option<(VariableType, Spanned<Token>)> {
        if let Some(variable_type) = self.get_variable_type() {
            let token = self.save_spanned_token();
            self.next_token();
            return Some((variable_type, token));
        }

        let Some(Token::Identifier(alias)) = self.get_cur_token() else { return None };
        let module = self.imports.iter().find(|import| import.alias() == &alias)?.module_name().clone();
        let start = self.save_token_span().start;
        self.next_token();
        if self.get_cur_token() != Some(Token::Dot) {
            return None;
        }
        self.next_token();
        let Some(Token::Identifier(name)) = self.get_cur_token() else { return None };
        let qualified = UserTypeRegistry::module_type_name(&module, &name);
        let variable_type = self.type_registry.get_declared_type(&qualified)?;
        let end = self.save_token_span().end;
        self.next_token();
        Some((variable_type, Spanned::new(Token::Identifier(qualified), start..end)))
    }

    /// Returns the parse var info of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_var_info(&mut self, can_be_empty: bool) -> Option<VariableSpecifier> {
        if can_be_empty && (matches!(self.get_cur_token(), Some(Token::Comma)) || matches!(self.get_cur_token(), Some(Token::RPar))) {
            return None;
        }
        let Some(Token::Identifier(_)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));
            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();
        let mut dimensions = Vec::new();
        let mut leftpar_token = None;
        let mut rightpar_token = None;
        let is_lpar = matches!(self.get_cur_token(), Some(Token::LPar));
        if is_lpar || matches!(self.get_cur_token(), Some(Token::LBracket)) {
            if self.lang_version >= 400 && is_lpar {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_warning(self.lex.span(), ParserWarningType::ArrayBracketsRequired);
            }
            leftpar_token = Some(self.save_spanned_token());
            self.next_token();
            if !is_lpar && matches!(self.get_cur_token(), Some(Token::RBracket) | Some(Token::Comma)) {
                dimensions.push(DimensionSpecifier::dynamic());
                while matches!(self.get_cur_token(), Some(Token::Comma)) {
                    self.next_token();
                    dimensions.push(DimensionSpecifier::dynamic());
                }
                if dimensions.len() > 3 {
                    self.report_error(self.lex.span(), ParserErrorType::TooManyDimensions(dimensions.len()));
                    return None;
                }
                if !matches!(self.get_cur_token(), Some(Token::RBracket)) {
                    self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                    return None;
                }
                rightpar_token = Some(self.save_spanned_token());
                self.next_token();
                // A dynamic array may still take an initializer, e.g. `STRING p[] = a.Split(",")`.
                if self.lang_version >= 350
                    && let Some(Token::Eq) = self.get_cur_token()
                {
                    let eq_token = self.save_spanned_token();
                    self.next_token();
                    let initializer = self.parse_expression();
                    return Some(VariableSpecifier::new(
                        identifier_token,
                        leftpar_token,
                        dimensions,
                        rightpar_token,
                        Some(eq_token),
                        initializer,
                    ));
                }
                return Some(VariableSpecifier::new(identifier_token, leftpar_token, dimensions, rightpar_token, None, None));
            }
            let Some(Token::Const(Constant::Integer(_, _))) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::NumberExpected(self.save_token()));
                return None;
            };
            dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
            self.next_token();

            while let Some(Token::Comma) = &self.get_cur_token() {
                self.next_token();
                let Some(Token::Const(Constant::Integer(_, _))) = self.get_cur_token() else {
                    self.report_error(self.lex.span(), ParserErrorType::NumberExpected(self.save_token()));

                    return None;
                };
                dimensions.push(DimensionSpecifier::new(self.save_spanned_token()));
                self.next_token();
            }

            if dimensions.len() > 3 {
                self.report_error(self.lex.span(), ParserErrorType::TooManyDimensions(dimensions.len()));

                return None;
            }

            if is_lpar && !matches!(self.get_cur_token(), Some(Token::RPar)) || !is_lpar && !matches!(self.get_cur_token(), Some(Token::RBracket)) {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));
                return None;
            }
            rightpar_token = Some(self.save_spanned_token());
            self.next_token();
        } else if self.lang_version >= 350
            && let Some(Token::Eq) = self.get_cur_token()
        {
            let eq_token = self.save_spanned_token();
            self.next_token();
            let initializer = self.parse_expression();
            return Some(VariableSpecifier::new(identifier_token, None, dimensions, None, Some(eq_token), initializer));
        }

        Some(VariableSpecifier::new(identifier_token, leftpar_token, dimensions, rightpar_token, None, None))
    }

    /// Returns the parse function declaration of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_declaration(&mut self) -> Option<AstNode> {
        let declare_token = self.save_spanned_token();
        self.next_token();

        let is_function = if Some(Token::Procedure) == self.get_cur_token() {
            false
        } else if Some(Token::Function) == self.get_cur_token() {
            true
        } else {
            self.report_error(self.lex.span(), ParserErrorType::InvalidDeclaration(self.save_token()));
            return None;
        };
        let func_or_proc_token = self.save_spanned_token();
        self.next_token();

        let Some(Token::Identifier(identifier)) = self.get_cur_token() else {
            self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

            return None;
        };
        let identifier_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
            return None;
        }

        let leftpar_token = self.save_spanned_token();
        self.next_token();

        let mut parameters = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                return None;
            }

            let mut var_token = None;

            if self.lang_version >= 350 {
                if let Some(Token::Function) = self.get_cur_token() {
                    parameters.push(self.parse_function_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
                if let Some(Token::Procedure) = self.get_cur_token() {
                    parameters.push(self.parse_procedure_parameter_specifier());
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                    continue;
                }
            }

            if let Some(Token::Identifier(id)) = self.get_cur_token()
                && id == Ascii::new("VAR".to_string())
            {
                if is_function {
                    self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                } else {
                    var_token = Some(self.save_spanned_token());
                }
                self.next_token();
            }
            if let Some((var_type, type_token)) = self.parse_variable_type() {
                let info = self.parse_var_info(true);
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                    var_token, type_token, var_type, info,
                )));
            } else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return None;
            }

            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
            }
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();
        if !is_function {
            self.check_eol();
            if StatementDefinition::get_statement_definition(&identifier).is_some() {
                self.report_error(identifier_token.span, ParserErrorType::StatementAlreadyDefined(self.save_token()));
                return None;
            }

            return Some(AstNode::ProcedureDeclaration(ProcedureDeclarationAstNode::new(
                declare_token,
                func_or_proc_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
            )));
        }
        if !FunctionDefinition::get_function_definitions(&identifier).is_empty() {
            self.report_error(identifier_token.span, ParserErrorType::FunctionAlreadyDefined(self.save_token()));
            return None;
        }
        let Some((return_type, return_type_token)) = self.parse_variable_type() else {
            self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
            return None;
        };
        let return_rank = self.parse_dynamic_array_rank();
        self.check_eol();
        Some(AstNode::FunctionDeclaration(FunctionDeclarationAstNode::new(
            declare_token,
            func_or_proc_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token,
            return_type,
            return_rank,
        )))
    }

    fn check_eol(&mut self) -> bool {
        if self.get_cur_token() != Some(Token::Eol) && !matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
            let err_token = self.save_spanned_token();
            self.next_token();
            self.report_error(err_token.span, ParserErrorType::EolExpected(err_token.token));
            false
        } else {
            true
        }
    }

    /// Returns the parse procedure of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_procedure(&mut self) -> Option<ProcedureImplementation> {
        if Some(Token::Procedure) == self.get_cur_token() {
            let procedure_token = self.save_spanned_token();
            self.next_token();

            let Some(Token::Identifier(_)) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                    return None;
                }
                if self.lang_version >= 350 {
                    if let Some(Token::Function) = self.get_cur_token() {
                        parameters.push(self.parse_function_parameter_specifier());
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                        }
                        continue;
                    }
                    if let Some(Token::Procedure) = self.get_cur_token() {
                        parameters.push(self.parse_procedure_parameter_specifier());
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                        }
                        continue;
                    }
                }

                let mut var_token = None;
                if let Some(Token::Identifier(id)) = self.get_cur_token()
                    && id.eq_ignore_ascii_case("VAR")
                {
                    var_token = Some(self.save_spanned_token());
                    self.next_token();
                }

                if let Some((var_type, type_token)) = self.parse_variable_type() {
                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(
                        var_token, type_token, var_type, info,
                    )));
                } else {
                    self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                    return None;
                }

                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            let rightpar_token = self.save_spanned_token();
            self.next_token();

            self.skip_eol();

            let mut statements = Vec::new();

            while self.get_cur_token() != Some(Token::EndProc) && self.get_cur_token() != Some(Token::EndFunc) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            let endproc_token = self.save_spanned_token();
            if endproc_token.token == Token::EndFunc {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_warning(endproc_token.span.clone(), ParserWarningType::ProcedureClosedWithEndFunc);
            }
            self.next_token();

            return Some(ProcedureImplementation::new(
                usize::MAX,
                procedure_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
                statements.into_iter().flatten().collect(),
                endproc_token,
            ));
        }
        None
    }

    /// Returns the parse function of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_function(&mut self) -> Option<FunctionImplementation> {
        if Some(Token::Function) == self.get_cur_token() {
            let function_token = self.save_spanned_token();
            self.next_token();

            let Some(Token::Identifier(_)) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::IdentifierExpected(self.save_token()));

                return None;
            };
            let identifier_token = self.save_spanned_token();
            self.next_token();
            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(self.lex.span(), ParserErrorType::MissingOpenParens(self.save_token()));
                return None;
            }

            let leftpar_token = self.save_spanned_token();
            self.next_token();

            let mut parameters = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::MissingCloseParens(self.save_token()));

                    return None;
                }
                if let Some(Token::Identifier(id)) = self.get_cur_token()
                    && id == Ascii::new("VAR".to_string())
                {
                    self.report_error(self.lex.span(), ParserErrorType::VarNotAllowedInFunctions);
                    self.next_token();
                }

                if let Some((var_type, type_token)) = self.parse_variable_type() {
                    let info = self.parse_var_info(false);
                    parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::new(None, type_token, var_type, info)));
                } else {
                    self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                    return None;
                }

                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            let rightpar_token = self.save_spanned_token();
            self.next_token();

            let Some((return_type, return_type_token)) = self.parse_variable_type() else {
                self.report_error(self.lex.span(), ParserErrorType::TypeExpected(self.save_token()));
                return None;
            };
            let return_rank = self.parse_dynamic_array_rank();
            self.skip_eol();

            let mut statements = Vec::new();
            self.in_function = true;
            while self.get_cur_token() != Some(Token::EndProc) && self.get_cur_token() != Some(Token::EndFunc) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            self.in_function = false;

            let endfunc_token = self.save_spanned_token();
            if endfunc_token.token == Token::EndProc {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_warning(endfunc_token.span.clone(), ParserWarningType::FunctionClosedWithEndProc);
            }
            self.next_token();

            return Some(FunctionImplementation::new(
                usize::MAX,
                function_token,
                identifier_token,
                leftpar_token,
                parameters,
                rightpar_token,
                return_type_token,
                return_type,
                return_rank,
                statements.into_iter().flatten().collect(),
                endfunc_token.clone(),
            ));
        }
        None
    }
}
