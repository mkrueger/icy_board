use icy_ppe::Res;
use serde::{Deserialize, Serialize};

use super::{security::RequiredSecurity, IcyBoardSerializer, PCBoardRecordImporter};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CommandType {
    /// Do nothing
    Disabled,

    /// If you have assigned a menu command to have this type,
    /// you can load another MNU file as specified in the Parameters field.
    /// This would effectively let you create a sub-menu type system that is very
    /// easy to navigate.
    Menu,

    /// Using this option, you can execute any PPE file you wish.
    /// This only further enhances the options or tasks you can perform with
    /// each menu.
    PPE,

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

    /// G command
    Goodbye,

    /// H command
    Help,

    /// I command (moved to IW)
    InitialWelcome,

    /// J command
    JoinConference,

    /// New "I" command
    MessageAreas,

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
    WriteUserSettings,

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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Command {
    pub input: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub help: String,

    pub command_type: CommandType,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parameter: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub security: RequiredSecurity,
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
        let name = icy_ppe::tables::import_cp437_string(&data[..15], true);
        let security = data[15];

        let uc = name.to_uppercase();
        let command_type = if uc.ends_with(".MNU") {
            CommandType::Menu
        } else if uc.ends_with(".PPE") {
            CommandType::PPE
        } else {
            CommandType::StuffText
        };

        let parameter = icy_ppe::tables::import_cp437_string(&data[16..56], true);
        Ok(Command {
            input: vec![name],
            help: "".to_string(),
            command_type,
            parameter,
            security: RequiredSecurity::new(security),
        })
    }
}

impl IcyBoardSerializer for CommandList {
    const FILE_TYPE: &'static str = "commands";
}
