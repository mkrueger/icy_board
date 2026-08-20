use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use crate::Res;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DisplayFromStr, serde_as};

use super::{IcyBoardSerializer, PCBoardRecordImporter, is_null_64, security_expr::SecurityExpression};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub enum CommandType {
    /// Do nothing
    #[default]
    Disabled,

    /// If you have assigned a menu command to have this type,
    /// you can load another MNU file as specified in the Parameters field.
    /// This would effectively let you create a sub-menu type system that is very
    /// easy to navigate.
    Menu,

    /// Execute a script file. The script number to execute should be specified
    /// in the Parameters field.
    /// For example, if you want to execute script #3 in the current conference
    /// for a particular menu option, set the type of the option to SCR and
    /// in the parameters field, enter 3.
    Script,

    /// This option type enables you to change the conference number.
    /// In the Parameters field, specify the conference name or number you wish to join.
    Conference,

    /// You can display any of the file directories available in the current conference.
    /// Specify the directory number you wish to display in the Parameters field.
    DisplayDir,

    /// If you want to disable a menu option without actually deleting it from the list of
    /// options available, use this option.
    DisableMenuOption,

    /// If you want to execute a door application from a menu, you may do so using this option type.
    /// Only the doors normally available in the current conference will be available for execution.
    /// Specify the door number or name to execute in the Parameters field of the option you are defining.
    Door,

    /// While this option type is similar to `QuitMenu`, it is different because it will
    /// quit all active menus.
    ExitMenus,

    /// To quit the current menu and return to the previous menu (if any), define a menu option
    /// that uses this option type. Remember that only the current menu will be exited.
    /// To exit all menus, use the `ExitMenus` option instead.
    QuitMenu,

    /// If you want to display a text file to the caller, you may do so using this option type.
    /// As with normal `PCBoard` display files, you can create security, graphics, and language specific
    /// versions of the file you are displaying to the caller.
    /// In the Parameters field, specify the path and filename to display.
    DisplayFile,

    /// To increase the capability of MNU files, this option type enables you to stuff any
    /// text into the keyboard.
    ///
    /// The text to stuff comes from the file specified in the Parameters field.
    /// Stuffing the keyboard will make it appear the user typed in the text when in reality it
    /// is your menu. Once the stuffed text has been acted upon, the user will not be returned
    /// to the menu file.
    StuffTextAndExitMenu,

    /// Stuff the keyboard with the text entered in the Parameters field.
    /// Once the stuffted text has been acted upon, the user will not be
    /// returned to the menu.
    StuffTextAndExitMenuSilent,

    /// Stuff the keyboard with the text entered in the Parameters field.
    StuffText,

    /// Stuff the keyboard with the text entered in the Parameters field.
    StuffTextSilent,

    /// Stuff the keyboard with the contents of the file specified in the
    /// Parameters field. Once the stuffed text has been acted upon, the user
    /// will be returned to the menu.
    StuffFile,

    /// Stuff the keyboard with the contents of the file specified in
    /// the Parameters field. The stuffed text will not be shown on the screen.
    StuffFileSilent,

    /// Moves caret to a specific position
    GotoXY,

    /// Print a text
    PrintText,

    /// Refreshes the display string of the command.
    RefreshDisplayString,

    // user commands
    /// A command
    AbandonConference,

    /// B command
    BulletinList,

    /// C command
    CommentToSysop,

    /// D command
    Download,

    /// E command
    EnterMessage,

    /// F command
    FileDirectory,

    /// Flag command
    FlagFiles,

    /// G command
    Goodbye,

    /// BYE commend (same as G;Y) - skips file flag scan
    Bye,

    /// H command
    Help,

    /// I command (moved to IW)
    InitialWelcome,

    /// J command
    JoinConference,

    /// K command
    DeleteMessage,

    /// L command
    LocateFile,

    /// M command
    ToggleGraphics,

    /// N command
    NewFileScan,

    /// O command
    PageSysop,

    /// P command
    SetPageLength,

    /// Q command
    QuickMessageScan,

    /// R command
    ReadMessages,

    /// S command
    Survey,

    /// T command
    SetTransferProtocol,

    /// U command
    UploadFile,

    /// V command
    ViewSettings,

    /// W command
    WriteSettings,

    /// X command
    ExpertMode,

    /// Y command
    YourMailScan,

    /// Z command
    ZippyDirectoryScan,

    /// CHAT command
    GroupChat,

    /// DOOR command
    OpenDoor,

    /// TEST command
    TestFile,

    /// USER command
    UserList,

    /// WHO command
    WhoIsOnline,

    /// MENU command
    ShowMenu,

    /// Execute command in parameters
    Command,

    /// Execute command in parameters (only global commands)
    GlobalCommand,

    DisplayNews,

    SetLanguage,

    // Like "E" but as reply
    ReplyMessage,

    // "ALIAS" command
    EnableAlias,

    // Sysop commands
    Broadcast,

    // SYSOP '4' command
    RestoreMessage,

    /// SYSOP '1' command
    ViewCallerLog,

    /// SYSOP '2' command
    ViewUserFile,

    /// SYSOP '3' command
    PackMessageBase,

    /// SYSOP '6' command
    ViewTextFile,

    /// SYSOP '7' command
    UserMaintenance,

    /// SYSOP '8' command
    PackUserFile,

    /// SYSOP '11' command
    NodeList,

    /// SYSOP '13' command
    NodeCallerLog,

    /// SYSOP '5' command
    HeaderScan,

    /// SYSOP '12' command
    LogoffNode,

    /// SYSOP '16' command
    DirCommand,

    // '@'
    ReadEmail,

    // '@W'
    WriteEmail,

    /// Using this option, you can execute any PPE file you wish.
    /// This only further enhances the options or tasks you can perform with
    /// each menu.
    RunPPE,

    // 'TS'
    TextSearch,

    // 'QWK'
    QWK,

    // 'SELECT'
    SelectConferences,

    /// BD command
    BatchDownload,

    /// BU command
    BatchUpload,

    /// RM command
    ReadMemorizedMessage(u8),

    // 'AREA'
    ChangeMessageArea,
}

