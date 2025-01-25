use std::fmt::Display;

use super::VariableType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatementSignature {
    Invalid,
    /// The first usize is the variable - 0 for none, second is the number of arguments
    ArgumentsWithVariable(usize, usize),

    /// The first i32 is a variable, 0 for none
    VariableArguments(usize),

    /// 3 arguments, first expression, after that a single i16 for the table number of a variable, third an expression
    SpecialCaseDlockg,

    /// 4 arguments, 3 expressions after that a single i16 for the table number of a variable
    SpecialCaseDcreate,

    /// 2 arguments, 2 single numbers for the table numbers of the arguments
    SpecialCaseSort,

    /// 2 arguments, 2 id expressions (2 i16 each)
    SpecialCaseVarSeg,

    /// n arguments, first 1 number for the number of arguments, followed by n table lookups
    SpecialCasePop,
}

#[repr(i16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OpCode {
    END = 1,
    CLS = 2,
    CLREOL = 3,
    MORE = 4,
    WAIT = 5,
    COLOR = 6,
    GOTO = 7,
    LET = 8,
    PRINT = 9,
    PRINTLN = 10,
    IFNOT = 11,
    CONFFLAG = 12,
    CONFUNFLAG = 13,
    DISPFILE = 14,
    INPUT = 15,
    FCREATE = 16,
    FOPEN = 17,
    FAPPEND = 18,
    FCLOSE = 19,
    FGET = 20,
    FPUT = 21,
    FPUTLN = 22,
    RESETDISP = 23,
    STARTDISP = 24,
    FPUTPAD = 25,
    HANGUP = 26,
    GETUSER = 27,
    PUTUSER = 28,
    DEFCOLOR = 29,
    DELETE = 30,
    DELUSER = 31,
    ADJTIME = 32,
    LOG = 33,
    INPUTSTR = 34,
    INPUTYN = 35,
    INPUTMONEY = 36,
    INPUTINT = 37,
    INPUTCC = 38,
    INPUTDATE = 39,
    INPUTTIME = 40,
    GOSUB = 41,
    RETURN = 42,
    PROMPTSTR = 43,
    DTRON = 44,
    DTROFF = 45,
    CDCHKON = 46,
    CDCHKOFF = 47,
    DELAY = 48,
    SENDMODEM = 49,
    INC = 50,
    DEC = 51,
    NEWLINE = 52,
    NEWLINES = 53,
    TOKENIZE = 54,
    GETTOKEN = 55,
    SHELL = 56,
    DISPTEXT = 57,
    STOP = 58,
    INPUTTEXT = 59,
    BEEP = 60,
    PUSH = 61,
    POP = 62,
    KBDSTUFF = 63,
    CALL = 64,
    JOIN = 65,
    QUEST = 66,
    BLT = 67,
    DIR = 68,
    KBDFILE = 69,
    BYE = 70,
    GOODBYE = 71,
    BROADCAST = 72,
    WAITFOR = 73,
    KBDCHKON = 74,
    KBDCHKOFF = 75,
    OPTEXT = 76,
    DISPSTR = 77,
    RDUNET = 78,
    WRUNET = 79,
    DOINTR = 80,
    VARSEG = 81,
    VAROFF = 82,
    POKEB = 83,
    POKEW = 84,
    VARADDR = 85,
    ANSIPOS = 86,
    BACKUP = 87,
    FORWARD = 88,
    FRESHLINE = 89,
    WRUSYS = 90,
    RDUSYS = 91,
    NEWPWD = 92,
    OPENCAP = 93,
    CLOSECAP = 94,
    MESSAGE = 95,
    SAVESCRN = 96,
    RESTSCRN = 97,
    SOUND = 98,
    CHAT = 99,
    SPRINT = 100,
    SPRINTLN = 101,
    MPRINT = 102,
    MPRINTLN = 103,
    RENAME = 104,
    FREWIND = 105,
    POKEDW = 106,
    DBGLEVEL = 107,
    SHOWON = 108,
    SHOWOFF = 109,
    PAGEON = 110,
    PAGEOFF = 111,
    FSEEK = 112,
    FFLUSH = 113,
    FREAD = 114,
    FWRITE = 115,
    FDEFIN = 116,
    FDEFOUT = 117,
    FDGET = 118,
    FDPUT = 119,
    FDPUTLN = 120,
    FDPUTPAD = 121,
    FDREAD = 122,
    FDWRITE = 123,
    ADJBYTES = 124,
    KBDSTRING = 125,
    ALIAS = 126,
    REDIM = 127,
    APPEND = 128,
    COPY = 129,
    KBDFLUSH = 130,
    MDMFLUSH = 131,
    KEYFLUSH = 132,
    LASTIN = 133,
    FLAG = 134,
    DOWNLOAD = 135,
    WRUSYSDOOR = 136,
    GETALTUSER = 137,
    ADJDBYTES = 138,
    ADJTBYTES = 139,
    ADJTFILES = 140,
    LANG = 141,
    SORT = 142,
    MOUSEREG = 143,
    SCRFILE = 144,
    SEARCHINIT = 145,
    SEARCHFIND = 146,
    SEARCHSTOP = 147,
    PRFOUND = 148,
    PRFOUNDLN = 149,
    TPAGET = 150,
    TPAPUT = 151,
    TPACGET = 152,
    TPACPUT = 153,
    TPAREAD = 154,
    TPAWRITE = 155,
    TPACREAD = 156,
    TPACWRITE = 157,
    BITSET = 158,
    BITCLEAR = 159,
    BRAG = 160,
    FREALTUSER = 161,
    SETLMR = 162,
    SETENV = 163,
    FCLOSEALL = 164,
    DECLARE = 165,
    FUNCTION = 166,
    PROCEDURE = 167,
    PCALL = 168,
    FPCLR = 169,
    BEGIN = 170,
    FEND = 171,
    STATIC = 172,
    STACKABORT = 173,
    DCREATE = 174,
    DOPEN = 175,
    DCLOSE = 176,
    DSETALIAS = 177,
    DPACK = 178,
    DCLOSEALL = 179,
    DLOCK = 180,
    DLOCKR = 181,
    DLOCKG = 182,
    DUNLOCK = 183,
    DNCREATE = 184,
    DNOPEN = 185,
    DNCLOSE = 186,
    DNCLOSEALL = 187,
    DNEW = 188,
    DADD = 189,
    DAPPEND = 190,
    DTOP = 191,
    DGO = 192,
    DBOTTOM = 193,
    DSKIP = 194,
    DBLANK = 195,
    DDELETE = 196,
    DRECALL = 197,
    DTAG = 198,
    DSEEK = 199,
    DFBLANK = 200,
    DGET = 201,
    DPUT = 202,
    DFCOPY = 203,

    EVAL = 204,
    ACCOUNT = 205,
    RECORDUSAGE = 206,
    MSGTOFILE = 207,
    QWKLIMITS = 208,
    COMMAND = 209,
    USELMRS = 210,
    CONFINFO = 211,
    ADJTUBYTES = 212,
    GRAFMODE = 213,
    ADDUSER = 214,
    KILLMSG = 215,
    CHDIR = 216,
    MKDIR = 217,
    RMDIR = 218,
    FDOWRAKA = 219,
    FDOADDAKA = 220,
    FDOWRORG = 221,
    FDOADDORG = 222,
    FDOQMOD = 223,
    FDOQADD = 224,
    FDOQDEL = 225,
    SOUNDDELAY = 226,
    ShortDesc = 227,
    MoveMsg = 228,
    SetBankBal = 229,
}
pub const LAST_STMT: i16 = OpCode::SetBankBal as i16;

