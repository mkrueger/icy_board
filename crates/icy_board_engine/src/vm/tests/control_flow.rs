//! The branches and loops, which PPLC lays out in two different shapes depending
//! on whether the body is a block or a single statement.

use super::run_ppl;

#[test]
fn test_a_single_statement_while_repeats_its_statement() {
    assert_eq!(run_ppl("INTEGER i\ni = 0\nWHILE (i < 3) i = i + 1\nPRINT i"), "3");
}

#[test]
fn test_a_single_statement_while_skips_a_body_it_never_enters() {
    assert_eq!(run_ppl("INTEGER i\ni = 9\nWHILE (i < 3) i = i + 1\nPRINT i"), "9");
}

#[test]
fn test_a_while_block_repeats_until_its_condition_fails() {
    assert_eq!(run_ppl("INTEGER i\ni = 0\nWHILE (i < 3) DO\n  i = i + 1\nENDWHILE\nPRINT i"), "3");
}

#[test]
fn test_break_leaves_a_while_block() {
    assert_eq!(run_ppl("INTEGER i\ni = 0\nWHILE (TRUE) DO\n  i = i + 1\n  IF (i = 2) BREAK\nENDWHILE\nPRINT i"), "2");
}

#[test]
fn test_continue_goes_back_to_the_condition_of_a_while_block() {
    assert_eq!(
        run_ppl("INTEGER i, n\ni = 0\nn = 0\nWHILE (i < 5) DO\n  i = i + 1\n  IF (i = 3) CONTINUE\n  n = n + 1\nENDWHILE\nPRINT n"),
        "4"
    );
}

#[test]
fn test_a_loop_that_starts_the_program_still_loops() {
    // The label sits at offset zero, which used to read as an undefined label.
    assert_eq!(run_ppl("WHILE (FALSE) PRINT \"no\"\nPRINT \"done\""), "done");
}

#[test]
fn test_a_then_block_runs_only_when_its_condition_holds() {
    assert_eq!(run_ppl("IF (1 = 1) THEN\n  PRINT \"yes\"\nENDIF\nIF (1 = 2) THEN\n  PRINT \"no\"\nENDIF"), "yes");
}

#[test]
fn test_an_elseif_chain_takes_the_first_branch_that_holds() {
    assert_eq!(
        run_ppl("INTEGER i\ni = 2\nIF (i = 1) THEN\n  PRINT \"one\"\nELSEIF (i = 2) THEN\n  PRINT \"two\"\nELSE\n  PRINT \"other\"\nENDIF"),
        "two"
    );
}

#[test]
fn test_an_elseif_chain_falls_through_to_the_else() {
    assert_eq!(
        run_ppl("INTEGER i\ni = 7\nIF (i = 1) THEN\n  PRINT \"one\"\nELSEIF (i = 2) THEN\n  PRINT \"two\"\nELSE\n  PRINT \"other\"\nENDIF"),
        "other"
    );
}

#[test]
fn test_nested_while_blocks_keep_their_own_break_targets() {
    assert_eq!(
        run_ppl(
            "INTEGER i, j, n\nn = 0\nFOR i = 1 TO 3\n  j = 0\n  WHILE (TRUE) DO\n    j = j + 1\n    IF (j = 2) BREAK\n  ENDWHILE\n  n = n + j\nNEXT\nPRINT n"
        ),
        "6"
    );
}
