//! Source evidence: TRANSFER.C scanfornames/getnames (3568-3720), editlist
//! (3773-3844), successful (968-980), checkdlfiles (3370-3493), and download
//! cleanup (4274-4275); FILELIST.C removefinishedfilesfromlist (311-331).
use std::{sync::Arc, time::Duration};

use dizbase::file_base::FileBase;
use icy_net::{Connection, ConnectionType, channel::ChannelConnection, protocol::TransferState};
use tempfile::TempDir;
use tokio::sync::{Mutex, mpsc};

use super::*;
use crate::icy_board::{
    IcyBoard,
    bbs::BBS,
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
    security_expr::SecurityExpression,
    state::{
        KeyChar, KeySource,
        local_transfer::{LocalFilePickerKind, LocalFilePickerRequest},
    },
    user_base::User,
    xfer_protocols::SupportedProtocols,
};

async fn fixture(input: &str) -> (TempDir, IcyBoardState, ChannelConnection) {
    let root = tempfile::tempdir().unwrap();
    let mut directories = DirectoryList::default();
    for (name, free) in [("paid", false), ("free", true)] {
        let path = root.path().join(name);
        std::fs::create_dir(&path).unwrap();
        for file in ["A.ZIP", "B.ZIP", "C.ZIP", "Z"] {
            std::fs::write(path.join(file), vec![0u8; 2048]).unwrap();
        }
        let metadata_path = root.path().join(format!("{name}-metadata"));
        FileBase::open(&path, &metadata_path).unwrap();
        directories.push(FileDirectory {
            path,
            metadata_path,
            is_free: free,
            ..Default::default()
        });
    }
    // Expose only the paid area to filename lookup by default.
    directories.truncate(1);
    let conference = Conference {
        directories: Some(Arc::new(directories)),
        ..Default::default()
    };
    let mut board = IcyBoard::new();
    board.protocols = SupportedProtocols::generate_pcboard_defaults();
    board.config.file_transfer.promote_to_batch_transfers = false;
    board.config.paths.statistics_file = root.path().join("statistics.toml");
    board.config.paths.transfer_log = root.path().join("transfer.log");
    board.users.new_user(User {
        name: "DOWNLOAD TEST".into(),
        security_level: 255,
        protocol: "Z".into(),
        ..Default::default()
    });
    board.conferences.clear();
    board.conferences.push(conference.clone());
    let user = board.users[0].clone();
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.cur_security = 255;
    state.session.current_conference = conference;
    state.session.user_name = "DOWNLOAD TEST".into();
    state.session.page_len = 0;
    state.session.bytes_remaining = -1;
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    (root, state, peer)
}

#[tokio::test]
async fn accounting_only_finished_paid_files_and_per_file_whole_kib() {
    let (root, mut state, _peer) = fixture("").await;
    enable_activity_accounting(
        &mut state,
        crate::icy_board::accounting_cfg::AccountingConfig {
            charge_per_download_file: 3.0,
            charge_per_download_bytes: 2.0,
            ..Default::default()
        },
    )
    .await;
    let a = root.path().join("paid/A.ZIP");
    let b = root.path().join("paid/B.ZIP");
    let c = root.path().join("paid/C.ZIP");
    let base = state.get_filebase(&root.path().join("paid"), &root.path().join("paid-metadata")).await.unwrap();
    base.lock().await.iter_mut().find(|header| header.name() == "C.ZIP").unwrap().set_free(true);
    state.session.flagged_files = vec![a.clone(), b.clone(), c.clone()];
    let offered = vec![(a.clone(), 1536), (b.clone(), 1536), (c.clone(), 2048)];
    let mut transfer = TransferState::new("Accounting test".into());
    // Aborted with no completed files must not debit or refund anything.
    state.finish_download_batch(&offered, &transfer, "Z", 10).await.unwrap();
    assert_eq!(state.session.current_user.as_ref().unwrap().account.as_ref().unwrap().debit_download_file, 0.0);
    transfer.send_state.finished_files = vec![("A.ZIP".into(), a), ("C.ZIP".into(), c)];
    transfer.send_state.total_bytes_transfered = 50_000; // Includes failed wire bytes.
    state.finish_download_batch(&offered, &transfer, "Z", 10).await.unwrap();
    let account = state.session.current_user.as_ref().unwrap().account.as_ref().unwrap();
    assert_eq!(account.debit_download_file, 3.0);
    assert_eq!(account.debit_download_bytes, 2.0); // 1536 / 1024, not 1.5 KiB.
    assert_eq!(account.credit_special, 0.0);
    assert_eq!(account.debit_time, 0.0); // FREE is not NOTIME.
    assert_eq!(state.session.flagged_files, [b]);
}

#[tokio::test]
async fn accounting_queue_preflight_reserves_prior_files_without_debiting() {
    let (root, mut state, _peer) = fixture("").await;
    enable_activity_accounting(
        &mut state,
        crate::icy_board::accounting_cfg::AccountingConfig {
            charge_per_download_file: 3.0,
            charge_per_download_bytes: 2.0,
            ..Default::default()
        },
    )
    .await;
    state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = 10.0;
    let a = root.path().join("paid/A.ZIP");
    let b = root.path().join("paid/B.ZIP");
    state.add_flagged_file(&a, false, false).await.unwrap();
    state.add_flagged_file(&b, false, false).await.unwrap();
    assert_eq!(state.session.flagged_files, [a.clone()]);
    assert_eq!(state.accounting_queued_download_cost(&b).await.unwrap(), 7.0);
    assert_eq!(state.accounting_queued_download_cost(&a).await.unwrap(), 0.0);
    assert_eq!(state.session.current_user.as_ref().unwrap().account.as_ref().unwrap().debit_download_file, 0.0);
}