impl OpCode {
    pub fn get_definition(self) -> &'static StatementDefinition {
        &STATEMENT_DEFINITIONS[self as usize]
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, PartialEq)]
pub enum ArgumentDefinitionFlags {
    None,

    /// GRAPH | SEC | LANG
    DisplayFileFlags,

    /// NC | FNS | FCL
    StartDisplayFlags,

    /// O_RD, O_WR, O_RW
    FileAccessMode,

    /// S_DN, S_DR, S_DW, S_DB  
    FileShareMode,

    /// ECHODOTS, FIELDLEN, GUIDE, UPCASE, STACKED, ERASELINE, NEWLINE, LFBEFORE, LFAFTER, WORDWRAP, NOCLEAR, HIGHASCII, AUTO, YESNO  
    InputFlags,

    /// NEWLINE, LFBEFORE, LFAFTER, BELL, LOGIT, LOGITLEFT
    DisplayTextFlags,

    /// SEEK_SET, SEEK_CUR, SEEK_END
    SeekPosition,

    /// NORMAL = 1, CRASH  = 2, HOLD   = 3
    FidoFlags,
}

#[derive(Debug, PartialEq)]
pub struct ArgumentDefinition {
    pub name: &'static str,
    pub arg_type: VariableType,
    pub flags: ArgumentDefinitionFlags,
}

impl ArgumentDefinition {
    pub fn new(name: &'static str, arg_type: VariableType) -> Self {
        Self {
            name,
            arg_type,
            flags: ArgumentDefinitionFlags::None,
        }
    }

