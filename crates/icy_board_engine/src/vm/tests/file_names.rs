//! How a PPE's file names are read.

use super::run_ppl_with_files;

const CONTENT: &[u8] = b"the file was found\r\n";

/// A PPE that pulls a name out of a fixed width record, which `MID` pads out to the
/// width it asked for, still hands PCBoard a name it opens.
#[test]
fn a_file_name_padded_out_by_mid_still_finds_the_file() {
    let output = run_ppl_with_files(
        r#"
        STRING name
        LET name = MID("Header=found.pcb", 8, 255)
        PRINTLN "len=", LEN(name)
        PRINTLN "exist=", EXIST(name)
        PRINTLN "size=", FILEINF(name, 4)
        DISPFILE name, 0
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(output, "len=255\nexist=1\nsize=20\nthe file was found\n");
}

#[test]
fn a_padded_name_opens_on_a_file_channel() {
    let output = run_ppl_with_files(
        r#"
        STRING line
        FOPEN 1, "found.pcb     ", 0, 0
        FGET 1, line
        PRINTLN "err=", FERR(1), " [", line, "]"
        FCLOSE 1
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(output, "err=0 [the file was found]\n");
}

/// DISPSTR takes the same file specs as any other display line, so a leading
/// `%` names a file to show rather than text to print.
#[test]
fn dispstr_shows_the_file_a_percent_names() {
    let output = run_ppl_with_files(
        r#"
        DISPSTR "%found.pcb"
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(output, "the file was found\n");
}

#[test]
fn dispstr_prints_a_string_that_names_no_file() {
    let output = run_ppl_with_files(r#"DISPSTR "plain text""#, &[]);
    assert_eq!(output, "plain text");
}

/// The words after the file name are arguments, not part of the name.
#[test]
fn dispstr_hands_the_words_after_the_name_over_as_tokens() {
    let output = run_ppl_with_files(
        r#"
        DISPSTR "%found.pcb ONE TWO"
        PRINTLN "tokens=", TOKCOUNT()
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(output, "the file was found\ntokens=2\n");
}

/// PCBoard zeroed its find record for a file that is not there, so asking for the size
/// of one answers 0. Boards are full of PPEs that write `EXIST(f) & FILEINF(f,4)`.
#[test]
fn fileinf_of_a_file_that_is_not_there_answers_zero() {
    let output = run_ppl_with_files(
        r#"
        PRINTLN "size=", FILEINF("missing.pcb", 4)
        PRINTLN "still running"
    "#,
        &[],
    );
    assert_eq!(output, "size=0\nstill running\n");
}

/// FILEINF hands out the name without its extension and the extension without its dot.
/// PCBoard uppercased the path string before splitting (EVALP.CPP TOK_OP_FILEINF).
#[test]
fn fileinf_splits_a_name_the_way_pcboard_did() {
    let output = run_ppl_with_files(
        r#"
        PRINTLN "name=", FILEINF("found.pcb", 8), " ext=", FILEINF("found.pcb", 9)
        PRINTLN "drive=", FILEINF("D:\FOO\BAR.DAT", 6)
        PRINTLN "path=", FILEINF("D:\FOO\BAR.DAT", 7)
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(output, "name=FOUND ext=PCB\ndrive=D\npath=\\FOO\\\n");
}

/// FILEINF date/time/size/attrs come from the file; a missing file zeroes them
/// (PCBoard zeroed the find block). Printed DATE/TIME of 0 show as the type defaults.
#[test]
fn fileinf_date_time_and_attrs_are_real_for_existing_files() {
    let output = run_ppl_with_files(
        r#"
        PRINTLN "size=", FILEINF("found.pcb", 4)
        PRINTLN "attr0=", FILEINF("missing.pcb", 5)
        PRINTLN "date_ok=", FILEINF("found.pcb", 2) > 0
        PRINTLN "time_ok=", FILEINF("found.pcb", 3) >= 0
        PRINTLN "attr=", FILEINF("found.pcb", 5)
        PRINTLN "miss_date_zero=", FILEINF("missing.pcb", 2) = 0
        PRINTLN "miss_time_zero=", FILEINF("missing.pcb", 3) = 0
    "#,
        &[("found.pcb", CONTENT)],
    );
    assert_eq!(
        output,
        "size=20\nattr0=0\ndate_ok=1\ntime_ok=1\nattr=32\nmiss_date_zero=1\nmiss_time_zero=1\n"
    );
}

/// PCBoard looked for files with dosfindfirst and never passed FA_DIREC, so a directory
/// answers like a name that is not there. Verified against PCBoard 15.4/M.
#[test]
fn fileinf_reads_a_directory_as_nothing() {
    let output = run_ppl_with_files(
        r#"
        PRINTLN "exist=", FILEINF("sub", 1)
        PRINTLN "attr=", FILEINF("sub", 5)
        PRINTLN "size=", FILEINF("sub", 4)
    "#,
        &[("sub/inner.txt", CONTENT)],
    );
    assert_eq!(output, "exist=0\nattr=0\nsize=0\n");
}

/// PCBoard printed such a line as it stood when the file behind it was not there -
/// displayfile() falls through to printxlated() when runscriptwithparams() finds nothing.
#[test]
fn a_line_naming_a_file_that_is_not_there_is_printed() {
    let output = run_ppl_with_files(r#"DISPSTR "%missing.pcb""#, &[]);
    assert_eq!(output, "%missing.pcb");
}

#[test]
fn a_line_naming_a_ppe_that_is_not_there_is_printed() {
    let output = run_ppl_with_files(r#"DISPSTR "!C:\TEMP\TOP.PPE""#, &[]);
    assert_eq!(output, "!C:\\TEMP\\TOP.PPE");
}

/// The names in such a line are the ones the sysop's DOS drive had.
#[test]
fn a_dos_path_in_a_display_line_finds_the_file() {
    let output = run_ppl_with_files(r#"DISPSTR "%C:\FOUND.PCB""#, &[("found.pcb", CONTENT)]);
    assert_eq!(output, "the file was found\n");
}
