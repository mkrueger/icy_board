//! dBase opcodes, checked against tables `PCBoard` 15.4 itself wrote.
//!
//! The fixtures in `tests/dbase_data` came out of a real `PCBoard` running under DOS, and
//! the expected strings below are what the original printed for the same operations. PPL
//! hands a PPE the raw padded field bytes, so the padding is part of what is being tested.

use super::{run_ppl, run_ppl_with_files};

const PCBOARD_DBF: &[u8] = include_bytes!("../../../tests/dbase_data/pcboard.dbf");
const PCBOARD_NDX: &[u8] = include_bytes!("../../../tests/dbase_data/pcboard.ndx");
/// A table this engine created and `PCBoard` then read, deleted from and appended to.
const ROUNDTRIP_DBF: &[u8] = include_bytes!("../../../tests/dbase_data/roundtrip.dbf");

fn on_fixture(source: &str) -> String {
    run_ppl_with_files(source, &[("PCBOARD.DBF", PCBOARD_DBF), ("PCBOARD.NDX", PCBOARD_NDX)])
}

#[test]
fn reads_a_table_pcboard_wrote() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        PRINTLN "cnt=", DRECCOUNT(0), " fields=", DFIELDS(0), " alias=", DGETALIAS(0)
        DGO 0, 1
        PRINTLN "r1=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "][", DGET(0, "RATE"), "][", DGET(0, "BIRTH"), "][", DGET(0, "ACTIVE"), "]"
        DGO 0, 2
        PRINTLN "r2=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "][", DGET(0, "RATE"), "][", DGET(0, "BIRTH"), "][", DGET(0, "ACTIVE"), "]"
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "cnt=4 fields=5 alias=PCBOARD\n\
         r1=[ALICE               ][   30][     12.5000][19940527][1]\n\
         r2=[BOB                 ][   41][     -3.2500][20011231][0]\n"
    );
}

#[test]
fn a_successful_dbase_operation_clears_an_older_error_immediately() {
    let output = on_fixture(
        r#"
        Terminal.LoadFont(43, "missing.fnt")
        PRINTLN DOPEN(0, "PCBOARD", 0), ":", Error.Last().Code
        DCLOSE 0
        "#,
    );

    assert_eq!(output, "0:0\n");
}

#[test]
fn the_first_dbase_failure_in_a_statement_wins() {
    let output = on_fixture(
        r#"
        PRINTLN DOPEN(0, "MISSING", 0), ":", DOPEN(1, "PCBOARD", 0), ":", Error.Last().Kind, ":", Error.Last().Code, ":", Error.Last().Channel
        DCLOSE 1
        "#,
    );

    assert_eq!(output, "0:0:2:3:0\n");
}

#[test]
fn reports_the_field_layout() {
    let output = on_fixture(
        r#"
        INTEGER i
        STRING n
        DOPEN 0, "PCBOARD", 0
        FOR i = 1 TO DFIELDS(0)
          n = DNAME(0, i)
          PRINTLN n, ",", DTYPE(0, n), ",", DLENGTH(0, n), ",", DDECIMALS(0, n)
        NEXT
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "NAME,C,20,0\n\
         AGE,N,5,0\n\
         RATE,N,12,4\n\
         BIRTH,D,8,0\n\
         ACTIVE,L,1,0\n"
    );
}

#[test]
fn navigates_the_way_dbase_does() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DTOP 0
        PRINTLN "top=", DRECNO(0), " bof=", DBOF(0)
        DSKIP 0, 2
        PRINTLN "skip=", DRECNO(0)
        DSKIP 0, -1
        PRINTLN "back=", DRECNO(0)
        DBOTTOM 0
        PRINTLN "bottom=", DRECNO(0), " eof=", DEOF(0)
        DSKIP 0, 1
        PRINTLN "past=", DRECNO(0), " eof=", DEOF(0)
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "top=1 bof=0\n\
         skip=3\n\
         back=2\n\
         bottom=4 eof=0\n\
         past=5 eof=1\n"
    );
}

#[test]
fn a_deleted_record_stays_readable() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DGO 0, 3
        PRINTLN "del=", DDELETED(0), " name=[", DGET(0, "NAME"), "]"
        DRECALL 0
        PRINTLN "recall=", DDELETED(0)
        DDELETE 0
        PRINTLN "redel=", DDELETED(0)
        DCLOSE 0
    "#,
    );
    assert_eq!(output, "del=1 name=[CAROL               ]\nrecall=0\nredel=1\n");
}

#[test]
fn packing_drops_the_deleted_records() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DPACK 0
        PRINTLN "cnt=", DRECCOUNT(0)
        DGO 0, 3
        PRINTLN "r3=[", DGET(0, "NAME"), "]"
        DCLOSE 0
    "#,
    );
    assert_eq!(output, "cnt=3\nr3=[DAVE                ]\n");
}

#[test]
fn seeking_matches_a_whole_key_or_a_prefix() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DNOPEN 0, "PCBOARD"
        PRINTLN "exact=", DSEEK(0, "CAROL"), " no=", DRECNO(0)
        PRINTLN "prefix=", DSEEK(0, "BO"), " no=", DRECNO(0)
        PRINTLN "first=", DSEEK(0, "A"), " no=", DRECNO(0)
        PRINTLN "miss=", DSEEK(0, "ZZ"), " no=", DRECNO(0), " eof=", DEOF(0)
        DNCLOSE 0, "PCBOARD"
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "exact=1 no=3\n\
         prefix=1 no=2\n\
         first=1 no=1\n\
         miss=0 no=5 eof=1\n"
    );
}