    pub fn new_flags(name: &'static str, flags: ArgumentDefinitionFlags) -> Self {
        Self {
            name,
            arg_type: VariableType::Integer,
            flags,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct StatementDefinition {
    pub name: &'static str,
    pub version: u16,
    pub opcode: OpCode,
    pub sig: StatementSignature,
    pub args: Option<Vec<ArgumentDefinition>>,
}

impl StatementDefinition {
    pub(crate) fn get_statement_definition(identifier: &unicase::Ascii<String>) -> Option<&'static StatementDefinition> {
        STATEMENT_DEFINITIONS.iter().find(|def| unicase::Ascii::new(def.name) == identifier)
    }

    pub fn get_signature(&self) -> String {
        match self.opcode {
            OpCode::GOTO => "GOTO LABEL".to_string(),
            OpCode::LET => "LET var:multitype = EXP".to_string(),
            OpCode::IFNOT => "IF … THEN … ELSE …".to_string(),
            OpCode::REDIM => "REDIM var:MULTITYPE, dim1:INTEGER [,dim2:INTEGER [,dim3:INTEGER]] ".to_string(),

            _ => {
                let mut res = self.name.to_ascii_uppercase();
                if let Some(args) = &self.args {
                    res.push_str(" ");
                    res.push_str(&args.iter().map(|arg| format_argument(arg)).collect::<Vec<String>>().join(", "));

                    if matches!(self.sig, StatementSignature::VariableArguments(_)) {
                        res.push_str("[, ");
                        res.push_str(&format_argument(args.iter().last().unwrap()));
                        res.push_str("]*");
                    }
                }
                res
            }
        }
    }
}

fn format_argument(arg: &ArgumentDefinition) -> String {
    let ts = if arg.arg_type == VariableType::None {
        "multitype".to_string()
    } else {
        arg.arg_type.to_string()
    };
    format!("{} {}", ts, arg.name)
}

lazy_static::lazy_static! {
    // Missing:
    // "LAST IN" == "LASTIN"
    // "WAIT FOR" == "WAITFOR"
    // "GO SUB"
    // " GO TO"
    pub static ref STATEMENT_DEFINITIONS: [StatementDefinition; 234] = [
        StatementDefinition {
            // helps to map opcode to array index.
            name: "Placeholder",
            version: 100,
            opcode: OpCode::END,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "END",
            version: 100,
            opcode: OpCode::END,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Cls",
            version: 100,
            opcode: OpCode::CLS,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "ClrEOL",
            version: 100,
            opcode: OpCode::CLREOL,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "More",
            version: 100,
            opcode: OpCode::MORE,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Wait",
            version: 100,
            opcode: OpCode::WAIT,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Color",
            version: 100,
            opcode: OpCode::COLOR,
            args: Some(vec![
                ArgumentDefinition::new("fg", VariableType::Integer)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "GOTO",
            version: 100,
            opcode: OpCode::GOTO,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "LET",
            version: 100,
            opcode: OpCode::LET,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(1, 2),
        },
        StatementDefinition {
            name: "Print",
            version: 100,
            opcode: OpCode::PRINT,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "PrintLn",
            version: 100,
            opcode: OpCode::PRINTLN,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "IF",
            version: 100,
            opcode: OpCode::IFNOT,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "ConfFlag",
            version: 100,
            opcode: OpCode::CONFFLAG,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("flags", VariableType::Integer)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "ConfUnflag",
            version: 100,
            opcode: OpCode::CONFUNFLAG,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("flags", VariableType::Integer)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DispFile",
            version: 100,
            opcode: OpCode::DISPFILE,
            args: Some(vec![
                ArgumentDefinition::new("file", VariableType::Integer),
                ArgumentDefinition::new_flags("flag", ArgumentDefinitionFlags::DisplayFileFlags)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "Input",
            version: 100,
            opcode: OpCode::INPUT,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "FCreate",
            version: 100,
            opcode: OpCode::FCREATE,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("file", VariableType::String),
                ArgumentDefinition::new_flags("access", ArgumentDefinitionFlags::FileAccessMode),
                ArgumentDefinition::new_flags("shrmod", ArgumentDefinitionFlags::FileShareMode),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 4),
        },
        StatementDefinition {
            name: "FOpen",
            version: 100,
            opcode: OpCode::FOPEN,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("file", VariableType::String),
                ArgumentDefinition::new_flags("access", ArgumentDefinitionFlags::FileAccessMode),
                ArgumentDefinition::new_flags("shrmod", ArgumentDefinitionFlags::FileShareMode),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 4),
        },
        StatementDefinition {
            name: "FAppend",
            version: 100,
            opcode: OpCode::FAPPEND,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("file", VariableType::String),
                ArgumentDefinition::new_flags("access", ArgumentDefinitionFlags::FileAccessMode),
                ArgumentDefinition::new_flags("shrmod", ArgumentDefinitionFlags::FileShareMode),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 4),
        },
        StatementDefinition {
            name: "FClose",
            version: 100,
            opcode: OpCode::FCLOSE,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FGet",
            version: 100,
            opcode: OpCode::FGET,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "FPut",
            version: 100,
            opcode: OpCode::FPUT,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "FPutLn",
            version: 100,
            opcode: OpCode::FPUTLN,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "ResetDisp",
            version: 100,
            opcode: OpCode::RESETDISP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "StartDisp",
            version: 100,
            opcode: OpCode::STARTDISP,
            args: Some(vec![
                ArgumentDefinition::new_flags("str", ArgumentDefinitionFlags::StartDisplayFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FPutPad",
            version: 100,
            opcode: OpCode::FPUTPAD,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("str", VariableType::String),
                ArgumentDefinition::new("len", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "Hangup",
            version: 100,
            opcode: OpCode::HANGUP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "GetUser",
            version: 100,
            opcode: OpCode::GETUSER,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "PutUser",
            version: 100,
            opcode: OpCode::PUTUSER,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "DefColor",
            version: 100,
            opcode: OpCode::DEFCOLOR,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Delete",
            version: 100,
            opcode: OpCode::DELETE,
            args: Some(vec![
                ArgumentDefinition::new("file", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DelUser",
            version: 100,
            opcode: OpCode::DELUSER,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "AdjTime",
            version: 100,
            opcode: OpCode::ADJTIME,
            args: Some(vec![
                ArgumentDefinition::new("min", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Log",
            version: 100,
            opcode: OpCode::LOG,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
                ArgumentDefinition::new("just", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "InputStr",
            version: 100,
            opcode: OpCode::INPUTSTR,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
                ArgumentDefinition::new("len", VariableType::Integer),
                ArgumentDefinition::new("valid", VariableType::String),
                ArgumentDefinition::new_flags("flags", ArgumentDefinitionFlags::InputFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 6),
        },
        StatementDefinition {
            name: "InputYN",
            version: 100,
            opcode: OpCode::INPUTYN,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "InputMoney",
            version: 100,
            opcode: OpCode::INPUTMONEY,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "InputInt",
            version: 100,
            opcode: OpCode::INPUTINT,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "InputCC",
            version: 100,
            opcode: OpCode::INPUTCC,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "InputDate",
            version: 100,
            opcode: OpCode::INPUTDATE,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "InputTime",
            version: 100,
            opcode: OpCode::INPUTTIME,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "GOSUB",
            version: 100,
            opcode: OpCode::GOSUB,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "RETURN",
            version: 100,
            opcode: OpCode::RETURN,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "PromptStr",
            version: 100,
            opcode: OpCode::PROMPTSTR,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::Integer),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("len", VariableType::Integer),
                ArgumentDefinition::new("valid", VariableType::String),
                ArgumentDefinition::new_flags("flags", ArgumentDefinitionFlags::InputFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 5),
        },
        StatementDefinition {
            name: "DtrOn",
            version: 100,
            opcode: OpCode::DTRON,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "DtrOff",
            version: 100,
            opcode: OpCode::DTROFF,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "CdchkOn",
            version: 100,
            opcode: OpCode::CDCHKON,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "CdchkOff",
            version: 100,
            opcode: OpCode::CDCHKOFF,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Delay",
            version: 100,
            opcode: OpCode::DELAY,
            args: Some(vec![
                ArgumentDefinition::new("dlay", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "SendModem",
            version: 100,
            opcode: OpCode::SENDMODEM,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Inc",
            version: 100,
            opcode: OpCode::INC,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 1),
        },
        StatementDefinition {
            name: "Dec",
            version: 100,
            opcode: OpCode::DEC,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 1),
        },
        StatementDefinition {
            name: "NewLine",
            version: 100,
            opcode: OpCode::NEWLINE,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "NewLines",
            version: 100,
            opcode: OpCode::NEWLINES,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Tokenize",
            version: 100,
            opcode: OpCode::TOKENIZE,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "GetToken",
            version: 100,
            opcode: OpCode::GETTOKEN,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 1),
        },
        StatementDefinition {
            name: "Shell",
            version: 100,
            opcode: OpCode::SHELL,
            args: Some(vec![
                ArgumentDefinition::new("com", VariableType::Boolean),
                ArgumentDefinition::new("code", VariableType::Integer),
                ArgumentDefinition::new("prog", VariableType::String),
                ArgumentDefinition::new("arg", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 4),
        },
        StatementDefinition {
            name: "DispText",
            version: 100,
            opcode: OpCode::DISPTEXT,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::Integer),
                ArgumentDefinition::new_flags("flagson", ArgumentDefinitionFlags::DisplayTextFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "STOP",
            version: 100,
            opcode: OpCode::STOP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "InputText",
            version: 100,
            opcode: OpCode::INPUTTEXT,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::String),
                ArgumentDefinition::new("color", VariableType::Integer),
                ArgumentDefinition::new("len", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 4),
        },
        StatementDefinition {
            name: "Beep",
            version: 100,
            opcode: OpCode::BEEP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Push",
            version: 100,
            opcode: OpCode::PUSH,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "Pop",
            version: 100,
            opcode: OpCode::POP,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::SpecialCasePop,
        },
        StatementDefinition {
            name: "KbdStuff",
            version: 100,
            opcode: OpCode::KBDSTUFF,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Call",
            version: 100,
            opcode: OpCode::CALL,
            args: Some(vec![
                ArgumentDefinition::new("ppename", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Join",
            version: 100,
            opcode: OpCode::JOIN,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Quest",
            version: 100,
            opcode: OpCode::QUEST,
            args: Some(vec![
                ArgumentDefinition::new("nr", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Blt",
            version: 100,
            opcode: OpCode::BLT,
            args: Some(vec![
                ArgumentDefinition::new("nr", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Dir",
            version: 100,
            opcode: OpCode::DIR,
            args: Some(vec![
                ArgumentDefinition::new("arg", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "KbdFile",
            version: 100,
            opcode: OpCode::KBDFILE,
            args: Some(vec![
                ArgumentDefinition::new("file", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Bye",
            version: 100,
            opcode: OpCode::BYE,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Goodbye",
            version: 100,
            opcode: OpCode::GOODBYE,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Broadcast",
            version: 100,
            opcode: OpCode::BROADCAST,
            args: Some(vec![
                ArgumentDefinition::new("first", VariableType::Integer),
                ArgumentDefinition::new("last", VariableType::Integer),
                ArgumentDefinition::new("message", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "WaitFor",
            version: 100,
            opcode: OpCode::WAITFOR,
            args: Some(vec![
                ArgumentDefinition::new("prompt", VariableType::String),
                ArgumentDefinition::new("var", VariableType::Boolean),
                ArgumentDefinition::new("seconds", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "KbdchkOn",
            version: 100,
            opcode: OpCode::KBDCHKON,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "KbdchkOff",
            version: 100,
            opcode: OpCode::KBDCHKOFF,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "OpText",
            version: 100,
            opcode: OpCode::OPTEXT,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DispStr",
            version: 100,
            opcode: OpCode::DISPSTR,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "RDUnet",
            version: 100,
            opcode: OpCode::RDUNET,
            args: Some(vec![
                ArgumentDefinition::new("node", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "WRUnet",
            version: 100,
            opcode: OpCode::WRUNET,
            args: Some(vec![
                ArgumentDefinition::new("node", VariableType::Integer),
                ArgumentDefinition::new("stat", VariableType::String),
                ArgumentDefinition::new("username", VariableType::String),
                ArgumentDefinition::new("city", VariableType::String),
                ArgumentDefinition::new("optext", VariableType::String),
                ArgumentDefinition::new("broadcasttext", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 6),
        },
        StatementDefinition {
            name: "DoIntr",
            version: 100,
            opcode: OpCode::DOINTR,
            args: Some(vec![
                ArgumentDefinition::new("intr", VariableType::Integer),
                ArgumentDefinition::new("ax", VariableType::Integer),
                ArgumentDefinition::new("bx", VariableType::Integer),
                ArgumentDefinition::new("cx", VariableType::Integer),
                ArgumentDefinition::new("dx", VariableType::Integer),
                ArgumentDefinition::new("si", VariableType::Integer),
                ArgumentDefinition::new("di", VariableType::Integer),
                ArgumentDefinition::new("flags", VariableType::Integer),
                ArgumentDefinition::new("ds", VariableType::Integer),
                ArgumentDefinition::new("es", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 10),
        },
        StatementDefinition {
            name: "VarSeg",
            version: 100,
            opcode: OpCode::VARSEG,
            args: Some(vec![
                ArgumentDefinition::new("var1", VariableType::Integer),
                ArgumentDefinition::new("var2", VariableType::Integer),
            ]),
            sig: StatementSignature::SpecialCaseVarSeg,
        },
        StatementDefinition {
            name: "VarOff",
            version: 100,
            opcode: OpCode::VAROFF,
            args: Some(vec![
                ArgumentDefinition::new("var1", VariableType::Integer),
                ArgumentDefinition::new("var2", VariableType::Integer),
            ]),
            sig: StatementSignature::SpecialCaseVarSeg,
        },
        StatementDefinition {
            name: "PokeB",
            version: 100,
            opcode: OpCode::POKEB,
            args: Some(vec![
                ArgumentDefinition::new("addr", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Byte),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "PokeW",
            version: 100,
            opcode: OpCode::POKEW,
            args: Some(vec![
                ArgumentDefinition::new("addr", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Word),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "VarAddr",
            version: 100,
            opcode: OpCode::VARADDR,
            args: Some(vec![
                ArgumentDefinition::new("var1", VariableType::Integer),
                ArgumentDefinition::new("var2", VariableType::Integer),
            ]),
            sig: StatementSignature::SpecialCaseVarSeg,
        },
        StatementDefinition {
            name: "AnsiPos",
            version: 100,
            opcode: OpCode::ANSIPOS,
            args: Some(vec![
                ArgumentDefinition::new("col", VariableType::Integer),
                ArgumentDefinition::new("row", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "Backup",
            version: 100,
            opcode: OpCode::BACKUP,
            args: Some(vec![
                ArgumentDefinition::new("col", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Forward",
            version: 100,
            opcode: OpCode::FORWARD,
            args: Some(vec![
                ArgumentDefinition::new("col", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Freshline",
            version: 100,
            opcode: OpCode::FRESHLINE,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "WRUSys",
            version: 100,
            opcode: OpCode::WRUSYS,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "RDUSys",
            version: 100,
            opcode: OpCode::RDUSYS,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "NewPwd",
            version: 100,
            opcode: OpCode::NEWPWD,
            args: Some(vec![
                ArgumentDefinition::new("pw", VariableType::String),
                ArgumentDefinition::new("success", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "OpenCap",
            version: 100,
            opcode: OpCode::OPENCAP,
            args: Some(vec![
                ArgumentDefinition::new("captfile", VariableType::String),
                ArgumentDefinition::new("error", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "CloseCap",
            version: 100,
            opcode: OpCode::CLOSECAP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Message",
            version: 100,
            opcode: OpCode::MESSAGE,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("to", VariableType::String),
                ArgumentDefinition::new("from", VariableType::String),
                ArgumentDefinition::new("subject", VariableType::String),
                ArgumentDefinition::new("sec", VariableType::String),
                ArgumentDefinition::new("msgdate", VariableType::Date),
                ArgumentDefinition::new("retreceipt", VariableType::Boolean),
                ArgumentDefinition::new("echo", VariableType::Boolean),
                ArgumentDefinition::new("file", VariableType::String)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 9),
        },
        StatementDefinition {
            name: "SaveScrn",
            version: 100,
            opcode: OpCode::SAVESCRN,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "RestScrn",
            version: 100,
            opcode: OpCode::RESTSCRN,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Sound",
            version: 100,
            opcode: OpCode::SOUND,
            args: Some(vec![
                ArgumentDefinition::new("freq", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Chat",
            version: 100,
            opcode: OpCode::CHAT,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "SPrint",
            version: 100,
            opcode: OpCode::SPRINT,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "SPrintLN",
            version: 100,
            opcode: OpCode::SPRINTLN,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "MPrint",
            version: 100,
            opcode: OpCode::MPRINT,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "MPrintLn",
            version: 100,
            opcode: OpCode::MPRINTLN,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "Rename",
            version: 100,
            opcode: OpCode::RENAME,
            args: Some(vec![
                ArgumentDefinition::new("oldname", VariableType::String),
                ArgumentDefinition::new("newname", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "FRewind",
            version: 100,
            opcode: OpCode::FREWIND,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "PokeDW",
            version: 100,
            opcode: OpCode::POKEDW,
            args: Some(vec![
                ArgumentDefinition::new("addr", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DbgLevel",
            version: 100,
            opcode: OpCode::DBGLEVEL,
            args: Some(vec![
                ArgumentDefinition::new("level", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "ShowOn",
            version: 100,
            opcode: OpCode::SHOWON,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "ShowOff",
            version: 100,
            opcode: OpCode::SHOWOFF,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "PageOn",
            version: 100,
            opcode: OpCode::PAGEON,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "PageOFf",
            version: 100,
            opcode: OpCode::PAGEOFF,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "FSeek",
            version: 200,
            opcode: OpCode::FSEEK,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("byte", VariableType::Integer),
                ArgumentDefinition::new_flags("position", ArgumentDefinitionFlags::SeekPosition),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "FFlush",
            version: 200,
            opcode: OpCode::FFLUSH,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FRead",
            version: 200,
            opcode: OpCode::FREAD,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("size", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "FWrite",
            version: 200,
            opcode: OpCode::FWRITE,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("size", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "FDefIn",
            version: 200,
            opcode: OpCode::FDEFIN,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FDefOut",
            version: 200,
            opcode: OpCode::FDEFOUT,
            args: Some(vec![
                ArgumentDefinition::new("chnl", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FDGet",
            version: 200,
            opcode: OpCode::FDGET,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 1),
        },
        StatementDefinition {
            name: "FDPut",
            version: 200,
            opcode: OpCode::FDPUT,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "FDPutLn",
            version: 200,
            opcode: OpCode::FDPUTLN,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "FDPutPad",
            version: 200,
            opcode: OpCode::FDPUTPAD,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
                ArgumentDefinition::new("len", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "FDRead",
            version: 200,
            opcode: OpCode::FDREAD,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("size", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 2),
        },
        StatementDefinition {
            name: "FDWrite",
            version: 200,
            opcode: OpCode::FDWRITE,
            args: Some(vec![
                ArgumentDefinition::new("exp", VariableType::None),
                ArgumentDefinition::new("size", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "AdjBytes",
            version: 200,
            opcode: OpCode::ADJBYTES,
            args: Some(vec![
                ArgumentDefinition::new("bytes", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "KbdString",
            version: 200,
            opcode: OpCode::KBDSTRING,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Alias",
            version: 200,
            opcode: OpCode::ALIAS,
            args: Some(vec![
                ArgumentDefinition::new("on", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "ReDim",
            version: 200,
            opcode: OpCode::REDIM,
            args: None,
            sig: StatementSignature::VariableArguments(1),
        },
        StatementDefinition {
            name: "Append",
            version: 200,
            opcode: OpCode::APPEND,
            args: Some(vec![
                ArgumentDefinition::new("srcfile", VariableType::String),
                ArgumentDefinition::new("destfile", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "Copy",
            version: 200,
            opcode: OpCode::COPY,
            args: Some(vec![
                ArgumentDefinition::new("srcfile", VariableType::String),
                ArgumentDefinition::new("destfile", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "KbdFlush",
            version: 200,
            opcode: OpCode::KBDFLUSH,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "MdmFlush",
            version: 200,
            opcode: OpCode::MDMFLUSH,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "KeyFlush",
            version: 200,
            opcode: OpCode::KEYFLUSH,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "LastIn",
            version: 200,
            opcode: OpCode::LASTIN,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Flag",
            version: 200,
            opcode: OpCode::FLAG,
            args: Some(vec![
                ArgumentDefinition::new("filepath", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Download",
            version: 200,
            opcode: OpCode::DOWNLOAD,
            args: Some(vec![
                ArgumentDefinition::new("cmd", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "WRUsysDoor",
            version: 200,
            opcode: OpCode::WRUSYSDOOR,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "GetAltUser",
            version: 200,
            opcode: OpCode::GETALTUSER,
            args: Some(vec![
                ArgumentDefinition::new("user", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "AdjDBytes",
            version: 200,
            opcode: OpCode::ADJDBYTES,
            args: Some(vec![
                ArgumentDefinition::new("bytes", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "AdjTBytes",
            version: 200,
            opcode: OpCode::ADJTBYTES,
            args: Some(vec![
                ArgumentDefinition::new("bytes", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "AdjTFiles",
            version: 200,
            opcode: OpCode::ADJTFILES,
            args: Some(vec![
                ArgumentDefinition::new("files", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Lang",
            version: 200,
            opcode: OpCode::LANG,
            args: Some(vec![
                ArgumentDefinition::new("num", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Sort",
            version: 200,
            opcode: OpCode::SORT,
            args: Some(vec![
                ArgumentDefinition::new("sortArray", VariableType::None),
                ArgumentDefinition::new("pointerArray", VariableType::None),
            ]),
            sig: StatementSignature::SpecialCaseSort,
        },
        StatementDefinition {
            name: "MouseReg",
            version: 200,
            opcode: OpCode::MOUSEREG,
            args: Some(vec![
                ArgumentDefinition::new("num", VariableType::Integer),
                ArgumentDefinition::new("x1", VariableType::Integer),
                ArgumentDefinition::new("y1", VariableType::Integer),
                ArgumentDefinition::new("x2", VariableType::Integer),
                ArgumentDefinition::new("y2", VariableType::Integer),
                ArgumentDefinition::new("fontX", VariableType::Integer),
                ArgumentDefinition::new("fontY", VariableType::Integer),
                ArgumentDefinition::new("invert", VariableType::Boolean),
                ArgumentDefinition::new("clear", VariableType::Boolean),
                ArgumentDefinition::new("text", VariableType::String)
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 10),
        },
        StatementDefinition {
            name: "ScrFile",
            version: 200,
            opcode: OpCode::SCRFILE,
            args: Some(vec![
                ArgumentDefinition::new("line", VariableType::Integer),
                ArgumentDefinition::new("filename", VariableType::String),
            ]),
            sig: StatementSignature::SpecialCaseVarSeg,
        },
        StatementDefinition {
            name: "SearchInit",
            version: 200,
            opcode: OpCode::SEARCHINIT,
            args: Some(vec![
                ArgumentDefinition::new("line", VariableType::Integer),
                ArgumentDefinition::new("filename", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "SearchFind",
            version: 200,
            opcode: OpCode::SEARCHFIND,
            args: Some(vec![
                ArgumentDefinition::new("buffer", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "SearchStop",
            version: 200,
            opcode: OpCode::SEARCHSTOP,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "PrFound",
            version: 200,
            opcode: OpCode::PRFOUND,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "PrFoundLn",
            version: 200,
            opcode: OpCode::PRFOUNDLN,
            args: Some(vec![
                ArgumentDefinition::new("str", VariableType::String),
            ]),
            sig: StatementSignature::VariableArguments(0),
        },
        StatementDefinition {
            name: "TPAGet",
            version: 200,
            opcode: OpCode::TPAGET,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "TPAPut",
            version: 200,
            opcode: OpCode::TPAPUT,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("expr", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "TPACGet",
            version: 200,
            opcode: OpCode::TPACGET,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "TPACPut",
            version: 200,
            opcode: OpCode::TPACPUT,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("expr", VariableType::None),
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "TPARead",
            version: 200,
            opcode: OpCode::TPAREAD,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 2),
        },
        StatementDefinition {
            name: "TPAWrite",
            version: 200,
            opcode: OpCode::TPAWRITE,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("expr", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "TPACRead",
            version: 200,
            opcode: OpCode::TPACREAD,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(2, 3),
        },
        StatementDefinition {
            name: "TPACWrite",
            version: 200,
            opcode: OpCode::TPACWRITE,
            args: Some(vec![
                ArgumentDefinition::new("keyword", VariableType::String),
                ArgumentDefinition::new("expr", VariableType::None),
                ArgumentDefinition::new("conf", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "BitSet",
            version: 200,
            opcode: OpCode::BITSET,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("bit", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 2),
        },
        StatementDefinition {
            name: "BitClear",
            version: 200,
            opcode: OpCode::BITCLEAR,
            args: Some(vec![
                ArgumentDefinition::new("var", VariableType::None),
                ArgumentDefinition::new("bit", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(1, 2),
        },
        StatementDefinition {
            name: "Brag",
            version: 200,
            opcode: OpCode::BRAG,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "FRealTUser",
            version: 200,
            opcode: OpCode::FREALTUSER,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "SetLMR",
            version: 200,
            opcode: OpCode::SETLMR,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("msg", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "SetEnv",
            version: 200,
            opcode: OpCode::SETENV,
            args: Some(vec![
                ArgumentDefinition::new("envVar", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FCloseAll",
            version: 200,
            opcode: OpCode::FCLOSEALL,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Declare",
            version: 200,
            opcode: OpCode::DECLARE,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "Function",
            version: 200,
            opcode: OpCode::FUNCTION,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "Procedure",
            version: 200,
            opcode: OpCode::PROCEDURE,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "PCALL",
            version: 200,
            opcode: OpCode::PCALL,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "FPCLR",
            version: 200,
            opcode: OpCode::FPCLR,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Begin",
            version: 200,
            opcode: OpCode::BEGIN,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "FEND",
            version: 200,
            opcode: OpCode::FEND,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Static",
            version: 200,
            opcode: OpCode::STATIC,
            args: None,
            sig: StatementSignature::Invalid,
        },
        StatementDefinition {
            name: "StackAbort",
            version: 200,
            opcode: OpCode::STACKABORT,
            args: Some(vec![
                ArgumentDefinition::new("abort", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DCreate",
            version: 300,
            opcode: OpCode::DCREATE,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("exclusive", VariableType::Boolean),
                ArgumentDefinition::new("fieldInfo", VariableType::String)
            ]),
            sig: StatementSignature::SpecialCaseDcreate,
        },
        StatementDefinition {
            name: "DOpen",
            version: 300,
            opcode: OpCode::DOPEN,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("exclusive", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "DClose",
            version: 300,
            opcode: OpCode::DCLOSE,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DSetAlias",
            version: 300,
            opcode: OpCode::DSETALIAS,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("alias", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DPack",
            version: 300,
            opcode: OpCode::DPACK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DCloseAll",
            version: 300,
            opcode: OpCode::DCLOSEALL,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "DLock",
            version: 300,
            opcode: OpCode::DLOCK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DLockR",
            version: 300,
            opcode: OpCode::DLOCKR,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("recNo", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DLockG",
            version: 300,
            opcode: OpCode::DLOCKG,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("recNos", VariableType::Integer),
                ArgumentDefinition::new("count", VariableType::Integer),
            ]),
            sig: StatementSignature::SpecialCaseDlockg,
        },
        StatementDefinition {
            name: "DUnlock",
            version: 300,
            opcode: OpCode::DUNLOCK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DNCreate",
            version: 300,
            opcode: OpCode::DNCREATE,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("expression", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "DNOpen",
            version: 300,
            opcode: OpCode::DNOPEN,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DNClose",
            version: 300,
            opcode: OpCode::DNCLOSE,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DNCloseAll",
            version: 300,
            opcode: OpCode::DNCLOSEALL,
            args: Some(vec![
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DNew",
            version: 300,
            opcode: OpCode::DNEW,
            args: Some(vec![
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DAdd",
            version: 300,
            opcode: OpCode::DADD,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DAppend",
            version: 300,
            opcode: OpCode::DAPPEND,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DTop",
            version: 300,
            opcode: OpCode::DTOP,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DGo",
            version: 300,
            opcode: OpCode::DGO,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("recNo", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DBottom",
            version: 300,
            opcode: OpCode::DBOTTOM,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DSkip",
            version: 300,
            opcode: OpCode::DSKIP,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("number", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DBlank",
            version: 300,
            opcode: OpCode::DBLANK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DDelete",
            version: 300,
            opcode: OpCode::DDELETE,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DRecall",
            version: 300,
            opcode: OpCode::DRECALL,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "DTag",
            version: 300,
            opcode: OpCode::DTAG,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DSeek",
            version: 300,
            opcode: OpCode::DSEEK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("expr", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DFBlank",
            version: 300,
            opcode: OpCode::DFBLANK,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "DGet",
            version: 300,
            opcode: OpCode::DGET,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("var", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(3, 3),
        },
        StatementDefinition {
            name: "DPut",
            version: 300,
            opcode: OpCode::DPUT,
            args: Some(vec![
                ArgumentDefinition::new("channel", VariableType::Integer),
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("expression", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "DFCopy",
            version: 300,
            opcode: OpCode::DFCOPY,
            args: Some(vec![
                ArgumentDefinition::new("srcchannel", VariableType::Integer),
                ArgumentDefinition::new("srcname", VariableType::String),
                ArgumentDefinition::new("dstchannel", VariableType::Integer),
                ArgumentDefinition::new("dstname", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 4),
        },
        StatementDefinition {
            name: "Eval",
            version: 300,
            opcode: OpCode::EVAL,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Account",
            version: 300,
            opcode: OpCode::ACCOUNT,
            args: Some(vec![
                ArgumentDefinition::new("field", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "RecordUsage",
            version: 300,
            opcode: OpCode::RECORDUSAGE,
            args: Some(vec![
                ArgumentDefinition::new("field", VariableType::Integer),
                ArgumentDefinition::new("desc1", VariableType::String),
                ArgumentDefinition::new("desc2", VariableType::String),
                ArgumentDefinition::new("unitcost", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 5),
        },
        StatementDefinition {
            name: "MsgToFile",
            version: 300,
            opcode: OpCode::MSGTOFILE,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("msg_no", VariableType::Integer),
                ArgumentDefinition::new("filename", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "QwkLimits",
            version: 300,
            opcode: OpCode::QWKLIMITS,
            args: Some(vec![
                ArgumentDefinition::new("maxmsgs", VariableType::Integer),
                ArgumentDefinition::new("no", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "Command",
            version: 300,
            opcode: OpCode::COMMAND,
            args: Some(vec![
                ArgumentDefinition::new("viaCmdLst", VariableType::Boolean),
                ArgumentDefinition::new("cmd", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "UseLMRs",
            version: 300,
            opcode: OpCode::USELMRS,
            args: Some(vec![
                ArgumentDefinition::new("useLMR", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "ConfInfo",
            version: 300,
            opcode: OpCode::CONFINFO,
            args: Some(vec![
                ArgumentDefinition::new("confnum", VariableType::Integer),
                ArgumentDefinition::new("field", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::None),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "AdjTUBytes",
            version: 300,
            opcode: OpCode::ADJTUBYTES,
            args: Some(vec![
                ArgumentDefinition::new("bytes", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "GrafMode",
            version: 300,
            opcode: OpCode::GRAFMODE,
            args: Some(vec![
                ArgumentDefinition::new("mode", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "AddUser",
            version: 300,
            opcode: OpCode::ADDUSER,
            args: Some(vec![
                ArgumentDefinition::new("name", VariableType::String),
                ArgumentDefinition::new("uservars", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "KillMsg",
            version: 300,
            opcode: OpCode::KILLMSG,
            args: Some(vec![
                ArgumentDefinition::new("confnum", VariableType::Integer),
                ArgumentDefinition::new("msgnum", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "ChDir",
            version: 300,
            opcode: OpCode::CHDIR,
            args: Some(vec![
                ArgumentDefinition::new("path", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "MkDir",
            version: 300,
            opcode: OpCode::MKDIR,
            args: Some(vec![
                ArgumentDefinition::new("path", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "RmDir",
            version: 300,
            opcode: OpCode::RMDIR,
            args: Some(vec![
                ArgumentDefinition::new("path", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "FDOWRAka",
            version: 300,
            opcode: OpCode::FDOWRAKA,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "FDOADDAka",
            version: 300,
            opcode: OpCode::FDOADDAKA,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "FDOWROrg",
            version: 300,
            opcode: OpCode::FDOWRORG,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        StatementDefinition {
            name: "FDOADDOrg",
            version: 300,
            opcode: OpCode::FDOADDORG,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "FDOQMod",
            version: 300,
            opcode: OpCode::FDOQMOD,
            args: Some(vec![
                ArgumentDefinition::new("recnum", VariableType::Integer),
                ArgumentDefinition::new("addr", VariableType::String),
                ArgumentDefinition::new("file", VariableType::String),
                ArgumentDefinition::new_flags("flags", ArgumentDefinitionFlags::FidoFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 4),
        },
        StatementDefinition {
            name: "FDOQAdd",
            version: 300,
            opcode: OpCode::FDOQADD,
            args: Some(vec![
                ArgumentDefinition::new("addr", VariableType::String),
                ArgumentDefinition::new("file", VariableType::String),
                ArgumentDefinition::new_flags("flags", ArgumentDefinitionFlags::FidoFlags),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "FDOQDel",
            version: 300,
            opcode: OpCode::FDOQDEL,
            args: Some(vec![
                ArgumentDefinition::new("recnum", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "SoundDelay",
            version: 300,
            opcode: OpCode::SOUNDDELAY,
            args: Some(vec![
                ArgumentDefinition::new("freq", VariableType::Integer),
                ArgumentDefinition::new("duration", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },

        // 3.4 statements
        StatementDefinition {
            name: "ShortDesc",
            version: 340,
            opcode: OpCode::ShortDesc,
            args: Some(vec![
                ArgumentDefinition::new("val", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "MoveMsg",
            version: 340,
            opcode: OpCode::MoveMsg,
            args: Some(vec![
                ArgumentDefinition::new("conf", VariableType::Integer),
                ArgumentDefinition::new("message", VariableType::Integer),
                ArgumentDefinition::new("movetype", VariableType::Boolean),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 3),
        },
        StatementDefinition {
            name: "SetBankBal",
            version: 340,
            opcode: OpCode::SetBankBal,
            args: Some(vec![
                ArgumentDefinition::new("field", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Integer),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
        // Alias section
        // Moving to the end, so that the opcode <--> index mapping is not broken
        StatementDefinition {
            name: "DLockF",
            version: 300,
            opcode: OpCode::DLOCK,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "PutAltUser",
            version: 100,
            opcode: OpCode::PUTUSER,
            args: None,
            sig: StatementSignature::ArgumentsWithVariable(0, 0),
        },
        StatementDefinition {
            name: "Erase",
            version: 100,
            opcode: OpCode::DELETE,
            args: Some(vec![
                ArgumentDefinition::new("file", VariableType::String),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 1),
        },
        StatementDefinition {
            name: "Poke",
            version: 100,
            opcode: OpCode::POKEB,
            args: Some(vec![
                ArgumentDefinition::new("addr", VariableType::Integer),
                ArgumentDefinition::new("value", VariableType::Byte),
            ]),
            sig: StatementSignature::ArgumentsWithVariable(0, 2),
        },
    ];
}

#[test]
fn check_table_consistency() {
    for def in STATEMENT_DEFINITIONS.iter() {
        assert!(
            def.opcode == STATEMENT_DEFINITIONS[def.opcode as usize].opcode,
            "Opcode table mismatch: {:?}/{} was '{}':{:?}",
            def.opcode,
            def.opcode as usize,
            STATEMENT_DEFINITIONS[def.opcode as usize].name,
            STATEMENT_DEFINITIONS[def.opcode as usize].opcode
        );
    }
}
