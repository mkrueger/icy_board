use crate::tests::{compile_test_ppe, fixture, setup_conference, test_output, test_ppe_output, test_user_output};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::commands::{Command, CommandAction, CommandType};
use icy_board_engine::icy_board::conferences::Conference;
use icy_board_engine::icy_board::icb_config::DisplayNewsBehavior;

/// Conference 7 with its own news and intro. The news file is newer than the caller's last call.
fn setup_join_files(board: &mut IcyBoard) {
    setup_conference(board);
    board.conferences.resize_with(8, Conference::default);
    board.conferences[7].name = "SEVENTH".to_string();
    board.conferences[7].is_public = true;
    board.conferences[7].news_file = fixture("main/blt1");
    board.conferences[7].intro_file = fixture("main/blt2");
    board.config.switches.display_news_behavior = DisplayNewsBehavior::OnlyNewer;
}

fn setup_forced_join_files(board: &mut IcyBoard) {
    setup_join_files(board);
    board.config.switches.display_news_behavior = DisplayNewsBehavior::Always;
    board.config.switches.force_intro_on_join = true;
}

fn position(output: &str, text: &str) -> usize {
    output.find(text).unwrap_or_else(|| panic!("{text:?} missing from {output}"))
}

#[test]
fn a_mapped_ppe_can_stuff_a_complete_builtin_join_command() {
    let ppe = compile_test_ppe("KBDSTUFF \"J;1^M\"");
    let output = test_user_output("J\n".to_string(), |board| {
        setup_conference(board);
        board.commands.push(Command {
            keyword: "J".to_string(),
            actions: vec![CommandAction {
                command_type: CommandType::RunPPE,
                parameter: ppe.to_string_lossy().into_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });
    });

    assert!(output.contains("TESTCONF (1) Joined"), "{output}");
}

#[test]
fn a_cnfn_ppe_can_stuff_only_the_selected_conference() {
    let ppe = compile_test_ppe("KBDSTUFF \"1^M\"");
    let menu = ppe.with_extension("");
    let output = test_output("J\n".to_string(), |board| {
        setup_conference(board);
        board.config.paths.conf_join_menu = menu.clone();
    });

    assert!(output.contains("TESTCONF (1) Joined"), "{output}");
}

#[test]
fn test_cmd_j_asks_to_view_members_on_the_first_join() {
    let output = test_output("J 1\n\n\n".to_string(), |board| {
        setup_conference(board);
        board.conferences[1].allow_view_conf_members = true;
    });
    assert!(output.contains("View other Conference members"), "{output}");
}

#[test]
fn test_cmd_j_does_not_ask_to_view_members_again() {
    let output = test_output("J 1\n\nJ 0\n\nJ 1\n\n\n".to_string(), |board| {
        setup_conference(board);
        board.conferences[1].allow_view_conf_members = true;
    });
    assert_eq!(output.matches("View other Conference members").count(), 1, "{output}");
}

#[test]
fn test_cmd_j_asks_to_scan_the_message_base() {
    let output = test_output("J 1\nN\n\n\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.disable_message_scan_prompt = false;
    });
    assert!(output.contains("Scan Message Base Since"), "{output}");
}

#[test]
fn test_cmd_j_empty_confs() {
    let output = test_output("J 1\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{7}\u{1b}[1;31mSorry, Sysop, no Conferences are presently available!\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_cmd_j_join() {
    let output = test_output("J 1\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{1b}[1;32mTESTCONF (1) Joined\n\nPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_cmd_j_quick_join_after_the_number_skips_news_and_intro() {
    let output = test_output("J;7;Q\n".to_string(), setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(!output.contains("BULLETIN1"), "{output}");
    assert!(!output.contains("BULLETIN2"), "{output}");
}

/// After Q the digits are part of a conference name, as on `PCBoard` 15.4.
#[test]
fn test_cmd_j_a_number_after_quick_join_is_a_name() {
    let output = test_output("J;Q;7\n\n".to_string(), setup_join_files);
    assert!(output.contains("(7) is an invalid Conference selection!"), "{output}");
    assert!(output.contains("Conference # to join (Enter)=none"), "{output}");
    assert!(!output.contains("SEVENTH (7) Joined"), "{output}");
}

#[test]
fn test_cmd_j_quick_join_alone_is_invalid_and_does_not_carry_over() {
    let output = test_output("J Q\n7\n".to_string(), setup_join_files);
    assert!(output.contains("(Q) is an invalid Conference selection!"), "{output}");
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(output.contains("BULLETIN1"), "{output}");
    assert!(output.contains("BULLETIN2"), "{output}");
}

#[test]
fn test_cmd_j_quick_join_by_name_skips_news_and_intro() {
    let output = test_output("J Q SEVENTH\n".to_string(), setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(!output.contains("BULLETIN1"), "{output}");
    assert!(!output.contains("BULLETIN2"), "{output}");
}

#[test]
fn test_cmd_j_joined_is_shown_before_intro_and_news() {
    let output = test_output("J;7\n".to_string(), setup_join_files);
    let joined = position(&output, "SEVENTH (7) Joined");
    let intro = position(&output, "BULLETIN2");
    let news = position(&output, "BULLETIN1");
    assert!(joined < intro && intro < news, "{output}");
}

#[test]
fn test_cmd_j_news_is_not_repeated_on_a_second_join() {
    let output = test_output("J;7\n\nJ;0\n\nJ;7\n".to_string(), setup_join_files);
    assert_eq!(output.matches("BULLETIN1").count(), 1, "{output}");
    assert_eq!(output.matches("BULLETIN2").count(), 2, "{output}");
}

/// Always showing news and forcing the intro both win over Q.
#[test]
fn test_cmd_j_forced_news_and_intro_ignore_quick_join() {
    let output = test_output("J;7;Q\n\nJ;0\n\nJ;7;Q\n".to_string(), setup_forced_join_files);
    assert_eq!(output.matches("BULLETIN1").count(), 2, "{output}");
    assert_eq!(output.matches("BULLETIN2").count(), 2, "{output}");
}

#[test]
fn a_ppe_command_can_quick_join() {
    let output = test_ppe_output("COMMAND TRUE, \"J;7;Q\"", setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(!output.contains("BULLETIN1"), "{output}");
    assert!(!output.contains("BULLETIN2"), "{output}");
}

#[test]
fn a_ppe_join_can_quick_join_by_number_or_name() {
    for selection in ["\"7;Q\"", "\"SEVENTH;Q\""] {
        let output = test_ppe_output(&format!("JOIN {selection}"), setup_join_files);
        assert!(output.contains("SEVENTH (7) Joined"), "{output}");
        assert!(!output.contains("BULLETIN1"), "{output}");
        assert!(!output.contains("BULLETIN2"), "{output}");
    }
}

#[test]
fn a_ppe_join_without_q_displays_intro_and_news() {
    let output = test_ppe_output("JOIN \"7\"", setup_join_files);
    let joined = position(&output, "SEVENTH (7) Joined");
    let intro = position(&output, "BULLETIN2");
    let news = position(&output, "BULLETIN1");
    assert!(joined < intro && intro < news, "{output}");
}

#[test]
fn a_ppe_join_still_accepts_numeric_arguments() {
    let output = test_ppe_output("JOIN 7", setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(output.contains("BULLETIN1"), "{output}");
    assert!(output.contains("BULLETIN2"), "{output}");
}

#[test]
fn a_ppe_join_can_return_to_main_by_name() {
    let output = test_ppe_output("JOIN \"7;Q\"\nJOIN \"MAIN\"", setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(output.contains("SEVENTH (7) Abandoned"), "{output}");
}

#[test]
fn a_ppe_join_uses_j_selection_rules() {
    let output = test_ppe_output("JOIN \"Q;7\"", setup_join_files);
    assert!(output.contains("(7) is an invalid Conference selection!"), "{output}");
    assert!(!output.contains("SEVENTH (7) Joined"), "{output}");
}

#[test]
fn a_ppe_join_respects_private_conference_access() {
    let output = test_ppe_output("JOIN \"7;Q\"", |board| {
        setup_join_files(board);
        board.conferences[7].is_public = false;
    });
    assert!(output.contains("you are not registered in Conference 7"), "{output}");
    assert!(!output.contains("SEVENTH (7) Joined"), "{output}");
}

#[test]
fn a_caller_cannot_join_an_unregistered_private_conference() {
    let output = test_ppe_output("COMMAND TRUE, \"J;7\"", |board| {
        setup_join_files(board);
        board.conferences[7].is_public = false;
    });
    assert!(output.contains("you are not registered in Conference 7"), "{output}");
    assert!(!output.contains("SEVENTH (7) Joined"), "{output}");
}

#[test]
fn test_cmd_j_search_lists_numbered_matches_by_name() {
    let output = test_output("J;S;E\n\n".to_string(), |board| {
        setup_join_files(board);
        board.conferences[2].name = "ANOTHER".to_string();
    });
    let another = position(&output, "    2) ANOTHER");
    let seventh = position(&output, "    7) SEVENTH");
    let testconf = position(&output, "    1) TESTCONF");
    assert!(another < seventh && seventh < testconf, "{output}");
    assert!(output.contains("Conference # to join (Enter)=none"), "{output}");
}

#[test]
fn test_cmd_j_search_asks_for_text() {
    let output = test_output("J S\nSEVEN\n\n".to_string(), setup_join_files);
    assert!(output.contains("    7) SEVENTH"), "{output}");
    assert!(!output.contains("TESTCONF"), "{output}");
}

#[test]
fn test_cmd_j_r_relists_the_conferences() {
    let output = test_output("J\nR\n\n".to_string(), setup_join_files);
    assert_eq!(output.matches("Conference # to join (Enter)=none").count(), 2, "{output}");
    assert!(!output.contains("invalid Conference selection"), "{output}");
}

#[test]
fn test_cmd_j_an_unnamed_conference_is_invalid() {
    let output = test_output("J 5\n\n".to_string(), setup_join_files);
    assert!(output.contains("(5) is an invalid Conference selection!"), "{output}");
}

#[test]
fn test_cmd_j_abandon() {
    let output = test_output("J 1\n\nJ 0\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{1b}[1;32mTESTCONF (1) Joined\n\nPress (Enter) to continue? \u{1b}[0m\r\u{1b}[K\u{1b}[1;32m\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) TESTCONF (1) Conference Command? \u{1b}[0mJ 0\n\n\u{1b}[1;36mTESTCONF (1) Abandoned\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}