#[tokio::test]
async fn a4_directory_flag_reserves_credit_without_debit_or_output() {
    use crate::{
        compiler::user_data::UserDataValue,
        executable::VariableValue,
        vm::{DiskIO, VirtualMachine},
    };
    let (root, mut state, mut peer) = fixture("").await;
    enable_activity_accounting(
        &mut state,
        crate::icy_board::accounting_cfg::AccountingConfig {
            charge_per_download_file: 3.0,
            charge_per_download_bytes: 2.0,
            ..Default::default()
        },
    )
    .await;
    state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = 10.0;
    state.session.batch_limit = 10;
    let mut directory = state.session.current_conference.directories.as_ref().unwrap()[0].clone();
    directory.valid = true;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
    let mut vm = VirtualMachine::new(root.path().join("flag.ppe"), &registry, &mut io, &mut state);
    let member = unicase::Ascii::new("Flag".into());
    for (name, expected) in [("A.ZIP", true), ("B.ZIP", false), ("A.ZIP", true)] {
        vm.error_pending = false;
        let result = directory
            .call_function(&mut vm, &member, &[VariableValue::new_unbounded_string(name.into())])
            .await
            .unwrap();
        assert_eq!(result.as_bool(), expected, "{name}");
        assert_eq!(vm.last_error.code, if expected { 0 } else { crate::icy_board::state::ppl_error::ERR_DENIED });
    }
    assert_eq!(vm.icy_board_state.session.flagged_files, [root.path().join("paid/A.ZIP")]);
    let account = vm.icy_board_state.session.current_user.as_ref().unwrap().account.as_ref().unwrap();
    assert_eq!(account.debit_download_file, 0.0);
    assert_eq!(account.debit_download_bytes, 0.0);
    assert_eq!(vm.icy_board_state.session.calculate_balance(), 10.0);
    assert_eq!(output(&mut peer).await, "");
}

#[tokio::test]
async fn accounting_command_writes_use_target_surcharge_and_one_category() {
    use crate::icy_board::message_area::{AreaList, MessageArea};
    use jamjam::jam::{JamMessage, JamMessageBase, attributes};
    let (root, mut state, _peer) = fixture("").await;
    enable_activity_accounting(
        &mut state,
        crate::icy_board::accounting_cfg::AccountingConfig {
            charge_per_msg_written: 2.0,
            charge_per_msg_write_echoed: 5.0,
            charge_per_msg_write_private: 7.0,
            ..Default::default()
        },
    )
    .await;
    state.session.current_conference.charge_msg_write = 100.0;
    {
        let mut board = state.get_board().await;
        board.conferences[0].charge_msg_write = 3.0;
        board.conferences[0].areas = Some(Arc::new(AreaList::new(vec![MessageArea {
            path: root.path().join("messages"),
            ..Default::default()
        }])));
    }
    for flags in [
        attributes::MSG_TYPELOCAL,
        attributes::MSG_TYPEECHO,
        attributes::MSG_PRIVATE | attributes::MSG_TYPEECHO,
    ] {
        let message = JamMessage::default()
            .with_from("AUTHOR".into())
            .with_to("READER".into())
            .with_subject("Accounting".into())
            .with_text("Body".into())
            .with_attributes(flags);
        state.send_accounted_message(0, 0, message, IceText::SavingMessage).await.unwrap();
    }
    // Bad destinations must neither append nor charge.
    assert!(
        state
            .send_accounted_message(0, 99, JamMessage::default(), IceText::SavingMessage)
            .await
            .is_err()
    );
    let account = state.session.current_user.as_ref().unwrap().account.as_ref().unwrap();
    assert_eq!(account.debit_msg_write, 5.0);
    assert_eq!(account.debit_msg_write_echoed, 8.0);
    assert_eq!(account.debit_msg_write_private, 10.0);
    assert_eq!(state.session.current_conference.charge_msg_write, 100.0);
    assert_eq!(JamMessageBase::open(root.path().join("messages")).unwrap().highest_message_number(), 3);
}

