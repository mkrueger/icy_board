//! What a conference, area, directory and door tell a PPE about themselves.

use crate::icy_board::{
    bulletins::{Bullettin, BullettinList},
    conferences::Conference,
    doors::{Door, DoorList},
    file_directory::{DirectoryList, FileDirectory},
    message_area::{AreaList, MessageArea},
    security_expr::SecurityExpression,
    surveys::{Survey, SurveyList},
    user_base::Password,
};

use super::{compile_errors, run_ppl_on, run_ppl_seeded, run_ppl_with_messages};

/// One conference carrying one of everything, configured so each answer differs
/// from the default it would otherwise report.
fn seed_conference(board: &mut crate::icy_board::IcyBoard) {
    board.conferences.clear();
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        is_public: true,
        is_read_only: true,
        allow_aliases: true,
        echo_mail_in_conference: true,
        auto_rejoin: true,
        private_uploads: true,
        password: Password::PlainText("joinme".to_string()),
        sec_write_message: SecurityExpression::from_req_security(10),
        areas: Some(std::sync::Arc::new(AreaList::new(vec![MessageArea {
            name: "General".to_string(),
            is_read_only: true,
            allow_aliases: true,
            qwk_name: "GENERAL".to_string(),
            ftn_area_tag: "FIDO.GENERAL".to_string(),
            ftn_origin: "Just this area".to_string(),
            req_level_to_save_attach: SecurityExpression::from_req_security(20),
            ..Default::default()
        }]))),
        directories: Some(std::sync::Arc::new(directory_list())),
        doors: Some(std::sync::Arc::new(DoorList {
            doors: vec![Door {
                name: "Tradewars".to_string(),
                description: "A game".to_string(),
                path: "doors/tw2002".to_string(),
                password: "letmein".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })),
        ..Default::default()
    });
}

fn directory_list() -> DirectoryList {
    let mut list = DirectoryList::default();
    list.push(FileDirectory {
        name: "Uploads".to_string(),
        path: std::path::PathBuf::from("files/uploads"),
        is_free: true,
        has_new_files: true,
        password: Password::PlainText("openup".to_string()),
        download_security: SecurityExpression::from_req_security(30),
        ..Default::default()
    });
    list
}

#[test]
fn bulletin_and_news_metadata_are_readable() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf = Board.Conferences[0]
        PRINTLN conf.NewsFile
        BULLETIN item
        FOREACH item IN conf.Bulletins
            PRINTLN item.Number, " ", item.Valid, " ", item.Path, " ", item.HasAccess()
        ENDFOREACH
        item = conf.Bulletins[99]
        PRINTLN item.Valid, " ", item.HasAccess(), " [", item.Path, "]"
        PRINTLN Board.Conferences[99].Bulletins.Len(), " [", Board.Conferences[99].NewsFile, "]"
        "#,
        |board| {
            seed_conference(board);
            board.conferences[0].news_file = "display/news".into();
            board.conferences[0].bulletins = Some(BullettinList {
                bullettins: vec![
                    Bullettin::new(std::path::Path::new("display/welcome")),
                    Bullettin {
                        path: "display/staff".into(),
                        required_security: SecurityExpression::from_req_security(100),
                    },
                ],
            });
        },
    );

    assert_eq!(output, "display/news\n0 1 display/welcome 1\n1 1 display/staff 0\n0 0 []\n0 []\n");
}

#[test]
fn bulletin_and_news_paths_can_be_displayed_by_an_addon() {
    for (bulletin, news) in [("Welcome bulletin", "Board news"), ("Willkommen", "Neuigkeiten")] {
        let bulletin_file = format!("{bulletin}\r\n");
        let news_file = format!("{news}\r\n");
        let output = run_ppl_seeded(
            r#"
            CONFERENCE conf = Board.Conferences[0]
            BULLETIN item
            FOREACH item IN conf.Bulletins
                IF item.HasAccess() DISPFILE item.Path, 0
            ENDFOREACH
            IF conf.HasAccess() THEN
                IF conf.NewsFile <> "" DISPFILE conf.NewsFile, 0
            ENDIF
            "#,
            |board| {
                seed_conference(board);
                board.conferences[0].news_file = "display/news".into();
                board.conferences[0].bulletins = Some(BullettinList {
                    bullettins: vec![
                        Bullettin::new(std::path::Path::new("display/bulletin")),
                        Bullettin {
                            path: "display/staff".into(),
                            required_security: SecurityExpression::from_req_security(100),
                        },
                    ],
                });
            },
            &[
                ("display/bulletin", bulletin_file.as_bytes()),
                ("display/news", news_file.as_bytes()),
                ("display/staff", b"STAFF CONTENT MUST NOT BE SHOWN"),
            ],
        );
        assert_eq!(output, format!("{bulletin}\n{news}\n"));
    }
}

