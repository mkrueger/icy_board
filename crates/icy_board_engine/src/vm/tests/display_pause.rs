//! What counts towards the MORE prompt.
//!
//! PCBoard counted a line in `newline()` and nowhere else, so a PPE that draws its own
//! screen with `PRINT` and cursor positioning never ran into a MORE prompt halfway
//! through. See newline() and print() in DISPLAY.C.

use super::run_ppl;

#[test]
fn print_does_not_count_the_line_breaks_it_writes() {
    let output = run_ppl(
        r#"
        PRINT "one", CHR(13) + CHR(10), "two", CHR(13) + CHR(10)
        PRINTLN "lines=", LPRINTED()
    "#,
    );
    assert_eq!(output, "one\ntwo\nlines=0\n");
}

#[test]
fn println_counts_a_line() {
    let output = run_ppl(
        r#"
        PRINTLN "one"
        PRINTLN "two"
        PRINT "lines=", LPRINTED()
    "#,
    );
    assert_eq!(output, "one\ntwo\nlines=2");
}

/// A screen a PPE draws with `PRINT` stays below the page length however long it is.
#[test]
fn a_screen_drawn_with_print_never_reaches_the_page_length() {
    let output = run_ppl(
        r#"
        INTEGER i
        FOR i = 1 TO 40
            PRINT "line", CHR(13) + CHR(10)
        NEXT
        PRINT "lines=", LPRINTED()
    "#,
    );
    assert!(output.ends_with("lines=0"), "{output}");
}

/// A PPE that turns the pause off and clears the screen afterwards - the way a menu PPE
/// starts - stays in non stop mode. Clearing the screen starts the page over, it does not
/// turn counting back on.
#[test]
fn clearing_the_screen_does_not_undo_pause_off() {
    let output = run_ppl(
        r#"
        PRINT "@POFF@@CLS@"
        PRINTLN "one"
        PRINTLN "two"
        PRINT "nonstop=", ISNONSTOP(), " lines=", LPRINTED()
    "#,
    );
    assert!(output.ends_with("nonstop=1 lines=0"), "{output}");
}

#[test]
fn pause_on_counts_again() {
    let output = run_ppl(
        r#"
        PRINT "@POFF@"
        PRINTLN "one"
        PRINT "@PON@"
        PRINTLN "two"
        PRINT "nonstop=", ISNONSTOP(), " lines=", LPRINTED()
    "#,
    );
    assert!(output.ends_with("nonstop=0 lines=1"), "{output}");
}