async fn output(peer: &mut ChannelConnection) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = peer.try_read(&mut buffer).await.unwrap();
        if count == 0 {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

async fn accounting_message_fixture(balance: f64) -> (TempDir, IcyBoardState, ChannelConnection) {
    use crate::icy_board::{
        accounting_cfg::AccountingConfig,
        message_area::{AreaList, MessageArea},
    };
    let (root, mut state, peer) = fixture("").await;
    enable_activity_accounting(
        &mut state,
        AccountingConfig {
            charge_per_msg_written: 2.0,
            charge_per_msg_write_echoed: 5.0,
            charge_per_msg_write_private: 7.0,
            ..Default::default()
        },
    )
    .await;
    state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = balance;
    {
        let mut board = state.get_board().await;
        board.config.paths.user_file = root.path().join("users.toml");
        board.config.paths.email_msgbase = root.path().join("mail");
        board.conferences[0].charge_msg_write = 3.0;
        board.conferences[0].areas = Some(Arc::new(AreaList::new(vec![MessageArea {
            path: root.path().join("messages"),
            ..Default::default()
        }])));
    }
    state.session.current_conference.charge_msg_write = 20.0;
    (root, state, peer)
}

fn accounting_message(flags: u32) -> jamjam::jam::JamMessage {
    jamjam::jam::JamMessage::default()
        .with_from("AUTHOR".into())
        .with_to("READER".into())
        .with_subject("Accounting".into())
        .with_text("Body".into())
        .with_attributes(flags)
}

#[tokio::test]
async fn accounting_message_preflight_denies_before_composition_and_rechecks_each_save() {
    use crate::icy_board::state::user_commands::{mods::editor::EditResult, pcb::message_attachment::MessageCreditDenied};
    use jamjam::jam::{JamMessageBase, attributes};
    for (flags, cost) in [
        (0, 5.0),
        (attributes::MSG_TYPEECHO, 8.0),
        (attributes::MSG_PRIVATE | attributes::MSG_TYPEECHO, 10.0),
    ] {
        let (root, mut state, mut peer) = accounting_message_fixture(cost - 1.0).await;
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            state.write_message_context(0, 0, accounting_message(flags), Vec::new(), IceText::SavingMessage, false),
        )
        .await
        .expect("insufficient balance must not enter editor")
        .unwrap();
        assert_eq!(result, EditResult::Abort);
        assert!(!root.path().join("messages.jhr").exists());
        assert!(output(&mut peer).await.contains("Insufficient Credits"));
        // Balance can change while composing. A successful entry preflight is
        // not authority to save later, including forward/copy/QWK adapters.
        state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = cost + 1.0;
        assert!(state.message_write_allowed(0, &accounting_message(flags)).await.unwrap());
        state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = cost - 1.0;
        let error = state
            .send_accounted_message(0, 0, accounting_message(flags), IceText::SavingMessage)
            .await
            .unwrap_err();
        assert!(error.is::<MessageCreditDenied>());
        assert!(!root.path().join("messages.jhr").exists());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.messages_left, 0);
        state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = cost + 1.0;
        state
            .send_accounted_message(0, 0, accounting_message(flags), IceText::SavingMessage)
            .await
            .unwrap();
        let error = state
            .send_accounted_message(0, 0, accounting_message(flags), IceText::SavingMessage)
            .await
            .unwrap_err();
        assert!(error.is::<MessageCreditDenied>(), "each additional copy needs fresh authorization");
        assert_eq!(JamMessageBase::open(root.path().join("messages")).unwrap().highest_message_number(), 1);
        assert_eq!(state.session.calculate_balance(), 1.0);
        assert_eq!(state.session.current_conference.charge_msg_write, 20.0);
    }
}

#[tokio::test]
async fn accounting_message_open_and_create_errors_display_failure_without_save_or_debit() {
    use crate::icy_board::message_area::{AreaList, MessageArea};
    for (email, existing) in [(false, false), (false, true), (true, false), (true, true)] {
        let (root, mut state, mut peer) = accounting_message_fixture(100.0).await;
        let blocker = root.path().join("blocker");
        std::fs::write(&blocker, b"unchanged").unwrap();
        let path = if existing { root.path().join("corrupt") } else { blocker.join("messages") };
        if existing {
            std::fs::write(path.with_extension("jhr"), b"bad header").unwrap();
        }
        {
            let mut board = state.get_board().await;
            board.config.paths.email_msgbase = path.clone();
            board.conferences[0].areas = Some(Arc::new(AreaList::new(vec![MessageArea { path, ..Default::default() }])));
        }
        let error = state
            .send_accounted_message(if email { -1 } else { 0 }, 0, accounting_message(0), IceText::SavingMessage)
            .await;
        assert!(error.is_err());
        let text = output(&mut peer).await;
        assert!(text.contains(state.get_display_text(IceText::MessageBaseError).unwrap().trim()), "{text}");
        assert!(!text.contains("Saving Message"), "{text}");
        assert_eq!(state.session.calculate_balance(), 100.0);
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.messages_left, 0);
        assert_eq!(std::fs::read(blocker).unwrap(), b"unchanged");
    }
}

#[tokio::test]
async fn accounting_message_exact_balance_is_allowed_and_tracking_does_not_enforce() {
    use crate::icy_board::accounting::AccountingMode;
    use jamjam::jam::JamMessageBase;
    for mode in [AccountingMode::Enforced, AccountingMode::Tracking, AccountingMode::Disabled] {
        let (root, mut state, mut peer) = accounting_message_fixture(if mode == AccountingMode::Enforced { 5.0 } else { 1.0 }).await;
        {
            let mut board = state.get_board().await;
            board.config.accounting.enabled = mode != AccountingMode::Disabled;
            board.sec_levels[0].accounting_tracking = mode == AccountingMode::Tracking;
            board.config.accounting.tracking_file = root.path().join("accounting.dbf");
        }
        state.accounting_refresh().await.unwrap();
        assert_eq!(state.session.accounting.mode, mode);
        state.send_accounted_message(0, 0, accounting_message(0), IceText::SavingMessage).await.unwrap();
        assert_eq!(JamMessageBase::open(root.path().join("messages")).unwrap().highest_message_number(), 1);
        assert_eq!(
            state.session.current_user.as_ref().unwrap().account.as_ref().unwrap().debit_msg_write,
            if mode == AccountingMode::Disabled { 0.0 } else { 5.0 }
        );
        assert!(!output(&mut peer).await.contains("Insufficient Credits"));
    }
}

#[tokio::test]
async fn accounting_message_post_save_error_commits_once_and_blocks_retry() {
    use crate::icy_board::state::user_commands::pcb::message_attachment::{MessageCreditDenied, MessagePersistedError};
    use jamjam::jam::JamMessageBase;
    let (root, mut state, peer) = accounting_message_fixture(100.0).await;
    // save_statistics logs rather than returns failures. A closed receiver
    // deterministically fails the saved-message notification after the append.
    drop(peer);
    let error = state
        .send_accounted_message(0, 0, accounting_message(0), IceText::SavingMessage)
        .await
        .unwrap_err();
    assert!(error.is::<MessagePersistedError>());
    assert!(state.session.request_logoff);
    assert_eq!(state.session.calculate_balance(), 95.0);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.messages_left, 1);
    let error = state
        .send_accounted_message(0, 0, accounting_message(0), IceText::SavingMessage)
        .await
        .unwrap_err();
    assert!(error.is::<MessageCreditDenied>());
    assert_eq!(state.session.calculate_balance(), 95.0);
    assert_eq!(JamMessageBase::open(root.path().join("messages")).unwrap().highest_message_number(), 1);
}