#[test]
fn bulletin_access_requires_access_to_its_conference() {
    let output = run_ppl_on(
        r#"
        BULLETIN item = Board.Conferences[0].Bulletins[0]
        PRINTLN item.Valid, " ", item.HasAccess()
        "#,
        |board| {
            seed_conference(board);
            board.conferences[0].required_security = SecurityExpression::from_req_security(100);
            board.conferences[0].bulletins = Some(BullettinList {
                bullettins: vec![Bullettin::new(std::path::Path::new("display/bulletin"))],
            });
        },
    );
    assert_eq!(output, "1 0\n");
}

#[test]
fn intro_and_survey_metadata_are_readable() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf = Board.Conferences[0]
        PRINTLN conf.IntroFile
        SURVEY item
        FOREACH item IN conf.Surveys
            PRINTLN item.Number, " ", item.Valid, " ", item.Path, " ", item.AnswerFile, " ", item.HasAccess()
        ENDFOREACH
        item = conf.Surveys[99]
        PRINTLN item.Valid, " ", item.HasAccess(), " [", item.Path, "] [", item.AnswerFile, "]"
        PRINTLN Board.Conferences[99].Surveys.Len(), " [", Board.Conferences[99].IntroFile, "]"
        PRINTLN Board.Conferences[1].Surveys.Len(), " [", Board.Conferences[1].IntroFile, "]"
        "#,
        |board| {
            seed_conference(board);
            board.conferences[0].intro_file = "display/intro".into();
            board.conferences[0].surveys = Some(SurveyList {
                surveys: vec![
                    Survey {
                        survey_file: "surveys/welcome".into(),
                        answer_file: "answers/welcome".into(),
                        ..Default::default()
                    },
                    Survey {
                        survey_file: "surveys/staff".into(),
                        answer_file: "answers/staff".into(),
                        required_security: SecurityExpression::from_req_security(100),
                    },
                ],
            });
            board.conferences.push(Conference::default());
        },
    );
    assert_eq!(
        output,
        "display/intro\n0 1 surveys/welcome answers/welcome 1\n1 1 surveys/staff answers/staff 0\n0 0 [] []\n0 []\n0 []\n"
    );
}

#[test]
fn survey_access_requires_access_to_its_conference() {
    let output = run_ppl_on(
        r#"
        SURVEY item = Board.Conferences[0].Surveys[0]
        PRINTLN item.Valid, " ", item.HasAccess()
        "#,
        |board| {
            seed_conference(board);
            board.conferences[0].required_security = SecurityExpression::from_req_security(100);
            board.conferences[0].surveys = Some(SurveyList {
                surveys: vec![Survey {
                    survey_file: "surveys/welcome".into(),
                    ..Default::default()
                }],
            });
        },
    );
    assert_eq!(output, "1 0\n");
}

#[tokio::test]
async fn intro_and_surveys_work_from_the_current_session() {
    use crate::icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState};
    use crate::vm::{DiskIO, run};
    use icy_engine::{Position, TextPane};
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use std::{sync::Arc, time::Duration};

    for (intro, heading) in [("Conference introduction", "Welcome survey"), ("Konferenzvorstellung", "Willkommensumfrage")] {
        let directory = tempfile::tempdir().unwrap();
        let intro_file = directory.path().join("intro");
        let survey_file = directory.path().join("survey");
        let answer_file = directory.path().join("answers");
        let denied_file = directory.path().join("staff");
        std::fs::write(&intro_file, format!("{intro}\r\n")).unwrap();
        std::fs::write(&survey_file, format!("{heading}\r\n*****\r\nQuestion?\r\n")).unwrap();
        std::fs::write(&answer_file, "EXISTING ANSWERS\n").unwrap();
        std::fs::write(&denied_file, "DENIED SURVEY\r\n").unwrap();

        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        board.conferences.push(Conference {
            intro_file,
            surveys: Some(SurveyList {
                surveys: vec![
                    Survey {
                        survey_file,
                        answer_file: answer_file.clone(),
                        ..Default::default()
                    },
                    Survey {
                        survey_file: denied_file,
                        answer_file: answer_file.clone(),
                        required_security: SecurityExpression::from_req_security(100),
                    },
                ],
            }),
            ..Default::default()
        });
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.page_len = 0;
        assert!(state.set_current_conference(0).await.unwrap());
        let executable = super::compile(
            r#"
            CONFERENCE conf = Session.Conference
            IF conf.HasAccess() THEN
                IF conf.IntroFile <> "" DISPFILE conf.IntroFile, 0
            ENDIF
            SURVEY item
            FOREACH item IN conf.Surveys
                IF item.HasAccess() THEN
                    TOKENIZE "N"
                    QUEST item.Number
                ENDIF
            ENDFOREACH
        "#,
        );
        let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
        tokio::time::timeout(
            Duration::from_secs(5),
            run(&directory.path().join("addon.ppe"), &executable, &mut io, &mut state),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(state.session.tokens.is_empty());
        assert_eq!(state.session.current_conference_number, 0);
        assert_eq!(std::fs::read_to_string(&answer_file).unwrap(), "EXISTING ANSWERS\n");
        let mut output = Vec::new();
        let mut packet = [0; 4096];
        loop {
            let size = peer.try_read(&mut packet).await.unwrap();
            if size == 0 {
                break;
            }
            output.extend_from_slice(&packet[..size]);
        }
        let text = String::from_utf8(output.clone()).unwrap();
        assert!(!text.contains("DENIED SURVEY"), "{text:?}");
        assert!(!text.contains("Question?"), "{text:?}");
        let mut screen = crate::icy_board::state::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
        screen.write_bytes(&output);
        for (row, expected) in [(0, intro), (1, heading)] {
            let actual: String = (0..80).map(|column| screen.buffer.char_at(Position::new(column, row)).ch).collect();
            assert_eq!(actual.trim_end(), expected);
        }
    }
}

