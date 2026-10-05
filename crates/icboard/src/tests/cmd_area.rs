use crate::tests::{setup_conference, test_output};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::message_area::MessageArea;

fn setup_two_areas(board: &mut IcyBoard) {
    setup_conference(board);
    let mut areas = board.conferences[0].areas.as_deref().cloned().unwrap_or_default();
    let dir = areas[0].path.parent().unwrap().to_path_buf();
    areas.push(MessageArea {
        name: "Second".to_string(),
        path: dir.join("second"),
        ..Default::default()
    });
    board.conferences[0].areas = Some(std::sync::Arc::new(areas));
}

#[test]
fn area_reports_the_number_that_was_typed() {
    let output = test_output("AREA 2\n".to_string(), setup_two_areas);
    assert!(output.contains("Second (2) Selected"), "{output}");
}

#[test]
fn area_search_lists_the_numbers_the_command_takes() {
    let output = test_output("AREA S SEC\n2\n".to_string(), setup_two_areas);
    assert!(output.contains("2) "), "{output}");
    assert!(!output.contains("1) "), "{output}");
    assert!(output.contains("Second (2) Selected"), "{output}");
}

#[test]
fn a_ppe_can_pass_area_number_to_the_area_command() {
    let output = test_output("AREA\n".to_string(), |board| {
        setup_two_areas(board);
        // The area menu is a PPE, so it can stuff the number it reads from the object.
        let ppe = crate::tests::compile_test_ppe("KBDSTUFF STRING(Board.Conferences[0].Areas[1].Number) + CHR(13)");
        board.conferences[0].area_menu = ppe.with_extension("");
    });
    assert!(output.contains("Second (2) Selected"), "{output}");
}

/// Typing the smallest `i32` used to overflow while turning it into an index and
/// took the whole session down; it is an invalid area like any other.
#[test]
fn area_rejects_the_smallest_number_without_crashing() {
    let output = test_output("AREA -2147483648\n\n".to_string(), setup_two_areas);
    assert!(output.contains("(-2147483648) is an invalid Area selection!"), "{output}");
}
