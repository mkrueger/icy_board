//! Configured command/door surcharges: independent from global online time.
//! Source references: PCB.H cmdtype; CMDS.C runcmds; DOORS.C searchdoorlist;
//! INIT.C backfromdos. Unlike PCBoard, failed door launches are free and door
//! minutes use elapsed nearest-minute rounding rather than log timestamps.
use super::*;
use crate::icy_board::{
    IcyBoard, PCBoardRecordImporter,
    accounting_cfg::AccountingConfig,
    bbs::BBS,
    commands::CommandList,
    doors::{Door, DoorList, DoorType},
    pcb::user_inf::AccountUserInf,
    sec_levels::SecurityLevel,
    state::{KeyChar, KeySource},
    user_base::User,
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::sync::Mutex;

async fn fixture(enabled: bool) -> (TempDir, IcyBoardState, ChannelConnection) {
    let root = tempfile::tempdir().unwrap();
    let mut board = IcyBoard::new();
    board.root_path = root.path().into();
    board.config.paths.user_file = root.path().join("users.toml");
    board.config.paths.caller_log = PathBuf::new();
    board.config.accounting.enabled = enabled;
    board.config.accounting.ignore_empty_sec_level = false;
    board.config.accounting.concurrent_tracking = false;
    board.config.accounting.info_file = PathBuf::new();
    board.config.accounting.warning_file = PathBuf::new();
    board.config.accounting.logoff_file = PathBuf::new();
    board.config.accounting.tracking_file = root.path().join("usage.dbf");
    board.config.accounting.peak_holiday_list_file = PathBuf::new();
    board.config.accounting.accounting_config = Some(AccountingConfig::default());
    board.sec_levels.push(SecurityLevel {
        security: 10,
        is_enabled: true,
        time_per_day: 60,
        ..Default::default()
    });
    board.users.new_user(User {
        name: "USAGE TEST".into(),
        security_level: 10,
        account: Some(AccountUserInf {
            starting_balance: 100.0,
            ..Default::default()
        }),
        ..Default::default()
    });
    let user = board.users[0].clone();
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.session.user_name = "USAGE TEST".into();
    state.session.page_len = 0;
    state.session.disp_options.count_lines = false;
    state.accounting_start().await.unwrap();
    assert_eq!(state.accounting_active(), enabled);
    (root, state, peer)
}

fn account(state: &IcyBoardState) -> &AccountUserInf {
    state.session.current_user.as_ref().unwrap().account.as_ref().unwrap()
}

fn balance(state: &mut IcyBoardState, amount: f64) {
    state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = amount;
}

fn command(kind: CommandType, parameter: &str) -> Command {
    Command {
        keyword: "PAID".into(),
        charge_per_use: 3.0,
        charge_per_minute: 2.0,
        actions: vec![CommandAction {
            command_type: kind,
            parameter: parameter.into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn door(path: &str) -> Door {
    Door {
        name: "GAME".into(),
        path: path.into(),
        charge_per_use: 7.0,
        charge_per_minute: 4.0,
        ..Default::default()
    }
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

#[tokio::test]
async fn custom_bye_command_settles_minutes_before_final_summary_and_shutdown() {
    let (root, mut state, mut peer) = fixture(true).await;
    let file = root.path().join("final-balance");
    std::fs::write(&file, "FINAL-LEFT=@CREDLEFT@ FINAL-NOW=@CREDNOW@\r\n").unwrap();
    state.session.accounting.options.logoff_file = file;
    state.get_board().await.config.paths.command_display_path = root.path().join("missing-commands");
    let cmd = command(CommandType::Bye, "");
    let mut usage = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", &cmd.keyword, cmd.charge_per_use, cmd.charge_per_minute);
    usage.start(&mut state).unwrap();
    // Exercise the real configured BYE action without sleeping for billable time.
    usage.started = Some(Instant::now() - Duration::from_secs(95));
    let result = state.run_action(&cmd, &cmd.actions[0], false, Some(&mut usage)).await;
    assert!(result.is_ok());
    assert!(state.accounting_active());
    assert!(state.session.request_logoff);
    assert!(output(&mut peer).await.is_empty());
    // hangup has not closed the socket while the command is still unwinding.
    state.connection.send(b"STILL-OPEN\r\n").await.unwrap();
    usage.finish(&mut state, result).await.unwrap();
    assert!(!state.accounting_active());
    assert!(!state.accounting_invocation_active());
    assert_eq!(account(&state).debit_tpu, 7.0);
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 7.0);
    let text = output(&mut peer).await;
    assert!(text.contains("STILL-OPEN"), "{text}");
    assert!(text.contains("FINAL-LEFT=93 FINAL-NOW=7"), "{text}");
    for marker in ["FINAL-LEFT", "Credits Used:", "Credits Left:", "Minutes Used"] {
        assert_eq!(text.matches(marker).count(), 1, "{marker}: {text}");
    }
    let audit = std::fs::read(root.path().join("usage.dbf")).unwrap();
    state.logoff_user(false).await.unwrap();
    state.accounting_finish().await.unwrap();
    assert!(output(&mut peer).await.is_empty());
    assert_eq!(std::fs::read(root.path().join("usage.dbf")).unwrap(), audit);
}

#[tokio::test]
async fn nested_command_and_door_finish_requests_settle_once_even_on_handler_error() {
    let (root, mut state, mut peer) = fixture(true).await;
    let file = root.path().join("final-balance");
    std::fs::write(&file, "FINAL-LEFT=@CREDLEFT@ FINAL-NOW=@CREDNOW@\r\n").unwrap();
    state.session.accounting.options.logoff_file = file;
    let mut outer = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", "OUTER", 3.0, 2.0);
    let mut inner = ActivityUsage::new("DOOR USAGE", "DOOR USAGE MIN", "INNER", 7.0, 4.0);
    outer.start(&mut state).unwrap();
    inner.start(&mut state).unwrap();
    outer.started = Some(Instant::now() - Duration::from_secs(95));
    inner.started = Some(Instant::now() - Duration::from_secs(95));
    state.logoff_user(false).await.unwrap();
    state.accounting_finish().await.unwrap();
    let error = inner.finish(&mut state, Err("door handler failed".into())).await.unwrap_err();
    assert!(state.accounting_active());
    assert!(state.accounting_invocation_active());
    assert_eq!(account(&state).debit_tpu, 18.0);
    assert!(output(&mut peer).await.is_empty());
    let error = outer.finish(&mut state, Err(error)).await.unwrap_err();
    assert_eq!(error.to_string(), "door handler failed");
    assert!(!state.accounting_active());
    assert!(!state.accounting_invocation_active());
    assert_eq!(account(&state).debit_tpu, 22.0);
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 22.0);
    let text = output(&mut peer).await;
    assert!(text.contains("FINAL-LEFT=78 FINAL-NOW=22"), "{text}");
    assert_eq!(text.matches("Credits Used:").count(), 1);
    let audit = std::fs::read(root.path().join("usage.dbf")).unwrap();
    state.accounting_finish().await.unwrap();
    assert_eq!(std::fs::read(root.path().join("usage.dbf")).unwrap(), audit);
}

#[tokio::test]
async fn explicit_finish_inside_nested_usage_keeps_outer_posting_enabled() {
    let (_root, mut state, _peer) = fixture(true).await;
    let mut outer = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", "OUTER", 3.0, 2.0);
    let mut inner = ActivityUsage::new("DOOR USAGE", "DOOR USAGE MIN", "INNER", 7.0, 4.0);
    outer.start(&mut state).unwrap();
    inner.start(&mut state).unwrap();
    outer.started = Some(Instant::now() - Duration::from_secs(95));
    inner.started = Some(Instant::now() - Duration::from_secs(95));
    balance(&mut state, 10.0);
    state.accounting_finish().await.unwrap();
    assert!(!state.session.request_logoff, "explicit finish does not require a display/hangup");
    inner.finish(&mut state, Ok(())).await.unwrap();
    assert!(state.accounting_active(), "a security drop during unwind would discard the outer charge");
    outer.finish(&mut state, Ok(())).await.unwrap();
    assert!(!state.accounting_active());
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 22.0);
    assert_eq!(state.session.calculate_balance(), -12.0);
}

#[tokio::test]
async fn failed_nested_minute_post_suppresses_incomplete_final_summary() {
    let (_root, mut state, mut peer) = fixture(true).await;
    let mut outer = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", "OUTER", 3.0, 2.0);
    let mut inner = ActivityUsage::new("DOOR USAGE", "DOOR USAGE MIN", "INNER", 0.0, f64::MAX);
    outer.start(&mut state).unwrap();
    inner.start(&mut state).unwrap();
    outer.started = Some(Instant::now() - Duration::from_secs(95));
    inner.started = Some(Instant::now() - Duration::from_secs(95));
    state.logoff_user(false).await.unwrap();
    let error = inner.finish(&mut state, Ok(())).await.unwrap_err();
    assert!(state.accounting_active());
    assert!(outer.finish(&mut state, Err(error)).await.is_err());
    assert!(!state.accounting_invocation_active());
    assert!(!state.accounting_active());
    assert!(state.session.logoff_pending.is_none());
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 7.0);
    assert!(output(&mut peer).await.is_empty(), "failed charges must not be presented as a final total");
}

#[tokio::test]
async fn deferred_finish_save_failure_unwinds_and_retry_does_not_rebill_or_show_summary() {
    let (root, mut state, mut peer) = fixture(true).await;
    let mut usage = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", "BYE", 3.0, 2.0);
    usage.start(&mut state).unwrap();
    usage.started = Some(Instant::now() - Duration::from_secs(95));
    state.logoff_user(false).await.unwrap();
    state.get_board().await.config.paths.user_file = root.path().to_path_buf();
    assert!(usage.finish(&mut state, Ok(())).await.is_err());
    assert!(!state.accounting_invocation_active());
    assert!(!state.accounting_active());
    assert!(state.session.logoff_pending.is_none());
    assert_eq!(account(&state).debit_tpu, 7.0);
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 0.0);
    assert!(output(&mut peer).await.is_empty());
    state.get_board().await.config.paths.user_file = root.path().join("users.toml");
    state.accounting_finish().await.unwrap();
    state.accounting_finish().await.unwrap();
    assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, 7.0);
    assert!(output(&mut peer).await.is_empty());
}

