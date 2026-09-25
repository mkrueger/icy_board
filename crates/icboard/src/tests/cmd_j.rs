use crate::tests::{compile_test_ppe, fixture, setup_conference, test_output, test_ppe_output, test_user_output};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::commands::{Command, CommandAction, CommandType};
use icy_board_engine::icy_board::conferences::Conference;
use icy_board_engine::icy_board::icb_config::DisplayNewsBehavior;

fn setup_join_files(board: &mut IcyBoard) {
    setup_conference(board);
    board.conferences.resize_with(8, Conference::default);
    board.conferences[7].name = "SEVENTH".to_string();
    board.conferences[7].news_file = fixture("main/blt1");
    board.conferences[7].intro_file = fixture("main/blt2");
    board.config.switches.display_news_behavior = DisplayNewsBehavior::Always;
    board.config.switches.force_intro_on_join = true;
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
fn test_cmd_j_quick_join_by_number_skips_news_and_intro() {
    for command in ["J;Q;7\n", "J;7;Q\n"] {
        let output = test_output(command.to_string(), setup_join_files);
        assert!(output.contains("SEVENTH (7) Joined"), "{command}: {output}");
        assert!(!output.contains("invalid Conference selection"), "{command}: {output}");
        assert!(!output.contains("BULLETIN1"), "{command}: {output}");
        assert!(!output.contains("BULLETIN2"), "{command}: {output}");
    }
}

#[test]
fn a_ppe_command_can_quick_join_without_displaying_conference_files() {
    let output = test_ppe_output("COMMAND TRUE, \"J;Q;7\"", setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(!output.contains("BULLETIN1"), "{output}");
    assert!(!output.contains("BULLETIN2"), "{output}");
}

#[test]
fn test_cmd_j_normal_join_still_displays_news_and_intro() {
    let output = test_output("J;7\n".to_string(), setup_join_files);
    assert!(output.contains("SEVENTH (7) Joined"), "{output}");
    assert!(output.contains("BULLETIN1"), "{output}");
    assert!(output.contains("BULLETIN2"), "{output}");
}

#[test]
fn test_cmd_j_quick_join_by_name_or_prompt_skips_news_and_intro() {
    for command in ["J Q TESTCONF\n", "J Q\n7\n"] {
        let output = test_output(command.to_string(), |board| {
            setup_join_files(board);
            board.conferences[1].news_file = fixture("main/blt1");
            board.conferences[1].intro_file = fixture("main/blt2");
        });
        let joined = if command.contains("TESTCONF") {
            "TESTCONF (1) Joined"
        } else {
            "SEVENTH (7) Joined"
        };
        assert!(output.contains(joined), "{command}: {output}");
        assert!(!output.contains("invalid Conference selection"), "{command}: {output}");
        assert!(!output.contains("BULLETIN1"), "{command}: {output}");
        assert!(!output.contains("BULLETIN2"), "{command}: {output}");
    }
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
