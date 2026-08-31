; Scopes, definitions and references for PPL.

(module_declaration) @local.scope
(module_declaration name: (identifier) @local.definition.namespace)
(import_declaration alias: (identifier) @local.definition.namespace)

(function_definition) @local.scope
(procedure_definition) @local.scope

(parameter name: (identifier) @local.definition.parameter)
(function_parameter name: (identifier) @local.definition.parameter)
(procedure_parameter name: (identifier) @local.definition.parameter)

(variable_declarator name: (identifier) @local.definition.var)
(const_declaration name: (identifier) @local.definition.constant)
(field_declaration name: (identifier) @local.definition.field)

(type_declaration name: (identifier) @local.definition.type)
(enum_declaration name: (identifier) @local.definition.type)
(enum_variant name: (identifier) @local.definition.constant)
(function_definition name: (identifier) @local.definition.function)
(procedure_definition name: (identifier) @local.definition.function)
(function_declaration name: (identifier) @local.definition.function)
(procedure_declaration name: (identifier) @local.definition.function)

(label) @local.definition.label

(identifier) @local.reference
