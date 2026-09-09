//! Focused PCBoard PWRD/PPE accounting regressions; no external board or PPE required.

use std::sync::Arc;

use crate::{
    executable::{EntryType, PPEExpr, TableEntry, VarHeader, VariableValue},
    icy_board::{
        IcyBoard, PCBoardTextImport,
        accounting::AccountingMode,
        accounting_cfg::AccountingConfig,
        bbs::BBS,
        pcb::user_inf::AccountUserInf,
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
        state::IcyBoardState,
        user_base::{User, UserBase},
    },
    vm::{DiskIO, VirtualMachine, expressions},
};
use icy_net::{ConnectionType, channel::ChannelConnection};

#[test]
fn pwrd_accounting_modes_roundtrip_and_keep_legacy_enabled() {
    let levels = SecurityLevelDefinitions::import_data(
        ["N", "Y", "T"]
            .iter()
            .map(|mode| format!(",10,60,100,0,0,0,0,0,0,N,N,N,N,N,0,0,{mode}\r\n"))
            .collect(),
    )
    .unwrap();
    assert!(!levels[0].is_enabled && !levels[0].accounting_tracking);
    assert!(levels[1].is_enabled && !levels[1].accounting_tracking);
    assert!(!levels[2].is_enabled && levels[2].accounting_tracking);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("pwrd");
    levels.export_pcboard(&file).unwrap();
    let exported = std::fs::read_to_string(&file).unwrap();
    assert!(levels == SecurityLevelDefinitions::import_data(exported).unwrap());

    let legacy: SecurityLevel = toml::from_str("enabled = true").unwrap();
    assert!(legacy.is_enabled && !legacy.accounting_tracking);
    let serialized = toml::to_string(&legacy).unwrap();
    assert!(serialized.contains("enabled = true"));
    assert!(!serialized.contains("accounting_tracking"));
    let tracking: SecurityLevel = toml::from_str("enabled = true\naccounting_tracking = true").unwrap();
    let serialized = toml::to_string(&tracking).unwrap();
    assert!(tracking == toml::from_str::<SecurityLevel>(&serialized).unwrap());
    SecurityLevelDefinitions { levels: vec![tracking] }.export_pcboard(&file).unwrap();
    assert!(std::fs::read_to_string(file).unwrap().ends_with(",T\r\n"));
}

async fn state() -> IcyBoardState {
    state_with_tracking(None).await
}