#[tokio::test]
async fn accounting_logoff_displays_once_after_pending_activity_settles() {
    let (root, mut state, mut peer) = accounting_message_fixture(100.0).await;
    let file = root.path().join("account-logoff");
    std::fs::write(&file, b"ACCOUNT-LOGOFF @HANGUP@\r\n").unwrap();
    state.session.accounting.options.logoff_file = file;
    state.accounting_begin_invocation();
    state.logoff_user(false).await.unwrap();
    state.logoff_user(false).await.unwrap();
    assert!(state.accounting_active(), "G must not finish an enclosing command's accounting");
    assert!(output(&mut peer).await.is_empty(), "no premature final summary");
    state.accounting_record(12, "COMMAND", "G", 4.0, 1).unwrap();
    state.accounting_end_invocation().await.unwrap();
    let text = output(&mut peer).await;
    for marker in ["ACCOUNT-LOGOFF", "Credits Used:", "Credits Left:", "Minutes Used"] {
        assert_eq!(text.matches(marker).count(), 1, "{marker}: {text}");
    }
    assert!(text.find("ACCOUNT-LOGOFF").unwrap() < text.find("Credits Used:").unwrap());
    state.accounting_finish().await.unwrap();
    state.accounting_finish().await.unwrap();
    assert!(!state.accounting_active());
    assert_eq!(state.session.current_user.as_ref().unwrap().account.as_ref().unwrap().debit_tpu, 4.0);
}

#[tokio::test]
async fn accounting_logoff_auto_skips_file_and_disabled_skips_all_credit_output() {
    for active in [false, true] {
        let (root, mut state, mut peer) = accounting_message_fixture(100.0).await;
        let file = root.path().join("account-logoff");
        std::fs::write(&file, b"ACCOUNT-LOGOFF\r\n").unwrap();
        state.session.accounting.options.logoff_file = file;
        if !active {
            state.session.accounting = Default::default();
        }
        state.logoff_user(true).await.unwrap();
        let text = output(&mut peer).await;
        assert!(!text.contains("ACCOUNT-LOGOFF"));
        assert_eq!(text.matches("Credits Used:").count(), usize::from(active));
        assert_eq!(text.matches("Credits Left:").count(), usize::from(active));
    }
}

#[tokio::test]
async fn accounting_logoff_tracking_shows_used_but_not_enforced_balance() {
    let (root, mut state, mut peer) = accounting_message_fixture(100.0).await;
    {
        let mut board = state.get_board().await;
        board.sec_levels[0].accounting_tracking = true;
        board.config.accounting.tracking_file = root.path().join("accounting.dbf");
    }
    state.accounting_refresh().await.unwrap();
    state.logoff_user(false).await.unwrap();
    let text = output(&mut peer).await;
    assert_eq!(text.matches("Credits Used:").count(), 1);
    assert!(!text.contains("Credits Left:"));
}

async fn command(state: &mut IcyBoardState, line: &str) {
    state.session.push_tokens(line);
    tokio::time::timeout(Duration::from_secs(3), state.run_single_command(false))
        .await
        .expect("download command requested unexpected input")
        .unwrap();
}

async fn local_picker(state: &mut IcyBoardState) -> mpsc::Receiver<LocalFilePickerRequest> {
    state.session.is_local = true;
    state.session.is_sysop = false;
    state.session.current_user.as_mut().unwrap().protocol = "N".into();
    // Local calls are excluded from transfer logs by default. Enable logging
    // so successful/cancelled command assertions exercise actual log effects.
    state.get_board().await.config.switches.exclude_local_calls_stats = false;
    let (tx, rx) = mpsc::channel(1);
    state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
    rx
}

async fn command_with_picker(state: &mut IcyBoardState, line: &str, picker: &mut mpsc::Receiver<LocalFilePickerRequest>, destination: Option<PathBuf>) {
    // Bound the entire join: a regression that never requests the picker must
    // fail too, rather than leaving the simulated UI waiting forever.
    tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(command(state, line), async {
            let request = picker.recv().await.expect("local download did not request a destination");
            assert_eq!(request.kind, LocalFilePickerKind::DownloadDirectory);
            request.response.send(destination).unwrap();
        });
    })
    .await
    .expect("local download command/picker bridge stalled");
    assert!(picker.try_recv().is_err(), "one download batch must use only one destination picker");
    assert!(state.char_buffer.is_empty(), "command left an expected prompt answer unread");
    assert_eq!(state.session.current_user.as_ref().unwrap().protocol, "N");
}

fn assert_no_protocol_output(text: &str) {
    for forbidden in ["Protocol Type", "Zmodem", "Sending files", "(A)bort", "Protocol not found"] {
        assert!(!text.contains(forbidden), "unexpected local protocol output {forbidden:?}: {text}");
    }
    assert!(!text.as_bytes().contains(&0x18), "local copy emitted native transfer framing: {text}");
}

