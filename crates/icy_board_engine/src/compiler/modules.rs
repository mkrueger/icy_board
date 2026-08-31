use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use unicase::Ascii;

use crate::{
    ast::{
        Ast, AstNode, AstVisitor, AstVisitorMut, Expression, FunctionDeclarationAstNode, FunctionImplementation, IdentifierExpression,
        MemberReferenceExpression, ParameterSpecifier, ProcedureCallStatement, Statement, TypeDeclarationAstNode, VariableDeclarationStatement,
        VariableSpecifier, Visibility, walk_function_declaration, walk_function_implementation, walk_procedure_declaration, walk_procedure_implementation,
        walk_variable_declaration_statement,
    },
    compiler::CompilationErrorType,
    parser::{
        ErrorReporter, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Value,
    Type,
}

#[derive(Clone)]
struct ModuleSymbol {
    lowered: Ascii<String>,
    visibility: Visibility,
    kind: SymbolKind,
}

struct ModuleInfo {
    symbols: HashMap<Ascii<String>, ModuleSymbol>,
    implicit: bool,
}

type ModuleCatalog = HashMap<Ascii<String>, ModuleInfo>;

pub fn lower_modules(asts: &[&Ast], errors: Arc<Mutex<ErrorReporter>>) -> Vec<Ast> {
    let mut catalog = ModuleCatalog::new();

    for (module_index, ast) in asts.iter().enumerate() {
        let Some(module) = &ast.module else { continue };
        let existing = catalog.get(module.name());
        if existing.is_some() && !(module.is_implicit() && existing.is_some_and(|info| info.implicit)) {
            errors.lock().unwrap().report_error_file(
                ast.file_name.clone(),
                module.name_token.span.clone(),
                CompilationErrorType::ModuleAlreadyDefined(module.name().to_string()),
            );
            continue;
        }
        let symbols = &mut catalog
            .entry(module.name().clone())
            .or_insert_with(|| ModuleInfo {
                symbols: HashMap::new(),
                implicit: module.is_implicit(),
            })
            .symbols;
        for (name, span, kind) in global_symbols(ast) {
            let lowered = if kind == SymbolKind::Type {
                UserTypeRegistry::module_type_name(module.name(), &name)
            } else {
                Ascii::new(format!("__M{module_index}_{}", name.as_str()))
            };
            symbols.entry(name).or_insert_with(|| ModuleSymbol {
                lowered,
                visibility: module.visibility_at(span),
                kind,
            });
        }
    }

    asts.iter()
        .map(|ast| {
            let own = ast
                .module
                .as_ref()
                .and_then(|module| catalog.get(module.name()))
                .map(|module| module.symbols.clone())
                .unwrap_or_default();
            let mut imports = HashMap::new();
            for import in &ast.imports {
                if !catalog.contains_key(import.module_name()) {
                    errors.lock().unwrap().report_error_file(
                        ast.file_name.clone(),
                        import.module_token.span.clone(),
                        CompilationErrorType::ModuleNotFound(import.module_name().to_string()),
                    );
                    continue;
                }
                if imports.insert(import.alias().clone(), import.module_name().clone()).is_some() {
                    errors.lock().unwrap().report_error_file(
                        ast.file_name.clone(),
                        import.alias_token.span.clone(),
                        CompilationErrorType::ImportAliasAlreadyDefined(import.alias().to_string()),
                    );
                }
            }
            ast.visit(&mut TypeVisibilityValidator {
                imports: &imports,
                catalog: &catalog,
                errors: errors.clone(),
                file: ast.file_name.clone(),
            });
            ast.visit_mut(&mut ModuleLowering {
                own,
                imports,
                catalog: &catalog,
                errors: errors.clone(),
                file: ast.file_name.clone(),
                locals: HashSet::new(),
            })
        })
        .collect()
}

struct TypeVisibilityValidator<'a> {
    imports: &'a HashMap<Ascii<String>, Ascii<String>>,
    catalog: &'a ModuleCatalog,
    errors: Arc<Mutex<ErrorReporter>>,
    file: std::path::PathBuf,
}

