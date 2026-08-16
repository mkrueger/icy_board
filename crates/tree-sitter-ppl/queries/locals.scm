; Scopes, definitions and references for PPL.

(function_definition) @local.scope
(procedure_definition) @local.scope

(parameter name: (identifier) @local.definition.parameter)
(function_parameter name: (identifier) @local.definition.parameter)
(procedure_parameter name: (identifier) @local.definition.parameter)

(variable_declarator name: (identifier) @local.definition.var)
(const_declaration name: (identifier) @local.definition.constant)
(field_declaration name: (identifier) @local.definition.field)

(type_declaration name: (identifier) @local.definition.type)
(function_definition name: (identifier) @local.definition.function)
(procedure_definition name: (identifier) @local.definition.function)
(function_declaration name: (identifier) @local.definition.function)
(procedure_declaration name: (identifier) @local.definition.function)

(label) @local.definition.label

(identifier) @local.reference
