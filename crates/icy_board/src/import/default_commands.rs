use icy_board_engine::icy_board::{
    commands::{CommandList, CommandType},
    pcboard_data::PcbBoardData,
    security::RequiredSecurity,
    PCBoardRecordImporter,
};

fn convert_cmd(name: &str, cmd_type: CommandType, security: i32) -> icy_board_engine::icy_board::commands::Command {
    icy_board_engine::icy_board::commands::Command {
        keyword: name.to_string(),
        display: "".to_string(),
        lighbar_display: "".to_string(),
        help: format!("hlp{}", name.to_ascii_lowercase()),
        command_type: cmd_type,
        parameter: "".to_string(),
        security: RequiredSecurity::new(security as u8),
    }
}

pub fn add_default_commands(data: &PcbBoardData, cmd_list: &mut CommandList) {
    cmd_list.push(convert_cmd("!", CommandType::RedisplayCommand, 0));
    cmd_list.push(convert_cmd("A", CommandType::AbandonConference, data.user_levels.cmd_a));
    cmd_list.push(convert_cmd("B", CommandType::BulletinList, data.user_levels.cmd_b));
    cmd_list.push(convert_cmd("C", CommandType::CommentToSysop, data.user_levels.cmd_c));
    cmd_list.push(convert_cmd("DOWN", CommandType::Download, data.user_levels.cmd_d));
    cmd_list.push(convert_cmd("FLAG", CommandType::Download, data.user_levels.cmd_d));
    cmd_list.push(convert_cmd("E", CommandType::EnterMessage, data.user_levels.cmd_e));
    cmd_list.push(convert_cmd("F", CommandType::FileDirectory, data.user_levels.cmd_f));

    // doesn't make sense to have a sec for that but it's in the record
    cmd_list.push(convert_cmd("G", CommandType::Goodbye, data.user_levels.cmd_g));
    cmd_list.push(convert_cmd("BYE", CommandType::Bye, data.user_levels.cmd_g));

    cmd_list.push(convert_cmd("HELP", CommandType::Help, data.user_levels.cmd_h));
    cmd_list.push(convert_cmd("?", CommandType::Help, data.user_levels.cmd_h));

    cmd_list.push(convert_cmd("I", CommandType::InitialWelcome, data.user_levels.cmd_i));
    cmd_list.push(convert_cmd("JOIN", CommandType::JoinConference, data.user_levels.cmd_j));
    cmd_list.push(convert_cmd("K", CommandType::DeleteMessage, data.user_levels.cmd_k));
    cmd_list.push(convert_cmd("L", CommandType::LocateFile, data.user_levels.cmd_l));
    cmd_list.push(convert_cmd("M", CommandType::ToggleGraphics, data.user_levels.cmd_m));
    cmd_list.push(convert_cmd("N", CommandType::NewFileScan, data.user_levels.cmd_n));
    cmd_list.push(convert_cmd("O", CommandType::PageSysop, data.user_levels.cmd_o));
    cmd_list.push(convert_cmd("P", CommandType::SetPageLength, data.user_levels.cmd_p));
    cmd_list.push(convert_cmd("Q", CommandType::QuickMessageScan, data.user_levels.cmd_q));
    cmd_list.push(convert_cmd("R", CommandType::ReadMessages, data.user_levels.cmd_r));
    cmd_list.push(convert_cmd("S", CommandType::Survey, data.user_levels.cmd_s));
    cmd_list.push(convert_cmd("T", CommandType::SetTransferProtocol, data.user_levels.cmd_t));
    cmd_list.push(convert_cmd("U", CommandType::UploadFile, data.user_levels.cmd_u));
    cmd_list.push(convert_cmd("V", CommandType::ViewSettings, data.user_levels.cmd_v));
    cmd_list.push(convert_cmd("W", CommandType::WriteSettings, data.user_levels.cmd_w));
    cmd_list.push(convert_cmd("X", CommandType::ExpertMode, data.user_levels.cmd_x));
    cmd_list.push(convert_cmd("Y", CommandType::PersonalMail, data.user_levels.cmd_y));
    cmd_list.push(convert_cmd("Z", CommandType::ZippyDirectoryScan, data.user_levels.cmd_z));

    cmd_list.push(convert_cmd("CHAT", CommandType::GroupChat, data.user_levels.cmd_chat));
    cmd_list.push(convert_cmd("DOOR", CommandType::OpenDoor, data.user_levels.cmd_open_door));
    cmd_list.push(convert_cmd("OPEN", CommandType::OpenDoor, data.user_levels.cmd_open_door));
    cmd_list.push(convert_cmd("TEST", CommandType::TestFile, data.user_levels.cmd_test_file));
    cmd_list.push(convert_cmd("USER", CommandType::UserList, data.user_levels.cmd_show_user_list));
    cmd_list.push(convert_cmd("WHO", CommandType::WhoIsOnline, data.user_levels.cmd_who));
    cmd_list.push(convert_cmd("MENU", CommandType::ShowMenu, 0));
    cmd_list.push(convert_cmd("NEWS", CommandType::DisplayNews, 0));
    cmd_list.push(convert_cmd("LANG", CommandType::SetLanguage, 0));
    cmd_list.push(convert_cmd("REPLY", CommandType::ReplyMessage, 0));
    cmd_list.push(convert_cmd("ALIAS", CommandType::EnableAlias, 0));

    cmd_list.push(convert_cmd("BR", CommandType::Broadcast, data.sysop_security.sysop));

    cmd_list.push(convert_cmd("4", CommandType::RestoreMessage, data.sysop_security.sysop));
    cmd_list.push(convert_cmd("PPE", CommandType::RunPPE, data.sysop_security.sysop));

    cmd_list.push(convert_cmd("@", CommandType::ReadEmail, data.sysop_security.sysop));
    cmd_list.push(convert_cmd("@W", CommandType::WriteEmail, data.sysop_security.sysop));
}
