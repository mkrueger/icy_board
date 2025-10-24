use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use icy_board_engine::{
    executable::{PPEExpr, PPEScript, TableEntry, VariableTable, VariableValue},
    icy_board::{bbs::BBS, commands::CommandList, state::IcyBoardState, user_base::User, xfer_protocols::SupportedProtocols},
    parser::UserTypeRegistry,
    vm::{DiskIO, VirtualMachine},
};
use icy_net::{ConnectionType, channel::ChannelConnection};

async fn test_fmtreal(value: f64, field_width: i32, decimal_places: i32) -> String {
    let bbs: Arc<tokio::sync::Mutex<BBS>> = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut icy_board = icy_board_engine::icy_board::IcyBoard::new();

    icy_board.commands = CommandList::new();
    icy_board.protocols = SupportedProtocols::generate_pcboard_defaults();
    icy_board.default_display_text = icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    icy_board.users.new_user(User {
        name: "SYSOP".to_string(),
        security_level: 255,
        protocol: "Z".to_string(),
        ..Default::default()
    });
    icy_board.users.new_user(User {
        name: "TEST USER".to_string(),
        security_level: 10,
        protocol: "Z".to_string(),
        ..Default::default()
    });

    let node: usize = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let node_state: Arc<tokio::sync::Mutex<Vec<Option<icy_board_engine::icy_board::state::NodeState>>>> = bbs.lock().await.open_connections.clone();
    let (_ui_connection, connection) = ChannelConnection::create_pair();

    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await;

    let type_registry = UserTypeRegistry::icy_board_registry();
    let mut io = DiskIO::new(".", None);

    let mut vm = VirtualMachine {
        file_name: "<test>".into(),
        type_registry: &type_registry,
        return_addresses: Vec::new(),
        script: PPEScript::default(),
        io: &mut io,
        is_running: true,
        fpclear: false,
        icy_board_state: &mut state,
        pcb_node: None,
        variable_table: VariableTable::default(),
        cur_ptr: 0,
        label_table: HashMap::new(),
        call_local_value_stack: Vec::new(),
        write_back_stack: Vec::new(),
        push_pop_stack: Vec::new(),
        user_data: Vec::new(),
        stored_screen: None,
        fd_default_in: 0,
        fd_default_out: 0,
        file_list: VecDeque::new(),
        user: User::default(),
        use_lmrs: true,
        cached_msg_header: None,
    };

    vm.variable_table.push(TableEntry {
        name: "real_value".to_string(),
        value: VariableValue::new_double(value),
        header: icy_board_engine::executable::VarHeader {
            id: 1,
            variable_type: icy_board_engine::executable::VariableType::Double,
            ..Default::default()
        },
        entry_type: icy_board_engine::executable::EntryType::Constant,
        function_id: 0,
    });

    vm.variable_table.push(TableEntry {
        name: "field_width".to_string(),
        value: VariableValue::new_int(field_width),
        header: icy_board_engine::executable::VarHeader {
            id: 2,
            variable_type: icy_board_engine::executable::VariableType::Integer,
            ..Default::default()
        },
        entry_type: icy_board_engine::executable::EntryType::Constant,
        function_id: 0,
    });

    vm.variable_table.push(TableEntry {
        name: "decimal_places".to_string(),
        value: VariableValue::new_int(decimal_places),
        header: icy_board_engine::executable::VarHeader {
            id: 3,
            variable_type: icy_board_engine::executable::VariableType::Integer,
            ..Default::default()
        },
        entry_type: icy_board_engine::executable::EntryType::Constant,
        function_id: 0,
    });

    let args = vec![PPEExpr::Value(1), PPEExpr::Value(2), PPEExpr::Value(3)];
    icy_board_engine::vm::expressions::fmtreal(&mut vm, &args).await.unwrap().as_string()
}

#[tokio::test]
async fn test_fmtreal_basic() {
    // Example from documentation
    assert_eq!(test_fmtreal(19.95, 10, 2).await, "     19.95");
    assert_eq!(test_fmtreal(1.646, 10, 2).await, "      1.65");
    assert_eq!(test_fmtreal(21.60, 10, 2).await, "     21.60");
}

#[tokio::test]
async fn test_fmtreal_rounding() {
    // Test rounding behavior
    assert_eq!(test_fmtreal(1.994, 10, 2).await, "      1.99");
    assert_eq!(test_fmtreal(1.995, 10, 2).await, "      2.00");
    assert_eq!(test_fmtreal(1.996, 10, 2).await, "      2.00");
    assert_eq!(test_fmtreal(9.999, 10, 2).await, "     10.00");
}