#[tokio::test]
async fn local_d_and_bd_none_default_copy_only_completed_files_and_retain_collision() {
    for verb in ["D", "BD", "DB"] {
        let (root, mut state, mut peer) = fixture("\r").await;
        let destination = root.path().join("destination");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("b.zip"), b"existing destination").unwrap();
        let a = root.path().join("paid/A.ZIP");
        let b = root.path().join("paid/B.ZIP");
        let c = root.path().join("paid/C.ZIP");
        std::fs::write(&a, vec![b'A'; 2048]).unwrap();
        std::fs::write(&c, vec![b'C'; 2048]).unwrap();
        state.session.bytes_remaining = 10_000;
        let mut picker = local_picker(&mut state).await;
        command_with_picker(&mut state, &format!("{verb} A.ZIP B.ZIP C.ZIP"), &mut picker, Some(destination.clone())).await;

        assert_eq!(std::fs::read(destination.join("A.ZIP")).unwrap(), std::fs::read(&a).unwrap());
        assert_eq!(std::fs::read(destination.join("C.ZIP")).unwrap(), std::fs::read(&c).unwrap());
        assert_eq!(std::fs::read(destination.join("b.zip")).unwrap(), b"existing destination");
        assert!(!destination.join("B.ZIP").exists());
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 3, "no staging files may remain");
        assert_eq!(std::fs::read(&b).unwrap(), vec![0u8; 2048]);
        assert_eq!(state.session.flagged_files, [b]);
        assert_eq!(state.transfer_statistics.downloaded_files, 2);
        assert_eq!(state.transfer_statistics.downloaded_bytes, 4096);
        let user = state.session.current_user.as_ref().unwrap();
        assert_eq!(user.stats.num_downloads, 2);
        assert_eq!(user.stats.today_num_downloads, 2);
        assert_eq!(user.stats.total_dnld_bytes, 4096);
        assert_eq!(user.stats.today_dnld_bytes, 4096);
        assert_eq!(state.session.bytes_remaining, 5904);
        assert_eq!(state.get_board().await.statistics.total.downloads, 2);
        assert_eq!(state.get_board().await.statistics.total.downloads_kb, 4);
        let base = state.get_filebase(&root.path().join("paid"), &root.path().join("paid-metadata")).await.unwrap();
        let base = base.lock().await;
        for (name, expected) in [("A.ZIP", 1), ("B.ZIP", 0), ("C.ZIP", 1)] {
            assert_eq!(base.iter().find(|header| header.name() == name).unwrap().dl_counter, expected, "{verb}: {name}");
        }
        let log = std::fs::read_to_string(root.path().join("transfer.log")).unwrap();
        assert_eq!(log.lines().count(), 2);
        assert!(log.contains("A.ZIP") && log.contains("C.ZIP") && log.contains("Local"), "{log}");
        assert!(!log.contains("B.ZIP"), "{log}");
        let text = output(&mut peer).await;
        assert!(text.contains("Batch Transfer Ended."), "{text}");
        assert!(text.contains("Files: 2") && text.contains("Bytes: 4096"), "{text}");
        assert!(text.contains("Unsent files remain flagged."), "{text}");
        assert_no_protocol_output(&text);
    }
}

#[tokio::test]
async fn local_d_and_bd_picker_cancel_preserves_queue_and_does_not_credit_or_complete() {
    for verb in ["D", "BD", "DB"] {
        let (root, mut state, mut peer) = fixture("Y\r\r").await;
        let queued = ["A.ZIP", "B.ZIP"].map(|name| root.path().join("paid").join(name)).to_vec();
        state.session.flagged_files.clone_from(&queued);
        state.session.bytes_remaining = 10_000;
        state.transfer_statistics.downloaded_files = 42;
        state.transfer_statistics.downloaded_bytes = 9999;
        let mut picker = local_picker(&mut state).await;
        command_with_picker(&mut state, verb, &mut picker, None).await;

        assert_eq!(state.session.flagged_files, queued);
        assert_eq!(state.transfer_statistics.downloaded_files, 0);
        assert_eq!(state.transfer_statistics.downloaded_bytes, 0);
        let user = state.session.current_user.as_ref().unwrap();
        assert_eq!(user.stats.num_downloads, 0);
        assert_eq!(user.stats.today_num_downloads, 0);
        assert_eq!(user.stats.total_dnld_bytes, 0);
        assert_eq!(user.stats.today_dnld_bytes, 0);
        assert_eq!(state.session.bytes_remaining, 10_000);
        assert_eq!(state.get_board().await.statistics.total.downloads, 0);
        assert_eq!(state.get_board().await.statistics.total.downloads_kb, 0);
        let base = state.get_filebase(&root.path().join("paid"), &root.path().join("paid-metadata")).await.unwrap();
        assert!(base.lock().await.iter().all(|header| header.dl_counter == 0));
        assert!(!root.path().join("transfer.log").exists());
        assert!(queued.iter().all(|path| std::fs::read(path).unwrap() == vec![0u8; 2048]));
        let text = output(&mut peer).await;
        assert!(!text.contains("Batch Transfer Ended.") && !text.contains("Files:"), "{text}");
        assert_no_protocol_output(&text);
    }
}

#[tokio::test]
async fn local_explicit_bd_numbers_filename_prompts_but_unpromoted_d_does_not() {
    // TRANSFER.C getnames selects TXT_FILENAMETODNLDBTCH only for
    // Status.Batch; a normal D stops asking after its first accepted file.
    for verb in ["D", "BD", "DB"] {
        let batch = verb != "D";
        let (root, mut state, mut peer) = fixture(if batch { "A.ZIP\r\r" } else { "A.ZIP\r" }).await;
        let mut picker = local_picker(&mut state).await;
        command_with_picker(&mut state, verb, &mut picker, None).await;
        assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP")]);
        let text = output(&mut peer).await;
        // File-selection summaries also contain '(1)', so match the actual
        // filename prompt, not an unrelated numbered row.
        assert_eq!(text.contains("(1) Enter the filename to Download"), batch, "{verb}: {text}");
        assert_eq!(text.contains("(2) Enter the filename to Download"), batch, "{verb}: {text}");
        assert_eq!(text.matches("Enter the filename to Download").count(), if batch { 2 } else { 1 }, "{text}");
        assert_no_protocol_output(&text);
    }
}

