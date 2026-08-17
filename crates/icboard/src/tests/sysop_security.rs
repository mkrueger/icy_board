use icy_board_engine::icy_board::{
    IcyBoard,
    security_expr::{SecurityExpression, Value},
};

use crate::tests::test_output;

fn denied() -> SecurityExpression {
    SecurityExpression::Constant(Value::Bool(false))
}

macro_rules! denied_sysop_command {
    ($name:ident, $command:literal, $field:ident, $handler_prompt:literal) => {
        #[test]
        fn $name() {
            let output = test_output(concat!($command, "\n").to_string(), |board: &mut IcyBoard| {
                board.config.sysop_command_level.$field = denied();
            });
            assert!(
                output.contains("Menu Selection is not available"),
                "security denial is missing:\n{output}"
            );
            assert!(!output.contains($handler_prompt), "the handler ran despite its security level:\n{output}");
        }
    };
}

denied_sysop_command!(command_1_checks_view_caller_log, "1", sec_1_view_caller_log, "View, Print, Scan or Delete");
denied_sysop_command!(command_2_checks_view_users, "2", sec_2_view_usr_list, "View or Print the User File");
denied_sysop_command!(command_3_checks_pack_message_base, "3", sec_3_pack_renumber_msg, "Pack the message base");
denied_sysop_command!(command_4_checks_recover_message, "4", sec_4_recover_deleted_msg, "Message Number to Activate");
denied_sysop_command!(command_5_checks_header_scan, "5", sec_5_list_message_hdr, "Message Scan Command");
denied_sysop_command!(command_6_checks_view_file, "6", sec_6_view_any_file, "Filename to View");
denied_sysop_command!(command_10_checks_run_ppe, "PPE TEST", sec_10_shelled_dos_func, "Unable to run PPE");
denied_sysop_command!(command_11_checks_node_list, "11", sec_11_view_other_nodes, "Node");
denied_sysop_command!(command_12_checks_logoff_node, "12", sec_12_logoff_alt_node, "Node Number to Logoff");
denied_sysop_command!(command_13_checks_node_caller_log, "13", sec_13_view_alt_node_callers, "Node to View");
