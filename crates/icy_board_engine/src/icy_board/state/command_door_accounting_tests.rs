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
    state::{KeyChar, KeySource, Logoff},
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

#[cfg(unix)]
fn door_payload_before_reset(bytes: &[u8]) -> &[u8] {
    bytes
        .strip_suffix(b"\x18\x1b[?6l\x1b[r\x1b[?69l\x1b[?7h\x1b[?25h\x1b[0m")
        .expect("door output must end with terminal restoration")
}

#[tokio::test]
async fn pcboard_drop_file_advertises_the_serial_door_connection() {
    let (root, mut state, _peer) = fixture(false).await;
    state.session.is_local = true;
    state.session.cur_user_id = 0;
    state.session.login_date = chrono::Utc::now() - chrono::Duration::minutes(12);
    state.session.current_conference_number = 300;
    for node in [0, 1, 254, 255] {
        state.node = node;
        crate::icy_board::doors::pcboard::create_pcboard(&state, root.path()).await.unwrap();
        let contents = std::fs::read(root.path().join("PCBOARD.SYS")).unwrap();
        assert_eq!(contents.len(), 148);
        assert_eq!(i16::from_le_bytes(contents[54..56].try_into().unwrap()), -12);
        assert_eq!(contents[65], 255);
        assert_eq!(u16::from_le_bytes(contents[131..133].try_into().unwrap()), 437);
        assert_eq!(contents[125], b'1', "PCBoard stores the COM port as an ASCII digit");
        assert_eq!(&contents[13..18], b"57600");
        assert_eq!(&contents[18..23], b"57600", "even a local caller reaches a DOS door over COM1");
        assert_eq!(u16::from_le_bytes(contents[23..25].try_into().unwrap()), 1);
        assert_eq!(contents[111], (node + 1).min(255) as u8);
        assert_eq!(u16::from_le_bytes(contents[146..148].try_into().unwrap()), (node + 1) as u16);
    }
}

