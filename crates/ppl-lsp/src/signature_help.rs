//! Signature help for user routines, built-in functions and built-in statements.

use icy_board_engine::{
    ast::ParameterSpecifier,
    executable::{FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS, StatementSignature, VariableType, format_argument},
    semantic::{FunctionDeclaration, SemanticVisitor},
};
use tower_lsp::lsp_types::{ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation};

use crate::{context::call_context, type_lookup::type_name};

/// Builds one signature out of a name, its parameters and what it returns.
struct SignatureBuilder {
    label: String,
    parameters: Vec<ParameterInformation>,
}

impl SignatureBuilder {
    fn new(head: &str, open: &str) -> Self {
        Self {
            label: format!("{head}{open}"),
            parameters: Vec::new(),
        }
    }

    fn push(&mut self, text: &str) {
        if !self.parameters.is_empty() {
            self.label.push_str(", ");
        }
        let start = self.label.chars().count() as u32;
        self.label.push_str(text);
        let end = self.label.chars().count() as u32;
        self.parameters.push(ParameterInformation {
            label: ParameterLabel::LabelOffsets([start, end]),
            documentation: None,
        });
    }

    fn finish(mut self, tail: &str) -> SignatureInformation {
        self.label.push_str(tail);
        SignatureInformation {
            label: self.label,
            documentation: None,
            parameters: Some(self.parameters),
            active_parameter: None,
        }
    }
}

fn render_parameter(visitor: &SemanticVisitor, parameter: &ParameterSpecifier) -> String {
    match parameter {
        ParameterSpecifier::Variable(variable) => {
            let mut text = String::new();
            if variable.is_var() {
                text.push_str("VAR ");
            }
            text.push_str(&type_name(&visitor.type_registry, variable.get_variable_type()));
            if let Some(specifier) = variable.get_variable() {
                text.push(' ');
                text.push_str(&specifier.get_identifier().to_string());
                if !specifier.get_dimensions().is_empty() {
                    let dimensions = specifier.get_dimensions().iter().map(|d| d.get_dimension().to_string()).collect::<Vec<_>>();
                    text.push_str(&format!("({})", dimensions.join(", ")));
                }
            }
            text
        }
        ParameterSpecifier::Function(function) => {
            let parameters = function
                .get_parameters()
                .iter()
                .map(|p| render_parameter(visitor, p))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "FUNCTION {}({}) {}",
                function.get_identifier(),
                parameters,
                type_name(&visitor.type_registry, function.get_return_type())
            )
        }
        ParameterSpecifier::Procedure(procedure) => {
            let parameters = procedure
                .get_parameters()
                .iter()
                .map(|p| render_parameter(visitor, p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("PROCEDURE {}({})", procedure.get_identifier(), parameters)
        }
    }
}

fn user_routine(visitor: &SemanticVisitor, name: &str) -> Option<SignatureInformation> {
    let container = visitor.function_containers.iter().find(|c| c.name.eq_ignore_ascii_case(name))?;
    match &container.functions {
        FunctionDeclaration::Function(function) => {
            let mut builder = SignatureBuilder::new(&format!("FUNCTION {}", function.get_identifier()), "(");
            for parameter in function.get_parameters() {
                let text = render_parameter(visitor, parameter);
                builder.push(&text);
            }
            Some(builder.finish(&format!(") {}", type_name(&visitor.type_registry, function.get_return_type()))))
        }
        FunctionDeclaration::Procedure(procedure) => {
            let mut builder = SignatureBuilder::new(&format!("PROCEDURE {}", procedure.get_identifier()), "(");
            for parameter in procedure.get_parameters() {
                let text = render_parameter(visitor, parameter);
                builder.push(&text);
            }
            Some(builder.finish(")"))
        }
    }
}

fn builtin_functions(name: &str) -> Vec<SignatureInformation> {
    let mut signatures = Vec::new();
    for def in FUNCTION_DEFINITIONS.iter() {
        if !def.name.eq_ignore_ascii_case(name) {
            continue;
        }
        let Some(arguments) = &def.args else {
            continue;
        };
        let mut builder = SignatureBuilder::new(&def.name.to_ascii_uppercase(), "(");
        for argument in arguments {
            let text = format_argument(argument);
            builder.push(&text);
        }
        let return_type = if def.return_type == VariableType::None {
            "MULTITYPE".to_string()
        } else {
            def.return_type.to_string().to_ascii_uppercase()
        };
        signatures.push(builder.finish(&format!(") {return_type}")));
    }
    signatures
}

fn builtin_statement(name: &str) -> Option<SignatureInformation> {
    let def = STATEMENT_DEFINITIONS
        .iter()
        .find(|def| def.name.eq_ignore_ascii_case(name) && def.sig != StatementSignature::Invalid)?;
    let arguments = def.args.as_ref()?;
    let mut builder = SignatureBuilder::new(&def.name.to_ascii_uppercase(), " ");
    for argument in arguments {
        let text = format_argument(argument);
        builder.push(&text);
    }
    Some(builder.finish(""))
}

/// The signature help for the call the cursor is writing arguments for.
pub fn get_signature_help(line_before_cursor: &str, visitor: &SemanticVisitor) -> Option<SignatureHelp> {
    let call = call_context(line_before_cursor)?;

    let signatures = if call.bare {
        builtin_statement(&call.name).into_iter().collect::<Vec<_>>()
    } else {
        let mut signatures = user_routine(visitor, &call.name).into_iter().collect::<Vec<_>>();
        if signatures.is_empty() {
            signatures = builtin_functions(&call.name);
        }
        signatures
    };
    if signatures.is_empty() {
        return None;
    }

    // Of several overloads the one that still has room for this argument fits best.
    let active = signatures
        .iter()
        .position(|signature| signature.parameters.as_ref().is_some_and(|p| p.len() > call.argument))
        .unwrap_or(0);

    Some(SignatureHelp {
        signatures,
        active_signature: Some(active as u32),
        active_parameter: Some(call.argument as u32),
    })
}
