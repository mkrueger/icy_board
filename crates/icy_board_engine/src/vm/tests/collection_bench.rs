//! Timings kept while the collection API is being built, so the old pair and the
//! new walk can be compared on the same board. Run with:
//! `cargo test -p icy_board_engine --release --lib collection_bench -- --nocapture`

use crate::icy_board::{
    conferences::Conference,
    message_area::{AreaList, MessageArea},
};
use crate::vm::tests::run_ppl_on;
use std::time::Instant;

const AREAS: usize = 2000;

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
}

fn time(label: &str, source: &str) {
    // Once to warm the board up, then the one that counts.
    let _ = run_ppl_on(source, seed);
    let start = Instant::now();
    let out = run_ppl_on(source, seed);
    println!("{label}: {} ms  (out {out})", start.elapsed().as_millis());
}

#[test]
#[ignore = "timings, run on demand"]
fn walk_every_area() {
    time(
        "held in a variable, Count + GetArea(i)",
        r#"
CONFERENCE c = Board.GetConference(0)
INTEGER i
INTEGER n
FOR i = 0 TO c.Areas.Count - 1
    IF (c.Areas[i].Name <> "") LET n = n + 1
NEXT
PRINT n
"#,
    );
    time(
        "reached through the board each step",
        r#"
INTEGER i
INTEGER n
FOR i = 0 TO Board.GetConference(0).Areas.Count - 1
    IF (Board.GetConference(0).Areas[i].Name <> "") LET n = n + 1
NEXT
PRINT n
"#,
    );
    time(
        "FOREACH over the collection",
        r#"
INTEGER n
AREA a
FOREACH a IN Board.GetConference(0).Areas
    IF (a.Name <> "") LET n = n + 1
ENDFOREACH
PRINT n
"#,
    );
}

#[test]
fn a_collection_reports_its_count_and_indexes() {
    let out = run_ppl_on(
        r#"
CONFERENCE c = Board.GetConference(0)
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
FOREACH a IN Board.GetConference(0).Areas
    LET n = n + 1
ENDFOREACH
PRINT n
",
        seed,
    );
    assert_eq!("2000", out);
}
