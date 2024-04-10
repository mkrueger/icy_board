use std::fmt;

use crate::executable::{VariableData, VariableType, VariableValue};

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone)]
pub enum Constant {
    Money(i32),
    Integer(i32),
    Unsigned(u64),
    String(String),
    Double(f64),
    Boolean(bool),
    Builtin(&'static BuiltinConst),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuiltinConst {
    pub name: &'static str,
    pub value: i32,
    pub used_by: &'static [ConstantType],
}

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub enum ConstantType {
    General,
    Account,
    DispText,
    InputStr,
    PcbAccount,
    Crc,
    ConfFlag,
    StartDisp,
    DispFile,
    ScanMsgHdr,
    FileAccess,
    FileSeek,
    FileSec,
}

impl BuiltinConst {
    pub const TRUE: BuiltinConst = BuiltinConst {
        name: "TRUE",
        value: 0x01,
        used_by: &[ConstantType::General],
    };
    pub const FALSE: BuiltinConst = BuiltinConst {
        name: "FALSE",
        value: 0x00,
        used_by: &[ConstantType::General],
    };
}

pub const STACK_LIMIT: i32 = 6022 + 1024;
pub const BUILTIN_CONSTS: [BuiltinConst; 104] = [
    BuiltinConst {
        name: "TRUE",
        value: 0x01,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "FALSE",
        value: 0x00,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "STK_LIMIT",
        value: STACK_LIMIT,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "ATTACH_LIM_P",
        value: 0x03,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "ATTACH_LIM_U",
        value: 0x02,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "ACC_CUR_BAL",
        value: 0x04,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "F_NET",
        value: 0x20,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "CMAXMSGS",
        value: 0x01,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "MAXMSGS",
        value: 0x00,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "CUR_USER",
        value: 0,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "NO_USER",
        value: -1,
        used_by: &[ConstantType::General],
    },
    BuiltinConst {
        name: "ACC_STAT",
        value: 0x00,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "ACC_TIME",
        value: 0x01,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "ACC_MSGREAD",
        value: 0x02,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "ACC_MSGWRITE",
        value: 0x03,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEFS",
        value: 0x00,
        used_by: &[ConstantType::DispText, ConstantType::InputStr],
    },
    BuiltinConst {
        name: "BELL",
        value: 0x00800,
        used_by: &[ConstantType::DispText],
    },
    BuiltinConst {
        name: "LOGIT",
        value: 0x08000,
        used_by: &[ConstantType::DispText],
    },
    BuiltinConst {
        name: "LOGITLEFT",
        value: 0x10000,
        used_by: &[ConstantType::DispText],
    },
    BuiltinConst {
        name: "AUTO",
        value: 0x02000,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "ECHODOTS",
        value: 0x00001,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "ERASELINE",
        value: 0x00020,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "FIELDLEN",
        value: 0x00002,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "GUIDE",
        value: 0x00004,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "HIGHASCII",
        value: 0x01000,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "LFAFTER",
        value: 0x00100,
        used_by: &[ConstantType::InputStr, ConstantType::DispText],
    },
    BuiltinConst {
        name: "LFBEFORE",
        value: 0x00080,
        used_by: &[ConstantType::InputStr, ConstantType::DispText],
    },
    BuiltinConst {
        name: "NEWLINE",
        value: 0x00040,
        used_by: &[ConstantType::InputStr, ConstantType::DispText],
    },
    BuiltinConst {
        name: "NOCLEAR",
        value: 0x00400,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "STACKED",
        value: 0x00010,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "UPCASE",
        value: 0x00008,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "WORDWRAP",
        value: 0x00200,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "YESNO",
        value: 0x04000,
        used_by: &[ConstantType::InputStr],
    },
    BuiltinConst {
        name: "NEWBALANCE",
        value: 0x00,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_CALL",
        value: 0x01,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_TIME",
        value: 0x02,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_PEAKTIME",
        value: 0x03,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_CHAT",
        value: 0x04,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_MSGREAD",
        value: 0x05,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_MSGCAP",
        value: 0x06,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_MSGWRITE",
        value: 0x07,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_MSGECHOED",
        value: 0x08,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_MSGPRIVATE",
        value: 0x09,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_DOWNFILE",
        value: 0x0A,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CHRG_DOWNBYTES",
        value: 0x0B,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "PAY_UPFILE",
        value: 0x0C,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "PAY_UPBYTES",
        value: 0x0D,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "WARNLEVEL",
        value: 0x0E,
        used_by: &[ConstantType::PcbAccount],
    },
    BuiltinConst {
        name: "CRC_FILE",
        value: 0x01,
        used_by: &[ConstantType::Crc],
    },
    BuiltinConst {
        name: "CRC_STR",
        value: 0x00,
        used_by: &[ConstantType::Crc],
    },
    BuiltinConst {
        name: "START_BAL",
        value: 0x00,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "START_SESSION",
        value: 0x01,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_CALL",
        value: 0x02,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_TIME",
        value: 0x03,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_MSGREAD",
        value: 0x04,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_MSGCAP",
        value: 0x05,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_MSGWRITE",
        value: 0x06,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_MSGECHOED",
        value: 0x07,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_MSGPRIVATE",
        value: 0x08,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_DOWNFILE",
        value: 0x09,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_DOWNBYTES",
        value: 0x0A,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_CHAT",
        value: 0x0B,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_TPU",
        value: 0x0C,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "DEB_SPECIAL",
        value: 0x0D,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "CRED_UPFILE",
        value: 0x0E,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "CRED_UPBYTES",
        value: 0x0F,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "CRED_SPECIAL",
        value: 0x10,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "SEC_DROP",
        value: 0x11,
        used_by: &[ConstantType::Account],
    },
    BuiltinConst {
        name: "F_EXP",
        value: 0x02,
        used_by: &[ConstantType::ConfFlag],
    },
    BuiltinConst {
        name: "F_MW",
        value: 0x10,
        used_by: &[ConstantType::ConfFlag],
    },
    BuiltinConst {
        name: "F_REG",
        value: 0x01,
        used_by: &[ConstantType::ConfFlag],
    },
    BuiltinConst {
        name: "F_SEL",
        value: 0x04,
        used_by: &[ConstantType::ConfFlag],
    },
    BuiltinConst {
        name: "F_SYS",
        value: 0x08,
        used_by: &[ConstantType::ConfFlag],
    },
    BuiltinConst {
        name: "FCL",
        value: 0x02,
        used_by: &[ConstantType::StartDisp],
    },
    BuiltinConst {
        name: "FNS",
        value: 0x01,
        used_by: &[ConstantType::StartDisp],
    },
    BuiltinConst {
        name: "NC",
        value: 0x00,
        used_by: &[ConstantType::StartDisp],
    },
    BuiltinConst {
        name: "GRAPH",
        value: 0x01,
        used_by: &[ConstantType::DispFile],
    },
    BuiltinConst {
        name: "SEC",
        value: 0x02,
        used_by: &[ConstantType::DispFile],
    },
    BuiltinConst {
        name: "LANG",
        value: 0x04,
        used_by: &[ConstantType::DispFile],
    },
    BuiltinConst {
        name: "HDR_ACTIVE",
        value: 0x0E,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_BLOCKS",
        value: 0x04,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_DATE",
        value: 0x05,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_ECHO",
        value: 0x0F,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_FROM",
        value: 0x0B,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_MSGNUM",
        value: 0x02,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_MSGREF",
        value: 0x03,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_PWD",
        value: 0x0D,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_REPLY",
        value: 0x0A,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_RPLYDATE",
        value: 0x08,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_RPLYTIME",
        value: 0x09,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_STATUS",
        value: 0x01,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_SUBJ",
        value: 0x0C,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_TIME",
        value: 0x06,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "HDR_TO",
        value: 0x07,
        used_by: &[ConstantType::ScanMsgHdr],
    },
    BuiltinConst {
        name: "O_RD",
        value: 0x00,
        used_by: &[ConstantType::FileAccess],
    },
    BuiltinConst {
        name: "O_RW",
        value: 0x02,
        used_by: &[ConstantType::FileAccess],
    },
    BuiltinConst {
        name: "O_WR",
        value: 0x01,
        used_by: &[ConstantType::FileAccess],
    },
    BuiltinConst {
        name: "SEEK_CUR",
        value: 0x01,
        used_by: &[ConstantType::FileSeek],
    },
    BuiltinConst {
        name: "SEEK_END",
        value: 0x02,
        used_by: &[ConstantType::FileSeek],
    },
    BuiltinConst {
        name: "SEEK_SET",
        value: 0x00,
        used_by: &[ConstantType::FileSeek],
    },
    BuiltinConst {
        name: "S_DB",
        value: 0x03,
        used_by: &[ConstantType::FileSec],
    },
    BuiltinConst {
        name: "S_DN",
        value: 0x00,
        used_by: &[ConstantType::FileSec],
    },
    BuiltinConst {
        name: "S_DR",
        value: 0x01,
        used_by: &[ConstantType::FileSec],
    },
    BuiltinConst {
        name: "S_DW",
        value: 0x02,
        used_by: &[ConstantType::FileSec],
    },
];

impl Constant {
    pub fn get_var_type(&self) -> VariableType {
        match self {
            Constant::Money(_) => VariableType::Money,
            Constant::Unsigned(_) => VariableType::Unsigned,
            Constant::String(_) => VariableType::String,
            Constant::Double(_) => VariableType::Float,
            Constant::Boolean(_) => VariableType::Boolean,
            Constant::Integer(_) | Constant::Builtin(_) => VariableType::Integer,
        }
    }

