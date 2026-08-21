use crate::tests::{compile_test_ppe, setup_conference, test_output, test_user_output};
use icy_board_engine::icy_board::commands::{Command, CommandAction, CommandType};

#[test]
fn a_mapped_ppe_can_stuff_the_builtin_join_command() {
    let ppe = compile_test_ppe("KBDSTUFF \"J1^M\"");
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
fn test_cmd_j_abandon() {
    let output = test_output("J 1\n\nJ 0\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{1b}[1;32mTESTCONF (1) Joined\n\nPress (Enter) to continue? \u{1b}[0m\r\u{1b}[K\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) TESTCONF (1) Conference Command? \u{1b}[0mJ 0\n\n\u{1b}[1;36mTESTCONF (1) Abandoned\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}