impl Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::Disabled => write!(f, "Disabled"),
            CommandType::Menu => write!(f, "Menu"),
            //         CommandType::PPE => write!(f, "PPE"),
            CommandType::Script => write!(f, "Script"),
            CommandType::Conference => write!(f, "Conference"),
            CommandType::DisplayDir => write!(f, "DisplayDir"),
            CommandType::DisableMenuOption => write!(f, "DisableMenuOption"),
            CommandType::Door => write!(f, "Door"),
            CommandType::ExitMenus => write!(f, "ExitMenus"),
            CommandType::QuitMenu => write!(f, "QuitMenu"),
            CommandType::DisplayFile => write!(f, "DisplayFile"),
            CommandType::StuffTextAndExitMenu => write!(f, "StuffTextAndExitMenu"),
            CommandType::StuffTextAndExitMenuSilent => write!(f, "StuffTextAndExitMenuSilent"),
            CommandType::StuffText => write!(f, "StuffText"),
            CommandType::StuffTextSilent => write!(f, "StuffTextSilent"),
            CommandType::StuffFile => write!(f, "StuffFile"),
            CommandType::StuffFileSilent => write!(f, "StuffFileSilent"),
            CommandType::AbandonConference => write!(f, "AbandonConference"),
            CommandType::BulletinList => write!(f, "BulletinList"),
            CommandType::CommentToSysop => write!(f, "CommentToSysop"),
            CommandType::Download => write!(f, "Download"),
            CommandType::FlagFiles => write!(f, "FlagFiles"),
            CommandType::EnterMessage => write!(f, "EnterMessage"),
            CommandType::FileDirectory => write!(f, "FileDirectory"),
            CommandType::Goodbye => write!(f, "Goodbye"),
            CommandType::Bye => write!(f, "Bye"),
            CommandType::Help => write!(f, "Help"),
            CommandType::InitialWelcome => write!(f, "InitialWelcome"),
            CommandType::JoinConference => write!(f, "JoinConference"),
            CommandType::DeleteMessage => write!(f, "DeleteMessage"),
            CommandType::LocateFile => write!(f, "LocateFile"),
            CommandType::ToggleGraphics => write!(f, "ToggleGraphics"),
            CommandType::NewFileScan => write!(f, "NewFileScan"),
            CommandType::PageSysop => write!(f, "PageSysop"),
            CommandType::SetPageLength => write!(f, "SetPageLength"),
            CommandType::QuickMessageScan => write!(f, "QuickMessageScan"),
            CommandType::ReadMessages => write!(f, "ReadMessages"),
            CommandType::Survey => write!(f, "Survey"),
            CommandType::SetTransferProtocol => write!(f, "SetTransferProtocol"),
            CommandType::UploadFile => write!(f, "UploadFile"),
            CommandType::ViewSettings => write!(f, "ViewSettings"),
            CommandType::WriteSettings => write!(f, "WriteSettings"),
            CommandType::ExpertMode => write!(f, "ExpertMode"),
            CommandType::YourMailScan => write!(f, "YourMailScan"),
            CommandType::ZippyDirectoryScan => write!(f, "ZippyDirectoryScan"),
            CommandType::GroupChat => write!(f, "GroupChat"),
            CommandType::OpenDoor => write!(f, "OpenDoor"),
            CommandType::TestFile => write!(f, "TestFile"),
            CommandType::UserList => write!(f, "UserList"),
            CommandType::WhoIsOnline => write!(f, "WhoIsOnline"),
            CommandType::ShowMenu => write!(f, "ShowMenu"),
            CommandType::Command => write!(f, "Command"),
            CommandType::GlobalCommand => write!(f, "GlobalCommand"),
            CommandType::DisplayNews => write!(f, "DisplayNews"),
            CommandType::SetLanguage => write!(f, "SetLanguage"),
            CommandType::ReplyMessage => write!(f, "ReplyMessage"),
            CommandType::EnableAlias => write!(f, "EnableAlias"),
            CommandType::Broadcast => write!(f, "Broadcast"),
            CommandType::RestoreMessage => write!(f, "RestoreMessage"),
            CommandType::ViewCallerLog => write!(f, "ViewCallerLog"),
            CommandType::ViewUserFile => write!(f, "ViewUserFile"),
            CommandType::PackMessageBase => write!(f, "PackMessageBase"),
            CommandType::ViewTextFile => write!(f, "ViewTextFile"),
            CommandType::UserMaintenance => write!(f, "UserMaintenance"),
            CommandType::PackUserFile => write!(f, "PackUserFile"),
            CommandType::NodeList => write!(f, "NodeList"),
            CommandType::NodeCallerLog => write!(f, "NodeCallerLog"),
            CommandType::HeaderScan => write!(f, "HeaderScan"),
            CommandType::LogoffNode => write!(f, "LogoffNode"),
            CommandType::DirCommand => write!(f, "DirCommand"),
            CommandType::ReadEmail => write!(f, "ReadEmail"),
            CommandType::WriteEmail => write!(f, "WriteEmail"),
            CommandType::RunPPE => write!(f, "RunPPE"),
            CommandType::TextSearch => write!(f, "TextSearch"),
            CommandType::QWK => write!(f, "QWK"),
            CommandType::SelectConferences => write!(f, "SelectConferences"),
            CommandType::BatchDownload => write!(f, "BatchDownload"),
            CommandType::BatchUpload => write!(f, "BatchUpload"),
            CommandType::ChangeMessageArea => write!(f, "MessageArea"),
            CommandType::ReadMemorizedMessage(_) => write!(f, "ReadMemorizedMessage"),
            CommandType::GotoXY => write!(f, "GotoXY"),
            CommandType::PrintText => write!(f, "PrintText"),
            CommandType::RefreshDisplayString => write!(f, "RefreshDisplayString"),
        }
    }
}

