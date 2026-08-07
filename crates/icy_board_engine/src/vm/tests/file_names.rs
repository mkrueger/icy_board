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
