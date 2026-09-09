use super::*;
use crate::{
    executable::OpCode,
    icy_board::{
        IcyBoard, IcyBoardSerializer, accounting_cfg::AccountingConfig, bbs::BBS, password_recovery::security_fingerprint, sec_levels::SecurityLevel,
        user_base::UserBase, user_inf::AccountUserInf,
    },
};
use icy_net::{ConnectionType, channel::ChannelConnection};

async fn state(directory: &Path) -> IcyBoardState {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = directory.to_path_buf();
    board.config.paths.user_file = directory.join("users.toml");
    board.config.accounting.enabled = true;
    board.config.accounting.ignore_empty_sec_level = true;
    board.config.accounting.tracking_file = directory.join("usage.log");
    board.config.accounting.accounting_config = Some(AccountingConfig::default());
    board.sec_levels.levels.push(SecurityLevel {
        security: 10,
        is_enabled: true,
        ..Default::default()
    });
    for name in ["CALLER", "OTHER"] {
        board.users.new_user(User {
            name: name.into(),
            city_or_state: "Berlin".into(),
            country: "DE".into(),
            custom_comment1: "original note".into(),
            security_level: 10,
            account: Some(AccountUserInf {
                starting_balance: 100.0,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    board.edit_users(|_| Ok(())).unwrap();
    let user = board.users[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.accounting_start().await.unwrap();
    state
}

// Execute real serialized compiler output, pausing between statements for a second writer.
fn load(vm: &mut VirtualMachine<'_>, source: &str) -> Vec<PPECommand> {
    let executable = tests::compile(source);
    let script = PPEScript::from_ppe_file(&executable).unwrap();
    vm.variable_table = executable.variable_table;
    vm.user_types = executable.user_types;
    vm.snapshot_user_variables();
    script.statements.into_iter().map(|statement| statement.command).collect()
}

fn is_put(command: &PPECommand) -> bool {
    matches!(command, PPECommand::PredefinedCall(definition, _) if definition.opcode == OpCode::PUTUSER)
}

async fn before_put(vm: &mut VirtualMachine<'_>, commands: &[PPECommand]) -> usize {
    let index = commands.iter().position(is_put).unwrap();
    for command in &commands[..index] {
        vm.execute_statement(command).await.unwrap();
    }
    index
}

#[test]
fn compiled_getaltuser_caller_merges_only_changed_profile_and_array_fields() {
    let output = tests::run_ppl_on(
        r#"
GETALTUSER 1
U_CMNT1 = "draft"
U_NOTES[1] = "second note"
Session.User.City = "Hamburg"
Session.User.SetNote(0, "live note")
Session.User.Alias = "live alias"
PUTUSER
Session.User.City = "Bremen"
PUTUSER
GETUSER
PRINTLN U_CITY, "|", U_CMNT1, "|", U_NOTES[0], "|", U_NOTES[1], "|", U_ALIAS, "|", U_ADDR[5]
"#,
        |board| {
            board.config.paths.user_file = board.root_path.join("users.toml");
            board.users[0].city_or_state = "Berlin".into();
            board.users[0].country = "DE".into();
            board.users[0].custom_comment1 = "original note".into();
        },
    );
    assert_eq!(output, "Bremen|draft|live note|second note|live alias|DE\n");
}

#[test]
fn compiled_city_writes_back_through_either_of_its_two_variables() {
    let output = tests::run_ppl_on(
        r#"
GETUSER
U_ADDR[2] = "Hamburg"
PUTUSER
GETUSER
PRINTLN U_CITY, "|", U_ADDR[2]
U_CITY = "Bremen"
PUTUSER
GETUSER
PRINTLN U_CITY, "|", U_ADDR[2]
"#,
        |board| {
            board.config.paths.user_file = board.root_path.join("users.toml");
            board.users[0].city_or_state = "Berlin".into();
        },
    );
    assert_eq!(output, "Hamburg|Hamburg\nBremen|Bremen\n");
}

#[test]
fn compiled_file_description_flag_round_trips_through_both_of_its_variables() {
    // U_LONGHDR is the inverse of the stored flag, U_SHORTDESC states it directly.
    let output = tests::run_ppl_on(
        r#"
GETUSER
PRINTLN U_LONGHDR, "|", U_SHORTDESC
U_LONGHDR = FALSE
PUTUSER
GETUSER
PRINTLN U_LONGHDR, "|", U_SHORTDESC
U_SHORTDESC = FALSE
PUTUSER
GETUSER
PRINTLN U_LONGHDR, "|", U_SHORTDESC
"#,
        |board| {
            board.config.paths.user_file = board.root_path.join("users.toml");
            board.users[0].flags.use_short_filedescr = false;
        },
    );
    assert_eq!(output, "1|0\n0|1\n1|0\n");
}

#[tokio::test]
async fn compiled_stale_password_and_dates_are_not_implicit_credential_edits() {
    for hashed in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut state = state(directory.path()).await;
        state
            .get_board()
            .await
            .edit_users(|users| {
                users[1].password.password = if hashed {
                    Password::new_argon2("original")
                } else {
                    Password::PlainText("original".into())
                };
                users[1].expiration_date = chrono::Utc::now();
                users[1].password.expire_date = chrono::Utc::now();
                users[1].birth_date = chrono::Utc::now();
                Ok(())
            })
            .unwrap();
        let registry = UserTypeRegistry::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        let commands = load(&mut vm, "GETALTUSER 2\nU_CITY = \"Hamburg\"\nPUTUSER\nU_PWD = \"draft-password\"\nPUTUSER\n");
        let index = before_put(&mut vm, &commands).await;
        let original = vm.user_baseline.clone();
        vm.execute_statement(&commands[index]).await.unwrap();
        assert_eq!(security_fingerprint(&vm.user), security_fingerprint(&original));
        assert_eq!(vm.user.expiration_date, original.expiration_date);
        assert_eq!(vm.user.password.expire_date, original.password.expire_date);
        assert_eq!(vm.user.birth_date, original.birth_date);
        vm.icy_board_state
            .get_board()
            .await
            .edit_users(|users| {
                users[1].password.password = Password::PlainText("live-password".into());
                Ok(())
            })
            .unwrap();
        let persisted = std::fs::read(directory.path().join("users.toml")).unwrap();
        for command in &commands[index + 1..] {
            let result = vm.execute_statement(command).await;
            if is_put(command) {
                assert!(result.unwrap_err().to_string().contains("credentials"));
            } else {
                result.unwrap();
            }
        }
        assert_eq!(std::fs::read(directory.path().join("users.toml")).unwrap(), persisted);
        assert_eq!(security_fingerprint(&vm.user_baseline), security_fingerprint(&original));
    }
}

#[tokio::test]
async fn compiled_removed_alternate_never_writes_a_reused_record_or_alias() {
    for replacement in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut state = state(directory.path()).await;
        let registry = UserTypeRegistry::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        let commands = load(&mut vm, "GETALTUSER 2\nU_CITY = \"draft\"\nPUTUSER\n");
        let index = before_put(&mut vm, &commands).await;
        vm.icy_board_state
            .get_board()
            .await
            .edit_users(|users| {
                if replacement {
                    users[1].stats.first_date_on += chrono::Duration::seconds(1);
                } else {
                    users.remove(1);
                    users[0].alias = "OTHER".into();
                }
                Ok(())
            })
            .unwrap();
        let persisted = std::fs::read(directory.path().join("users.toml")).unwrap();
        let error = vm.execute_statement(&commands[index]).await.unwrap_err();
        assert!(error.downcast_ref::<crate::icy_board::user_store::UserUpdateError>().is_some());
        assert_eq!(std::fs::read(directory.path().join("users.toml")).unwrap(), persisted);
        assert_eq!(vm.user_baseline.name, "OTHER");
        assert_eq!(vm.variable_table.get_value(U_CITY).as_string(), "draft");
    }
}

#[tokio::test]
async fn compiled_alternate_put_keeps_the_selected_daily_bucket_and_cumulative_deltas() {
    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
    state
        .get_board()
        .await
        .edit_users(|users| {
            users[1].stats.last_on = yesterday;
            users[1].stats.today_dnld_bytes = 100;
            users[1].stats.messages_read = 10;
            Ok(())
        })
        .unwrap();
    let registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let commands = load(&mut vm, "GETALTUSER 2\nU_CMNT1 = \"draft\"\nPUTUSER\nPUTUSER\n");
    let index = before_put(&mut vm, &commands).await;
    // Model selected-record activity such as RDUSYS, independent of U_* profile edits.
    vm.user.stats.today_dnld_bytes += 25;
    vm.user.stats.messages_read += 2;
    vm.icy_board_state
        .get_board()
        .await
        .edit_users(|users| {
            users[1].stats.last_on = yesterday + chrono::Duration::days(1);
            users[1].stats.today_dnld_bytes = 5;
            users[1].stats.messages_read += 3;
            Ok(())
        })
        .unwrap();
    for command in &commands[index..] {
        vm.execute_statement(command).await.unwrap();
    }
    assert_eq!(vm.user.stats.today_dnld_bytes, 5);
    assert_eq!(vm.user.stats.messages_read, 15);
}

#[tokio::test]
async fn compiled_alternate_put_merges_live_changes_and_account_deltas_once() {
    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    let registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let commands = load(
        &mut vm,
        "GETALTUSER 2\nU_CMNT1 = \"draft\"\nACCOUNT 13, 2\nRECORDUSAGE 13, \"PPE TEST\", \"\", 0.25, 3\nPUTUSER\nPUTUSER\nACCOUNT 13, 1\nPUTUSER\n",
    );
    let index = before_put(&mut vm, &commands).await;
    assert_eq!(vm.user_baseline.account.as_ref().unwrap().debit_special, 0.0);
    vm.icy_board_state
        .get_board()
        .await
        .edit_users(|users| {
            users[1].city_or_state = "Hamburg".into();
            users[1].stats.messages_read = 42;
            users[1].account.as_mut().unwrap().debit_special = 5.0;
            users.swap(0, 1);
            Ok(())
        })
        .unwrap();
    for command in &commands[index..] {
        vm.execute_statement(command).await.unwrap();
    }
    let users = UserBase::load(&directory.path().join("users.toml")).unwrap();
    assert_eq!(users[0].name, "OTHER");
    assert_eq!(users[0].city_or_state, "Hamburg");
    assert_eq!(users[0].user_comment, "draft");
    assert_eq!(users[0].stats.messages_read, 42);
    assert_eq!(users[0].account.as_ref().unwrap().debit_special, 8.75);
    assert_eq!(vm.user_baseline.account, users[0].account);
    let caller = vm.icy_board_state.session.current_user.as_ref().unwrap();
    assert_eq!(caller.account.as_ref().unwrap().debit_special, 0.0);
    assert_eq!(std::fs::read_to_string(directory.path().join("usage.log")).unwrap().lines().count(), 1);
}

#[tokio::test]
async fn compiled_caller_account_refresh_never_advances_profile_baseline_or_replays_charges() {
    for selection in ["GETUSER", "GETALTUSER 1", ""] {
        let directory = tempfile::tempdir().unwrap();
        let mut state = state(directory.path()).await;
        let persisted = std::fs::read(directory.path().join("users.toml")).unwrap();
        let registry = UserTypeRegistry::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        let commands = load(
            &mut vm,
            &format!(
                "{selection}\nU_CMNT1 = \"draft\"\nACCOUNT 13, 2\nRECORDUSAGE 13, \"PPE TEST\", \"\", 0.25, 3\nDOUBLE balance = ACCOUNT(13)\nPUTUSER\nPUTUSER\nACCOUNT 13, 1\nPUTUSER\n"
            ),
        );
        let index = before_put(&mut vm, &commands).await;
        assert_eq!(vm.user_baseline.account.as_ref().unwrap().debit_special, 0.0);
        vm.icy_board_state.accounting_record(13, "NATIVE", "", 5.0, 1).unwrap();
        let current = vm.icy_board_state.session.current_user.as_mut().unwrap();
        current.city_or_state = "Hamburg".into();
        current.stats.messages_read = 42;
        for command in &commands[index..] {
            vm.execute_statement(command).await.unwrap();
        }
        let current = vm.icy_board_state.session.current_user.as_ref().unwrap();
        assert_eq!(current.city_or_state, "Hamburg");
        assert_eq!(current.user_comment, "draft");
        assert_eq!(current.stats.messages_read, 42);
        assert_eq!(current.account.as_ref().unwrap().debit_special, 8.75);
        assert_eq!(
            std::fs::read(directory.path().join("users.toml")).unwrap(),
            persisted,
            "caller PUTUSER must remain delayed"
        );
        assert_eq!(std::fs::read_to_string(directory.path().join("usage.log")).unwrap().lines().count(), 2);
    }
}

#[tokio::test]
async fn compiled_put_conflicts_preserve_draft_baseline_and_live_record() {
    for alternate in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut state = state(directory.path()).await;
        let registry = UserTypeRegistry::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
        let selection = if alternate { "GETALTUSER 2" } else { "GETUSER" };
        let commands = load(
            &mut vm,
            &format!("{selection}\nU_CITY = \"Munich\"\nACCOUNT 13, 2\nPUTUSER\nU_CITY = \"Berlin\"\nPUTUSER\nPUTUSER\n"),
        );
        let index = before_put(&mut vm, &commands).await;
        if alternate {
            vm.icy_board_state
                .get_board()
                .await
                .edit_users(|users| {
                    users[1].city_or_state = "Hamburg".into();
                    users[1].account.as_mut().unwrap().debit_special = 5.0;
                    Ok(())
                })
                .unwrap();
        } else {
            vm.icy_board_state.session.current_user.as_mut().unwrap().city_or_state = "Hamburg".into();
        }
        let persisted = std::fs::read(directory.path().join("users.toml")).unwrap();
        let baseline = security_fingerprint(&vm.user_baseline);
        let error = vm.execute_statement(&commands[index]).await.unwrap_err().to_string();
        assert!(error.contains("city_or_state"), "{error}");
        assert_eq!(vm.variable_table.get_value(U_CITY).as_string(), "Munich");
        assert_eq!(vm.user_baseline.city_or_state, "Berlin");
        assert_eq!(security_fingerprint(&vm.user_baseline), baseline);
        assert_eq!(vm.user_baseline.account.as_ref().unwrap().debit_special, 0.0);
        assert_eq!(vm.user.account.as_ref().unwrap().debit_special, 2.0);
        assert_eq!(std::fs::read(directory.path().join("users.toml")).unwrap(), persisted);
        for command in &commands[index + 1..] {
            vm.execute_statement(command).await.unwrap();
        }
        assert_eq!(vm.user.city_or_state, "Hamburg");
        assert_eq!(vm.user.account.as_ref().unwrap().debit_special, if alternate { 7.0 } else { 2.0 });
    }
}

#[tokio::test]
async fn compiled_alternate_failed_persistence_does_not_publish_or_acknowledge_edits() {
    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    let registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let commands = load(&mut vm, "GETALTUSER 2\nU_CITY = \"Munich\"\nACCOUNT 13, 2\nPUTUSER\nPUTUSER\n");
    let index = before_put(&mut vm, &commands).await;
    vm.icy_board_state.get_board().await.config.paths.user_file = directory.path().to_path_buf();
    let persisted = std::fs::read(directory.path().join("users.toml")).unwrap();
    assert!(vm.execute_statement(&commands[index]).await.is_err());
    assert_eq!(vm.user_baseline.city_or_state, "Berlin");
    assert_eq!(vm.user_baseline.account.as_ref().unwrap().debit_special, 0.0);
    assert_eq!(vm.variable_table.get_value(U_CITY).as_string(), "Munich");
    assert_eq!(vm.icy_board_state.get_board().await.users[1].city_or_state, "Berlin");
    assert_eq!(std::fs::read(directory.path().join("users.toml")).unwrap(), persisted);
    vm.icy_board_state.get_board().await.config.paths.user_file = directory.path().join("users.toml");
    for command in &commands[index + 1..] {
        vm.execute_statement(command).await.unwrap();
    }
    assert_eq!(vm.user.city_or_state, "Munich");
    assert_eq!(vm.user.account.as_ref().unwrap().debit_special, 2.0);
}

#[tokio::test]
async fn compiled_selection_changes_and_invalid_getaltuser_preserve_the_right_baseline() {
    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    let registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let commands = load(
        &mut vm,
        "GETALTUSER 2\nU_CMNT1 = \"other draft\"\nACCOUNT 13, 2\nGETALTUSER 0\nGETALTUSER 999\nADDUSER \"NEW\", FALSE\nPUTUSER\nADDUSER \"SELECTED\", TRUE\nU_CMNT1 = \"new draft\"\nACCOUNT 13, 3\nPUTUSER\nFREALTUSER\nU_CMNT1 = \"caller draft\"\nPUTUSER\n",
    );
    for command in &commands {
        vm.execute_statement(command).await.unwrap();
    }
    let users = UserBase::load(&directory.path().join("users.toml")).unwrap();
    assert_eq!(users[1].user_comment, "other draft");
    assert_eq!(users[1].account.as_ref().unwrap().debit_special, 2.0);
    assert_eq!(users[2].name, "NEW");
    assert!(users[2].user_comment.is_empty());
    assert_eq!(users[3].user_comment, "new draft");
    assert_eq!(users[3].account.as_ref().unwrap().debit_special, 3.0);
    assert_eq!(vm.user_baseline.name, "CALLER");
    assert_eq!(vm.icy_board_state.session.current_user.as_ref().unwrap().user_comment, "caller draft");
    assert!(users[0].user_comment.is_empty());
}