#[tokio::test]
async fn pcboard_users_sys_matches_the_fixed_record_contract() {
    use crate::icy_board::doors::pcboard::{create_pcboard, read_user_sys};
    let (root, mut state, _peer) = fixture(false).await;
    state.session.page_len = 43;
    state.session.login_date = chrono::Utc::now() - chrono::Duration::minutes(12);
    let user = state.session.current_user.as_mut().unwrap();
    user.alias = "ALIAS".into();
    user.street1 = "STREET".into();
    user.verify_answer = "VERIFY".into();
    user.stats.num_times_on = 123;
    user.stats.total_dnld_bytes = (1u64 << 33) + 456789;
    create_pcboard(&state, root.path()).await.unwrap();
    assert!(!root.path().join("USER.SYS").exists());
    let path = root.path().join("USERS.SYS");
    let mut bytes = std::fs::read(&path).unwrap();
    assert_eq!(u16::from_le_bytes(bytes[0..2].try_into().unwrap()), 1530);
    assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()), 1007);
    assert_eq!(&bytes[8..14], &[1, 0, 7, 0, 5, 0]);
    assert_eq!(bytes.len(), 40 + 1007 + 4 + 35);
    let record = &bytes[40..1047];
    assert_eq!(&record[..10], b"USAGE TEST");
    assert_eq!(record[99], 0);
    assert_eq!(&record[105..110], &[10, 0, 123, 0, 43]);
    assert_eq!(i16::from_le_bytes(record[180..182].try_into().unwrap()), 12);
    assert_eq!(&record[197..201], &[1, 0, 0, 0]);
    assert_eq!(&record[218..225], b"\x01ALIAS\0");
    assert_eq!(&record[458..466], b"\x01VERIFY\0");
    assert_eq!(f64::from_le_bytes(record[991..999].try_into().unwrap()), ((1u64 << 33) + 456789) as f64);
    bytes[40 + 105..40 + 107].copy_from_slice(&25u16.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let mut returned = User::default();
    read_user_sys(&mut returned, root.path()).unwrap();
    assert_eq!(returned.name, "USAGE TEST");
    assert_eq!(returned.security_level, 25);
    assert_eq!(returned.stats.num_times_on, 123);
    assert_eq!(returned.page_len, 43);
    assert_eq!(returned.alias, "ALIAS");
    assert_eq!(returned.street1, "STREET");
    assert_eq!(returned.verify_answer, "VERIFY");
    assert_eq!(returned.stats.total_dnld_bytes, (1u64 << 33) + 456789);
    for length in [0, 39, 100, bytes.len() - 1] {
        std::fs::write(&path, &bytes[..length]).unwrap();
        assert!(read_user_sys(&mut returned, root.path()).is_err());
        assert_eq!(returned.name, "USAGE TEST");
        assert_eq!(returned.security_level, 25);
    }
    let mut malformed = bytes.clone();
    malformed[8..14].fill(255);
    std::fs::write(&path, malformed).unwrap();
    assert!(read_user_sys(&mut returned, root.path()).is_err());
    bytes[40 + 991..40 + 999].copy_from_slice(&f64::NAN.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!(read_user_sys(&mut returned, root.path()).is_err());
    assert_eq!(returned.stats.total_dnld_bytes, (1u64 << 33) + 456789);
    while state.get_board().await.conferences.len() < 41 {
        state.get_board().await.conferences.push(Default::default());
    }
    state
        .session
        .current_user
        .as_mut()
        .unwrap()
        .lastread_ptr_flags
        .entry((40, 0))
        .or_default()
        .last_read = 1234;
    create_pcboard(&state, root.path()).await.unwrap();
    let contents = std::fs::read(&path).unwrap();
    assert_eq!(&contents[8..14], &[41, 0, 7, 0, 6, 0]);
    assert_eq!(contents.len(), 40 + 1007 + 41 * 4 + 7 * 6);
    assert_eq!(&contents[1047 + 40 * 4..1047 + 41 * 4], &1234u32.to_le_bytes());
    state.session.current_user = None;
    create_pcboard(&state, root.path()).await.unwrap();
    let contents = std::fs::read(&path).unwrap();
    assert!(contents[40..].iter().all(|byte| *byte == 0));
}

#[tokio::test]
#[ignore = "downloads the pinned FreeDOS/BIOS assets and boots the native emulator"]
async fn dos_first_launch_downloads_assets_and_runs_door() {
    let (root, mut state, mut peer) = fixture(false).await;
    state.session.time_limit = 0;
    let source = root.path().join("game");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("START.BAT"), b"@ECHO OFF\r\nECHO FIRST-LAUNCH-OK > COM1\r\n").unwrap();
    let game = Door {
        door_type: DoorType::Dos,
        dos_command: "CALL START.BAT".into(),
        ..door(source.to_str().unwrap())
    };
    state.run_door(&DoorList::default(), &game, 0).await.unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("Preparing DOS files"), "{text}");
    assert!(text.contains("FIRST-LAUNCH-OK"), "{text}");
    let assets = root.path().join("assets/dos");
    assert!(crate::icy_board::doors::dos::dos_assets_ready(&assets));
    assert!(assets.join("doors/game.img").is_file());
    state.run_door(&DoorList::default(), &game, 0).await.unwrap();
    let text = output(&mut peer).await;
    assert!(!text.contains("Preparing DOS files"), "{text}");
    assert!(text.contains("FIRST-LAUNCH-OK"), "{text}");
}

