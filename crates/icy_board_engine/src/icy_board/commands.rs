use std::fmt::Display;

use crate::Res;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use strum::{EnumIter, EnumString};

use super::{security::RequiredSecurity, IcyBoardSerializer, PCBoardRecordImporter};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default, EnumIter, EnumString)]
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

    /// While this option type is similar to QuitMenu, it is different because it will
    /// quit all active menus.
    ExitMenus,

    /// To quit the current menu and return to the previous menu (if any), define a menu option
    /// that uses this option type. Remember that only the current menu will be exited.
    /// To exit all menus, use the ExitMenus option instead.
    QuitMenu,

    /// If you want to display a text file to the caller, you may do so using this option type.
    /// As with normal PCBoard display files, you can create security, graphics, and language specific
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
    /// ! command
    RedisplayCommand,

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
    PersonalMail,

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
            CommandType::RedisplayCommand => write!(f, "(!)\tRedisplayCommand"),
            CommandType::AbandonConference => write!(f, "(A)\tAbandonConference"),
            CommandType::BulletinList => write!(f, "(B)\tBulletinList"),
            CommandType::CommentToSysop => write!(f, "(C)\tCommentToSysop"),
            CommandType::Download => write!(f, "(D)\tDownload"),
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
            CommandType::PersonalMail => write!(f, "(Y)\tPersonalMail"),
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
            CommandType::ReadEmail => write!(f, "(@)\tReadEmail"),
            CommandType::WriteEmail => write!(f, "(@W)\tWriteEmail"),
            CommandType::RunPPE => write!(f, "(PPE)\tRunPPE"),
            CommandType::TextSearch => write!(f, "(TS)\tTextSearch"),
            CommandType::GotoXY => write!(f, "GotoXY"),
            CommandType::PrintText => write!(f, "PrintText"),
            CommandType::RefreshDisplayString => write!(f, "RefreshDisplayString"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
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

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, EnumString, EnumIter, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Command {
    #[serde(default)]
    pub display: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lighbar_display: String,

    #[serde(default)]
    pub position: Position,

    #[serde(default)]
    pub keyword: String,

    #[serde(default)]
    pub auto_run: AutoRun,

    #[serde(default)]
    pub autorun_time: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub help: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub security: RequiredSecurity,

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

#[derive(Serialize, Clone, Deserialize, Default, PartialEq)]
pub struct CommandAction {
    pub command_type: CommandType,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parameter: String,

    pub trigger: ActionTrigger,
}

#[derive(Serialize, Deserialize, Default)]
pub struct CommandList {
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
}

impl PCBoardRecordImporter<Command> for CommandList {
    const RECORD_SIZE: usize = 0x40;

    fn push(&mut self, value: Command) {
        self.commands.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<Command> {
        let name = crate::tables::import_cp437_string(&data[..15], true);
        let security = data[15];

        let uc = name.to_uppercase();
        let command_type = if uc.ends_with(".MNU") {
            CommandType::Menu
        } else if uc.ends_with(".PPE") {
            CommandType::RunPPE
        } else {
            CommandType::StuffText
        };

        let parameter = crate::tables::import_cp437_string(&data[16..56], true);
        Ok(Command {
            keyword: name,
            display: "".to_string(),
            lighbar_display: "".to_string(),
            help: "".to_string(),
            auto_run: AutoRun::Disabled,
            autorun_time: 0,
            position: Position::default(),
            actions: vec![CommandAction {
                command_type,
                parameter,
                trigger: ActionTrigger::Activation,
            }],
            security: RequiredSecurity::new(security),
        })
    }
}

impl IcyBoardSerializer for CommandList {
    const FILE_TYPE: &'static str = "commands";
}