#[test]
fn a_conference_reports_how_it_is_configured() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf = Board.Conferences[0]
        PrintLn conf.IsPublic, " ", conf.IsReadOnly, " ", conf.AllowAliases
        PrintLn conf.EchoMail, " ", conf.AutoRejoin, " ", conf.PrivateUploads
        "#,
        seed_conference,
    );

    assert_eq!(output, "1 1 1\n1 1 1\n");
}

/// Listing a conference says nothing about who may write in it, so the two
/// security questions a lister asks are separate from `HasAccess()`.
#[test]
fn a_conference_answers_what_the_caller_may_do_in_it() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf = Board.Conferences[0]
        PrintLn conf.HasAccess(), " ", conf.CanPost(), " ", conf.CanAttach()
        "#,
        seed_conference,
    );

    assert_eq!(output, "1 0 1\n");
}

#[test]
fn an_area_reports_how_it_is_configured() {
    let output = run_ppl_on(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn area.IsReadOnly, " ", area.AllowAliases
        PrintLn "[", area.QwkName, "] [", area.EchoTag, "]"
        PrintLn "[", area.EchoOrigin, "]"
        PrintLn area.HasAccess(), " ", area.CanEnter(), " ", area.CanAttach()
        "#,
        seed_conference,
    );

    assert_eq!(output, "1 1\n[GENERAL] [FIDO.GENERAL]\n[Just this area]\n1 1 0\n");
}

/// A local area has no echo tag, which is how a PPE tells the two apart.
#[test]
fn a_local_area_has_no_echo_tag() {
    let output = run_ppl_on(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn "[", area.EchoTag, "]"
        "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(Conference {
                name: "Main Board".to_string(),
                areas: Some(std::sync::Arc::new(AreaList::new(vec![MessageArea {
                    name: "Local".to_string(),
                    ..Default::default()
                }]))),
                ..Default::default()
            });
        },
    );

    assert_eq!(output, "[]\n");
}

/// The number a scan starts from, without joining the area first.
#[test]
fn an_area_reports_its_highest_message() {
    let output = run_ppl_with_messages(
        r"
        PrintLn Board.Conferences[0].Areas[0].HighMsg()
        ",
        &[("SYSOP", "STAN", "one"), ("SYSOP", "STAN", "two")],
    );

    assert_eq!(output, "2\n");
}

/// An area nobody has still answers, so a walk cannot fall over a bad index.
#[test]
fn an_unknown_area_reports_no_messages() {
    let output = run_ppl_on(
        r#"
        AREA area = Board.Conferences[0].Areas[99]
        PrintLn area.Valid, " ", area.HighMsg()
        "#,
        seed_conference,
    );

    assert_eq!(output, "0 0\n");
}

#[test]
fn a_directory_reports_how_it_is_configured() {
    let output = run_ppl_on(
        r#"
        DIRECTORY dir = Board.Conferences[0].Directories[0]
        PrintLn dir.Name, " ", dir.IsFree, " ", dir.HasNewFiles
        PrintLn dir.HasAccess(), " ", dir.CanDownload()
        "#,
        seed_conference,
    );

    assert_eq!(output, "Uploads 1 1\n1 0\n");
}