    pub fn get_value(&self) -> VariableValue {
        let mut data = VariableData::default();
        match self {
            Constant::Money(i) => {
                data.money_value = *i;
                VariableValue::new(VariableType::Money, data)
            }
            Constant::Integer(i) => {
                data.int_value = *i;
                VariableValue::new(VariableType::Integer, data)
            }
            Constant::Unsigned(i) => {
                data.unsigned_value = *i;
                VariableValue::new(VariableType::Unsigned, data)
            }
            Constant::String(s) => VariableValue::new_string(s.clone()),
            Constant::Double(i) => {
                data.double_value = *i;
                VariableValue::new(VariableType::Double, data)
            }
            Constant::Boolean(b) => {
                data.bool_value = *b;
                VariableValue::new(VariableType::Boolean, data)
            }
            Constant::Builtin(s) => {
                data.int_value = s.value;
                VariableValue::new(VariableType::Integer, data)
            }
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Money(i) | Constant::Integer(i) => write!(f, "{i}"),
            Constant::Unsigned(i) => write!(f, "{i}"),
            Constant::String(str) => write!(f, "\"{str}\""),
            Constant::Double(i) => write!(f, "{i}"),
            Constant::Boolean(b) => write!(f, "{}", if *b { "1" } else { "0" }),
            Constant::Builtin(s) => write!(f, "{}", s.name),
        }
    }
}
