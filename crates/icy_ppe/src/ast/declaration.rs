use std::fmt;

use crate::{
    executable::{VariableType, VariableValue},
    parser::lexer::{Spanned, Token},
};

use super::{AstVisitorMut, Constant, Statement};
#[derive(Debug, PartialEq, Clone)]
pub struct DimensionSpecifier {
    dimension_token: Spanned<Token>,
}

impl DimensionSpecifier {
    /// Creates a new [`DimensionSpecifier`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn new(dimension_token: Spanned<Token>) -> Self {
        #[allow(clippy::manual_assert)]
        if !matches!(dimension_token.token, Token::Const(Constant::Integer(_))) {
            panic!("DimensionSpecifier::new: invalid token {dimension_token:?}");
        }
        Self { dimension_token }
    }

    pub fn empty(dimension: usize) -> Self {
        Self {
            dimension_token: Spanned::create_empty(Token::Const(Constant::Integer(
                dimension as i32,
            ))),
        }
    }

    pub fn get_dimension_token(&self) -> &Spanned<Token> {
        &self.dimension_token
    }

    /// Returns the get dimension of this [`DimensionSpecifier`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_dimension(&self) -> usize {
        if let Token::Const(Constant::Integer(i)) = self.dimension_token.token {
            i as usize
        } else {
            panic!("DimensionSpecifier::new: invalid token")
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableSpecifier {
    identifier_token: Spanned<Token>,
    leftpar_token: Option<Spanned<Token>>,
    dimensions: Vec<DimensionSpecifier>,
    rightpar_token: Option<Spanned<Token>>,
}

impl VariableSpecifier {
    pub fn new(
        identifier_token: Spanned<Token>,
        leftpar_token: Option<Spanned<Token>>,
        dimensions: Vec<DimensionSpecifier>,
        rightpar_token: Option<Spanned<Token>>,
    ) -> Self {
        Self {
            identifier_token,
            leftpar_token,
            dimensions,
            rightpar_token,
        }
    }

    pub fn empty(identifier: unicase::Ascii<String>, dimensions: Vec<usize>) -> Self {
        Self {
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: None,
            dimensions: dimensions
                .into_iter()
                .map(DimensionSpecifier::empty)
                .collect(),
            rightpar_token: None,
        }
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`ForStatement`].
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

    pub fn get_leftpar_token(&self) -> &Option<Spanned<Token>> {
        &self.leftpar_token
    }

    pub fn get_dimensions(&self) -> &Vec<DimensionSpecifier> {
        &self.dimensions
    }

    pub fn get_dimensions_mut(&mut self) -> &mut Vec<DimensionSpecifier> {
        &mut self.dimensions
    }

    pub fn get_rightpar_token(&self) -> &Option<Spanned<Token>> {
        &self.rightpar_token
    }

    pub fn create_empty_value(&self, variable_type: VariableType) -> VariableValue {
        let var_value = variable_type.create_empty_value();
        match self.dimensions.len() {
            0 => var_value,
            1 => VariableValue::new_vector(
                variable_type,
                vec![var_value; self.dimensions[0].get_dimension()],
            ),
            2 => VariableValue::new_matrix(
                variable_type,
                vec![
                    vec![var_value; self.dimensions[0].get_dimension()];
                    self.dimensions[1].get_dimension()
                ],
            ),
            _ => VariableValue::new_cube(
                variable_type,
                vec![
                    vec![
                        vec![var_value; self.dimensions[0].get_dimension()];
                        self.dimensions[1].get_dimension()
                    ];
                    self.dimensions[2].get_dimension()
                ],
            ),
        }
    }

    pub fn get_vector_size(&self) -> usize {
        if self.dimensions.is_empty() {
            return 0;
        }
        self.dimensions[0].get_dimension()
    }

    pub fn get_matrix_size(&self) -> usize {
        if self.dimensions.len() < 2 {
            return 0;
        }
        self.dimensions[1].get_dimension()
    }

    pub fn get_cube_size(&self) -> usize {
        if self.dimensions.len() < 3 {
            return 0;
        }
        self.dimensions[2].get_dimension()
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_variable_specifier(self)
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn is_similar(&self, check: &VariableSpecifier) -> bool {
        if self.get_identifier() != check.get_identifier() {
            return false;
        }
        if self.get_dimensions().len() != check.get_dimensions().len() {
            return false;
        }
        for (i, dim) in self.get_dimensions().iter().enumerate() {
            if dim.get_dimension() != check.get_dimensions()[i].get_dimension() {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDeclarationStatement {
    type_token: Spanned<Token>,
    variable_type: VariableType,
    variables: Vec<VariableSpecifier>,
}

impl VariableDeclarationStatement {
    pub fn new(
        type_token: Spanned<Token>,
        variable_type: VariableType,
        variables: Vec<VariableSpecifier>,
    ) -> Self {
        Self {
            type_token,
            variable_type,
            variables,
        }
    }

    pub fn empty(variable_type: VariableType, variables: Vec<VariableSpecifier>) -> Self {
        Self {
            type_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                variable_type.to_string(),
            ))),
            variable_type,
            variables,
        }
    }

    pub fn get_type_token(&self) -> &Spanned<Token> {
        &self.type_token
    }

    pub fn get_variable_type(&self) -> VariableType {
        self.variable_type
    }

    pub fn get_variables(&self) -> &Vec<VariableSpecifier> {
        &self.variables
    }

    pub fn get_variables_mut(&mut self) -> &mut Vec<VariableSpecifier> {
        &mut self.variables
    }

    pub fn create_empty_statement(
        variable_type: VariableType,
        variables: Vec<VariableSpecifier>,
    ) -> Statement {
        Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            variable_type,
            variables,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterSpecifier {
    var_token: Option<Spanned<Token>>,
    type_token: Spanned<Token>,
    variable_type: VariableType,
    variable: VariableSpecifier,
}

impl ParameterSpecifier {
    pub fn new(
        var_token: Option<Spanned<Token>>,
        type_token: Spanned<Token>,
        variable_type: VariableType,
        variable: VariableSpecifier,
    ) -> Self {
        Self {
            var_token,
            type_token,
            variable_type,
            variable,
        }
    }

    pub fn empty(is_var: bool, variable_type: VariableType, variable: VariableSpecifier) -> Self {
        Self {
            var_token: if is_var {
                Some(Spanned::create_empty(Token::Identifier(
                    unicase::Ascii::new("VAR".to_string()),
                )))
            } else {
                None
            },
            type_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                variable_type.to_string(),
            ))),
            variable_type,
            variable,
        }
    }

    pub fn get_var_token(&self) -> &Option<Spanned<Token>> {
        &self.var_token
    }

    pub fn is_var(&self) -> bool {
        self.var_token.is_some()
    }

    pub fn get_type_token(&self) -> &Spanned<Token> {
        &self.type_token
    }

    pub fn get_variable_type(&self) -> VariableType {
        self.variable_type
    }

    pub fn get_variable(&self) -> &VariableSpecifier {
        &self.variable
    }

    pub fn get_variable_mut(&mut self) -> &mut VariableSpecifier {
        &mut self.variable
    }

    pub fn create_empty_statement(
        variable_type: VariableType,
        variables: Vec<VariableSpecifier>,
    ) -> Statement {
        Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            variable_type,
            variables,
        ))
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_parameter_specifier(self)
    }

    pub fn visit<T: Default, V: super::AstVisitor<T>>(&self, visitor: &mut V) -> T {
        visitor.visit_parameter_specifier(self)
    }
}