#[test]
fn a_directory_and_a_door_report_where_they_live() {
    let output = run_ppl_on(
        r"
        PrintLn Board.Conferences[0].Directories[0].Path
        PrintLn Board.Conferences[0].Doors[0].Path
        ",
        seed_conference,
    );

    assert_eq!(output, "files/uploads\ndoors/tw2002\n");
}

/// Every password a board object hands out is the protected kind: a PPE may ask
/// whether it matches, but printing one can never spill the secret.
#[test]
fn a_board_object_password_compares_but_never_shows_itself() {
    let output = run_ppl_on(
        r#"
        PrintLn Board.Conferences[0].Password
        PrintLn Board.Conferences[0].Password = "joinme"
        PrintLn Board.Conferences[0].Directories[0].Password = "openup"
        PrintLn Board.Conferences[0].Doors[0].Password = "letmein"
        PrintLn Board.Conferences[0].Doors[0].Password = "wrong"
        PrintLn Board.Conferences[0].Password <> ""
        "#,
        seed_conference,
    );

    assert_eq!(output, "******\n1\n1\n1\n0\n1\n");
}

/// A conference without a password answers the empty string, so asking whether
/// one is set does not need a member of its own.
#[test]
fn a_conference_without_a_password_compares_equal_to_nothing() {
    let output = run_ppl_on(
        r#"
        PrintLn Board.Conferences[0].Password = ""
        "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(Conference {
                name: "Main Board".to_string(),
                ..Default::default()
            });
        },
    );

    assert_eq!(output, "1\n");
}

/// The board objects are snapshots of what the sysop configured, so a PPE reads
/// them and nothing more. Every member of every one of them refuses a write.
#[test]
fn a_board_object_member_cannot_be_assigned() {
    for write in [
        "CONFERENCE conf = Session.Conference\nconf.Name = \"x\"",
        "CONFERENCE conf = Session.Conference\nconf.IsReadOnly = TRUE",
        "CONFERENCE conf = Session.Conference\nconf.Password = \"x\"",
        "AREA area = Session.Area\narea.Name = \"x\"",
        "AREA area = Session.Area\narea.EchoTag = \"x\"",
        "DIRECTORY dir = Session.Directory\ndir.Path = \"x\"",
        "DIRECTORY dir = Session.Directory\ndir.IsFree = TRUE",
        "DOOR item = Session.Conference.Doors[0]\nitem.Path = \"x\"",
        "BULLETIN item = Session.Conference.Bulletins[0]\nitem.Path = \"x\"",
        "Board.Conferences[0].Bulletins[0].Number = 1",
        "Session.Conference.NewsFile = \"x\"",
        "Session.Conference.Bulletins = Session.Conference.Bulletins",
        "SURVEY item = Session.Conference.Surveys[0]\nitem.Path = \"x\"",
        "Board.Conferences[0].Surveys[0].AnswerFile = \"x\"",
        "Board.Conferences[0].Surveys[0].Number = 1",
        "Session.Conference.IntroFile = \"x\"",
        "Session.Conference.Surveys = Session.Conference.Surveys",
        "Session.Conference.Name = \"x\"",
        "Session.SecurityLevel = 10",
        "Board.Name = \"x\"",
    ] {
        let errors = compile_errors(write);
        assert!(errors.iter().any(|error| error.contains("can only be read")), "{write}: {errors:?}");
    }
}

#[test]
fn an_indexed_board_object_member_reaches_the_read_only_check() {
    for write in [
        "Board.Conferences[0].Name = \"x\"",
        "LET Board.Conferences[0].Name = \"x\"",
        "Board.Conferences[0].Areas[0].Name = \"x\"",
        "Board.Conferences[0].Doors[0].Description += \"x\"",
    ] {
        let errors = compile_errors(write);
        assert_eq!(
            errors,
            vec![format!(
                "'{}' can only be read",
                if write.contains("Description") { "Description" } else { "Name" }
            )]
        );
    }
}

#[test]
fn a_field_on_an_indexed_record_copy_is_not_an_assignment_target() {
    let errors = compile_errors("Session.User.Contacts[0].Account = \"x\"");
    assert_eq!(errors, vec!["Can't assign value to."]);
}

/// The other objects that stand for something the board owns rather than
/// something a PPE made are read-only for the same reason.
#[test]
fn a_snapshot_object_member_cannot_be_assigned() {
    for write in [
        "ERROR failed = Error.Last()\nfailed.Message = \"x\"",
        "TERMINFO info = Terminal.Info\ninfo.Rows = 5",
        "EVENT event = Terminal.Input.Poll()\nevent.Kind = EventKind.Key",
    ] {
        let errors = compile_errors(write);
        assert!(errors.iter().any(|error| error.contains("can only be read")), "{write}: {errors:?}");
    }
}
