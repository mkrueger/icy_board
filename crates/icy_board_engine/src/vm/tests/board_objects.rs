//! What a conference, area, directory and door tell a PPE about themselves.

use crate::icy_board::{
    conferences::Conference,
    doors::{Door, DoorList},
    file_directory::{DirectoryList, FileDirectory},
    message_area::{AreaList, MessageArea},
    security_expr::SecurityExpression,
    user_base::Password,
};

use super::{compile_errors, run_ppl_on, run_ppl_with_messages};

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
        PrintLn area.HasAccess(), " ", area.CanEnter(), " ", area.CanAttach()
        "#,
        seed_conference,
    );

    assert_eq!(output, "1 1\n[GENERAL] [FIDO.GENERAL]\n1 1 0\n");
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
