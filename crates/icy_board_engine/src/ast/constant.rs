use std::fmt;

use crate::executable::{Signature, VariableData, VariableType, VariableValue};

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
}

impl BuiltinConst {
    pub const TRUE: BuiltinConst = BuiltinConst { name: "TRUE", value: 0x01 };
    pub const FALSE: BuiltinConst = BuiltinConst { name: "FALSE", value: 0x00 };

    pub fn get_signature(&self) -> Signature {
        let mut res = self.name.to_ascii_uppercase();
        res.push_str(format!(" = {:X}h", self.value).as_str());
        Signature::new(res)
    }
}

pub const STACK_LIMIT: i32 = 6022 + 1024;
pub const BUILTIN_CONSTS: [BuiltinConst; 104] = [
    BuiltinConst { name: "TRUE", value: 0x01 },
    BuiltinConst { name: "FALSE", value: 0x00 },
    BuiltinConst {
        name: "STK_LIMIT",
        value: STACK_LIMIT,
    },
    BuiltinConst {
        name: "ATTACH_LIM_P",
        value: 0x03,
    },
    BuiltinConst {
        name: "ATTACH_LIM_U",
        value: 0x02,
    },
    BuiltinConst {
        name: "ACC_CUR_BAL",
        value: 0x04,
    },
    BuiltinConst { name: "F_NET", value: 0x20 },
    BuiltinConst { name: "CMAXMSGS", value: 0x01 },
    BuiltinConst { name: "MAXMSGS", value: 0x00 },
    BuiltinConst { name: "CUR_USER", value: 0 },
    BuiltinConst { name: "NO_USER", value: -1 },
    BuiltinConst { name: "ACC_STAT", value: 0x00 },
    BuiltinConst { name: "ACC_TIME", value: 0x01 },
    BuiltinConst {
        name: "ACC_MSGREAD",
        value: 0x02,
    },
    BuiltinConst {
        name: "ACC_MSGWRITE",
        value: 0x03,
    },
    BuiltinConst { name: "DEFS", value: 0x00 },
    BuiltinConst { name: "BELL", value: 0x00800 },
    BuiltinConst { name: "LOGIT", value: 0x08000 },
    BuiltinConst {
        name: "LOGITLEFT",
        value: 0x10000,
    },
    BuiltinConst { name: "AUTO", value: 0x02000 },
    BuiltinConst {
        name: "ECHODOTS",
        value: 0x00001,
    },
    BuiltinConst {
        name: "ERASELINE",
        value: 0x00020,
    },
    BuiltinConst {
        name: "FIELDLEN",
        value: 0x00002,
    },
    BuiltinConst { name: "GUIDE", value: 0x00004 },
    BuiltinConst {
        name: "HIGHASCII",
        value: 0x01000,
    },
    BuiltinConst {
        name: "LFAFTER",
        value: 0x00100,
    },
    BuiltinConst {
        name: "LFBEFORE",
        value: 0x00080,
    },
    BuiltinConst {
        name: "NEWLINE",
        value: 0x00040,
    },
    BuiltinConst {
        name: "NOCLEAR",
        value: 0x00400,
    },
    BuiltinConst {
        name: "STACKED",
        value: 0x00010,
    },
    BuiltinConst {
        name: "UPCASE",
        value: 0x00008,
    },
    BuiltinConst {
        name: "WORDWRAP",
        value: 0x00200,
    },
    BuiltinConst { name: "YESNO", value: 0x04000 },
    BuiltinConst {
        name: "NEWBALANCE",
        value: 0x00,
    },
    BuiltinConst {
        name: "CHRG_CALL",
        value: 0x01,
    },
    BuiltinConst {
        name: "CHRG_TIME",
        value: 0x02,
    },
    BuiltinConst {
        name: "CHRG_PEAKTIME",
        value: 0x03,
    },
    BuiltinConst {
        name: "CHRG_CHAT",
        value: 0x04,
    },
    BuiltinConst {
        name: "CHRG_MSGREAD",
        value: 0x05,
    },
    BuiltinConst {
        name: "CHRG_MSGCAP",
        value: 0x06,
    },
    BuiltinConst {
        name: "CHRG_MSGWRITE",
        value: 0x07,
    },
    BuiltinConst {
        name: "CHRG_MSGECHOED",
        value: 0x08,
    },
    BuiltinConst {
        name: "CHRG_MSGPRIVATE",
        value: 0x09,
    },
    BuiltinConst {
        name: "CHRG_DOWNFILE",
        value: 0x0A,
    },
    BuiltinConst {
        name: "CHRG_DOWNBYTES",
        value: 0x0B,
    },
    BuiltinConst {
        name: "PAY_UPFILE",
        value: 0x0C,
    },
    BuiltinConst {
        name: "PAY_UPBYTES",
        value: 0x0D,
    },
    BuiltinConst {
        name: "WARNLEVEL",
        value: 0x0E,
    },
    BuiltinConst { name: "CRC_FILE", value: 0x01 },
    BuiltinConst { name: "CRC_STR", value: 0x00 },
    BuiltinConst {
        name: "START_BAL",
        value: 0x00,
    },
    BuiltinConst {
        name: "START_SESSION",
        value: 0x01,
    },
    BuiltinConst { name: "DEB_CALL", value: 0x02 },
    BuiltinConst { name: "DEB_TIME", value: 0x03 },
    BuiltinConst {
        name: "DEB_MSGREAD",
        value: 0x04,
    },
    BuiltinConst {
        name: "DEB_MSGCAP",
        value: 0x05,
    },
    BuiltinConst {
        name: "DEB_MSGWRITE",
        value: 0x06,
    },
    BuiltinConst {
        name: "DEB_MSGECHOED",
        value: 0x07,
    },
    BuiltinConst {
        name: "DEB_MSGPRIVATE",
        value: 0x08,
    },
    BuiltinConst {
        name: "DEB_DOWNFILE",
        value: 0x09,
    },
    BuiltinConst {
        name: "DEB_DOWNBYTES",
        value: 0x0A,
    },
    BuiltinConst { name: "DEB_CHAT", value: 0x0B },
    BuiltinConst { name: "DEB_TPU", value: 0x0C },
    BuiltinConst {
        name: "DEB_SPECIAL",
        value: 0x0D,
    },
    BuiltinConst {
        name: "CRED_UPFILE",
        value: 0x0E,
    },
    BuiltinConst {
        name: "CRED_UPBYTES",
        value: 0x0F,
    },
    BuiltinConst {
        name: "CRED_SPECIAL",
        value: 0x10,
    },
    BuiltinConst { name: "SEC_DROP", value: 0x11 },
    BuiltinConst { name: "F_EXP", value: 0x02 },
    BuiltinConst { name: "F_MW", value: 0x10 },
    BuiltinConst { name: "F_REG", value: 0x01 },
    BuiltinConst { name: "F_SEL", value: 0x04 },
    BuiltinConst { name: "F_SYS", value: 0x08 },
    BuiltinConst { name: "FCL", value: 0x02 },
    BuiltinConst { name: "FNS", value: 0x01 },
    BuiltinConst { name: "NC", value: 0x00 },
    BuiltinConst { name: "GRAPH", value: 0x01 },
    BuiltinConst { name: "SEC", value: 0x02 },
    BuiltinConst { name: "LANG", value: 0x04 },
    BuiltinConst {
        name: "HDR_ACTIVE",
        value: 0x0E,
    },
    BuiltinConst {
        name: "HDR_BLOCKS",
        value: 0x04,
    },
    BuiltinConst { name: "HDR_DATE", value: 0x05 },
    BuiltinConst { name: "HDR_ECHO", value: 0x0F },
    BuiltinConst { name: "HDR_FROM", value: 0x0B },
    BuiltinConst {
        name: "HDR_MSGNUM",
        value: 0x02,
    },
    BuiltinConst {
        name: "HDR_MSGREF",
        value: 0x03,
    },
    BuiltinConst { name: "HDR_PWD", value: 0x0D },
    BuiltinConst {
        name: "HDR_REPLY",
        value: 0x0A,
    },
    BuiltinConst {
        name: "HDR_RPLYDATE",
        value: 0x08,
    },
    BuiltinConst {
        name: "HDR_RPLYTIME",
        value: 0x09,
    },
    BuiltinConst {
        name: "HDR_STATUS",
        value: 0x01,
    },
    BuiltinConst { name: "HDR_SUBJ", value: 0x0C },
    BuiltinConst { name: "HDR_TIME", value: 0x06 },
    BuiltinConst { name: "HDR_TO", value: 0x07 },
    BuiltinConst { name: "O_RD", value: 0x00 },
    BuiltinConst { name: "O_RW", value: 0x02 },
    BuiltinConst { name: "O_WR", value: 0x01 },
    BuiltinConst { name: "SEEK_CUR", value: 0x01 },
    BuiltinConst { name: "SEEK_END", value: 0x02 },
    BuiltinConst { name: "SEEK_SET", value: 0x00 },
    BuiltinConst { name: "S_DB", value: 0x03 },
    BuiltinConst { name: "S_DN", value: 0x00 },
    BuiltinConst { name: "S_DR", value: 0x01 },
    BuiltinConst { name: "S_DW", value: 0x02 },
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
            Constant::Boolean(b) => VariableValue::new_bool(*b),
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
/*
#[cfg(test)]
mod test {
    use super::BUILTIN_CONSTS;


    #[test]
    fn pr(){
        for c in BUILTIN_CONSTS.iter(){
            println!("{}", c.name);
        }
    }
}*/