#[tokio::test]
async fn test_fmtreal_field_width() {
    // Test different field widths
    assert_eq!(test_fmtreal(99.99, 5, 2).await, "99.99");
    assert_eq!(test_fmtreal(99.99, 6, 2).await, " 99.99");
    assert_eq!(test_fmtreal(99.99, 10, 2).await, "     99.99");
    assert_eq!(test_fmtreal(99.99, 0, 2).await, "99.99");
}

#[tokio::test]
async fn test_fmtreal_decimal_places() {
    // Test different decimal places
    assert_eq!(test_fmtreal(3.14159, 10, 0).await, "         3");
    assert_eq!(test_fmtreal(3.14159, 10, 1).await, "       3.1");
    assert_eq!(test_fmtreal(3.14159, 10, 2).await, "      3.14");
    assert_eq!(test_fmtreal(3.14159, 10, 3).await, "     3.142");
    assert_eq!(test_fmtreal(3.14159, 10, 4).await, "    3.1416");
    assert_eq!(test_fmtreal(3.14159, 10, 5).await, "   3.14159");
}

#[tokio::test]
async fn test_fmtreal_negative_numbers() {
    // Test negative numbers
    assert_eq!(test_fmtreal(-19.95, 10, 2).await, "    -19.95");
    assert_eq!(test_fmtreal(-1.5, 10, 2).await, "     -1.50");
    assert_eq!(test_fmtreal(-999.99, 10, 2).await, "   -999.99");
}

#[tokio::test]
async fn test_fmtreal_large_numbers() {
    // Test large numbers
    assert_eq!(test_fmtreal(1234567.89, 15, 2).await, "     1234567.89");
    assert_eq!(test_fmtreal(999999.99, 10, 2).await, " 999999.99");
    assert_eq!(test_fmtreal(1000000.0, 11, 2).await, " 1000000.00");
}

#[tokio::test]
async fn test_fmtreal_overflow() {
    // Test when formatted number exceeds field width
    assert_eq!(test_fmtreal(12345.67, 5, 2).await, "12345.67"); // No truncation
    assert_eq!(test_fmtreal(999.999, 5, 3).await, "999.999");
}

#[tokio::test]
async fn test_fmtreal_zero() {
    // Test zero values
    assert_eq!(test_fmtreal(0.0, 10, 2).await, "      0.00");
    // Both checked with PCB 15.3 :
    assert_eq!(test_fmtreal(0.0, 5, 0).await, "    0");
    assert_eq!(test_fmtreal(-0.0, 10, 2).await, "     -0.00");
}

#[tokio::test]
async fn test_fmtreal_small_numbers() {
    // Test very small numbers
    assert_eq!(test_fmtreal(0.001, 10, 2).await, "      0.00");
    assert_eq!(test_fmtreal(0.001, 10, 3).await, "     0.001");
    assert_eq!(test_fmtreal(0.0001, 10, 4).await, "    0.0001");
    assert_eq!(test_fmtreal(0.00001, 10, 4).await, "    0.0000");
}

#[tokio::test]
async fn test_fmtreal_edge_cases() {
    // Checked with PCB 15.3
    assert_eq!(test_fmtreal(0.5, 10, 0).await, "         0");
    assert_eq!(test_fmtreal(0.4, 10, 0).await, "         0");
    assert_eq!(test_fmtreal(-0.5, 10, 0).await, "        -0");
    assert_eq!(test_fmtreal(99.995, 10, 2).await, "    100.00");
}

#[tokio::test]
async fn test_fmtreal_report_formatting() {
    // Test typical report formatting scenarios
    let prices = vec![(19.95, "Price"), (1.646, "Tax"), (21.596, "Total")];

    let mut report = Vec::new();
    for (value, label) in prices {
        let formatted = test_fmtreal(value, 10, 2).await;
        report.push(format!("{:<10} {}", label, formatted));
    }

    assert_eq!(report[0], "Price           19.95");
    assert_eq!(report[1], "Tax              1.65");
    assert_eq!(report[2], "Total           21.60");
}

#[tokio::test]
async fn test_fmtreal_column_alignment() {
    // Test alignment for columnar data
    let values = vec![1.0, 10.0, 100.0, 1000.0, 10000.0];
    let mut results = Vec::new();

    for v in values {
        results.push(test_fmtreal(v, 12, 2).await);
    }

    assert_eq!(results[0], "        1.00");
    assert_eq!(results[1], "       10.00");
    assert_eq!(results[2], "      100.00");
    assert_eq!(results[3], "     1000.00");
    assert_eq!(results[4], "    10000.00");

    // All should have the same length
    assert!(results.iter().all(|s| s.len() == 12));
}