/*
  match self {
            CommandType::Disabled => write!(f, "Disabled"),
            CommandType::Menu => write!(f, "Menu"),
            //         CommandType::PPE => write!(f, "PPE"),
            CommandType::Script => write!(f, "Script"),
            CommandType::Conference => write!(f, "Conference"),
            CommandType::DisplayDir => write!(f, "DisplayDir"),
            CommandType::DisableMenuOption => write!(f, "DisableMenuOption"),
            CommandType::Door => write!(f, "Door"),
            CommandType::ExitMenus => write!(f, "ExitMenus"),
            CommandType::QuitMenu => write!(f, "QuitMenu"),
            CommandType::DisplayFile => write!(f, "DisplayFile"),
            CommandType::StuffTextAndExitMenu => write!(f, "StuffTextAndExitMenu"),
            CommandType::StuffTextAndExitMenuSilent => write!(f, "StuffTextAndExitMenuSilent"),
            CommandType::StuffText => write!(f, "StuffText"),
            CommandType::StuffTextSilent => write!(f, "StuffTextSilent"),
            CommandType::StuffFile => write!(f, "StuffFile"),
            CommandType::StuffFileSilent => write!(f, "StuffFileSilent"),
            CommandType::AbandonConference => write!(f, "(A)\tAbandonConference"),
            CommandType::BulletinList => write!(f, "(B)\tBulletinList"),
            CommandType::CommentToSysop => write!(f, "(C)\tCommentToSysop"),
            CommandType::Download => write!(f, "(D)\tDownload"),
            CommandType::FlagFiles => write!(f, "(FLAG)\tFlagFiles"),
            CommandType::EnterMessage => write!(f, "(E)\tEnterMessage"),
            CommandType::FileDirectory => write!(f, "(F)\tFileDirectory"),
            CommandType::Goodbye => write!(f, "(G)\tGoodbye"),
            CommandType::Bye => write!(f, "(G;Y)\tBye"),
            CommandType::Help => write!(f, "(H)\tHelp"),
            CommandType::InitialWelcome => write!(f, "(I)\tInitialWelcome"),
            CommandType::JoinConference => write!(f, "(J)\tJoinConference"),
            CommandType::DeleteMessage => write!(f, "(K)\tDeleteMessage"),
            CommandType::LocateFile => write!(f, "(L)\tLocateFile"),
            CommandType::ToggleGraphics => write!(f, "(M)\tToggleGraphics"),
            CommandType::NewFileScan => write!(f, "(N)\tNewFileScan"),
            CommandType::PageSysop => write!(f, "(O)\tPageSysop"),
            CommandType::SetPageLength => write!(f, "(P)\tSetPageLength"),
            CommandType::QuickMessageScan => write!(f, "(Q)\tQuickMessageScan"),
            CommandType::ReadMessages => write!(f, "(R)\tReadMessages"),
            CommandType::Survey => write!(f, "(S)\tSurvey"),
            CommandType::SetTransferProtocol => write!(f, "(T)\tSetTransferProtocol"),
            CommandType::UploadFile => write!(f, "(U)\tUploadFile"),
            CommandType::ViewSettings => write!(f, "(V)\tViewSettings"),
            CommandType::WriteSettings => write!(f, "(W)\tWriteSettings"),
            CommandType::ExpertMode => write!(f, "(X)\tExpertMode"),
            CommandType::YourMailScan => write!(f, "(Y)\tYourMailScan"),
            CommandType::ZippyDirectoryScan => write!(f, "(Z)\tZippyDirectoryScan"),
            CommandType::GroupChat => write!(f, "(CHAT)\tGroupChat"),
            CommandType::OpenDoor => write!(f, "(DOOR)\tOpenDoor"),
            CommandType::TestFile => write!(f, "(TEST)\tTestFile"),
            CommandType::UserList => write!(f, "(USER)\tUserList"),
            CommandType::WhoIsOnline => write!(f, "(WHO)\tWhoIsOnline"),
            CommandType::ShowMenu => write!(f, "(MENU)\tShowMenu"),
            CommandType::Command => write!(f, "Command"),
            CommandType::GlobalCommand => write!(f, "GlobalCommand"),
            CommandType::DisplayNews => write!(f, "(NEWS)\tDisplayNews"),
            CommandType::SetLanguage => write!(f, "(LANG)\tSetLanguage"),
            CommandType::ReplyMessage => write!(f, "(REPLY)\tReplyMessage"),
            CommandType::EnableAlias => write!(f, "(ALIAS)\tEnableAlias"),
            CommandType::Broadcast => write!(f, "(BROADCAST)\tBroadcast"),
            CommandType::RestoreMessage => write!(f, "(RESTORE)\tRestoreMessage"),
            CommandType::ViewCallerLog => write!(f, "(1)\tViewCallerLog"),
            CommandType::ViewUserFile => write!(f, "(2)\tViewUserFile"),
            CommandType::PackMessageBase => write!(f, "(3)\tPackMessageBase"),
            CommandType::ViewTextFile => write!(f, "(6)\tViewTextFile"),
            CommandType::UserMaintenance => write!(f, "(7)\tUserMaintenance"),
            CommandType::PackUserFile => write!(f, "(8)\tPackUserFile"),
            CommandType::NodeList => write!(f, "(11)\tNodeList"),
            CommandType::NodeCallerLog => write!(f, "(13)\tNodeCallerLog"),
            CommandType::HeaderScan => write!(f, "(5)\tHeaderScan"),
            CommandType::LogoffNode => write!(f, "(12)\tLogoffNode"),
            CommandType::DirCommand => write!(f, "(16)\tDirCommand"),
            CommandType::ReadEmail => write!(f, "(@)\tReadEmail"),
            CommandType::WriteEmail => write!(f, "(@W)\tWriteEmail"),
            CommandType::RunPPE => write!(f, "(PPE)\tRunPPE"),
            CommandType::TextSearch => write!(f, "(TS)\tTextSearch"),
            CommandType::QWK => write!(f, "(QWK)\tQWK"),
            CommandType::SelectConferences => write!(f, "(SELECT)\tSelectConferences"),
            CommandType::BatchDownload => write!(f, "(BD)\tBatchDownload"),
            CommandType::BatchUpload => write!(f, "(BU)\tBatchUpload"),
            CommandType::ChangeMessageArea => write!(f, "(AREA)\tMessageArea"),
            CommandType::ReadMemorizedMessage => write!(f, "(RM)\tReadMemorizedMessage"),
            CommandType::GotoXY => write!(f, "GotoXY"),
            CommandType::PrintText => write!(f, "PrintText"),
            CommandType::RefreshDisplayString => write!(f, "RefreshDisplayString"),
        }
*/