#[test]
fn an_unknown_field_leaves_the_last_value_and_flags_an_error() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DGO 0, 3
        PRINTLN "good=[", DGET(0, "NAME"), "] err=", DERR(0)
        PRINTLN "bad=[", DGET(0, "NOSUCH"), "] err=", DERR(0)
        DCLOSE 0
    "#,
    );
    assert_eq!(output, "good=[CAROL               ] err=0\nbad=[] err=1\n");
}

#[test]
fn aliases_pick_a_channel() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        PRINTLN "default=", DGETALIAS(0)
        DSETALIAS 0, "PEOPLE"
        PRINTLN "set=", DGETALIAS(0), " sel=", DSELECT("PEOPLE"), " miss=", DSELECT("NOBODY")
        PRINTLN "next=", DNEXT(), " stat=", DCHKSTAT(0)
        DCLOSE 0
    "#,
    );
    assert_eq!(output, "default=PCBOARD\nset=PEOPLE sel=0 miss=8\nnext=1 stat=0\n");
}

#[test]
fn writes_a_table_back_in_pcboards_layout() {
    let output = run_ppl(
        r#"
        STRING f(4)
        f(0) = "NAME,C,20,0"
        f(1) = "AGE,N,5,0"
        f(2) = "RATE,N,12,4"
        f(3) = "BIRTH,D,8,0"
        f(4) = "ACTIVE,L,1,0"
        DCREATE 0, "MADE", 0, f
        DNEW 0
        DPUT 0, "NAME", "ALICE"
        DPUT 0, "AGE", 30
        DPUT 0, "RATE", 12.5
        DPUT 0, "BIRTH", TODDATE(MKDATE(1994, 5, 27))
        DPUT 0, "ACTIVE", TRUE
        DADD 0
        PRINTLN "cnt=", DRECCOUNT(0), " no=", DRECNO(0)
        PRINTLN "r1=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "][", DGET(0, "RATE"), "][", DGET(0, "BIRTH"), "][", DGET(0, "ACTIVE"), "]"
        DCLOSE 0
        DOPEN 0, "MADE", 0
        PRINTLN "reread=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "][", DGET(0, "RATE"), "][", DGET(0, "BIRTH"), "][", DGET(0, "ACTIVE"), "]"
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "cnt=1 no=1\n\
         r1=[ALICE               ][   30][     12.5000][19940527][1]\n\
         reread=[ALICE               ][   30][     12.5000][19940527][1]\n"
    );
}

#[test]
fn a_new_record_does_not_overwrite_the_one_in_hand() {
    let output = on_fixture(
        r#"
        DOPEN 0, "PCBOARD", 0
        DGO 0, 2
        DNEW 0
        DPUT 0, "NAME", "ERIN"
        DPUT 0, "AGE", 55
        DADD 0
        PRINTLN "cnt=", DRECCOUNT(0), " no=", DRECNO(0)
        DGO 0, 2
        PRINTLN "r2=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "]"
        DGO 0, 5
        PRINTLN "r5=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "]"
        DCLOSE 0
    "#,
    );
    assert_eq!(
        output,
        "cnt=5 no=5\n\
         r2=[BOB                 ][   41]\n\
         r5=[ERIN                ][   55]\n"
    );
}

#[test]
fn the_header_it_writes_matches_the_one_pcboard_wrote() {
    use crate::vm::dbase::file::{DbaseFile, parse_field_info};

    let dir = std::env::temp_dir().join(format!("ppl-dbase-header-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("SAME.DBF");

    let fields: Vec<_> = ["NAME,C,20,0", "AGE,N,5,0", "RATE,F,12,4", "BIRTH,D,8,0", "ACTIVE,L,1,0"]
        .iter()
        .map(|spec| parse_field_info(spec).unwrap())
        .collect();
    DbaseFile::create(&path, &fields).unwrap();

    // Everything up to the header terminator: the header itself and one descriptor per
    // field. What follows differs only because the fixture already holds records.
    let header_size = 32 + 32 * fields.len() + 1;
    let mut ours = std::fs::read(&path).unwrap()[..header_size].to_vec();
    let mut theirs = PCBOARD_DBF[..header_size].to_vec();
    // The record count and the date the table was last written will never match.
    ours[1..8].fill(0);
    theirs[1..8].fill(0);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(ours, theirs);
}

#[test]
fn pcboard_can_work_on_a_table_this_engine_wrote() {
    let output = run_ppl_with_files(
        r#"
        DOPEN 0, "ROUNDTRIP", 0
        PRINTLN "cnt=", DRECCOUNT(0)
        DGO 0, 1
        PRINTLN "r1=[", DGET(0, "NAME"), "][", DGET(0, "RATE"), "][", DGET(0, "BIRTH"), "][", DGET(0, "ACTIVE"), "]"
        DGO 0, 2
        PRINTLN "r2=[", DGET(0, "NAME"), "] del=", DDELETED(0)
        DGO 0, 3
        PRINTLN "r3=[", DGET(0, "NAME"), "][", DGET(0, "AGE"), "]"
        DCLOSE 0
    "#,
        &[("ROUNDTRIP.DBF", ROUNDTRIP_DBF)],
    );
    assert_eq!(
        output,
        "cnt=3\n\
         r1=[ZOE                 ][      1.2500][20200101][1]\n\
         r2=[YURI                ] del=1\n\
         r3=[XAVIER              ][    3]\n"
    );
}

#[test]
fn a_memo_field_is_rejected_like_pcboard_rejects_it() {
    let output = run_ppl(
        r#"
        STRING f(1)
        f(0) = "NAME,C,20,0"
        f(1) = "NOTE,M,10,0"
        DCREATE 0, "MEMO", 0, f
        PRINTLN "fields=", DFIELDS(0), " stat=", DCHKSTAT(0)
    "#,
    );
    assert_eq!(output, "fields=0 stat=1\n");
}