async fn state_with_tracking(tracking: Option<&std::path::Path>) -> IcyBoardState {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.config.accounting.enabled = true;
    board.config.accounting.ignore_empty_sec_level = true;
    board.config.accounting.accounting_config = Some(AccountingConfig::default());
    if let Some(tracking) = tracking {
        board.config.accounting.tracking_file = tracking.to_path_buf();
    }
    board.sec_levels.levels.push(SecurityLevel {
        security: 10,
        is_enabled: true,
        ..Default::default()
    });
    board.users.new_user(User {
        name: "CALLER".into(),
        security_level: 10,
        account: Some(AccountUserInf {
            starting_balance: 100.0,
            ..Default::default()
        }),
        ..Default::default()
    });
    board.users.new_user(User {
        name: "OTHER".into(),
        account: Some(AccountUserInf::default()),
        ..Default::default()
    });
    let user = board.users[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.session.user_name = "CALLER".into();
    state.accounting_start().await.unwrap();
    state
}

fn arguments(vm: &mut VirtualMachine<'_>, values: Vec<VariableValue>) -> Vec<PPEExpr> {
    values
        .into_iter()
        .map(|value| {
            let id = vm.variable_table.len() + 1;
            vm.variable_table.push(TableEntry {
                name: format!("arg{id}"),
                header: VarHeader {
                    id,
                    variable_type: value.vtype,
                    ..Default::default()
                },
                value,
                entry_type: EntryType::Constant,
                function_id: 0,
            });
            PPEExpr::Value(id)
        })
        .collect()
}

async fn read_account(vm: &mut VirtualMachine<'_>, field: i32) -> f64 {
    let args = arguments(vm, vec![VariableValue::new_int(field)]);
    expressions::account(vm, &args).await.unwrap().as_double()
}

async fn adjust(vm: &mut VirtualMachine<'_>, field: i32, amount: f64) -> crate::Res<()> {
    let args = arguments(vm, vec![VariableValue::new_int(field), VariableValue::new_double(amount)]);
    super::account(vm, &args).await
}

async fn usage(vm: &mut VirtualMachine<'_>, field: i32, cost: f64, quantity: i32) -> crate::Res<()> {
    let args = arguments(
        vm,
        vec![
            VariableValue::new_int(field),
            VariableValue::new_string("PPE TEST".into()),
            VariableValue::new_string("DETAIL".into()),
            VariableValue::new_double(cost),
            VariableValue::new_int(quantity),
        ],
    );
    super::recordusage(vm, &args).await
}

fn prepare_user_variables(vm: &mut VirtualMachine<'_>) {
    // Direct GETUSER/GETALTUSER/PUTUSER calls need the same U_* storage as a
    // loaded PPE. A live U_* reference explicitly retains the complete block,
    // including array storage initialized by the executable round trip.
    vm.variable_table = crate::vm::tests::compile("GETUSER\nPRINT U_EXPERT\nPUTUSER\n").variable_table;
    assert!(vm.variable_table.has_user_vars());
    assert!(vm.variable_table.len() >= crate::executable::USER_VARIABLES.len());
}

#[tokio::test]
async fn account_is_additive_and_independent_of_mode_and_rates() {
    let mut state = state().await;
    state.session.accounting.mode = AccountingMode::Disabled;
    state.get_board().await.config.accounting.accounting_config = None;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    vm.user = vm.icy_board_state.session.current_user.clone().unwrap();
    for field in 0..=16 {
        let before = read_account(&mut vm, field).await;
        adjust(&mut vm, field, 2.5).await.unwrap();
        adjust(&mut vm, field, -0.75).await.unwrap();
        assert_eq!(read_account(&mut vm, field).await, before + 1.75, "field {field}");
    }
    for (amount, expected) in [(-1.0, 0.0), (42.0, 42.0), (300.0, 255.0)] {
        adjust(&mut vm, 17, amount).await.unwrap();
        assert_eq!(read_account(&mut vm, 17).await, expected);
    }
    let before = vm.user.account.clone();
    adjust(&mut vm, -1, 1.0).await.unwrap();
    adjust(&mut vm, 18, 1.0).await.unwrap();
    assert_eq!(vm.user.account, before);
}

#[tokio::test]
async fn live_runtime_charges_survive_account_read_write_and_putuser() {
    let mut state = state().await;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    prepare_user_variables(&mut vm);
    super::getuser(&mut vm, &[]).await.unwrap();
    vm.icy_board_state.accounting_record(9, "DOWNLOAD", "", 3.0, 1).unwrap();
    assert_eq!(read_account(&mut vm, 9).await, 3.0);
    vm.icy_board_state.accounting_record(9, "DOWNLOAD", "", 4.0, 1).unwrap();
    adjust(&mut vm, 13, 2.0).await.unwrap();
    vm.icy_board_state.accounting_record(9, "DOWNLOAD", "", 5.0, 1).unwrap();
    usage(&mut vm, 13, 0.25, 3).await.unwrap();
    vm.icy_board_state.accounting_record(9, "DOWNLOAD", "", 6.0, 1).unwrap();
    super::putuser(&mut vm, &[]).await.unwrap();
    let account = vm.icy_board_state.session.current_user.as_ref().unwrap().account.as_ref().unwrap();
    assert_eq!(account.debit_download_file, 18.0);
    assert_eq!(account.debit_special, 2.75);
    assert_eq!(vm.user.account.as_ref().unwrap(), account);
}

#[tokio::test]
async fn alternate_user_charges_persist_only_to_the_selected_user() {
    let mut state = state().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("users.toml");
    state.get_board().await.config.paths.user_file = path.clone();
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    prepare_user_variables(&mut vm);
    let args = arguments(&mut vm, vec![VariableValue::new_int(2)]);
    super::getaltuser(&mut vm, &args).await.unwrap();
    adjust(&mut vm, 0, 12.0).await.unwrap();
    usage(&mut vm, 13, 0.25, 3).await.unwrap();
    usage(&mut vm, 16, 0.5, -2).await.unwrap();
    assert_eq!(read_account(&mut vm, 13).await, 0.75);
    super::putuser(&mut vm, &[]).await.unwrap();
    let users: UserBase = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let saved = &users[1];
    assert_eq!(saved.account.as_ref().unwrap().debit_special, 0.75);
    assert_eq!(saved.account.as_ref().unwrap().credit_special, -1.0);
    assert_eq!(saved.account.as_ref().unwrap().starting_balance, 12.0);
    let caller = vm.icy_board_state.session.current_user.as_ref().unwrap().account.as_ref().unwrap();
    assert_eq!(caller.starting_balance, 100.0);
    assert_eq!(caller.debit_special, 0.0);
    super::getuser(&mut vm, &[]).await.unwrap();
    assert_eq!(read_account(&mut vm, 13).await, 0.0);
}

#[tokio::test]
async fn recordusage_obeys_session_mode_and_zero_tracking_policy() {
    for alternate in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.log");
        let mut state = state_with_tracking(Some(&path)).await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        vm.user = vm.icy_board_state.get_board().await.users[usize::from(alternate)].clone();
        for (mode, expected) in [(AccountingMode::Disabled, 0), (AccountingMode::Tracking, 1), (AccountingMode::Enforced, 2)] {
            vm.icy_board_state.session.accounting.mode = mode;
            let args = arguments(&mut vm, vec![VariableValue::new_int(0)]);
            assert_eq!(expressions::pcbaccstat(&mut vm, &args).await.unwrap().as_int(), expected);
            usage(&mut vm, 13, 0.0, 1).await.unwrap();
            usage(&mut vm, 13, 0.5, 3).await.unwrap();
        }
        assert_eq!(read_account(&mut vm, 13).await, 3.0);
        let log = std::fs::read_to_string(path).unwrap();
        assert_eq!(log.lines().count(), 3, "only tracking logs zero usage");
        assert!(log.contains("CALLER"));
        assert!(!log.contains("OTHER"));
        assert!(log.contains("PPE TEST") && log.contains("DETAIL"));
        assert!(!log.contains('\t'));
    }
}