impl TypeVisibilityValidator<'_> {
    fn validate_type(&self, token: &Spanned<Token>) {
        let Token::Identifier(name) = &token.token else { return };
        for module in self.imports.values() {
            let Some(module_info) = self.catalog.get(module) else { continue };
            if let Some((source_name, _)) = module_info
                .symbols
                .iter()
                .find(|(_, symbol)| symbol.kind == SymbolKind::Type && symbol.visibility == Visibility::Private && symbol.lowered == *name)
            {
                self.errors.lock().unwrap().report_error_file(
                    self.file.clone(),
                    token.span.clone(),
                    CompilationErrorType::PrivateModuleMember(module.to_string(), source_name.to_string()),
                );
            }
        }
    }

    fn validate_parameters(&mut self, parameters: &[ParameterSpecifier]) {
        for parameter in parameters {
            match parameter {
                ParameterSpecifier::Variable(value) => self.validate_type(value.get_type_token()),
                ParameterSpecifier::Function(value) => {
                    self.validate_type(value.get_return_type_token());
                    self.validate_parameters(value.get_parameters());
                }
                ParameterSpecifier::Procedure(value) => self.validate_parameters(value.get_parameters()),
            }
        }
    }
}

impl AstVisitor<()> for TypeVisibilityValidator<'_> {
    fn visit_variable_declaration_statement(&mut self, declaration: &VariableDeclarationStatement) {
        self.validate_type(declaration.get_type_token());
        walk_variable_declaration_statement(self, declaration);
    }

    fn visit_const_declaration_statement(&mut self, declaration: &crate::ast::ConstDeclarationStatement) {
        self.validate_type(declaration.get_type_token());
        declaration.get_value().visit(self);
    }

    fn visit_type_declaration(&mut self, declaration: &TypeDeclarationAstNode) {
        for field in declaration.get_fields() {
            self.validate_type(field.get_type_token());
        }
    }

    fn visit_function_declaration(&mut self, declaration: &FunctionDeclarationAstNode) {
        self.validate_type(declaration.get_return_type_token());
        self.validate_parameters(declaration.get_parameters());
        walk_function_declaration(self, declaration);
    }

    fn visit_procedure_declaration(&mut self, declaration: &crate::ast::ProcedureDeclarationAstNode) {
        self.validate_parameters(declaration.get_parameters());
        walk_procedure_declaration(self, declaration);
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) {
        self.validate_type(function.get_return_type_token());
        self.validate_parameters(function.get_parameters());
        walk_function_implementation(self, function);
    }

    fn visit_procedure_implementation(&mut self, procedure: &crate::ast::ProcedureImplementation) {
        self.validate_parameters(procedure.get_parameters());
        walk_procedure_implementation(self, procedure);
    }
}

fn global_symbols(ast: &Ast) -> Vec<(Ascii<String>, usize, SymbolKind)> {
    let mut result = Vec::new();
    for node in &ast.nodes {
        match node {
            AstNode::Function(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Value)),
            AstNode::Procedure(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Value)),
            AstNode::FunctionDeclaration(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Value)),
            AstNode::ProcedureDeclaration(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Value)),
            AstNode::TypeDeclaration(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Type)),
            AstNode::EnumDeclaration(value) => result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Type)),
            AstNode::TopLevelStatement(Statement::VariableDeclaration(value)) => {
                result.extend(
                    value
                        .get_variables()
                        .iter()
                        .map(|v| (v.get_identifier().clone(), v.get_identifier_token().span.start, SymbolKind::Value)),
                );
            }
            AstNode::TopLevelStatement(Statement::ConstDeclaration(value)) => {
                result.push((value.get_identifier().clone(), value.get_identifier_token().span.start, SymbolKind::Value));
            }
            AstNode::TopLevelStatement(_) | AstNode::Main(_) => {}
        }
    }
    result
}

struct LocalCollector {
    names: HashSet<Ascii<String>>,
}

impl AstVisitor<()> for LocalCollector {
    fn visit_variable_specifier(&mut self, var: &VariableSpecifier) {
        self.names.insert(var.get_identifier().clone());
    }

