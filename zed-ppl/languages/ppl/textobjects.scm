(function_definition
  body: (_)* @function.inside) @function.around

(procedure_definition
  body: (_)* @function.inside) @function.around

(function_declaration) @function.around
(procedure_declaration) @function.around

(type_declaration
  (field_declaration)* @class.inside) @class.around

(comment)+ @comment.around