#[tokio::test]
#[ignore = "requires ICB_DOS_ASSETS; validates both PCBoard drop files inside DOS"]
async fn dos_pcboard_launch_uses_the_primary_drop_file() {
    let (root, mut state, mut peer) = fixture(false).await;
    state.session.time_limit = 0;
    let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
    let destination = root.path().join("assets/dos");
    std::fs::create_dir_all(&destination).unwrap();
    for name in ["freedos.img", "seabios.bin", "vgabios.bin"] {
        std::fs::copy(assets.join(name), destination.join(name)).unwrap();
    }
    let source = root.path().join("game");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("START.BAT"), b"@ECHO OFF\r\nIF NOT \"%1\"==\"PCBOARD.SYS\" GOTO FAIL\r\nIF NOT EXIST PCBOARD.SYS GOTO FAIL\r\nIF NOT EXIST USERS.SYS GOTO FAIL\r\nECHO PCBOARD-DROP-OK > COM1\r\nGOTO END\r\n:FAIL\r\nECHO PCBOARD-DROP-FAILED > COM1\r\n:END\r\n").unwrap();
    let game = Door {
        door_type: DoorType::Dos,
        drop_file: crate::icy_board::doors::DropFile::PCBoard,
        dos_command: "CALL START.BAT {dropFile}".into(),
        ..door(source.to_str().unwrap())
    };
    state.run_door(&DoorList::default(), &game, 0).await.unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("PCBOARD-DROP-OK"), "{text}");
    assert!(!text.contains("PCBOARD-DROP-FAILED"), "{text}");
}

#[tokio::test]
async fn dos_preparation_failure_is_visible_and_not_billed() {
    let (root, mut state, mut peer) = fixture(true).await;
    let source = root.path().join("game");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("GAME.EXE"), []).unwrap();
    std::fs::create_dir_all(root.path().join("assets/dos/freedos.img")).unwrap();
    let game = Door {
        door_type: DoorType::Dos,
        dos_command: "GAME.EXE".into(),
        ..door(source.to_str().unwrap())
    };
    let error = state.run_door(&DoorList::default(), &game, 0).await.unwrap_err();
    assert!(error.to_string().contains("DOS asset is not a file"), "{error}");
    let text = output(&mut peer).await;
    assert!(text.contains("Preparing DOS files"), "{text}");
    assert!(text.contains("DOS preparation failed"), "{text}");
    assert_eq!(account(&state).debit_tpu, 0.0);
    assert!(!root.path().join("assets/dos/doors/game.img").exists());
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
    state.logoff_user(Logoff::NORMAL).await.unwrap();
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
    state.logoff_user(Logoff::NORMAL).await.unwrap();
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
    state.logoff_user(Logoff::NORMAL).await.unwrap();
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
    state.logoff_user(Logoff::NORMAL).await.unwrap();
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
        state.logoff_user(Logoff::NORMAL).await.unwrap();
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
async fn native_door32_socket_transport_and_expanded_arguments() {
    for shell in [false, true] {
        let (root, mut state, mut peer) = fixture(false).await;
        let work = root.path().join("work dir");
        let reference = root.path().join("reference");
        std::fs::create_dir(&work).unwrap();
        std::fs::create_dir(&reference).unwrap();
        let mut game = native_door32_fixture(root.path(), "roundtrip", shell);
        game.working_directory = work.to_str().unwrap().to_string();
        game.create_drop_file(&state, &reference, 0).await.unwrap();
        let local = std::fs::read_to_string(reference.join("door32.sys")).unwrap();
        assert_eq!(local.lines().take(2).collect::<Vec<_>>(), ["0", "0"]);
        let client = async {
            native_stdio_frame(&mut peer).await;
            let bytes: Vec<u8> = (0..=255).collect();
            peer.send(&bytes).await.unwrap();
            assert_eq!(native_stdio_frame(&mut peer).await, bytes);
            peer.send(b"Q").await.unwrap();
            assert_eq!(door_payload_before_reset(&native_stdio_frame(&mut peer).await), b"FINAL-OUTPUT");
            peer
        };
        let (result, _peer) = tokio::time::timeout(Duration::from_secs(10), async {
            let list = DoorList::default();
            tokio::join!(state.run_door(&list, &game, 0), client)
        })
        .await
        .unwrap();
        result.unwrap();
        let transport = std::fs::read_to_string(work.join("transport")).unwrap();
        let lines: Vec<_> = transport.lines().collect();
        assert_eq!(lines[0], "2");
        assert!(lines[1].parse::<i32>().unwrap() > 2);
        assert_eq!(lines.len(), 11);
        assert_eq!(&lines[2..], &local.lines().collect::<Vec<_>>()[2..]);
        assert_eq!(transport.matches("\r\n").count(), 11);
        assert_eq!(std::fs::read_to_string(work.join("cwd")).unwrap(), work.to_string_lossy());
        assert!(!work.join("door32.sys").exists() && !root.path().join("door32.sys").exists());
        assert!(!state.session.request_logoff);
    }
}

