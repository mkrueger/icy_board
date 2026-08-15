(_
  "("
  ")" @end) @indent

(_
  "["
  "]" @end) @indent

(_
  "{"
  "}" @end) @indent

(function_definition
  [
    "ENDFUNC"
    "ENDFUNCTION"
  ] @end) @indent

(procedure_definition
  [
    "ENDPROC"
    "ENDPROCEDURE"
  ] @end) @indent

(type_declaration
  "ENDTYPE" @end) @indent

(if_block
  "ENDIF" @end) @indent

(while_block
  "ENDWHILE" @end) @indent

(loop_statement
  "ENDLOOP" @end) @indent

(repeat_statement
  "UNTIL" @end) @indent

(select_statement
  "ENDSELECT" @end) @indent

(for_statement
  [
    "NEXT"
    "ENDFOR"
  ] @end) @indent

(case_clause) @indent

(default_clause) @indent

; A branch keyword belongs on the same column as the block it splits.
[
  "ELSE"
  "ELSEIF"
  "CASE"
  "DEFAULT"
] @outdent