impl FromStr for CommandType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = s.trim().to_ascii_lowercase();
        match key.as_str() {
            "disabled" => Ok(CommandType::Disabled),
            "menu" => Ok(CommandType::Menu),
            "script" => Ok(CommandType::Script),
            "conference" => Ok(CommandType::Conference),
            "displaydir" => Ok(CommandType::DisplayDir),
            "disablemenuoption" => Ok(CommandType::DisableMenuOption),
            "door" => Ok(CommandType::Door),
            "exitmenus" => Ok(CommandType::ExitMenus),
            "quitmenu" => Ok(CommandType::QuitMenu),
            "displayfile" => Ok(CommandType::DisplayFile),
            "stufftextandexitmenu" => Ok(CommandType::StuffTextAndExitMenu),
            "stufftextandexitmenusilent" => Ok(CommandType::StuffTextAndExitMenuSilent),
            "stufftext" => Ok(CommandType::StuffText),
            "stufftextsilent" => Ok(CommandType::StuffTextSilent),
            "stufffile" => Ok(CommandType::StuffFile),
            "stufffilesilent" => Ok(CommandType::StuffFileSilent),
            "abandonconference" => Ok(CommandType::AbandonConference),
            "bulletinlist" => Ok(CommandType::BulletinList),
            "commenttosysop" => Ok(CommandType::CommentToSysop),
            "download" => Ok(CommandType::Download),
            "entermessage" => Ok(CommandType::EnterMessage),
            "filedirectory" => Ok(CommandType::FileDirectory),
            "goodbye" => Ok(CommandType::Goodbye),
            "bye" => Ok(CommandType::Bye),
            "help" => Ok(CommandType::Help),
            "initialwelcome" => Ok(CommandType::InitialWelcome),
            "joinconference" => Ok(CommandType::JoinConference),
            "deletemessage" => Ok(CommandType::DeleteMessage),
            "locatefile" => Ok(CommandType::LocateFile),
            "togglegraphics" => Ok(CommandType::ToggleGraphics),
            "newfilescan" => Ok(CommandType::NewFileScan),
            "pagesysop" => Ok(CommandType::PageSysop),
            "setpagelength" => Ok(CommandType::SetPageLength),
            "quickmessagescan" => Ok(CommandType::QuickMessageScan),
            "readmessages" => Ok(CommandType::ReadMessages),
            "survey" => Ok(CommandType::Survey),
            "settransferprotocol" => Ok(CommandType::SetTransferProtocol),
            "uploadfile" => Ok(CommandType::UploadFile),
            "viewsettings" => Ok(CommandType::ViewSettings),
            "writesettings" => Ok(CommandType::WriteSettings),
            "expertmode" => Ok(CommandType::ExpertMode),
            "yourmailscan" => Ok(CommandType::YourMailScan),
            "zippydirectoryscan" => Ok(CommandType::ZippyDirectoryScan),
            "groupchat" => Ok(CommandType::GroupChat),
            "opendoor" => Ok(CommandType::OpenDoor),
            "testfile" => Ok(CommandType::TestFile),
            "userlist" => Ok(CommandType::UserList),
            "whoisonline" => Ok(CommandType::WhoIsOnline),
            "showmenu" => Ok(CommandType::ShowMenu),
            "command" => Ok(CommandType::Command),
            "globalcommand" => Ok(CommandType::GlobalCommand),
            "displaynews" => Ok(CommandType::DisplayNews),
            "setlanguage" => Ok(CommandType::SetLanguage),
            "replymessage" => Ok(CommandType::ReplyMessage),
            "enablealias" => Ok(CommandType::EnableAlias),
            "broadcast" => Ok(CommandType::Broadcast),
            "restoremessage" => Ok(CommandType::RestoreMessage),
            "viewcallerlog" => Ok(CommandType::ViewCallerLog),
            "viewuserfile" => Ok(CommandType::ViewUserFile),
            "packmessagebase" => Ok(CommandType::PackMessageBase),
            "viewtextfile" => Ok(CommandType::ViewTextFile),
            "usermaintenance" => Ok(CommandType::UserMaintenance),
            "packuserfile" => Ok(CommandType::PackUserFile),
            "nodelist" => Ok(CommandType::NodeList),
            "nodecallerlog" => Ok(CommandType::NodeCallerLog),
            "headerscan" => Ok(CommandType::HeaderScan),
            "logoffnode" => Ok(CommandType::LogoffNode),
            "dircommand" => Ok(CommandType::DirCommand),
            "reademail" => Ok(CommandType::ReadEmail),
            "writeemail" => Ok(CommandType::WriteEmail),
            "runppe" => Ok(CommandType::RunPPE),
            "textsearch" => Ok(CommandType::TextSearch),
            "gotoxy" => Ok(CommandType::GotoXY),
            "printtext" => Ok(CommandType::PrintText),
            "refreshdisplaystring" => Ok(CommandType::RefreshDisplayString),
            "changemessagearea" => Ok(CommandType::ChangeMessageArea),
            // Optionally add:
            // "qwk" => Ok(CommandType::QWK),
            // "selectconferences" => Ok(CommandType::SelectConferences),
            // "batchdownload" => Ok(CommandType::BatchDownload),
            // "batchupload" => Ok(CommandType::BatchUpload),
            // "readmemorizedmessage0" => Ok(CommandType::ReadMemorizedMessage(0)),
            // "readmemorizedmessage1" => Ok(CommandType::ReadMemorizedMessage(1)),
            // "readmemorizedmessage2" => Ok(CommandType::ReadMemorizedMessage(2)),
            _ => Err(format!("Invalid CommandType: {s}")),
        }
    }
}