#[tokio::test]
async fn invalid_amounts_and_overflow_leave_accounts_unchanged() {
    for alternate in [false, true] {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        vm.user = vm.icy_board_state.get_board().await.users[usize::from(alternate)].clone();
        super::refresh_accounting_user(&mut vm);
        let before = vm.user.account.clone();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for field in [0, 1, 2, 17] {
                assert!(adjust(&mut vm, field, value).await.is_err());
            }
            assert!(usage(&mut vm, 13, value, 1).await.is_err());
        }
        assert!(usage(&mut vm, 13, f64::MAX, 2).await.is_err());
        for field in [-1, 0, 1, 17, 18] {
            usage(&mut vm, field, 1.0, 1).await.unwrap();
        }
        assert_eq!(vm.user.account, before);
        for field in [0, 1, 2] {
            adjust(&mut vm, field, f64::MAX).await.unwrap();
            let before = vm.user.account.clone();
            assert!(adjust(&mut vm, field, f64::MAX).await.is_err());
            assert_eq!(vm.user.account, before);
        }
        let before = vm.user.account.clone();
        assert!(usage(&mut vm, 2, f64::MAX, 1).await.is_err());
        assert_eq!(vm.user.account, before);
    }
}

#[tokio::test]
async fn recordusage_covers_all_debit_credit_fields_and_shared_dbf_output() {
    for alternate in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.DBF");
        let mut state = state_with_tracking(Some(&path)).await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        vm.user = vm.icy_board_state.get_board().await.users[usize::from(alternate)].clone();
        for field in 2..=16 {
            usage(&mut vm, field, 0.25, field).await.unwrap();
            assert_eq!(read_account(&mut vm, field).await, 0.25 * f64::from(field));
        }
        let mut dbf = crate::vm::dbase::file::DbaseFile::open(&path).unwrap();
        assert_eq!(dbf.record_count(), 15);
        assert!(dbf.goto(1).unwrap());
        assert_eq!(std::str::from_utf8(dbf.get_field(2)).unwrap().trim(), "CALLER");
        assert_eq!(std::str::from_utf8(dbf.get_field(5)).unwrap().trim(), "PPE TEST");
        assert_eq!(std::str::from_utf8(dbf.get_field(7)).unwrap().trim(), "0.2500");
        assert_eq!(std::str::from_utf8(dbf.get_field(8)).unwrap().trim(), "2");
        assert_eq!(std::str::from_utf8(dbf.get_field(9)).unwrap().trim(), "0.5000");
    }
}

#[tokio::test]
async fn pcbaccstat_balance_uses_live_session_not_selected_snapshot() {
    let mut state = state().await;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    vm.user = vm.icy_board_state.get_board().await.users[1].clone();
    vm.icy_board_state.accounting_record(13, "RUNTIME", "", 2.5, 2).unwrap();
    let expected = vm.icy_board_state.session.calculate_balance();
    assert_eq!(expected, 95.0);
    let args = arguments(&mut vm, vec![VariableValue::new_int(4)]);
    assert_eq!(expressions::pcbaccstat(&mut vm, &args).await.unwrap().as_double(), expected);
    assert_eq!(read_account(&mut vm, 0).await, 0.0);
}

#[tokio::test]
async fn tracking_failure_does_not_retry_or_lose_a_posted_charge() {
    for alternate in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut state = state_with_tracking(Some(&directory.path().join("missing/usage.log"))).await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        vm.user = vm.icy_board_state.get_board().await.users[usize::from(alternate)].clone();
        usage(&mut vm, 13, 1.25, 2).await.unwrap();
        assert_eq!(read_account(&mut vm, 13).await, 2.5);
        adjust(&mut vm, 16, 1.0).await.unwrap();
        assert_eq!(read_account(&mut vm, 13).await, 2.5);
    }
}
