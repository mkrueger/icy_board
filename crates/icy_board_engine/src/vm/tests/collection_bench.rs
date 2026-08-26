//! Timings for walking a conference's areas. The rows are paired so that an indexed
//! loop and a FOREACH are only ever compared over the *same* source expression: the
//! cost is dominated by how long that expression is, because a walk evaluates it once
//! for the count and once for the element on every step.
//!
//! Run with:
//! `cargo test -p icy_board_engine --release --lib collection_bench -- --ignored --nocapture`

use crate::icy_board::{
    conferences::Conference,
    message_area::{AreaList, MessageArea},
};
use crate::vm::tests::run_ppl_on;
use std::time::Instant;

const AREAS: usize = 2000;

/// How many conferences the board carries besides the one being walked. `Board` is
/// rebuilt on every access, so this is what that costs.
const EXTRA_CONFERENCES: usize = 0;

// The extra conferences are a knob to turn while measuring, so the range is empty
// as it stands.
#[allow(clippy::reversed_empty_ranges)]
fn seed(board: &mut crate::icy_board::IcyBoard) {
    let areas: Vec<MessageArea> = (0..AREAS)
        .map(|i| MessageArea {
            name: format!("Area number {i}"),
            ..Default::default()
        })
        .collect();
    board.conferences.clear();
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        areas: Some(std::sync::Arc::new(AreaList::new(areas))),
        ..Default::default()
    });
    for i in 0..EXTRA_CONFERENCES {
        board.conferences.push(Conference {
            name: format!("Conference {i}"),
            ..Default::default()
        });
    }
}

fn time(label: &str, source: &str) {
    // Once to warm the board up, then the one that counts.
    let _ = run_ppl_on(source, seed);
    let start = Instant::now();
    let out = run_ppl_on(source, seed);
    println!("{label}: {} us  (out {out})", start.elapsed().as_micros());
}

#[test]
#[ignore = "timings, run on demand"]
fn walk_every_area() {
    time(
        "indexed loop, conference held in a variable",
        r#"
CONFERENCE c = Board.Conferences[0]
INTEGER i
INTEGER n
FOR i = 0 TO c.Areas.Count - 1
    IF (c.Areas[i].Name <> "") LET n = n + 1
NEXT
PRINT n
"#,
    );
    time(
        "indexed loop, source is the long chain",
        r#"
INTEGER i
INTEGER n
FOR i = 0 TO Board.Conferences[0].Areas.Count - 1
    IF (Board.Conferences[0].Areas[i].Name <> "") LET n = n + 1
NEXT
PRINT n
"#,
    );
    time(
        "FOREACH, source is the long chain",
        r#"
INTEGER n
AREA a
FOREACH a IN Board.Conferences[0].Areas
    IF (a.Name <> "") LET n = n + 1
ENDFOREACH
PRINT n
"#,
    );
    time(
        "FOREACH, conference held in a variable",
        r#"
CONFERENCE c = Board.Conferences[0]
INTEGER n
AREA a
FOREACH a IN c.Areas
    IF (a.Name <> "") LET n = n + 1
ENDFOREACH
PRINT n
"#,
    );
    time(
        "FOREACH, collection held in a variable",
        r#"
AREAS list = Board.Conferences[0].Areas
INTEGER n
AREA a
FOREACH a IN list
    IF (a.Name <> "") LET n = n + 1
ENDFOREACH
PRINT n
"#,
    );
}

/// A conference is a much bigger record than an area, so handing one out is where a
/// walk would notice a copy.
#[test]
#[ignore = "timings, run on demand"]
fn walk_every_conference() {
    time(
        "FOREACH over the conferences",
        r#"
CONFERENCES list = Board.Conferences
INTEGER n
CONFERENCE c
FOREACH c IN list
    IF (c.Name <> "") LET n = n + 1
ENDFOREACH
PRINT n
"#,
    );
}

#[test]
fn a_collection_reports_its_count_and_indexes() {
    let out = run_ppl_on(
        r#"
CONFERENCE c = Board.Conferences[0]
PRINT c.Areas.Count, " ", c.Areas[0].Name, " ", c.Areas[1999].Name
"#,
        seed,
    );
    assert_eq!("2000 Area number 0 Area number 1999", out);
}

#[test]
fn foreach_walks_a_collection() {
    let out = run_ppl_on(
        r"
INTEGER n
AREA a
FOREACH a IN Board.Conferences[0].Areas
    LET n = n + 1
ENDFOREACH
PRINT n
",
        seed,
    );
    assert_eq!("2000", out);
}