impl CommandType {
    pub fn iter() -> impl Iterator<Item = CommandType> {
        vec![
            CommandType::Disabled,
            CommandType::Menu,
            CommandType::Script,
            CommandType::Conference,
            CommandType::DisplayDir,
            CommandType::DisableMenuOption,
            CommandType::Door,
            CommandType::ExitMenus,
            CommandType::QuitMenu,
            CommandType::DisplayFile,
            CommandType::StuffTextAndExitMenu,
            CommandType::StuffTextAndExitMenuSilent,
            CommandType::StuffText,
            CommandType::StuffTextSilent,
            CommandType::StuffFile,
            CommandType::StuffFileSilent,
            CommandType::AbandonConference,
            CommandType::BulletinList,
            CommandType::CommentToSysop,
            CommandType::Download,
            CommandType::FlagFiles,
            CommandType::EnterMessage,
            CommandType::FileDirectory,
            CommandType::Goodbye,
            CommandType::Bye,
            CommandType::Help,
            CommandType::InitialWelcome,
            CommandType::JoinConference,
            CommandType::DeleteMessage,
            CommandType::LocateFile,
            CommandType::ToggleGraphics,
            CommandType::NewFileScan,
            CommandType::PageSysop,
            CommandType::SetPageLength,
            CommandType::QuickMessageScan,
            CommandType::ReadMessages,
            CommandType::Survey,
            CommandType::SetTransferProtocol,
            CommandType::UploadFile,
            CommandType::ViewSettings,
            CommandType::WriteSettings,
            CommandType::ExpertMode,
            CommandType::YourMailScan,
            CommandType::ZippyDirectoryScan,
            CommandType::GroupChat,
            CommandType::OpenDoor,
            CommandType::TestFile,
            CommandType::UserList,
            CommandType::WhoIsOnline,
            CommandType::ShowMenu,
            CommandType::Command,
            CommandType::GlobalCommand,
            CommandType::DisplayNews,
            CommandType::SetLanguage,
            CommandType::ReplyMessage,
            CommandType::EnableAlias,
            CommandType::Broadcast,
            CommandType::RestoreMessage,
            CommandType::ViewCallerLog,
            CommandType::ViewUserFile,
            CommandType::PackMessageBase,
            CommandType::ViewTextFile,
            CommandType::UserMaintenance,
            CommandType::PackUserFile,
            CommandType::NodeList,
            CommandType::NodeCallerLog,
            CommandType::HeaderScan,
            CommandType::LogoffNode,
            CommandType::DirCommand,
            CommandType::ReadEmail,
            CommandType::WriteEmail,
            CommandType::RunPPE,
            CommandType::TextSearch,
            CommandType::GotoXY,
            CommandType::PrintText,
            CommandType::RefreshDisplayString,
            CommandType::ChangeMessageArea,
            CommandType::QWK,
            CommandType::SelectConferences,
            CommandType::BatchDownload,
            CommandType::BatchUpload,
            CommandType::ReadMemorizedMessage(0),
            CommandType::ReadMemorizedMessage(1),
            CommandType::ReadMemorizedMessage(2),
        ]
        .into_iter()
    }