#[tokio::test]
async fn free_command_still_unwinds_logoff_and_disconnected_summary_cannot_lose_charges() {
    for per_use in [0.0, 3.0] {
        let (_root, mut state, peer) = fixture(true).await;
        let mut usage = ActivityUsage::new("CMD USAGE", "CMD USAGE MIN", "BYE", per_use, 0.0);
        usage.start(&mut state).unwrap();
        state.logoff_user(false).await.unwrap();
        assert!(state.accounting_active());
        drop(peer);
        let error = usage.finish(&mut state, Err("original I/O failure".into())).await.unwrap_err();
        assert_eq!(error.to_string(), "original I/O failure");
        assert!(!state.accounting_invocation_active());
        assert!(!state.accounting_active());
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_tpu, per_use);
    }
}

#[test]
fn rates_default_to_zero_and_roundtrip_only_when_configured() {
    let old: Command = toml::from_str("keyword = 'OLD'").unwrap();
    assert_eq!((old.charge_per_use, old.charge_per_minute), (0.0, 0.0));
    assert!(!toml::to_string(&old).unwrap().contains("charge_per_"));
    let paid = command(CommandType::QuitMenu, "");
    let loaded: Command = toml::from_str(&toml::to_string(&paid).unwrap()).unwrap();
    assert_eq!((loaded.charge_per_use, loaded.charge_per_minute), (3.0, 2.0));
    let old_text = toml::to_string(&Door::default()).unwrap();
    assert!(!old_text.contains("charge_per_"));
    let old: Door = toml::from_str(&old_text).unwrap();
    assert_eq!((old.charge_per_use, old.charge_per_minute), (0.0, 0.0));
    let paid = door("game");
    let loaded: Door = toml::from_str(&toml::to_string(&paid).unwrap()).unwrap();
    assert_eq!((loaded.charge_per_use, loaded.charge_per_minute), (7.0, 4.0));
}

