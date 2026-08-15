; Highlighting for PPL.
;
; Capture names follow the nvim-treesitter set; where Helix spells one
; differently the node carries both names and each editor picks the one it
; knows.

; ---------- Comments ----------
(comment) @comment @comment.line

; ---------- Literals ----------
(string_literal) @string
(number_literal) @number @constant.numeric
(money_literal) @number @constant.numeric
(color_code) @character.special @constant.character.escape
(boolean_literal) @boolean @constant.builtin.boolean
(builtin_constant) @constant.builtin

; ---------- Types ----------
(builtin_type) @type.builtin
(type_identifier) @type

; ---------- Declarations ----------
(type_declaration name: (identifier) @type)
(field_declaration name: (identifier) @variable.member @variable.other.member)

(function_declaration name: (identifier) @function)
(function_definition name: (identifier) @function)
(procedure_declaration name: (identifier) @function)
(procedure_definition name: (identifier) @function)
(function_parameter name: (identifier) @variable.parameter)
(procedure_parameter name: (identifier) @variable.parameter)
(parameter name: (identifier) @variable.parameter)

(variable_declaration
  (variable_declarator name: (identifier) @variable))

; ---------- Calls ----------
(builtin_statement) @function.builtin
(procedure_call name: (identifier) @function.call)
(call_expression function: (identifier) @function.call)
(call_expression
  function: (member_access member: (identifier) @function.method.call))

; ---------- Members ----------
(member_access member: (identifier) @variable.member @variable.other.member)
(record_literal_field name: (identifier) @variable.member @variable.other.member)

; ---------- Loop variables and labels ----------
(for_statement variable: (identifier) @variable)
(for_statement variable_end: (identifier) @variable)
(label) @label
(goto_statement label: (identifier) @label)
(gosub_statement label: (identifier) @label)

; ---------- Preprocessor ----------
[
  ";$DEFINE"
  ";$IF"
  ";$ELSEIF"
  ";$ELSE"
  ";$ENDIF"
  ";$USEFUNCS"
] @keyword.directive @preproc

(define_directive name: (identifier) @constant.macro)
(substitution) @constant.macro

; ---------- Keywords ----------
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
] @keyword.conditional @keyword.control.conditional

[
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
] @keyword.repeat @keyword.control.repeat

[
  "GOTO"
  "GOSUB"
] @keyword.control @keyword.control.jump

"RETURN" @keyword.return @keyword.control.return

[
  "DECLARE"
  "FUNCTION"
  "PROCEDURE"
  "ENDFUNC"
  "ENDFUNCTION"
  "ENDPROC"
  "ENDPROCEDURE"
] @keyword.function

[
  "TYPE"
  "ENDTYPE"
] @keyword.type @keyword.storage.type

[
  "LET"
  "VAR"
  "BEGIN"
  "END"
  "EXIT"
] @keyword

; ---------- Operators ----------
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

; ---------- Punctuation ----------
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

; ---------- Everything else ----------
(identifier) @variable