    fn visit_parameter_specifier(&mut self, parameter: &ParameterSpecifier) {
        match parameter {
            ParameterSpecifier::Variable(value) => {
                if let Some(variable) = value.get_variable() {
                    self.names.insert(variable.get_identifier().clone());
                }
            }
            ParameterSpecifier::Function(value) => {
                self.names.insert(value.get_identifier().clone());
            }
            ParameterSpecifier::Procedure(value) => {
                self.names.insert(value.get_identifier().clone());
            }
        }
    }
}

struct ModuleLowering<'a> {
    own: HashMap<Ascii<String>, ModuleSymbol>,
    imports: HashMap<Ascii<String>, Ascii<String>>,
    catalog: &'a ModuleCatalog,
    errors: Arc<Mutex<ErrorReporter>>,
    file: std::path::PathBuf,
    locals: HashSet<Ascii<String>>,
}

impl ModuleLowering<'_> {
    fn imported_symbol(&self, expression: &Expression) -> Option<(ModuleSymbol, Spanned<Token>)> {
        let Expression::MemberReference(member) = expression else { return None };
        let Expression::Identifier(base) = member.get_expression() else { return None };
        let module = self.imports.get(base.get_identifier())?;
        let symbols = &self.catalog.get(module)?.symbols;
        let Some(symbol) = symbols.get(member.get_identifier()) else {
            self.errors.lock().unwrap().report_error_file(
                self.file.clone(),
                member.get_identifier_token().span.clone(),
                CompilationErrorType::ModuleMemberNotFound(module.to_string(), member.get_identifier().to_string()),
            );
            return None;
        };
        if symbol.visibility == Visibility::Private {
            self.errors.lock().unwrap().report_error_file(
                self.file.clone(),
                member.get_identifier_token().span.clone(),
                CompilationErrorType::PrivateModuleMember(module.to_string(), member.get_identifier().to_string()),
            );
            return None;
        }
        Some((symbol.clone(), member.get_identifier_token().clone()))
    }

    fn with_routine_locals(&mut self, parameters: &[ParameterSpecifier], statements: &[Statement], f: impl FnOnce(&mut Self) -> AstNode) -> AstNode {
        let old = std::mem::take(&mut self.locals);
        let mut collector = LocalCollector { names: HashSet::new() };
        for parameter in parameters {
            parameter.visit(&mut collector);
        }
        for statement in statements {
            statement.visit(&mut collector);
        }
        self.locals = collector.names;
        let result = f(self);
        self.locals = old;
        result
    }
}