impl fmt::Display for VariableDeclarationStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.get_variable_type(),
            Statement::variable_list_to_string(self.get_variables())
        )
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct ProcedureDeclarationAstNode {
    declare_token: Spanned<Token>,
    procedure_token: Spanned<Token>,
    identifier_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    parameters: Vec<ParameterSpecifier>,
    rightpar_token: Spanned<Token>,
}

impl ProcedureDeclarationAstNode {
    pub fn new(
        declare_token: Spanned<Token>,
        procedure_token: Spanned<Token>,
        identifier_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        parameters: Vec<ParameterSpecifier>,
        rightpar_token: Spanned<Token>,
    ) -> Self {
        Self {
            declare_token,
            procedure_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
        }
    }

    pub fn empty(identifier: unicase::Ascii<String>, parameters: Vec<ParameterSpecifier>) -> Self {
        Self {
            declare_token: Spanned::create_empty(Token::Declare),
            procedure_token: Spanned::create_empty(Token::Procedure),
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: Spanned::create_empty(Token::LPar),
            parameters,
            rightpar_token: Spanned::create_empty(Token::RPar),
        }
    }
    pub fn get_declare_token(&self) -> &Spanned<Token> {
        &self.declare_token
    }

    pub fn get_procedure_token(&self) -> &Spanned<Token> {
        &self.procedure_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`ForStatement`].
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

    pub fn get_pass_flags(&self) -> u16 {
        let mut flags = 0;
        for param in &self.parameters {
            if param.is_var() {
                flags |= 1;
            }
        }
        flags
    }

    pub fn empty_node(
        identifier: unicase::Ascii<String>,
        parameters: Vec<ParameterSpecifier>,
    ) -> super::AstNode {
        super::AstNode::ProcedureDeclaration(ProcedureDeclarationAstNode::empty(
            identifier, parameters,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDeclarationAstNode {
    declare_token: Spanned<Token>,
    function_token: Spanned<Token>,
    identifier_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    parameters: Vec<ParameterSpecifier>,
    rightpar_token: Spanned<Token>,
    return_type_token: Spanned<Token>,
    return_type: VariableType,
}

impl FunctionDeclarationAstNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        declare_token: Spanned<Token>,
        function_token: Spanned<Token>,
        identifier_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        parameters: Vec<ParameterSpecifier>,
        rightpar_token: Spanned<Token>,
        return_type_token: Spanned<Token>,
        return_type: VariableType,
    ) -> Self {
        Self {
            declare_token,
            function_token,
            identifier_token,
            leftpar_token,
            parameters,
            rightpar_token,
            return_type_token,
            return_type,
        }
    }

    pub fn empty(
        identifier: unicase::Ascii<String>,
        parameters: Vec<ParameterSpecifier>,
        return_type: VariableType,
    ) -> Self {
        Self {
            declare_token: Spanned::create_empty(Token::Declare),
            function_token: Spanned::create_empty(Token::Function),
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: Spanned::create_empty(Token::LPar),
            parameters,
            rightpar_token: Spanned::create_empty(Token::RPar),
            return_type_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                return_type.to_string(),
            ))),
            return_type,
        }
    }
    pub fn get_declare_token(&self) -> &Spanned<Token> {
        &self.declare_token
    }

    pub fn get_function_token(&self) -> &Spanned<Token> {
        &self.function_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`ForStatement`].
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

    pub fn empty_node(
        identifier: unicase::Ascii<String>,
        parameters: Vec<ParameterSpecifier>,
        return_type: VariableType,
    ) -> super::AstNode {
        super::AstNode::FunctionDeclaration(FunctionDeclarationAstNode::empty(
            identifier,
            parameters,
            return_type,
        ))
    }
}
