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
                users[1].birth_date = Some(chrono::Utc::now().date_naive());
                Ok(())
            })
            .unwrap();
        let registry = crate::parser::icy_board_registry();
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
        let registry = crate::parser::icy_board_registry();
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
    let registry = crate::parser::icy_board_registry();
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
    let registry = crate::parser::icy_board_registry();
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
        let registry = crate::parser::icy_board_registry();
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
        let registry = crate::parser::icy_board_registry();
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
    let registry = crate::parser::icy_board_registry();
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
    let registry = crate::parser::icy_board_registry();
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

#[tokio::test]
async fn adduser_applies_the_new_user_defaults() {
    use crate::icy_board::{conferences::Conference, group_list::GroupList, security_expr::SecurityExpression, user_base::ConferenceFlags};

    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    {
        let mut board = state.get_board().await;
        board.config.new_user_settings.sec_level = 20;
        board.config.new_user_settings.auto_register_conferences = true;
        board.config.new_user_settings.new_user_groups = "new_users; trial".into();
        board.config.subscription_info.is_enabled = true;
        board.config.subscription_info.subscription_length = 30;
        board.config.subscription_info.default_expired_level = 5;
        board.config.paths.group_file = directory.path().join("groups.toml");
        board.groups = GroupList::new();
        board.groups.add_group("new_users", "New users");
        board.groups.add_group("trial", "Trial users");
        board.conferences.clear();
        for (is_public, required_security) in [
            (true, SecurityExpression::default()),
            (false, SecurityExpression::default()),
            (true, SecurityExpression::from_req_security(50)),
        ] {
            board.conferences.push(Conference {
                is_public,
                required_security,
                ..Default::default()
            });
        }
    }
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let before = chrono::Utc::now();
    for command in load(&mut vm, "ADDUSER \"NEW USER\", FALSE\n") {
        vm.execute_statement(&command).await.unwrap();
    }

    let users = UserBase::load(&directory.path().join("users.toml")).unwrap();
    let user = &users[2];
    assert_eq!(user.name, "NEW USER");
    assert_eq!(user.security_level, 20);
    assert_eq!(user.exp_security_level, 5);
    assert_eq!(user.protocol, "N");
    assert_eq!(user.page_len, 23);
    let days = (user.expiration_date - before).num_days();
    assert!((29..=30).contains(&days), "expiration {} days after creation", days);
    assert_eq!(
        user.conference_flags.get(&0).copied(),
        Some(ConferenceFlags::Registered | ConferenceFlags::Expired | ConferenceFlags::Selected)
    );
    assert!(!user.conference_flags.contains_key(&1));
    assert!(!user.conference_flags.contains_key(&2));

    assert_eq!(vm.icy_board_state.get_board().await.groups.get_groups("NEW USER"), vec!["new_users", "trial"]);
    let saved = GroupList::load(&directory.path().join("groups.toml")).unwrap();
    assert_eq!(saved.get_groups("NEW USER"), vec!["new_users", "trial"]);
}

/// The output, and the user file if the snippet saved one.
fn run_with_user_file(source: &str) -> (String, Option<UserBase>) {
    let directory = tempfile::tempdir().unwrap();
    let user_file = directory.path().join("users.toml");
    let output = tests::run_ppl_on(source, |board| board.config.paths.user_file = user_file.clone());
    (output, user_file.exists().then(|| UserBase::load(&user_file).unwrap()))
}

#[test]
fn board_add_user_returns_a_writable_record_that_board_users_sees() {
    let (output, users) = run_with_user_file(
        r#"
PRINTLN Board.Users.Len()
USER created = Board.AddUser("  New Caller ")
PRINTLN Error.Last().OK, "|", created.Valid, "|", created.Name, "|", created.RecordNumber, "|", created.Protocol, "|", created.PageLength
PRINTLN Board.Users.Len(), "|", Board.Users[created.RecordNumber - 1].Name
created.City = "Berlin"
created.SecurityLevel = 20
PRINTLN Error.Last().OK, "|", created.City, "|", created.SecurityLevel, "|", Board.Users[created.RecordNumber - 1].City
PRINTLN Session.User.Name, "|", Session.User.City
"#,
    );
    assert_eq!(output, "1\n1|1|New Caller|2|N|23\n2|New Caller\n1|Berlin|20|Berlin\nSYSOP|\n");
    let users = users.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[1].name, "New Caller");
    assert_eq!(users[1].city_or_state, "Berlin");
    assert_eq!(users[1].security_level, 20);
    assert!(users[0].city_or_state.is_empty());
}