    pub fn get_help(self) -> &'static str {
        match self {
            CommandType::AbandonConference => "hlpa",
            CommandType::BulletinList => "hlpb",
            CommandType::CommentToSysop => "hlpc",
            CommandType::Download | CommandType::BatchDownload => "hlpd",
            CommandType::EnterMessage => "hlpe",
            CommandType::FileDirectory => "hlpf",
            CommandType::FlagFiles => "hlpflag",
            CommandType::Goodbye | CommandType::Bye => "hlpg",
            CommandType::Help => "hlph",
            CommandType::InitialWelcome => "hlpi",
            CommandType::JoinConference => "hlpj",
            CommandType::DeleteMessage => "hlpk",
            CommandType::LocateFile => "hlpl",
            CommandType::ToggleGraphics => "hlpm",
            CommandType::NewFileScan => "hlpn",
            CommandType::PageSysop => "hlpo",
            CommandType::SetPageLength => "hlpp",
            CommandType::QuickMessageScan => "hlpq",
            CommandType::ReadMessages => "hlpr",
            CommandType::Survey => "hlps",
            CommandType::SetTransferProtocol => "hlpt",
            CommandType::UploadFile | CommandType::BatchUpload => "hlpu",
            CommandType::ViewSettings => "hlpv",
            CommandType::WriteSettings => "hlpw",
            CommandType::ExpertMode => "hlpx",
            CommandType::YourMailScan => "hlpy",
            CommandType::ZippyDirectoryScan => "hlpz",
            CommandType::GroupChat => "hlpchat",
            CommandType::OpenDoor => "hlpopen",
            CommandType::TestFile => "hlptest",
            CommandType::UserList => "hlpusers",
            CommandType::WhoIsOnline => "hlpwho",
            CommandType::DisplayNews => "hlpnews",
            CommandType::SetLanguage => "hlplang",
            CommandType::ReplyMessage => "hlprep",
            CommandType::EnableAlias => "hlpalias",
            CommandType::Broadcast => "hlpbrd",
            CommandType::RestoreMessage => "hlp4",
            CommandType::ViewCallerLog => "hlp1",
            CommandType::ViewUserFile => "hlp2",
            CommandType::PackMessageBase => "hlp3",
            CommandType::ViewTextFile => "hlp6",
            CommandType::UserMaintenance => "hlp7",
            CommandType::PackUserFile => "hlp8",
            CommandType::NodeList => "hlp11",
            CommandType::NodeCallerLog => "hlp13",
            CommandType::HeaderScan => "hlp5",
            CommandType::LogoffNode => "hlp12",
            CommandType::DirCommand => "hlp16",
            CommandType::ReadEmail => "hlp@",
            CommandType::WriteEmail => "hlp@w",
            CommandType::RunPPE => "hlpppe",
            CommandType::TextSearch => "hlpts",
            CommandType::ChangeMessageArea => "hlparea",
            CommandType::QWK => "hlpqwk",
            CommandType::SelectConferences => "hlpsel",
            CommandType::ReadMemorizedMessage(_) => "hlprm",
            // PCBoard sends the batch commands to the plain transfer help.
            _ => "",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn is_default(&self) -> bool {
        *self == Position::default()
    }

    pub fn parse(txt: &str) -> Self {
        let mut parts = txt.split(',');
        let x = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
        let y = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
        Position { x, y }
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.x, &self.y).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(x, y)| Position { x, y })
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub enum AutoRun {
    #[default]
    Disabled,

    /// Run the command the first time the menu is loaded
    FirstCmd,

    /// Run the command every time before the menu is displayed
    Every,

    /// Run the command every time after the menu is displayed
    After,

    /// Run the command after a certain timeout in a loop
    /// For example to display the current time or to update a scrolling message
    Loop,
}

impl AutoRun {
    pub fn is_default(&self) -> bool {
        matches!(self, AutoRun::Disabled)
    }
}

impl FromStr for AutoRun {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Disabled" => Ok(AutoRun::Disabled),
            "FirstCmd" => Ok(AutoRun::FirstCmd),
            "Every" => Ok(AutoRun::Every),
            "After" => Ok(AutoRun::After),
            "Loop" => Ok(AutoRun::Loop),
            _ => Err(format!("Invalid AutoRun: {s}")),
        }
    }
}