#[tokio::test]
async fn local_bd_batch_permission_denied_falls_back_to_unnumbered_d_and_picker() {
    // COMMAND.C O_BD/O_DB sets Status.Batch from SEC_BATCH, then dispatches
    // send using SEC_D. Batch denial alone must not deny the download command.
    for verb in ["BD", "DB"] {
        let (root, mut state, mut peer) = fixture("A.ZIP\r").await;
        let mut picker = local_picker(&mut state).await;
        state.session.cur_security = 10;
        state.session.current_user.as_mut().unwrap().security_level = 10;
        state.session.user_command_level.cmd_d = SecurityExpression::from_req_security(0);
        state.session.user_command_level.batch_file_transfer = SecurityExpression::from_req_security(20);
        assert!(!state.session.user_command_level.batch_file_transfer.session_can_access(&state.session));
        // Verify the actual dispatch gate first: denying BD here would leave
        // the simulated picker waiting, obscuring the cause as a timeout.
        let resolved = state.try_find_command(verb, false).await.unwrap();
        assert!(
            resolved.security.session_can_access(&state.session),
            "{verb} must allow cmd_d despite batch denial"
        );
        command_with_picker(&mut state, verb, &mut picker, None).await;
        assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP")]);
        assert_eq!(state.session.security_violations, 0);
        let text = output(&mut peer).await;
        assert_eq!(text.matches("Enter the filename to Download").count(), 1, "{text}");
        assert!(
            !text.contains("(1) Enter the filename to Download") && !text.contains("(2) Enter the filename to Download"),
            "{text}"
        );
        assert_no_protocol_output(&text);
    }
}

#[tokio::test]
async fn local_bd_uses_cmd_d_access_gate_even_when_batch_permission_is_allowed() {
    for verb in ["D", "BD", "DB"] {
        let (root, mut state, mut peer) = fixture("").await;
        let mut picker = local_picker(&mut state).await;
        let queued = vec![root.path().join("paid/A.ZIP")];
        state.session.flagged_files.clone_from(&queued);
        state.session.cur_security = 10;
        state.session.current_user.as_mut().unwrap().security_level = 10;
        state.session.user_command_level.cmd_d = SecurityExpression::from_req_security(20);
        state.session.user_command_level.batch_file_transfer = SecurityExpression::from_req_security(0);
        assert!(!state.session.user_command_level.cmd_d.session_can_access(&state.session));
        assert!(state.session.user_command_level.batch_file_transfer.session_can_access(&state.session));
        // An incorrectly allowed command would block at DownloadTagged with
        // no input; identify the wrong gate before executing the command.
        let resolved = state.try_find_command(verb, false).await.unwrap();
        assert!(
            !resolved.security.session_can_access(&state.session),
            "{verb} must deny cmd_d despite batch permission"
        );
        command(&mut state, &format!("{verb} B.ZIP")).await;
        assert!(picker.try_recv().is_err(), "denied command must not open a local picker");
        assert_eq!(state.session.flagged_files, queued);
        assert_eq!(state.session.security_violations, 1, "{verb} must be checked against cmd_d");
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_sec_viol, 1);
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 0);
        assert!(!root.path().join("transfer.log").exists());
        let text = output(&mut peer).await;
        assert!(
            !text.contains("Enter the filename to Download") && !text.contains("Batch Transfer Ended."),
            "{text}"
        );
        assert_no_protocol_output(&text);
    }
}

#[tokio::test]
async fn d_and_bd_scan_all_stacked_names_and_protocol_without_changing_default() {
    for verb in ["D", "BD", "DB"] {
        let (root, mut state, mut peer) = fixture("\rA\r").await;
        state.session.current_user.as_mut().unwrap().protocol = "X".into();
        command(&mut state, &format!("{verb} A.ZIP Z B.ZIP")).await;
        assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP"), root.path().join("paid/B.ZIP")]);
        assert_eq!(state.session.current_user.as_ref().unwrap().protocol, "X");
        let output = output(&mut peer).await;
        assert!(output.contains("Zmodem (batch)"), "{output}");
        assert!(!output.contains("not found on disk"), "{output}");
    }
}

#[tokio::test]
async fn tagged_question_does_not_eat_stacked_filename() {
    let (root, mut state, _peer) = fixture("Y\r\rA\r").await;
    state.session.flagged_files.push(root.path().join("paid/A.ZIP"));
    command(&mut state, "D B.ZIP Z").await;
    assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP"), root.path().join("paid/B.ZIP")]);
}

#[tokio::test]
async fn declining_tagged_files_discards_them_and_scans_new_names() {
    let (root, mut state, _peer) = fixture("N\rA\r").await;
    state.session.flagged_files.push(root.path().join("paid/A.ZIP"));
    command(&mut state, "D B.ZIP").await;
    assert_eq!(state.session.flagged_files, [root.path().join("paid/B.ZIP")]);
}