#[test]
fn cmdlst_import_widens_two_packed_ieee_floats_in_use_minute_order() {
    let mut record = [b' '; 64];
    record[..4].copy_from_slice(b"GAME");
    record[15] = 10;
    record[16..24].copy_from_slice(b"GAME.PPE");
    record[56..60].copy_from_slice(&1.25f32.to_le_bytes());
    record[60..64].copy_from_slice(&2.5f32.to_le_bytes());
    let loaded = CommandList::load_pcboard_record(&record).unwrap();
    assert_eq!((loaded.charge_per_use, loaded.charge_per_minute), (1.25, 2.5));
    assert!(loaded.actions[0].command_type == CommandType::RunPPE);
    assert!(CommandList::load_pcboard_record(&record[..56]).is_err());
}

#[test]
fn doorslst_import_accepts_old_and_optional_rate_columns() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("doors.lst");
    std::fs::write(
        &path,
        "OLD,,10,0,0,C:\\DOOR,0,N\nNEW,,10,0,0,C:\\DOOR,0,Y, 1.25 , 2.5 ,0\nONE,,10,0,0,C:\\DOOR,0,Y,3.75\n",
    )
    .unwrap();
    let list = DoorList::import_pcboard(&path).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!((list[0].charge_per_use, list[0].charge_per_minute), (0.0, 0.0));
    assert_eq!((list[1].charge_per_use, list[1].charge_per_minute), (1.25, 2.5));
    assert_eq!((list[2].charge_per_use, list[2].charge_per_minute), (3.75, 0.0));
}