#[test]
fn board_add_user_refuses_empty_and_taken_names() {
    let (output, users) = run_with_user_file(
        r#"
USER user = Board.AddUser("   ")
PRINTLN user.Valid, "|", Error.Last().OK, "|", Error.Last().Message
Error.Clear()
user = Board.AddUser("sysop")
PRINTLN user.Valid, "|", Error.Last().OK, "|", Error.Last().Message
Error.Clear()
user.City = "nowhere"
PRINTLN Error.Last().OK, "|", Board.Users.Len()
"#,
    );
    assert_eq!(
        output,
        "0|0|the user name cannot be empty\n0|0|a user with that name or alias already exists\n0|1\n"
    );
    assert!(users.is_none(), "nothing was created or saved");
}

#[test]
fn board_find_user_shares_one_record_and_hands_out_the_caller_as_session_user() {
    let (output, users) = run_with_user_file(
        r#"
ADDUSER "Other Caller", FALSE
USER first = Board.FindUser(" other caller ")
USER second = Board.FindUser("OTHER CALLER")
first.Comment = "shared"
PRINTLN second.Comment, "|", second.RecordNumber, "|", Board.Users[1].Comment
USER me = Board.FindUser("sysop")
me.City = "Home"
PRINTLN Session.User.City, "|", me.RecordNumber
USER nobody = Board.FindUser("nobody")
PRINTLN nobody.Valid, "|", Error.Last().OK
"#,
    );
    assert_eq!(output, "shared|2|shared\nHome|1\n0|1\n");
    let users = users.unwrap();
    assert_eq!(users[0].city_or_state, "Home");
    assert_eq!(users[1].user_comment, "shared");
}

#[test]
fn board_users_and_records_follow_legacy_adduser_and_putuser() {
    let (output, users) = run_with_user_file(
        r#"
PRINTLN Board.Users.Len()
ADDUSER "Legacy", FALSE
PRINTLN Board.Users.Len(), "|", Board.Users[1].Name
USER record = Board.FindUser("Legacy")
record.City = "FromObject"
GETALTUSER record.RecordNumber
U_CMNT1 = "FromLegacy"
PUTUSER
PRINTLN record.City, "|", record.Comment, "|", Board.Users[1].Comment, "|", U_CITY
PRINTLN Board.Users[1].SetNote(0, "refused"), "|", Error.Last().OK
"#,
    );
    assert_eq!(output, "1\n2|Legacy\nFromObject|FromLegacy|FromLegacy|FromObject\n0|0\n");
    let users = users.unwrap();
    assert_eq!(users[1].city_or_state, "FromObject");
    assert_eq!(users[1].user_comment, "FromLegacy");
}

async fn edit_stored_users(vm: &mut VirtualMachine<'_>, edit: impl FnOnce(&mut UserBase) + Send + 'static) {
    IcyBoard::write_users(&vm.icy_board_state.board, move |board| {
        board.edit_users(|users| {
            edit(users);
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_record_merges_with_another_writer_and_notices_when_it_is_packed_away() {
    use crate::icy_board::state::ppl_error::ERR_IO;

    let directory = tempfile::tempdir().unwrap();
    let mut state = state(directory.path()).await;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(".", None);
    let mut vm = VirtualMachine::new("test.ppe".into(), &registry, &mut io, &mut state);
    let commands = load(
        &mut vm,
        "USER record = Board.FindUser(\"OTHER\")\nrecord.City = \"Hamburg\"\nrecord.Comment = \"gone\"\nU_CMNT1 = record.Name\n",
    );
    assert_eq!(commands.len(), 5, "four statements and END");
    vm.execute_statement(&commands[0]).await.unwrap();

    edit_stored_users(&mut vm, |users| users[1].user_comment = "other node".into()).await;
    vm.execute_statement(&commands[1]).await.unwrap();
    assert!(vm.last_error.is_ok(), "{:?}", vm.last_error);
    let stored = vm.icy_board_state.get_board().await.users[1].clone();
    assert_eq!((stored.city_or_state.as_str(), stored.user_comment.as_str()), ("Hamburg", "other node"));

    edit_stored_users(&mut vm, |users| {
        users.remove(1);
    })
    .await;
    vm.execute_statement(&commands[2]).await.unwrap();
    assert_eq!(vm.last_error.code, ERR_IO);
    vm.clear_error();
    vm.variable_table.set_value(U_CMNT1, VariableValue::new_string("unset".into())).unwrap();
    vm.execute_statement(&commands[3]).await.unwrap();
    assert_eq!(vm.variable_table.get_value(U_CMNT1).as_string(), "", "the packed-away record reads invalid");
    let users = vm.icy_board_state.get_board().await.users.clone();
    assert_eq!(users.len(), 1);
    assert!(users.iter().all(|user| user.name != "OTHER"));
}
