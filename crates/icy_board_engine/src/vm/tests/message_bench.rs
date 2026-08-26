//! Timings for the documented message walk. Every PPL message call opens the JAM
//! base on its own, so this measures what that costs over a realistic base.
//!
//! Run with:
//! `cargo test -p icy_board_engine --release --lib message_bench -- --ignored --nocapture`

use std::time::Instant;

use crate::icy_board::{
    conferences::Conference,
    message_area::{AreaList, MessageArea},
};

use super::{run_ppl_on, scratch_dir};

const MESSAGES: usize = 2000;

/// A base with `MESSAGES` messages, kept for the whole run so the timings do not
/// pay for building it again.
fn seeded_base() -> std::path::PathBuf {
    use jamjam::jam::{JamMessage, JamMessageBase};

    static BASE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let path = scratch_dir("bench-base").join("general");
        let mut base = JamMessageBase::create(&path).expect("can't create the bench message base");
        for number in 0..MESSAGES {
            base.write_message(
                &JamMessage::default()
                    .with_from(bstr::BString::from("SYSOP"))
                    .with_to(bstr::BString::from("ALL"))
                    .with_subject(bstr::BString::from(format!("Subject {number}")))
                    .with_date_time(chrono::Utc::now())
                    .with_text(bstr::BString::from("body")),
            )
            .expect("can't write a bench message");
        }
        base.write_jhr_header().unwrap();
        path
    })
    .clone()
}

fn seed(board: &mut crate::icy_board::IcyBoard) {
    board.conferences.clear();
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        areas: Some(std::sync::Arc::new(AreaList::new(vec![MessageArea {
            name: "General".to_string(),
            path: seeded_base(),
            ..Default::default()
        }]))),
        ..Default::default()
    });
}

fn time(label: &str, source: &str) {
    let _ = run_ppl_on(source, seed);
    let start = Instant::now();
    let out = run_ppl_on(source, seed);
    println!("{label}: {} ms  (out {out})", start.elapsed().as_millis());
}

#[test]
#[ignore = "timings, run on demand"]
fn walk_every_message() {
    time(
        "bounds only",
        r#"
AREA area = Board.Conferences[0].Areas[0]
PRINT area.LowMsg(), " ", area.HighMsg()
"#,
    );
    time(
        "documented walk, headers only",
        r"
AREA area = Board.Conferences[0].Areas[0]
INTEGER n
INTEGER found
FOR n = area.LowMsg() TO area.HighMsg()
    MSG msg = area.Read(n)
    IF msg.Valid LET found = found + 1
NEXT
PRINT found
",
    );
    time(
        "walk with the bound hoisted out of the loop",
        r"
AREA area = Board.Conferences[0].Areas[0]
INTEGER n
INTEGER found
INTEGER last = area.HighMsg()
FOR n = area.LowMsg() TO last
    MSG msg = area.Read(n)
    IF msg.Valid LET found = found + 1
NEXT
PRINT found
",
    );
    time(
        "bound re-evaluated, nothing else in the body",
        r"
AREA area = Board.Conferences[0].Areas[0]
INTEGER n
INTEGER found
FOR n = area.LowMsg() TO area.HighMsg()
    LET found = found + 1
NEXT
PRINT found
",
    );
    time(
        "hoisted bound, nothing else in the body",
        r"
AREA area = Board.Conferences[0].Areas[0]
INTEGER n
INTEGER found
INTEGER last = area.HighMsg()
FOR n = area.LowMsg() TO last
    LET found = found + 1
NEXT
PRINT found
",
    );
    time(
        "documented walk, with bodies",
        r"
AREA area = Board.Conferences[0].Areas[0]
INTEGER n
INTEGER total
FOR n = area.LowMsg() TO area.HighMsg()
    MSG msg = area.Read(n)
    IF msg.Valid LET total = total + Len(msg.Text())
NEXT
PRINT total
",
    );
    time(
        "classic GETMSGHDR walk",
        r#"
INTEGER n
INTEGER found
FOR n = 1 TO 2000
    IF (GETMSGHDR(AreaId(0, 0), n, HDR_SUBJ) <> "") LET found = found + 1
NEXT
PRINT found
"#,
    );
    time(
        "Find over the whole base",
        r#"
AREA area = Board.Conferences[0].Areas[0]
PRINT area.Find(MsgField.Subject, "Subject 1999").Number
"#,
    );
}