#[tokio::test]
async fn command_multiple_actions_bill_once_and_disabled_or_selection_only_are_free() {
    let (_root, mut state, _peer) = fixture(true).await;
    let mut cmd = command(CommandType::QuitMenu, "");
    cmd.actions.push(CommandAction {
        command_type: CommandType::ExitMenus,
        ..Default::default()
    });
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert!(state.quit_menu && state.exit_menus);
    assert_eq!(account(&state).debit_tpu, 3.0);
    assert_eq!(account(&state).debit_special, 0.0);
    let disabled = command(CommandType::Disabled, "");
    state.dispatch_command("PAID", &disabled).await.unwrap();
    cmd.actions.iter_mut().for_each(|action| action.trigger = ActionTrigger::Selection);
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 3.0);
}

#[tokio::test]
async fn command_security_and_first_minute_preflight_are_unbilled() {
    let (_root, mut state, _peer) = fixture(true).await;
    let mut cmd = command(CommandType::QuitMenu, "");
    cmd.security = SecurityExpression::from_req_security(50);
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert!(!state.quit_menu);
    cmd.security = SecurityExpression::default();
    balance(&mut state, 4.0); // use fits, use + first minute does not
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert!(!state.quit_menu);
    assert_eq!(account(&state).debit_tpu, 0.0);
    balance(&mut state, 5.0);
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert!(state.quit_menu);
    assert_eq!(account(&state).debit_tpu, 3.0); // admission is not a minimum minute
}