#[cfg(unix)]
fn native_door32_fixture(root: &std::path::Path, mode: &str, shell: bool) -> Door {
    use std::os::unix::fs::PermissionsExt;

    let path = root.join("socket door");
    let executable = std::env::current_exe().unwrap();
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nICB_DOOR32_TEST_MODE={mode} ICB_DOOR32_DROPFILE=\"$1\" exec {} --exact icy_board::state::menu_runner::command_door_accounting_tests::native_door32_process_worker --ignored --nocapture\n",
            shell_words::quote(executable.to_str().unwrap())
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut game = door(path.to_str().unwrap());
    game.drop_file = crate::icy_board::doors::DropFile::Door32Sys;
    game.provide_socket_connection = true;
    game.args = vec!["{dropFilePath}".into()];
    game.use_shell_execute = shell;
    game
}

#[cfg(unix)]
#[test]
#[ignore = "subprocess helper invoked by native_door32 tests"]
fn native_door32_process_worker() {
    use std::io::{IsTerminal, Read, Write};
    use std::os::fd::FromRawFd;

    let Ok(mode) = std::env::var("ICB_DOOR32_TEST_MODE") else {
        return;
    };
    let drop_file = std::path::PathBuf::from(std::env::var_os("ICB_DOOR32_DROPFILE").unwrap());
    assert!(drop_file.is_absolute(), "{drop_file:?}");
    let transport = std::fs::read_to_string(&drop_file).unwrap();
    let lines: Vec<_> = transport.lines().collect();
    assert_eq!(lines[0], "2");
    let descriptor = lines[1].parse::<i32>().unwrap();
    assert!(descriptor > 2);
    let mut stream = unsafe { std::net::TcpStream::from_raw_fd(descriptor) };
    assert!(stream.peer_addr().unwrap().ip().is_loopback());
    assert!(stream.nodelay().unwrap());
    assert_eq!(rustix::io::fcntl_getfd(&stream).unwrap(), rustix::io::FdFlags::empty());
    assert!(!rustix::fs::fcntl_getfl(&stream).unwrap().contains(rustix::fs::OFlags::NONBLOCK));
    assert!(!std::io::stdin().is_terminal());
    assert!(!std::io::stdout().is_terminal());
    std::fs::write("transport", &transport).unwrap();
    std::fs::write("cwd", std::env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    #[cfg(target_os = "linux")]
    if let Ok(forbidden) = std::fs::read_to_string("forbidden-fds") {
        for entry in std::fs::read_dir("/proc/self/fd").unwrap() {
            if let Ok(target) = std::fs::read_link(entry.unwrap().path()) {
                assert!(!forbidden.lines().any(|line| std::path::Path::new(line) == target), "inherited {target:?}");
            }
        }
    }
    if mode == "drain" {
        stream.write_all(&vec![b'X'; 256 * 1024 + 17]).unwrap();
        stream.write_all(b"FINAL-OUTPUT").unwrap();
        return;
    }
    if mode == "close" {
        stream.write_all(b"CLOSED").unwrap();
        stream.shutdown(std::net::Shutdown::Both).unwrap();
        std::thread::park();
        unreachable!();
    }
    writeln!(stream, "{}", std::process::id()).unwrap();
    if mode == "roundtrip" {
        let mut bytes = [0; 256];
        stream.read_exact(&mut bytes).unwrap();
        assert_eq!(bytes.to_vec(), (0..=255).collect::<Vec<u8>>());
        stream.write_all(&bytes).unwrap();
        let mut quit = [0];
        stream.read_exact(&mut quit).unwrap();
        assert_eq!(&quit, b"Q");
        stream.write_all(b"FINAL-OUTPUT").unwrap();
    } else {
        let mut bytes = [0; 4096];
        loop {
            let count = stream.read(&mut bytes).unwrap();
            if count == 0 {
                break;
            }
            stream.write_all(&bytes[..count]).unwrap();
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_door32_disconnect_and_cancellation_reap_child() {
    for cancel in [false, true] {
        let (root, mut state, mut peer) = fixture(false).await;
        let game = native_door32_fixture(root.path(), "wait", cancel);
        let list = DoorList::default();
        let mut running = Box::pin(state.run_door(&list, &game, 0));
        let header = tokio::select! {
            header = native_stdio_frame(&mut peer) => header,
            result = &mut running => panic!("door ended before disconnect: {result:?}"),
        };
        let pid = rustix::process::Pid::from_raw(String::from_utf8(header).unwrap().trim().parse().unwrap()).unwrap();
        if !cancel {
            drop(peer);
            tokio::time::timeout(Duration::from_secs(3), &mut running).await.unwrap().unwrap();
        }
        drop(running);
        assert_eq!(state.session.request_logoff, !cancel);
        tokio::time::timeout(Duration::from_secs(3), async {
            while rustix::process::test_kill_process(pid).is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(rustix::process::test_kill_process(pid), Err(rustix::io::Errno::SRCH));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_door32_drains_output_and_observes_socket_eof() {
    for mode in ["drain", "close"] {
        let (root, mut state, mut peer) = fixture(false).await;
        let game = native_door32_fixture(root.path(), mode, false);
        tokio::time::timeout(Duration::from_secs(5), state.run_door(&DoorList::default(), &game, 0))
            .await
            .unwrap()
            .unwrap();
        let expected = if mode == "drain" {
            let mut expected = vec![b'X'; 256 * 1024 + 17];
            expected.extend_from_slice(b"FINAL-OUTPUT");
            expected
        } else {
            b"CLOSED".to_vec()
        };
        assert_eq!(door_payload_before_reset(output(&mut peer).await.as_bytes()), expected);
        assert!(!state.session.request_logoff);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_door_max_parallel_refuses_further_callers_for_free() {
    let (root, mut first, mut first_peer) = fixture(false).await;
    let (_second_root, mut second, _second_peer) = fixture(true).await;
    let mut game = native_door32_fixture(root.path(), "wait", false);
    game.name = "PARALLEL GAME".into();
    game.max_parallel = 1;
    let list = DoorList::default();
    let mut running = Box::pin(first.run_door(&list, &game, 0));
    tokio::select! {
        _ = native_stdio_frame(&mut first_peer) => {},
        result = &mut running => panic!("first door ended: {result:?}"),
    }
    tokio::time::timeout(Duration::from_secs(2), second.run_door(&list, &game, 0))
        .await
        .expect("a full door must be refused immediately")
        .unwrap();
    assert_eq!(account(&second).debit_tpu, 0.0);
    drop(running);
    let mut next = native_door32_fixture(root.path(), "drain", true);
    next.name = game.name.clone();
    next.max_parallel = 1;
    tokio::time::timeout(Duration::from_secs(5), second.run_door(&list, &next, 0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account(&second).debit_tpu, 7.0);
}

#[cfg(unix)]
#[tokio::test]
async fn native_door_rejects_socket_placeholder_without_a_socket_connection() {
    let (root, mut state, _peer) = fixture(true).await;
    let mut game = native_door32_fixture(root.path(), "wait", false);
    game.provide_socket_connection = false;
    game.args = vec!["{socketHandle}".into()];
    let error = state.run_door(&DoorList::default(), &game, 0).await.unwrap_err();
    assert!(error.to_string().contains("socket connection"), "{error}");
    assert_eq!(account(&state).debit_tpu, 0.0);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn native_door32_parallel_launches_do_not_inherit_other_sockets() {
    let (root, mut first, mut first_peer) = fixture(false).await;
    let (other_root, mut second, mut second_peer) = fixture(false).await;
    let game = native_door32_fixture(root.path(), "wait", false);
    let other = native_door32_fixture(other_root.path(), "drain", true);
    let list = DoorList::default();
    let mut running = Box::pin(first.run_door(&list, &game, 0));
    tokio::select! {
        _ = native_stdio_frame(&mut first_peer) => {},
        result = &mut running => panic!("first door ended: {result:?}"),
    }
    let forbidden: Vec<_> = std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| std::fs::read_link(entry.unwrap().path()).ok())
        .map(|target| target.to_string_lossy().into_owned())
        .filter(|target| target.starts_with("socket:["))
        .collect();
    assert!(!forbidden.is_empty());
    std::fs::write(other_root.path().join("forbidden-fds"), forbidden.join("\n")).unwrap();
    tokio::time::timeout(Duration::from_secs(5), second.run_door(&list, &other, 0))
        .await
        .unwrap()
        .unwrap();
    let mut expected = vec![b'X'; 256 * 1024 + 17];
    expected.extend_from_slice(b"FINAL-OUTPUT");
    assert_eq!(door_payload_before_reset(output(&mut second_peer).await.as_bytes()), expected);
    drop(first_peer);
    tokio::time::timeout(Duration::from_secs(3), running).await.unwrap().unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn native_door_return_restores_full_screen_rendering() {
    use icy_engine::{Position, TextPane};
    use std::os::unix::fs::PermissionsExt;

    for (width, height, tail) in [(80, 25, ""), (132, 43, ""), (80, 25, "\\033["), (132, 43, "\\033[")] {
        let (root, mut state, mut peer) = fixture(false).await;
        state.set_terminal_size(width, height);
        let path = root.path().join("dirty-terminal-door");
        std::fs::write(&path, format!("#!/bin/sh\nprintf '\\033[1;2r\\033[?6h\\033[?7lDOOR{tail}'\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        tokio::time::timeout(Duration::from_secs(5), state.run_door(&DoorList::default(), &door(path.to_str().unwrap()), 0))
            .await
            .unwrap()
            .unwrap();
        let menu = format!(
            "\x1b[2J\x1b[HHEADER\r\nROW TWO\r\nROW THREE\r\nROW FOUR\x1b[6;{}HXYZ\x1b[{};1HFOOTER",
            width - 1,
            height
        );
        state.print(crate::vm::TerminalTarget::Both, &menu).await.unwrap();
        let bytes = output(&mut peer).await;
        let mut screen = super::super::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
        super::super::virtual_screen::resize_screen(&mut screen.buffer, icy_engine::Size::new(width.into(), height.into()));
        for ch in bytes.chars() {
            screen.print_char(ch).unwrap();
        }
        for (row, column, text) in [
            (0, 0, "HEADER"),
            (1, 0, "ROW TWO"),
            (2, 0, "ROW THREE"),
            (3, 0, "ROW FOUR"),
            (5, i32::from(width) - 2, "XY"),
            (6, 0, "Z"),
            (i32::from(height) - 1, 0, "FOOTER"),
        ] {
            let rendered: String = (0..text.len())
                .map(|offset| screen.buffer.char_at(Position::new(column + offset as i32, row)).ch)
                .collect();
            assert_eq!(rendered, text, "{width}x{height}, row {row}");
            let tracked: String = (0..text.len())
                .map(|offset| state.display_screen().buffer.char_at(Position::new(column + offset as i32, row)).ch)
                .collect();
            assert_eq!(tracked, text, "tracked {width}x{height}, row {row}");
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_stdio_door_has_terminal_streams_and_preserves_ansi_bytes() {
    use std::os::unix::fs::PermissionsExt;

    for (shell, plain_text) in [(false, false), (true, false), (false, true)] {
        let (root, mut state, mut peer) = fixture(false).await;
        state.set_terminal_size(80, 25);
        if plain_text {
            state.session.disp_options.grapics_mode = crate::icy_board::state::GraphicsMode::Ctty;
        }
        let path = root.path().join("stdio-door");
        std::fs::write(
            &path,
            "#!/bin/sh\nif ! test -t 0 || ! test -t 1; then printf 'NOT-A-TERMINAL'; exit 1; fi\nprintf '\\033[2J\\033[9;25H(C) Enter chat!\\r\\nRAW\\r\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut game = door(path.to_str().unwrap());
        game.use_shell_execute = shell;
        tokio::time::timeout(Duration::from_secs(5), state.run_door(&DoorList::default(), &game, 0))
            .await
            .unwrap()
            .unwrap();
        let bytes = output(&mut peer).await;
        let payload = if plain_text {
            bytes.as_bytes()
        } else {
            door_payload_before_reset(bytes.as_bytes())
        };
        assert_eq!(
            payload, b"\x1b[2J\x1b[9;25H(C) Enter chat!\r\nRAW\r\n",
            "shell={shell}, plain_text={plain_text}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_stdio_door_raw_input_size_and_disconnect() {
    use std::os::unix::fs::PermissionsExt;

    for (width, height) in [(80, 25), (132, 43)] {
        for disconnect in [false, true] {
            let (root, mut state, mut peer) = fixture(false).await;
            state.set_terminal_size(width, height);
            let path = root.path().join("stdio door");
            std::fs::write(&path, "#!/bin/sh\nprintf '%s\\n' \"$$\"\nstty size\nexec cat\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut game = door(path.to_str().unwrap());
            game.use_shell_execute = disconnect;
            let client = async {
                let header = native_stdio_frame(&mut peer).await;
                let header = String::from_utf8(header).unwrap();
                let mut lines = header.lines();
                let pid = rustix::process::Pid::from_raw(lines.next().unwrap().parse().unwrap()).unwrap();
                assert_eq!(lines.next().unwrap(), format!("{height} {width}"));
                if !disconnect {
                    let input = [3, 13, 10, 17, 19, 0xff];
                    peer.send(&input).await.unwrap();
                    assert_eq!(native_stdio_frame(&mut peer).await, input);
                }
                drop(peer);
                pid
            };
            let (result, pid) = tokio::time::timeout(Duration::from_secs(10), async {
                let list = DoorList::default();
                tokio::join!(state.run_door(&list, &game, 0), client)
            })
            .await
            .unwrap();
            result.unwrap();
            assert!(state.session.request_logoff);
            assert_eq!(rustix::process::test_kill_process(pid), Err(rustix::io::Errno::SRCH));
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_stdio_door_cancellation_closes_child_and_terminal() {
    use std::os::unix::fs::PermissionsExt;

    let (root, mut state, mut peer) = fixture(false).await;
    let path = root.path().join("stdio-door");
    std::fs::write(&path, "#!/bin/sh\nprintf '%s\\n' \"$$\"\nexec cat\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let game = door(path.to_str().unwrap());
    let list = DoorList::default();
    let mut running = Box::pin(state.run_door(&list, &game, 0));
    let header = tokio::select! {
        header = native_stdio_frame(&mut peer) => header,
        result = &mut running => panic!("door ended before cancellation: {result:?}"),
    };
    let pid = rustix::process::Pid::from_raw(String::from_utf8(header).unwrap().trim().parse().unwrap()).unwrap();
    drop(running);
    tokio::time::timeout(Duration::from_secs(3), async {
        while rustix::process::test_kill_process(pid).is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(rustix::process::test_kill_process(pid), Err(rustix::io::Errno::SRCH));
}

#[cfg(unix)]
#[tokio::test]
async fn native_stdio_door_drains_output_after_process_exit() {
    use std::os::unix::fs::PermissionsExt;

    let (root, mut state, mut peer) = fixture(false).await;
    let path = root.path().join("stdio-door");
    std::fs::write(&path, "#!/bin/sh\ncat payload\nprintf 'FINAL-OUTPUT'\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut expected = vec![b'X'; 256 * 1024 + 17];
    std::fs::write(root.path().join("payload"), &expected).unwrap();
    expected.extend_from_slice(b"FINAL-OUTPUT");
    tokio::time::timeout(Duration::from_secs(10), state.run_door(&DoorList::default(), &door(path.to_str().unwrap()), 0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(door_payload_before_reset(output(&mut peer).await.as_bytes()), expected);
    assert!(!state.session.request_logoff);
}

#[cfg(unix)]
async fn native_stdio_frame(peer: &mut ChannelConnection) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let count = tokio::time::timeout(Duration::from_secs(3), peer.read(&mut buffer)).await.unwrap().unwrap();
    assert!(count > 0);
    bytes.extend_from_slice(&buffer[..count]);
    while let Ok(result) = tokio::time::timeout(Duration::from_millis(150), peer.read(&mut buffer)).await {
        let count = result.unwrap();
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    bytes
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires ICB_UMRC_TEST_DIR with original uMRC 105 and Probe fixture"]
async fn native_door_umrc_original_menu_roundtrip() {
    use icy_engine::{Position, TextPane};
    use std::os::unix::fs::{PermissionsExt, symlink};

    let source = PathBuf::from(std::env::var_os("ICB_UMRC_TEST_DIR").expect("set ICB_UMRC_TEST_DIR"));
    for (socket, width, height) in [(false, 80, 25), (false, 132, 43), (true, 80, 25), (true, 132, 43)] {
        let (root, mut state, mut peer) = fixture(false).await;
        state.set_terminal_size(width, height);
        state.session.user_name = "Probe".into();
        state.session.alias_name = "Probe".into();
        for name in ["mrc.cfg", "umrc-original"] {
            std::fs::copy(source.join(name), root.path().join(name)).unwrap();
        }
        std::fs::create_dir(root.path().join("userdata")).unwrap();
        std::fs::copy(source.join("userdata/Probe.dat"), root.path().join("userdata/Probe.dat")).unwrap();
        std::fs::write(root.path().join("mrcstats.dat"), "1 1 1 0 0\n").unwrap();
        for name in ["screens", "themes"] {
            symlink(source.join("assets").join(name), root.path().join(name)).unwrap();
        }
        let path = root.path().join("stdio-door");
        std::fs::write(&path, "#!/bin/sh\nexec ./umrc-original \"$@\"\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut game = door(path.to_str().unwrap());
        game.drop_file = crate::icy_board::doors::DropFile::Door32Sys;
        game.args = vec!["-D".into(), "{dropFilePath}".into()];
        game.provide_socket_connection = socket;
        let client = async {
            let mut screen = super::super::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
            super::super::virtual_screen::resize_screen(&mut screen.buffer, icy_engine::Size::new(width.into(), height.into()));
            for (index, input) in [b"I".as_slice(), b" ", b"Q"].into_iter().enumerate() {
                let frame = native_stdio_frame(&mut peer).await;
                for byte in frame {
                    screen.print_char(codepages::tables::CP437_TO_UNICODE[byte as usize]).unwrap();
                }
                if index != 1 {
                    for (column, row, text) in [(24, 8, "(C) Enter chat!"), (24, 12, "(Q) Quit to"), (28, 13, "Make a selection")] {
                        let rendered: String = (0..text.len())
                            .map(|offset| screen.buffer.char_at(Position::new(column + offset as i32, row)).ch)
                            .collect();
                        assert_eq!(rendered, text, "socket={socket} {width}x{height} menu {index}");
                    }
                }
                peer.send(input).await.unwrap();
            }
            peer
        };
        let (result, _peer) = tokio::time::timeout(Duration::from_secs(15), async {
            let list = DoorList::default();
            tokio::join!(state.run_door(&list, &game, 0), client)
        })
        .await
        .unwrap();
        result.unwrap();
        assert!(!state.session.request_logoff);
    }
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
