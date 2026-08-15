; Indentation.
;
; Written for both dialects: Neovim reads @indent.begin/@indent.end/
; @indent.branch, Helix reads @indent/@outdent. Each ignores the other's names.

[
  (function_definition)
  (procedure_definition)
  (type_declaration)
  (if_block)
  (elseif_clause)
  (else_clause)
  (while_block)
  (repeat_statement)
  (loop_statement)
  (for_statement)
  (select_statement)
  (case_clause)
  (default_clause)
  (block)
] @indent.begin @indent

[
  "ELSE"
  "ELSEIF"
  "CASE"
  "DEFAULT"
  "UNTIL"
] @indent.branch @outdent

[
  "ENDIF"
  "ENDWHILE"
  "ENDLOOP"
  "ENDSELECT"
  "ENDTYPE"
  "ENDFOR"
  "ENDFUNC"
  "ENDFUNCTION"
  "ENDPROC"
  "ENDPROCEDURE"
  "NEXT"
] @indent.end @outdent

; Only the END that closes a block, never a plain END statement.
(block "END" @indent.end @outdent)

[
  ")"
  "]"
  "}"
] @indent.end @outdent