#[tokio::test]
async fn minute_rounding_and_error_disconnect_settlement_use_tpu_and_exact_activities() {
    for (activity, minute_activity) in [("CMD USAGE", "CMD USAGE MIN"), ("DOOR USAGE", "DOOR USAGE MIN")] {
        for (seconds, minutes) in [(29, 0), (30, 1), (89, 1), (90, 2)] {
            let (root, mut state, _peer) = fixture(true).await;
            let mut usage = ActivityUsage::new(activity, minute_activity, "TEST", 3.0, 2.0);
            assert!(usage.allowed(&mut state).await.unwrap());
            let limit = state.session.time_limit;
            let login = state.session.login_date;
            usage.start(&mut state).unwrap();
            usage.start(&mut state).unwrap(); // idempotent across multiple actions
            usage.started = Some(Instant::now() - Duration::from_secs(seconds));
            state.session.request_logoff = true;
            let error = usage.finish(&mut state, Err("handler failed".into())).await.unwrap_err();
            assert_eq!(error.to_string(), "handler failed");
            assert_eq!(account(&state).debit_tpu, 3.0 + 2.0 * minutes as f64);
            assert_eq!(account(&state).debit_special, 0.0);
            assert_eq!(state.session.time_limit, limit);
            assert_eq!(state.session.login_date, login);
            let audit = std::fs::read(root.path().join("usage.dbf")).unwrap();
            assert!(audit.windows(activity.len()).any(|bytes| bytes == activity.as_bytes()));
            assert_eq!(
                audit.windows(minute_activity.len()).any(|bytes| bytes == minute_activity.as_bytes()),
                minutes > 0
            );
        }
    }
}

#[tokio::test]
async fn disabled_accounting_is_free_and_invalid_rates_fail_before_execution() {
    let (_root, mut state, _peer) = fixture(false).await;
    state.dispatch_command("PAID", &command(CommandType::QuitMenu, "")).await.unwrap();
    assert!(state.quit_menu);
    assert_eq!(account(&state).debit_tpu, 0.0);
    let (_root, mut state, _peer) = fixture(true).await;
    for rate in [f64::NAN, f64::INFINITY, -1.0] {
        let mut cmd = command(CommandType::QuitMenu, "");
        cmd.charge_per_minute = rate;
        assert!(state.dispatch_command("PAID", &cmd).await.is_err());
        assert!(!state.quit_menu);
        assert_eq!(account(&state).debit_tpu, 0.0);
    }
}

#[tokio::test]
async fn door_denial_password_cancel_selection_and_setup_failures_are_free() {
    let (root, mut state, _peer) = fixture(true).await;
    let list = DoorList::default();
    let mut game = door(root.path().join("missing").to_str().unwrap());
    game.securiy_level = SecurityExpression::from_req_security(50);
    state.run_door(&list, &game, 0).await.unwrap();
    game.securiy_level = SecurityExpression::default();
    game.password = "SECRET".into();
    state.char_buffer.extend("WRONG\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    state.run_door(&list, &game, 0).await.unwrap();
    game.password.clear();
    state.session.flagged_files.push(root.path().join("flagged.zip"));
    state.char_buffer.extend("N\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    state.run_door(&list, &game, 0).await.unwrap();
    state.session.flagged_files.clear();
    balance(&mut state, 10.0);
    state.run_door(&list, &game, 0).await.unwrap(); // 7 + 4 does not fit
    balance(&mut state, 100.0);
    assert!(state.run_door(&list, &game, 0).await.is_err()); // spawn fails
    game.door_type = DoorType::Dos;
    assert!(state.run_door(&list, &game, 0).await.is_err()); // invalid DOS directory
    game.door_type = DoorType::BBSlink;
    assert!(state.run_door(&list, &game, 0).await.is_err()); // no server account
    state.session.current_conference.doors = Some(Arc::new(DoorList {
        doors: vec![game],
        accounts: vec![],
    }));
    assert!(!state.run_named_door("ABSENT").await.unwrap());
    state.session.tokens.push_back("99".into());
    state.open_door().await.unwrap();
    assert_eq!(account(&state).debit_tpu, 0.0);
}

#[tokio::test]
async fn invalid_door_or_failed_spawn_does_not_charge_its_invoking_command() {
    let (root, mut state, _peer) = fixture(true).await;
    let game = door(root.path().join("missing").to_str().unwrap());
    state.session.current_conference.doors = Some(Arc::new(DoorList {
        doors: vec![game.clone()],
        accounts: vec![],
    }));
    state.dispatch_command("PAID", &command(CommandType::Door, "ABSENT")).await.unwrap();
    assert!(state.dispatch_command("PAID", &command(CommandType::Door, "GAME")).await.is_err());
    let mut denied = game;
    denied.securiy_level = SecurityExpression::from_req_security(50);
    state.session.current_conference.doors = Some(Arc::new(DoorList {
        doors: vec![denied],
        accounts: vec![],
    }));
    state.dispatch_command("PAID", &command(CommandType::Door, "GAME")).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 0.0);
}

#[tokio::test]
async fn missing_or_corrupt_ppe_and_invalid_menu_script_selection_are_free() {
    let (root, mut state, _peer) = fixture(true).await;
    let path = root.path().join("invalid.ppe");
    let game = door(path.to_str().unwrap());
    state.run_door(&DoorList::default(), &game, 0).await.unwrap();
    std::fs::write(&path, b"not an executable").unwrap();
    state.run_door(&DoorList::default(), &game, 0).await.unwrap();
    state
        .dispatch_command("PAID", &command(CommandType::Menu, root.path().join("missing").to_str().unwrap()))
        .await
        .unwrap();
    state.dispatch_command("PAID", &command(CommandType::Script, "999")).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 0.0);
}

