; filepath: /home/mkrueger/work/icy_board/crates/tree-sitter-ppl/queries/locals.scm
; Scopes
(function_implementation) @scope
(procedure_implementation) @scope
(block_statement) @scope
(if_block_statement) @scope
(for_block_statement) @scope
(while_block_statement) @scope

; Definitions
(variable_declaration name: (identifier) @definition.var)
(parameter name: (identifier) @definition.parameter)
(function_implementation name: (identifier) @definition.function)
(procedure_implementation name: (identifier) @definition.function)
(label name: (identifier) @definition.label)

; References
(identifier) @reference