; filepath: /home/mkrueger/work/icy_board/crates/tree-sitter-ppl/queries/highlights.scm
; -------- Literals --------
(string_literal)            @string
(number_literal)            @number
(int_number)                @number
(float_number)              @number
(hex_number)                @number
(boolean_literal)           @constant.builtin
(at_color_code)             @constant.character

; -------- Comments --------
(comment)                   @comment

; -------- Types --------
; Since types are now aliased tokens, we need to match them explicitly
[
  "BOOLEAN" "DATE" "DDATE" "INTEGER" "SDWORD"
  "LONG" "MONEY" "STRING" "TIME"
  "BIGSTR" "EDATE" "REAL" "FLOAT" "DREAL"
  "DOUBLE" "UNSIGNED" "DWORD" "UDWORD"
  "BYTE" "UBYTE" "WORD" "UWORD" "SBYTE"
  "SHORT" "SWORD" "INT" "MSGAREAID" "PASSWORD"
] @type.builtin

; Highlight types in variable declarations
(variable_declaration type: (_) @type.builtin)

; Highlight types in parameters
(parameter type: (_) @type.builtin)

; Highlight return types in function declarations
(function_declaration return_type: (_) @type.builtin)
(function_implementation return_type: (_) @type.builtin)

; -------- Structural / block keywords (literal tokens) --------
[
  "IF" "ELSEIF" "ELSE" "ENDIF" "THEN"
  "WHILE" "ENDWHILE" "DO"
  "REPEAT" "UNTIL"
  "LOOP" "ENDLOOP"
  "FOR" "NEXT" "STEP" "TO"
  "SELECT" "CASE" "DEFAULT" "ENDSELECT"
  "FUNCTION" "ENDFUNC"
  "PROCEDURE" "ENDPROC"
  "DECLARE"
  "BEGIN" "END"
  "LET" "VAR"
  "GOTO" "GOSUB"
  "RETURN"  "STOP"
] @keyword.control

; -------- Preprocessor directives --------
(define_directive)        @keyword.directive
(undef_directive)         @keyword.directive
(include_directive)       @keyword.directive
(if_directive)            @keyword.directive
(elif_directive)          @keyword.directive
(else_directive)          @keyword.directive
(endif_directive)         @keyword.directive
(version_directive)       @keyword.directive

(define_directive name: (identifier) @constant.macro)
(undef_directive  name: (identifier) @constant.macro)
(version_directive key: (identifier) @property)
(include_directive path: (string_literal) @string.special)

; -------- Function / procedure declarations --------
(function_declaration        name: (identifier) @function)
(function_implementation     name: (identifier) @function)
(procedure_declaration       name: (identifier) @function.method)
(procedure_implementation    name: (identifier) @function.method)

; -------- Builtin functions / statements --------
; Since builtins are now aliased tokens, match them by their token names
(builtin_function)           @function.builtin
(builtin_statement)          @function.builtin

(predefined_call
  name: (builtin_statement)  @function.builtin)

(call_expression
  function: (builtin_function) @function.builtin)

(call_expression
  function: (identifier)     @function.call)

; -------- Variables / parameters / labels --------
(variable_declaration name: (identifier) @variable)
(parameter           name: (identifier) @variable.parameter)

(for_block_statement var: (identifier) @variable)
(label name: (identifier) @label)

; -------- Member / property access --------
(member_reference member: (identifier) @property)

; -------- Control-flow statements (node-based) --------
(return_statement)    @keyword.return
(break_statement)     @keyword
(continue_statement)  @keyword
(end_statement)       @keyword
(stop_statement)      @keyword
(goto_statement)      @keyword
(gosub_statement)     @keyword

; -------- Operators --------
[
  "=" "+=" "-=" "*=" "/=" "%=" "&=" "|="
  "==" "!=" "<>" "<" ">" "<=" ">="
  "+" "-" "*" "/" "%" "^" "**"
  "&&" "||" "&" "|" "!" "NOT"
] @operator

; -------- Punctuation --------
; Parentheses
"(" @punctuation.bracket
")" @punctuation.bracket

; Delimiters
"," @punctuation.delimiter
":" @punctuation.delimiter
"." @punctuation.delimiter
".." @punctuation.delimiter

; -------- Builtin boolean / null-ish constants --------
; These are now proper tokens from boolean_literal
"TRUE"  @constant.builtin
"FALSE" @constant.builtin

; -------- Fallback: all other identifiers as variables --------
; This must come last to have lowest priority
(identifier) @variable