#[tokio::test]
async fn prompted_line_accepts_multiple_names_but_letter_is_not_a_protocol() {
    let (root, mut state, _peer) = fixture("A.ZIP;B.ZIP Z\r\rA\r").await;
    command(&mut state, "D").await;
    assert_eq!(
        state.session.flagged_files,
        [root.path().join("paid/A.ZIP"), root.path().join("paid/B.ZIP"), root.path().join("paid/Z")]
    );
}

#[tokio::test]
async fn stacked_none_requests_protocol_instead_of_starting_transfer() {
    let (root, mut state, mut peer) = fixture("\r").await;
    command(&mut state, "D A.ZIP N").await;
    assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP")]);
    let output = output(&mut peer).await;
    assert!(!output.contains("Sending files"), "{output}");
    assert_eq!(state.session.last_answer.as_deref(), Some("N"));
}

#[tokio::test]
async fn multi_file_batch_rejects_single_file_protocol() {
    // Enter at the required protocol prompt accepts N (abort), not X again.
    let (_root, mut state, mut peer) = fixture("\r\r").await;
    command(&mut state, "D A.ZIP B.ZIP X").await;
    let output = output(&mut peer).await;
    assert_eq!(state.session.flagged_files.len(), 2, "{output}");
    assert!(!output.contains("Sending files"), "{output}");
    assert_eq!(state.session.last_answer.as_deref(), Some("N"));
}

#[tokio::test]
async fn removal_accepts_spaces_semicolons_and_duplicates_only_remove_once() {
    let (root, mut state, mut peer) = fixture("1 3;3\r").await;
    state.session.flagged_files = ["A.ZIP", "B.ZIP", "C.ZIP"].map(|name| root.path().join("paid").join(name)).to_vec();
    state.remove_dl_batch().await.unwrap();
    assert_eq!(state.session.flagged_files, [root.path().join("paid/B.ZIP")]);
    let output = output(&mut peer).await;
    assert_eq!(output.matches("C.ZIP").count(), 1, "{output}");
}

#[tokio::test]
async fn removal_ignores_invalid_numbers_and_does_not_invent_range_syntax_or_eat_later_answers() {
    let (root, mut state, _peer) = fixture("").await;
    state.session.flagged_files = ["A.ZIP", "B.ZIP", "C.ZIP"].map(|name| root.path().join("paid").join(name)).to_vec();
    // Stuffed/token input bypasses the numeric mask; invalid ranges must not
    // remove arbitrary files. Native PCBoard's mask does not admit '-'.
    state.session.tokens.extend(["0;3;3;99;1-2;999999999999999999999999".into(), "L".into()]);
    state.remove_dl_batch().await.unwrap();
    assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP"), root.path().join("paid/B.ZIP")]);
    assert_eq!(state.session.tokens.front().map(String::as_str), Some("L"));
}

#[tokio::test]
async fn editing_away_entire_batch_returns_without_sending() {
    let (root, mut state, mut peer) = fixture("E\rR\r1 2\r\r").await;
    state.session.flagged_files = ["A.ZIP", "B.ZIP"].map(|name| root.path().join("paid").join(name)).to_vec();
    tokio::time::timeout(Duration::from_secs(3), state.download(false)).await.unwrap().unwrap();
    assert!(state.session.flagged_files.is_empty());
    let output = output(&mut peer).await;
    assert!(!output.contains("Sending files"), "{output}");
}

#[tokio::test]
async fn edit_add_prompts_repeatedly_and_honors_batch_limit() {
    let (root, mut state, _peer) = fixture("A\rB.ZIP\rC.ZIP\r\r").await;
    state.session.batch_limit = 3;
    state.session.flagged_files.push(root.path().join("paid/A.ZIP"));
    tokio::time::timeout(Duration::from_secs(3), state.edit_dl_batch()).await.unwrap().unwrap();
    assert_eq!(state.session.flagged_files.len(), 3);
    assert!(state.session.flagged_files.contains(&root.path().join("paid/C.ZIP")));
}

#[tokio::test]
async fn missing_and_same_named_files_do_not_discard_other_downloads() {
    let (root, mut state, _peer) = fixture("").await;
    let a = root.path().join("paid/A.ZIP");
    let b = root.path().join("paid/B.ZIP");
    let files = vec![root.path().join("missing.zip"), a.clone(), root.path().join("free/A.ZIP"), b.clone()];
    assert_eq!(state.screen_transfer_limits(files).await.unwrap(), [a, b]);
    assert!(state.session.flagged_files.is_empty());
}

#[tokio::test]
async fn limit_rejects_are_removed_and_later_smaller_files_still_fit() {
    let (root, mut state, mut peer) = fixture("").await;
    state.get_board().await.config.system_control.enforce_transfer_limits = true;
    state.session.bytes_remaining = 2500;
    let a = root.path().join("paid/A.ZIP");
    let b = root.path().join("paid/B.ZIP");
    let c = root.path().join("paid/C.ZIP");
    std::fs::write(&c, [0u8; 100]).unwrap();
    let queued = vec![a.clone(), b, c.clone()];
    state.session.flagged_files.clone_from(&queued);
    assert_eq!(state.screen_transfer_limits(queued.clone()).await.unwrap(), [a, c]);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_reach_dnld_lim, 1);
    assert_eq!(state.session.flagged_files, queued, "screening must restore the queue even after reporting");
    let output = output(&mut peer).await;
    assert!(output.contains("452"), "BYTESLEFT must deduct only the already accepted 2048 bytes: {output}");
}

