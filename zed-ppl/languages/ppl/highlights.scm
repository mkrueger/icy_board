; Comments
(comment) @comment

; Literals
(string_literal) @string
(number_literal) @number
(money_literal) @number
(color_code) @string.escape
(boolean_literal) @boolean
(builtin_constant) @constant.builtin

; Types
(builtin_type) @type.builtin
(type_identifier) @type
(type_declaration
  name: (identifier) @type)

; Declarations
(function_declaration
  name: (identifier) @function)
(function_definition
  name: (identifier) @function)
(procedure_declaration
  name: (identifier) @function)
(procedure_definition
  name: (identifier) @function)

(parameter
  name: (identifier) @variable.parameter)
(function_parameter
  name: (identifier) @variable.parameter)
(procedure_parameter
  name: (identifier) @variable.parameter)

; Calls
(builtin_statement) @function @function.builtin
(procedure_call
  name: (identifier) @function)
(call_expression
  function: (identifier) @function)
(call_expression
  function: (member_access
    member: (identifier) @function))

; Members
(field_declaration
  name: (identifier) @property)
(member_access
  member: (identifier) @property)
(record_literal_field
  name: (identifier) @property)

; Labels
(label) @label
(goto_statement
  label: (identifier) @label)
(gosub_statement
  label: (identifier) @label)

; Preprocessor
[
  ";$DEFINE"
  ";$IF"
  ";$ELSEIF"
  ";$ELSE"
  ";$ENDIF"
  ";$USEFUNCS"
] @preproc

(define_directive
  name: (identifier) @constant)
(substitution) @constant

; Keywords
[
  "IF"
  "THEN"
  "ELSE"
  "ELSEIF"
  "ENDIF"
  "SELECT"
  "CASE"
  "DEFAULT"
  "ENDSELECT"
  "WHILE"
  "DO"
  "ENDWHILE"
  "REPEAT"
  "UNTIL"
  "LOOP"
  "ENDLOOP"
  "FOR"
  "TO"
  "STEP"
  "NEXT"
  "ENDFOR"
  "BREAK"
  "CONTINUE"
  "GOTO"
  "GOSUB"
  "RETURN"
  "DECLARE"
  "FUNCTION"
  "PROCEDURE"
  "ENDFUNC"
  "ENDFUNCTION"
  "ENDPROC"
  "ENDPROCEDURE"
  "TYPE"
  "ENDTYPE"
  "LET"
  "VAR"
  "BEGIN"
  "END"
] @keyword

; Operators
[
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "=="
  "!="
  "<>"
  "><"
  "<"
  "<="
  "=<"
  ">"
  ">="
  "=>"
  "+"
  "-"
  "*"
  "/"
  "%"
  "^"
  "&"
  "&&"
  "|"
  "||"
  "!"
  ".."
] @operator

; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
] @punctuation.delimiter

(identifier) @variable