impl AstVisitorMut for ModuleLowering<'_> {
    fn visit_identifier(&mut self, id: &Ascii<String>) -> Ascii<String> {
        if self.locals.contains(id) {
            return id.clone();
        }
        self.own.get(id).map_or_else(|| id.clone(), |symbol| symbol.lowered.clone())
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) -> Expression {
        if let Some((symbol, token)) = self.imported_symbol(&Expression::MemberReference(member.clone())) {
            return Expression::Identifier(IdentifierExpression::new(Spanned::new(
                Token::Identifier(symbol.lowered.clone()),
                token.span.clone(),
            )));
        }
        Expression::MemberReference(MemberReferenceExpression::new(
            member.get_expression().visit_mut(self),
            member.get_dot_token().clone(),
            member.get_identifier_token().clone(),
        ))
    }

    fn visit_member_call_statement(&mut self, call: &crate::ast::MemberCallStatement) -> Statement {
        if let Expression::FunctionCall(function) = call.get_expression()
            && let Some((symbol, token)) = self.imported_symbol(function.get_expression())
        {
            return Statement::Call(ProcedureCallStatement::new(
                Spanned::new(Token::Identifier(symbol.lowered.clone()), token.span.clone()),
                function.get_lpar_token().clone(),
                function.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect(),
                function.get_rpar_token().clone(),
            ));
        }
        Statement::MemberCall(crate::ast::MemberCallStatement::new(call.get_expression().visit_mut(self)))
    }

    fn visit_function_implementation(&mut self, function: &crate::ast::FunctionImplementation) -> AstNode {
        self.with_routine_locals(function.get_parameters(), function.get_statements(), |this| {
            let name = this.visit_identifier(function.get_identifier());
            AstNode::Function(
                crate::ast::FunctionImplementation::new(
                    function.id,
                    function.get_function_token().clone(),
                    Spanned::new(Token::Identifier(name), function.get_identifier_token().span.clone()),
                    function.get_leftpar_token().clone(),
                    function.get_parameters().to_vec(),
                    function.get_rightpar_token().clone(),
                    function.get_return_type_token().clone(),
                    function.get_return_type(),
                    function.get_return_rank(),
                    function.get_statements().iter().map(|statement| statement.visit_mut(this)).collect(),
                    function.get_endfunc_token().clone(),
                )
                .with_documentation(function.get_documentation()),
            )
        })
    }

    fn visit_procedure_implementation(&mut self, procedure: &crate::ast::ProcedureImplementation) -> AstNode {
        self.with_routine_locals(procedure.get_parameters(), procedure.get_statements(), |this| {
            let name = this.visit_identifier(procedure.get_identifier());
            AstNode::Procedure(
                crate::ast::ProcedureImplementation::new(
                    procedure.id,
                    procedure.get_procedure_token().clone(),
                    Spanned::new(Token::Identifier(name), procedure.get_identifier_token().span.clone()),
                    procedure.get_leftpar_token().clone(),
                    procedure.get_parameters().to_vec(),
                    procedure.get_rightpar_token().clone(),
                    procedure.get_statements().iter().map(|statement| statement.visit_mut(this)).collect(),
                    procedure.get_endproc_token().clone(),
                )
                .with_documentation(procedure.get_documentation()),
            )
        })
    }

    fn visit_const_declaration_statement(&mut self, declaration: &crate::ast::ConstDeclarationStatement) -> Statement {
        let identifier = self.own.get(declaration.get_identifier()).map_or_else(
            || declaration.get_identifier_token().clone(),
            |symbol| Spanned::new(Token::Identifier(symbol.lowered.clone()), declaration.get_identifier_token().span.clone()),
        );
        Statement::ConstDeclaration(crate::ast::ConstDeclarationStatement::new(
            declaration.get_const_token().clone(),
            declaration.get_type_token().clone(),
            declaration.get_variable_type(),
            identifier,
            declaration.get_eq_token().clone(),
            declaration.get_value().visit_mut(self),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::{PPECompiler, workspace::Workspace},
        parser::{Encoding, UserTypeRegistry, parse_ast},
    };
    use std::path::PathBuf;

    fn compile(sources: &[(&str, &str)]) -> Arc<Mutex<ErrorReporter>> {
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let registry = UserTypeRegistry::icy_board_registry();
        let workspace = Workspace::default();
        let asts = sources
            .iter()
            .map(|(name, source)| parse_ast(PathBuf::from(name), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
            .collect::<Vec<_>>();
        let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
        compiler.compile(&asts.iter().collect::<Vec<_>>());
        errors
    }

    #[test]
    fn imported_public_procedure_compiles() {
        let errors = compile(&[
            ("greeter.pps", "MODULE Greeter\nPROCEDURE Hello()\n  PRINTLN \"hello\"\nENDPROC\nENDMODULE\n"),
            ("main.pps", "IMPORT Greeter AS G\nG.Hello()\n"),
        ]);
        assert!(errors.lock().unwrap().errors.is_empty(), "public import should compile");
    }

    #[test]
    fn private_procedure_is_not_importable() {
        let errors = compile(&[
            ("greeter.pps", "MODULE Greeter\nPRIVATE\nPROCEDURE Hello()\nENDPROC\nENDMODULE\n"),
            ("main.pps", "IMPORT Greeter AS G\nG.Hello()\n"),
        ]);
        let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
        assert!(messages.iter().any(|message| message.contains("private to module Greeter")), "{messages:?}");
    }

    #[test]
    fn equal_names_in_two_modules_do_not_collide() {
        let errors = compile(&[
            ("one.pps", "MODULE One\nPROCEDURE Run()\nENDPROC\nENDMODULE\n"),
            ("two.pps", "MODULE Two\nPROCEDURE Run()\nENDPROC\nENDMODULE\n"),
            ("main.pps", "IMPORT One AS A\nIMPORT Two AS B\nA.Run()\nB.Run()\n"),
        ]);
        assert!(errors.lock().unwrap().errors.is_empty(), "module names should isolate declarations");
    }

    #[test]
    fn public_and_private_remain_identifiers_outside_modules() {
        let errors = compile(&[("main.pps", "STRING public, private\npublic = \"yes\"\nprivate = public\n")]);
        assert!(errors.lock().unwrap().errors.is_empty(), "contextual words should remain valid identifiers");
    }

    #[test]
    fn module_words_remain_routine_identifiers() {
        let errors = compile(&[(
            "main.pps",
            "PROCEDURE module()\nENDPROC\nPROCEDURE import()\nENDPROC\nPROCEDURE public()\nENDPROC\n",
        )]);
        assert!(
            errors.lock().unwrap().errors.is_empty(),
            "module words should only be recognized in complete contextual forms"
        );
    }

    #[test]
    fn module_syntax_requires_language_400() {
        let errors = compile(&[("main.pps", ";$LANGVERSION 350\nMODULE Legacy\nENDMODULE\n")]);
        assert!(
            !errors.lock().unwrap().errors.is_empty(),
            "module syntax must not be enabled before language 4.00"
        );
    }

    #[test]
    fn public_routines_can_use_private_module_members() {
        let errors = compile(&[
            (
                "counter.pps",
                "MODULE Counter\nPROCEDURE Increment()\n  AddOne()\nENDPROC\nPRIVATE\nINTEGER value\nPROCEDURE AddOne()\n  value += 1\nENDPROC\nENDMODULE\n",
            ),
            ("main.pps", "IMPORT Counter AS C\nC.Increment()\n"),
        ]);
        let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
        assert!(messages.is_empty(), "private members should remain usable inside their module: {messages:?}");
    }

    #[test]
    fn imported_functions_and_constants_compile() {
        let errors = compile(&[
            (
                "math.pps",
                "MODULE Math\nCONST INTEGER One = 1\nFUNCTION Answer() INTEGER\n  Answer = One + 41\nENDFUNC\nENDMODULE\n",
            ),
            ("main.pps", "IMPORT Math AS M\nINTEGER answer = M.Answer() + M.One\n"),
        ]);
        assert!(errors.lock().unwrap().errors.is_empty(), "qualified values should lower to module globals");
    }

    #[test]
    fn imported_record_types_are_qualified() {
        let errors = compile(&[
            ("one.pps", "MODULE One\nTYPE Point\n  INTEGER X\nENDTYPE\nENDMODULE\n"),
            ("two.pps", "MODULE Two\nTYPE Point\n  INTEGER Y\nENDTYPE\nENDMODULE\n"),
            (
                "main.pps",
                "IMPORT One AS A\nIMPORT Two AS B\nA.Point first\nB.Point second\nfirst.X = 1\nsecond.Y = 2\n",
            ),
        ]);
        assert!(errors.lock().unwrap().errors.is_empty(), "equal type names should be isolated by their modules");
    }

    #[test]
    fn private_record_types_are_not_importable() {
        let errors = compile(&[
            ("hidden.pps", "MODULE Hidden\nPRIVATE\nTYPE Secret\n  INTEGER Value\nENDTYPE\nENDMODULE\n"),
            ("main.pps", "IMPORT Hidden AS H\nH.Secret value\n"),
        ]);
        let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
        assert!(messages.iter().any(|message| message.contains("private to module Hidden")), "{messages:?}");
    }

    #[test]
    fn public_section_restores_export_visibility() {
        let errors = compile(&[
            (
                "sections.pps",
                "MODULE Sections\nPRIVATE\nPROCEDURE Hidden()\nENDPROC\nPUBLIC\nPROCEDURE Visible()\nENDPROC\nENDMODULE\n",
            ),
            ("main.pps", "IMPORT Sections AS S\nS.Visible()\n"),
        ]);
        assert!(
            errors.lock().unwrap().errors.is_empty(),
            "PUBLIC should restore public visibility after a PRIVATE section"
        );
    }
}