#[tokio::test]
async fn partial_batch_counts_only_confirmed_paths_and_preserves_failed_queue() {
    let (root, mut state, _peer) = fixture("").await;
    let a = root.path().join("paid/A.ZIP");
    let b = root.path().join("paid/B.ZIP");
    let free = root.path().join("free/C.ZIP");
    let mut directories = state.session.current_conference.directories.as_ref().unwrap().as_ref().clone();
    directories.push(FileDirectory {
        path: root.path().join("free"),
        metadata_path: root.path().join("free-metadata"),
        is_free: true,
        ..Default::default()
    });
    state.session.current_conference.directories = Some(Arc::new(directories));
    state.session.bytes_remaining = 10_000;
    state.session.flagged_files = vec![a.clone(), b.clone(), free.clone()];
    let offered = vec![(a.clone(), 2048), (b.clone(), 2048), (free.clone(), 2048)];
    let mut transfer = TransferState::new("test".into());
    transfer.send_state.total_bytes_transfered = 5000; // includes incomplete B
    transfer.send_state.finished_files = vec![
        ("a.zip".into(), a.clone()),
        ("A.ZIP".into(), a.clone()), // duplicate completion must not charge twice
        ("C.ZIP".into(), free),
        ("B.ZIP".into(), root.path().join("elsewhere/B.ZIP")),
    ];
    state.finish_download_batch(&offered, &transfer, "Z", 1000).await.unwrap();
    assert_eq!(state.session.flagged_files, [b]);
    assert_eq!(state.transfer_statistics.downloaded_files, 2);
    assert_eq!(state.transfer_statistics.downloaded_bytes, 4096);
    let user = state.session.current_user.as_ref().unwrap();
    assert_eq!(user.stats.num_downloads, 1);
    assert_eq!(user.stats.today_num_downloads, 1);
    assert_eq!(user.stats.total_dnld_bytes, 2048);
    assert_eq!(user.stats.today_dnld_bytes, 2048);
    assert_eq!(state.session.bytes_remaining, 7952);
    assert_eq!(state.get_board().await.statistics.total.downloads, 2);
    assert_eq!(state.get_board().await.statistics.total.downloads_kb, 4);
    let base = state.get_filebase(&root.path().join("paid"), &root.path().join("paid-metadata")).await.unwrap();
    let base = base.lock().await;
    assert_eq!(base.iter().find(|header| header.name() == "A.ZIP").unwrap().dl_counter, 1);
    assert_eq!(base.iter().find(|header| header.name() == "B.ZIP").unwrap().dl_counter, 0);
    let log = std::fs::read_to_string(root.path().join("transfer.log")).unwrap();
    assert_eq!(log.lines().count(), 2);
    assert!(!log.contains("B.ZIP"));
}

#[tokio::test]
async fn entirely_failed_batch_neither_counts_partial_bytes_nor_claims_completion() {
    let (root, mut state, mut peer) = fixture("").await;
    let path = root.path().join("paid/A.ZIP");
    state.session.flagged_files.push(path.clone());
    let mut transfer = TransferState::new("test".into());
    transfer.is_finished = true; // finished session is not proof of file delivery
    transfer.send_state.total_bytes_transfered = 1024;
    state.finish_download_batch(&[(path.clone(), 2048)], &transfer, "Z", 1).await.unwrap();
    assert_eq!(state.session.flagged_files, [path]);
    assert_eq!(state.transfer_statistics.downloaded_files, 0);
    assert_eq!(state.transfer_statistics.downloaded_bytes, 0);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 0);
    assert_eq!(state.get_board().await.statistics.total.downloads_kb, 0);
    assert!(output(&mut peer).await.is_empty());
    assert!(!root.path().join("transfer.log").exists());
}

#[tokio::test]
async fn stacked_goodbye_aliases_skip_further_names_and_batch_edit_prompt() {
    for goodbye in ["GB", "BYE"] {
        let (root, mut state, mut peer) = fixture("").await;
        // An unsupported external protocol aborts safely before goodbye. This
        // tests token routing without logging the fixture user off the board.
        state.get_board().await.protocols.iter_mut().find(|p| p.char_code == "Z").unwrap().send_command =
            icy_net::protocol::TransferProtocolType::External("unused".into());
        command(&mut state, &format!("D A.ZIP B.ZIP Z {goodbye}")).await;
        assert_eq!(state.session.flagged_files, [root.path().join("paid/A.ZIP"), root.path().join("paid/B.ZIP")]);
        let output = output(&mut peer).await;
        assert!(!output.contains("not found on disk"), "{output}");
        assert!(!output.contains("Enter the filename to Download"), "{output}");
        assert!(!output.contains("(A)bort"), "{output}");
    }
}

struct ReadErrorConnection;

#[async_trait::async_trait]
impl Connection for ReadErrorConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, _buffer: &mut [u8]) -> icy_net::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "injected transfer failure").into())
    }

    async fn try_read(&mut self, buffer: &mut [u8]) -> icy_net::Result<usize> {
        self.read(buffer).await
    }

    async fn send(&mut self, _buffer: &[u8]) -> icy_net::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn native_protocol_io_failure_keeps_offered_files_and_resets_previous_summary() {
    let (root, mut state, _peer) = fixture("\r").await;
    let path = root.path().join("paid/A.ZIP");
    state.session.flagged_files.push(path.clone());
    state.transfer_statistics.downloaded_files = 42;
    state.transfer_statistics.downloaded_bytes = 9999;
    state.connection = Box::new(ReadErrorConnection);
    tokio::time::timeout(Duration::from_secs(3), state.download(false)).await.unwrap().unwrap();
    assert_eq!(state.session.flagged_files, [path]);
    assert_eq!(state.transfer_statistics.downloaded_files, 0);
    assert_eq!(state.transfer_statistics.downloaded_bytes, 0);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 0);
    assert_eq!(state.get_board().await.statistics.total.downloads, 0);
}