#[cfg(unix)]
#[tokio::test]
async fn started_local_door_bills_once_and_only_explicit_command_rates_add_to_it() {
    let (_root, mut state, _peer) = fixture(true).await;
    let game = door("/bin/true");
    state.session.current_conference.doors = Some(Arc::new(DoorList {
        doors: vec![game.clone()],
        accounts: vec![],
    }));
    let mut cmd = command(CommandType::Door, "GAME");
    cmd.charge_per_use = 0.0;
    cmd.charge_per_minute = 0.0;
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 7.0);
    cmd.charge_per_use = 3.0;
    cmd.charge_per_minute = 2.0;
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 17.0);
    assert_eq!(account(&state).debit_special, 0.0);
    // Both activities individually fit, but their combined admission does not.
    balance(&mut state, 17.0 + 15.0);
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 17.0);
}

#[cfg(unix)]
#[tokio::test]
async fn multiple_door_actions_charge_each_launch_but_the_command_only_once() {
    let (_root, mut state, _peer) = fixture(true).await;
    state.session.current_conference.doors = Some(Arc::new(DoorList {
        doors: vec![door("/bin/true")],
        accounts: vec![],
    }));
    let mut cmd = command(CommandType::Door, "GAME");
    cmd.actions.push(cmd.actions[0].clone());
    state.dispatch_command("PAID", &cmd).await.unwrap();
    assert_eq!(account(&state).debit_tpu, 3.0 + 7.0 + 7.0);
}

#[cfg(unix)]
#[tokio::test]
async fn started_local_door_settles_on_caller_eof() {
    let (_root, mut state, peer) = fixture(true).await;
    drop(peer);
    state.run_door(&DoorList::default(), &door("/bin/cat"), 0).await.unwrap();
    assert!(state.session.request_logoff);
    assert_eq!(account(&state).debit_tpu, 7.0);
}

#[tokio::test]
async fn command_handler_io_error_keeps_use_charge_and_stops_later_actions() {
    let (_root, mut state, peer) = fixture(true).await;
    drop(peer);
    let mut cmd = command(CommandType::PrintText, "Output to disconnected caller");
    cmd.actions.push(CommandAction {
        command_type: CommandType::QuitMenu,
        ..Default::default()
    });
    assert!(state.dispatch_command("PAID", &cmd).await.is_err());
    assert_eq!(account(&state).debit_tpu, 3.0);
    assert!(!state.quit_menu);
}
