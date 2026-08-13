; Text objects.
;
; Helix reads the `.inside`/`.around` names, Neovim the `.inner`/`.outer` ones,
; so both are given.

(function_definition) @function.around @function.outer
(procedure_definition) @function.around @function.outer
(function_declaration) @function.around @function.outer
(procedure_declaration) @function.around @function.outer

(type_declaration) @class.around @class.outer
(field_declaration) @entry.around

(parameter) @parameter.inside @parameter.inner
(function_parameter) @parameter.inside @parameter.inner
(procedure_parameter) @parameter.inside @parameter.inner

(argument_list
  (_) @parameter.inside @parameter.inner)

(record_literal_field) @entry.around

(comment) @comment.inside @comment.inner
(comment)+ @comment.around @comment.outer
