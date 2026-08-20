//! Where an imported PPE's file names lead.
//!
//! Such a PPE still carries the paths of the DOS installation it came from, so a path
//! that leads nowhere is looked for below the PPE's own directory and below the board.
//! A name without a directory is not - that one is the board's, the way `PCBoard` read it
//! from its own directory.

use super::run_ppl_in_ppe_dir;

const CONTENT: &[u8] = b"data\r\n";

/// GREED does `DELETE "GREED.TMP"`, which on `PCBoard` removed a file in the board
/// directory and never the one the PPE displays afterwards.
#[test]
fn a_bare_name_does_not_delete_the_file_next_to_the_ppe() {
    let output = run_ppl_in_ppe_dir(
        r#"
        DELETE "GREED.TMP"
        PRINTLN "kept=", EXIST("ppe/greed2/greed.tmp")
    "#,
        "ppe/greed2",
        &[("ppe/greed2/greed.tmp", CONTENT)],
    );
    assert_eq!(output, "kept=1\n");
}

#[test]
fn a_bare_name_is_not_taken_from_the_ppe_directory() {
    let output = run_ppl_in_ppe_dir(
        r#"
        PRINTLN "exist=", EXIST("GREED.TMP")
    "#,
        "ppe/greed2",
        &[("ppe/greed2/greed.tmp", CONTENT)],
    );
    assert_eq!(output, "exist=0\n");
}

#[test]
fn a_dos_path_below_the_ppe_directory_is_found() {
    let output = run_ppl_in_ppe_dir(
        r#"
        PRINTLN "exist=", EXIST("C:\PCB\PPE\GREED2\CFG\APPLY.CFG")
    "#,
        "ppe/greed2",
        &[("ppe/greed2/cfg/apply.cfg", CONTENT)],
    );
    assert_eq!(output, "exist=1\n");
}

/// One PPE reading another one's data directory, the way the stats PPE reads the
/// work directory of PNS-TOP.
#[test]
fn a_dos_path_into_another_ppe_directory_is_found() {
    let output = run_ppl_in_ppe_dir(
        r#"
        PRINTLN "exist=", EXIST("C:\PCB\PPE\PNS-TOP\WORK\UPLOAD.PAG")
    "#,
        "ppe/stats",
        &[("ppe/pns-top/work/upload.pag", CONTENT)],
    );
    assert_eq!(output, "exist=1\n");
}

/// A log the PPE writes does not exist yet, so nothing can be found - it belongs
/// next to the PPE rather than in a DOS directory that is not there.
#[test]
fn a_dos_path_for_a_new_file_lands_next_to_the_ppe() {
    let output = run_ppl_in_ppe_dir(
        r#"
        FCREATE 1, "C:\PCB\PPE\GREED2\NEW.LOG", 1, 0
        FPUTLN 1, "x"
        FCLOSE 1
        PRINTLN "written=", EXIST("ppe/greed2/NEW.LOG")
    "#,
        "ppe/greed2",
        &[],
    );
    assert_eq!(output, "written=1\n");
}

/// Without a drive there is nothing to rewrite, so a relative directory that does not
/// exist stays what the PPE asked for instead of turning into a file of the PPE.
#[test]
fn a_relative_directory_that_is_not_there_finds_nothing() {
    let output = run_ppl_in_ppe_dir(
        r#"
        PRINTLN "exist=", EXIST("work\greed.tmp")
    "#,
        "ppe/greed2",
        &[("ppe/greed2/greed.tmp", CONTENT)],
    );
    assert_eq!(output, "exist=0\n");
}
