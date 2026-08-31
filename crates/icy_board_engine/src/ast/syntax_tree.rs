use super::{AstNode, AstVisitor, AstVisitorMut};
use crate::executable::LAST_PPL_LANGUAGE_VERSION;
use crate::parser::lexer::{Spanned, Token};
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibilitySection {
    pub token: Spanned<Token>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDeclaration {
    pub import_token: Spanned<Token>,
    pub module_token: Spanned<Token>,
    pub as_token: Spanned<Token>,
    pub alias_token: Spanned<Token>,
}

impl ImportDeclaration {
    pub fn module_name(&self) -> &unicase::Ascii<String> {
        match &self.module_token.token {
            Token::Identifier(name) => name,
            _ => unreachable!("an import module is always an identifier"),
        }
    }

    pub fn alias(&self) -> &unicase::Ascii<String> {
        match &self.alias_token.token {
            Token::Identifier(name) => name,
            _ => unreachable!("an import alias is always an identifier"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDeclaration {
    pub module_token: Spanned<Token>,
    pub name_token: Spanned<Token>,
    pub endmodule_token: Spanned<Token>,
    pub visibility_sections: Vec<VisibilitySection>,
    pub implicit: bool,
}

impl ModuleDeclaration {
    pub fn implicit(name: impl Into<String>) -> Self {
        Self {
            module_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new("MODULE".to_string()))),
            name_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(name.into()))),
            endmodule_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new("ENDMODULE".to_string()))),
            visibility_sections: Vec::new(),
            implicit: true,
        }
    }

    pub fn name(&self) -> &unicase::Ascii<String> {
        match &self.name_token.token {
            Token::Identifier(name) => name,
            _ => unreachable!("a module name is always an identifier"),
        }
    }

    pub fn visibility_at(&self, offset: usize) -> Visibility {
        self.visibility_sections
            .iter()
            .take_while(|section| section.token.span.start < offset)
            .last()
            .map_or(Visibility::Public, |section| section.visibility)
    }

    pub fn is_implicit(&self) -> bool {
        self.implicit
    }
}

#[derive(Debug)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
    pub file_name: PathBuf,

    /// A source may wrap its declarations in one compile-time namespace.
    pub module: Option<ModuleDeclaration>,

    /// Imports are source-local aliases and never reach the PPE.
    pub imports: Vec<ImportDeclaration>,

    /// The language the file was read as, `;$LANGVERSION` included.
    pub language_version: u16,

    pub require_user_variables: bool,
}

impl Ast {
    pub fn new() -> Self {
        Ast {
            nodes: vec![],
            file_name: PathBuf::new(),
            module: None,
            imports: Vec::new(),
            language_version: LAST_PPL_LANGUAGE_VERSION,
            require_user_variables: false,
        }
    }

    pub fn visit<T: Default, V: AstVisitor<T>>(&self, visitor: &mut V) {
        visitor.visit_ast(self);
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_ast(self)
    }
}

impl Default for Ast {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output_visitor = crate::ast::output_visitor::OutputVisitor::default();
        self.visit(&mut output_visitor);

        write!(f, "{}", output_visitor.output)
    }
}