impl AutoRun {
    pub fn iter() -> impl Iterator<Item = AutoRun> {
        vec![AutoRun::Disabled, AutoRun::FirstCmd, AutoRun::Every, AutoRun::After, AutoRun::Loop].into_iter()
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Command {
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub display: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lighbar_display: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Position::is_default")]
    pub position: Position,

    #[serde(default)]
    pub keyword: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "AutoRun::is_default")]
    pub auto_run: AutoRun,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub autorun_time: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub help: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub security: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<CommandAction>,
}

#[derive(Serialize, Clone, Deserialize, PartialEq, Debug, Default)]
pub enum ActionTrigger {
    #[default]
    Activation,
    Selection,
}

impl ActionTrigger {
    pub fn is_default(&self) -> bool {
        matches!(self, ActionTrigger::Activation)
    }
}

#[derive(Serialize, Clone, Deserialize, Default, PartialEq)]
pub struct CommandAction {
    pub command_type: CommandType,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parameter: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "ActionTrigger::is_default")]
    pub trigger: ActionTrigger,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CommandList {
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
}

impl Deref for CommandList {
    type Target = Vec<Command>;
    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl DerefMut for CommandList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}
impl CommandList {
    pub fn new() -> Self {
        let commands = vec![];
        Self { commands }
    }
}

