pub mod constant;
use std::fmt;

pub use self::constant::Constant;

pub mod expression;
pub use self::expression::*;

pub mod statement;
pub use self::statement::*;

pub mod declaration;
pub use self::declaration::*;

pub mod syntax_tree;
pub use self::syntax_tree::*;

pub mod visitor;
pub use self::visitor::*;

pub mod output_visitor;
pub use self::output_visitor::*;

pub mod rename_visitor;
pub use self::rename_visitor::*;

pub mod expression_depth_visitor;
pub use self::expression_depth_visitor::*;

pub mod negate_expression_visitor;
pub use self::negate_expression_visitor::*;

use crate::executable::VariableType;
use crate::parser::lexer::{Spanned, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Function(FunctionImplementation),
    Procedure(ProcedureImplementation),
    VariableDeclaration(Statement),
    ProcedureDeclaration(ProcedureDeclarationAstNode),
    FunctionDeclaration(FunctionDeclarationAstNode),
    Main(BlockStatement),
}

impl AstNode {
    pub fn visit<T: Default, V: AstVisitor<T>>(&self, visitor: &mut V) -> T {
        match self {
            AstNode::Function(f) => visitor.visit_function_implementation(f),
            AstNode::Procedure(p) => visitor.visit_procedure_implementation(p),
            AstNode::VariableDeclaration(s) => s.visit(visitor),
            AstNode::ProcedureDeclaration(p) => visitor.visit_procedure_declaration(p),
            AstNode::FunctionDeclaration(f) => visitor.visit_function_declaration(f),
            AstNode::Main(m) => visitor.visit_main(m),
        }
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        match self {
            AstNode::Function(f) => visitor.visit_function_implementation(f),
            AstNode::Procedure(p) => visitor.visit_procedure_implementation(p),
            AstNode::VariableDeclaration(s) => AstNode::VariableDeclaration(s.visit_mut(visitor)),
            AstNode::ProcedureDeclaration(p) => visitor.visit_procedure_declaration(p),
            AstNode::FunctionDeclaration(f) => visitor.visit_function_declaration(f),
            AstNode::Main(m) => AstNode::Main(visitor.visit_block(m)),
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn is_similar(&self, check: &AstNode) -> bool {
        match (self, check) {
            (AstNode::VariableDeclaration(s1), AstNode::VariableDeclaration(s2)) => {
                s1.is_similar(s2)
            }

            (AstNode::FunctionDeclaration(f1), AstNode::FunctionDeclaration(f2)) => {
                if f1.get_identifier() != f2.get_identifier() {
                    return false;
                }

                if f1.get_parameters().len() != f2.get_parameters().len() {
                    return false;
                }

                for (p1, p2) in f1.get_parameters().iter().zip(f2.get_parameters()) {
                    if p1.get_variable_type() != p2.get_variable_type() {
                        return false;
                    }
                    if !p1.get_variable().is_similar(p2.get_variable()) {
                        return false;
                    }
                }
                f1.get_return_type() == f2.get_return_type()
            }
            (AstNode::ProcedureDeclaration(p1), AstNode::ProcedureDeclaration(p2)) => {
                if p1.get_identifier() != p2.get_identifier() {
                    return false;
                }

                if p1.get_parameters().len() != p2.get_parameters().len() {
                    return false;
                }

                for (p1, p2) in p1.get_parameters().iter().zip(p2.get_parameters()) {
                    if p1.get_variable_type() != p2.get_variable_type() {
                        return false;
                    }
                    if !p1.get_variable().is_similar(p2.get_variable()) {
                        return false;
                    }
                }
                true
            }
            (AstNode::Function(f1), AstNode::Function(f2)) => {
                if f1.get_identifier() != f2.get_identifier() {
                    return false;
                }

                if f1.get_parameters().len() != f2.get_parameters().len() {
                    return false;
                }

                for (p1, p2) in f1.get_parameters().iter().zip(f2.get_parameters()) {
                    if p1.get_variable_type() != p2.get_variable_type() {
                        return false;
                    }
                    if !p1.get_variable().is_similar(p2.get_variable()) {
                        return false;
                    }
                }
                for (s1, s2) in f1.get_statements().iter().zip(f2.get_statements()) {
                    if !s1.is_similar(s2) {
                        return false;
                    }
                }
                f1.get_return_type() == f2.get_return_type()
            }

            (AstNode::Procedure(p1), AstNode::Procedure(p2)) => {
                if p1.get_identifier() != p2.get_identifier() {
                    return false;
                }

                if p1.get_parameters().len() != p2.get_parameters().len() {
                    return false;
                }

                for (p1, p2) in p1.get_parameters().iter().zip(p2.get_parameters()) {
                    if p1.get_variable_type() != p2.get_variable_type() {
                        return false;
                    }
                    if !p1.get_variable().is_similar(p2.get_variable()) {
                        return false;
                    }
                }
                for (s1, s2) in p1.get_statements().iter().zip(p2.get_statements()) {
                    if !s1.is_similar(s2) {
                        return false;
                    }
                }
                true
            }

            _ => false,
        }
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output_visitor = crate::ast::output_visitor::OutputVisitor::default();
        self.visit(&mut output_visitor);
        write!(f, "{}", output_visitor.output)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionImplementation {
    pub id: usize,
    function_token: Spanned<Token>,
    identifier_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    parameters: Vec<ParameterSpecifier>,
    rightpar_token: Spanned<Token>,

    return_type_token: Spanned<Token>,
    return_type: VariableType,

    statements: Vec<Statement>,
    endfunc_token: Spanned<Token>,
}

impl FunctionImplementation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        function_token: Spanned<Token>,
        identifier_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        parameters: Vec<ParameterSpecifier>,
        rightpar_token: Spanned<Token>,
        return_type_token: Spanned<Token>,
        return_type: VariableType,
        statements: Vec<Statement>,
        endfunc_token: Spanned<Token>,
    ) -> Self {
        Self {
            id,
            function_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token,
            return_type,
            statements,
            endfunc_token,
        }
    }

    pub fn empty(
        id: usize,
        identifier: unicase::Ascii<String>,
        parameters: Vec<ParameterSpecifier>,
        return_type: VariableType,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            id,
            function_token: Spanned::create_empty(Token::Function),
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: Spanned::create_empty(Token::LPar),
            parameters,
            rightpar_token: Spanned::create_empty(Token::RPar),
            return_type_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                return_type.to_string(),
            ))),
            return_type,
            statements,
            endfunc_token: Spanned::create_empty(Token::EndFunc),
        }
    }

    pub fn get_function_token(&self) -> &Spanned<Token> {
        &self.function_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_identifier(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.identifier_token.token {
            *id = new_id;
        }
    }

    pub fn get_leftpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_parameters(&self) -> &Vec<ParameterSpecifier> {
        &self.parameters
    }

    pub fn get_parameters_mut(&mut self) -> &mut Vec<ParameterSpecifier> {
        &mut self.parameters
    }

    pub fn get_rightpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_return_type_token(&self) -> &Spanned<Token> {
        &self.return_type_token
    }

    pub fn get_return_type(&self) -> VariableType {
        self.return_type
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_endfunc_token(&self) -> &Spanned<Token> {
        &self.endfunc_token
    }
    /*
    pub fn print_content(&self) -> String {
        let mut res = self.declaration.print_header();
        res.push('\n');
        let mut indent = 1;

        if !self.variable_declarations.is_empty() {
            for v in &self.variable_declarations {
                res.push_str("    ");
                res.push_str(&v.to_string());
                res.push('\n');
            }
            res.push('\n');
        }

        for stmt in &self.block.statements {
            let out = stmt.to_string(self, indent);
            if indent > out.1 {
                indent = out.1;
            }
            for _ in 0..(indent + out.2) {
                res.push_str("    ");
            }
            res.push_str(out.0.as_str());
            indent = out.1;
            res.push('\n');
        }
        res.push_str(format!("ENDFUNC ;--{}", self.get_identifier()).as_str());
        //res.push_str(format!("ENDPROC ;--{name}").as_str());

        res.push('\n');

        res
    }*/
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureImplementation {
    pub id: usize,
    procedure_token: Spanned<Token>,
    identifier_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    parameters: Vec<ParameterSpecifier>,
    rightpar_token: Spanned<Token>,
    statements: Vec<Statement>,
    endproc_token: Spanned<Token>,
}

/*
impl fmt::Display for FunctionImplementation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.declaration)
    }
}
*/
impl ProcedureImplementation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        procedure_token: Spanned<Token>,
        identifier_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        parameters: Vec<ParameterSpecifier>,
        rightpar_token: Spanned<Token>,
        statements: Vec<Statement>,
        endproc_token: Spanned<Token>,
    ) -> Self {
        Self {
            id,
            procedure_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            statements,
            endproc_token,
        }
    }

    pub fn empty(
        id: usize,
        identifier: unicase::Ascii<String>,
        parameters: Vec<ParameterSpecifier>,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            id,
            procedure_token: Spanned::create_empty(Token::Procedure),
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: Spanned::create_empty(Token::LPar),
            parameters,
            rightpar_token: Spanned::create_empty(Token::RPar),
            statements,
            endproc_token: Spanned::create_empty(Token::EndProc),
        }
    }

    pub fn get_procedure_token(&self) -> &Spanned<Token> {
        &self.procedure_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_identifier(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.identifier_token.token {
            *id = new_id;
        }
    }

    pub fn get_leftpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_parameters(&self) -> &Vec<ParameterSpecifier> {
        &self.parameters
    }

    pub fn get_parameters_mut(&mut self) -> &mut Vec<ParameterSpecifier> {
        &mut self.parameters
    }

    pub fn get_rightpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_endproc_token(&self) -> &Spanned<Token> {
        &self.endproc_token
    }
    /*
    pub fn print_content(&self) -> String {
        let mut res = self.declaration.print_header();
        res.push('\n');
        let mut indent = 1;

        if !self.variable_declarations.is_empty() {
            for v in &self.variable_declarations {
                res.push_str("    ");
                res.push_str(&v.to_string());
                res.push('\n');
            }
            res.push('\n');
        }

        for stmt in &self.block.statements {
            let out = stmt.to_string(self, indent);
            if indent > out.1 {
                indent = out.1;
            }
            for _ in 0..(indent + out.2) {
                res.push_str("    ");
            }
            res.push_str(out.0.as_str());
            indent = out.1;
            res.push('\n');
        }
        res.push_str(format!("ENDPROC ;--{}", self.get_identifier()).as_str());
        res.push('\n');
        res
    }*/
}
