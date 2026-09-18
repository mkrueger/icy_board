use crate::tests::{fixture, setup_conference, test_login_output, test_ppe_output, test_ppe_output_with_input};
use icy_board_engine::icy_board::icb_config::DisplayNewsBehavior;
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

fn setup_login(board: &mut icy_board_engine::icy_board::IcyBoard, allow_comment: bool) {
    board.config.paths.welcome = crate::tests::fixture("main/blt1");
    board.config.system_control.allow_password_failure_comment = allow_comment;
}

#[test]
fn a_failed_password_can_offer_a_sysop_comment() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\nN\n".to_string(), |board| {
        setup_login(board, true);
    });
    assert!(
        !output.contains("Send a temporary password"),
        "default-off recovery changed the login dialogue:\n{output}"
    );
    assert!(
        output.contains("leave a comment to the sysop"),
        "the password failure comment was not offered:\n{output}"
    );
}

#[test]
fn a_failed_password_does_not_offer_a_comment_when_disabled() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\n".to_string(), |board| {
        setup_login(board, false);
    });
    assert!(
        !output.contains("Send a temporary password"),
        "default-off recovery changed the login dialogue:\n{output}"
    );
    assert!(
        !output.contains("leave a comment to the sysop"),
        "the password failure comment was offered:\n{output}"
    );
}

#[test]
fn a_direct_ppe_has_only_its_output_and_the_completion_prompt() {
    let output = test_ppe_output("PRINT \"PPE ONLY\"", |board| {
        setup_conference(board);
        board.conferences[0].news_file = fixture("main/blt1");
        board.config.paths.welcome = fixture("main/blt2");
        board.config.switches.display_news_behavior = DisplayNewsBehavior::Always;
        board.config.switches.scan_new_blt = true;
        board.config.password_recovery.enabled = true;
    });

    assert_eq!(output, format!("PPE ONLY\n{}\n", icy_board_tui::get_text("run_ppe_completed")));
}

/// The lines of an 80x25 screen after the board's own ANSI has been replayed into it.
fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
}

#[test]
fn a_long_input_field_keeps_the_cursor_on_its_prompt_line() {
    let output = test_ppe_output_with_input("STRING name\nINPUT \"What is your name? \", name", "test\r", |_| {});

    let lines = rendered_lines(&output);
    assert!(lines[0].contains("What is your name? ? (test"), "{:?}", lines[0]);
    assert!(!lines[1].contains("test"), "{:?}", lines[1]);
}

#[test]
fn a_default_answer_stays_inside_a_clamped_field() {
    // A prompt of this width keeps the field delimiters but leaves fewer than the
    // sixty columns the field asks for, so the field itself is clamped.
    let prompt = "x".repeat(30);
    let default = "y".repeat(60);
    let output = test_ppe_output_with_input(&format!("STRING name\nname = \"{default}\"\nINPUT \"{prompt}\", name"), "\r", |_| {});

    let lines = rendered_lines(&output);
    assert!(lines[0].contains(')'), "the field delimiters were dropped: {:?}", lines[0]);
    // A run, not a single letter: the completion prompt below is translated and
    // any single letter may well occur in it.
    assert!(!lines[1].contains("yyy"), "the default answer wrapped onto the next line: {:?}", lines[1]);
}

#[test]
fn a_local_session_knows_that_it_is_local() {
    // A session that does not know it is local runs into the keyboard timeout.
    let output = test_ppe_output("PRINT \"LOCAL=\", ONLOCAL()", |_| {});

    assert!(output.starts_with("LOCAL=1"), "{output:?}");
}

/// The console keeps its last row for the status bar, so the board is given the rows
/// above them and scrolls there rather than writing behind the bar.
#[test]
fn a_local_session_gets_the_rows_the_status_bar_leaves() {
    let output = test_ppe_output(";$LANGVERSION 400\nPRINT \"SIZE=\", Terminal.Info.Columns, \"x\", Terminal.Info.Rows", |_| {});

    assert!(output.starts_with("SIZE=80x24"), "{output:?}");
}

#[test]
fn login_ppe_display_matches_pcboard_local_paging() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("LOGIN.PCB");
    for newlines in [20, 21, 22, 23, 24, 25, 46] {
        let mut display = b"@CLS@@X07\r\n\r\n\r\n\r\n\r\n\r\n\r\n".to_vec();
        for (index, label) in ["", "LOGIN.AS.:", "PASSWORD.:", "", "FUNCTION.:", "LAST.SEEN:", "", "", ""].iter().enumerate() {
            display.extend_from_slice(b"@X08                                                  ");
            if matches!(index, 0 | 3 | 6 | 8) {
                display.push(if index == 0 {
                    0xDA
                } else if index == 8 {
                    0xC0
                } else {
                    0xC3
                });
                display.extend_from_slice(&[0xC4; 25]);
                display.push(if index == 0 {
                    0xBF
                } else if index == 8 {
                    0xD9
                } else {
                    0xB4
                });
            } else {
                display.push(0xB3);
                display.extend_from_slice(format!("@X07{label:25}@X08").as_bytes());
                display.push(0xB3);
            }
            display.extend_from_slice(b"\r\n");
        }
        display.extend_from_slice("\r\n".repeat(newlines - 16).as_bytes());
        display.extend_from_slice(b"@X07");
        if newlines <= 22
            && let Some(fixtures) = std::env::var_os("ICB_LOGIN_PAGING_FIXTURES")
        {
            display = std::fs::read(std::path::PathBuf::from(fixtures).join(format!("R7-LOGIN{}.pcb", newlines + 1))).unwrap();
            assert_eq!(display.windows(2).filter(|bytes| *bytes == b"\r\n").count(), newlines);
        }
        std::fs::write(&path, display).unwrap();
        let output = test_ppe_output_with_input(
            &format!(
                ";$LANGVERSION 320\nDISPFILE \"{}\", 0\nPRINT \"[counter=\", LPRINTED(), \"]Username:\"\nPRINT \"@POFF@\"",
                path.display()
            ),
            &"\r".repeat(newlines / 23),
            |board| {
                board.users[0].page_len = 24;
                board
                    .default_display_text
                    .update_record_number(icy_board_engine::icy_board::icb_text::IceText::MorePrompt as usize, "[login-more]")
                    .unwrap();
            },
        );
        assert!(output.contains(&format!("[counter={}]Username:", newlines % 23)), "{newlines}: {output:?}");
        if newlines < 23 {
            assert!(!output.contains("[login-more]"), "{newlines}: {output:?}");
            let mut screen = TextScreen::new((80, 24));
            screen.buffer.buffer_type = icy_engine::BufferType::Unicode;
            let login = &output[..output.find("Username:").unwrap() + "Username:".len()];
            AnsiParser::default().parse(login.replace('\n', "\r\n").as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
            let row = |row| (0..80).map(|column| screen.char_at((column, row).into()).ch).collect::<String>();
            assert!(row(8).contains("LOGIN.AS.:"), "{newlines}: {:?}", row(8));
            assert!(row(newlines as i32).contains("Username:"), "{newlines}: {:?}", row(newlines as i32));
        }
    }
}