/// A keyword comes out of a file a sysop typed, so its case says nothing.
pub fn find_exact<'a>(commands: &'a [Command], keyword: &str) -> Option<&'a Command> {
    commands.iter().find(|cmd| cmd.keyword.eq_ignore_ascii_case(keyword))
}

/// The same, for a keyword the caller has only typed the beginning of.
///
/// `PCBoard` took an abbreviation from two characters on - one character is never enough,
/// no matter what a command list holds, so `G` stays Goodbye next to a `GREED` entry.
/// See `runcmds()` in `PCBoard`'s CMDS.C.
pub fn find_prefix<'a>(commands: &'a [Command], keyword: &str) -> Option<&'a Command> {
    if keyword.len() < 2 {
        return None;
    }
    commands.iter().find(|cmd| {
        let name = cmd.keyword.as_bytes();
        let typed = keyword.as_bytes();
        name.len() >= typed.len() && name[..typed.len()].eq_ignore_ascii_case(typed) && (typed.len() + 1 >= name.len() || name.len() >= 3)
    })
}

impl PCBoardRecordImporter<Command> for CommandList {
    const RECORD_SIZE: usize = 0x40;

    fn push(&mut self, value: Command) {
        self.commands.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<Command> {
        let name = crate::tables::import_cp437_string(&data[..15], true);
        let security = data[15];

        let parameter = crate::tables::import_cp437_string(&data[16..56], true);

        let uc = parameter.to_uppercase();
        let command_type = if std::path::Path::new(&uc).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("MNU")) {
            CommandType::Menu
        } else if uc.contains(".PPE") {
            CommandType::RunPPE
        } else {
            CommandType::StuffText
        };

        Ok(Command {
            keyword: name,
            display: String::new(),
            lighbar_display: String::new(),
            help: String::new(),
            auto_run: AutoRun::Disabled,
            autorun_time: 0,
            position: Position::default(),
            actions: vec![CommandAction {
                command_type,
                parameter,
                trigger: ActionTrigger::Activation,
            }],
            security: SecurityExpression::from_req_security(security),
        })
    }
}

impl IcyBoardSerializer for CommandList {
    const FILE_TYPE: &'static str = "commands";
}

#[cfg(test)]
mod tests {
    use super::{Command, find_exact, find_prefix};

    fn list(keywords: &[&str]) -> Vec<Command> {
        keywords
            .iter()
            .map(|keyword| Command {
                keyword: keyword.to_string(),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn test_a_lower_case_keyword_answers_an_upper_case_command() {
        let commands = list(&["v"]);
        assert_eq!(find_exact(&commands, "V").unwrap().keyword, "v");
    }

    #[test]
    fn test_an_upper_case_keyword_answers_a_lower_case_command() {
        let commands = list(&["V"]);
        assert_eq!(find_exact(&commands, "v").unwrap().keyword, "V");
    }

    #[test]
    fn test_a_keyword_that_is_not_there_is_not_found() {
        assert!(find_exact(&list(&["V"]), "W").is_none());
    }

    #[test]
    fn test_the_first_of_two_equal_keywords_wins() {
        let commands = list(&["V", "v"]);
        assert_eq!(find_exact(&commands, "V").unwrap().keyword, "V");
    }

    #[test]
    fn test_a_prefix_finds_the_longer_keyword_whatever_its_case() {
        let commands = list(&["vote"]);
        assert_eq!(find_prefix(&commands, "VO").unwrap().keyword, "vote");
    }

    #[test]
    fn test_a_command_longer_than_the_keyword_is_no_prefix_of_it() {
        assert!(find_prefix(&list(&["V"]), "VOTE").is_none());
    }

    #[test]
    fn test_a_prefix_search_does_not_split_a_multi_byte_keyword() {
        assert!(find_prefix(&list(&["ä"]), "A").is_none());
    }

    #[test]
    fn test_a_single_character_is_no_abbreviation() {
        assert!(find_prefix(&list(&["GREED"]), "G").is_none());
    }

    #[test]
    fn test_two_characters_are_an_abbreviation() {
        assert_eq!(find_prefix(&list(&["GREED"]), "GR").unwrap().keyword, "GREED");
    }

    #[test]
    fn test_a_two_letter_keyword_needs_all_of_it() {
        assert!(find_prefix(&list(&["VW"]), "V").is_none());
    }
}
