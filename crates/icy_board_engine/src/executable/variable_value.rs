use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

use crate::{
    Res,
    datetime::{IcbDate, IcbTime},
    executable::{FunctionValue, ProcedureValue, VMError},
    icy_board::user_base::Password,
};

use super::{MsgAreaIdValue, Signature};

#[derive(Clone, Copy, PartialEq, Debug, Default, Eq, Hash)]
#[allow(dead_code)]
pub enum VariableType {
    None,

    /// unsigned character (1 byte) 0 = FALSE, non-0 = TRUE
    Boolean,

    /// 4-byte unsigned integer Range: 0 - 4,294,967,295
    Unsigned,

    /// unsigned integer (2 bytes) `PCBoard` julian date (count of days since 1/1/1900)
    Date,

    /// Julian date in earth date format Deals with dates formatted YYMM.DD Range: Same as DATE
    EDate,

    /// signed long integer (4 bytes) Range: -2,147,483,648 → +2,147,483,647
    #[default]
    Integer,
    /// signed long integer (4 bytes) Range: -$21,474,836.48 → +$21,474,836.47
    Money,

    ///  4-byte floating point number Range: +/-3.4E-38 - +/-3.4E+38 (7-digit precision)
    Float,

    /// far character pointer (4 bytes) NULL is an empty string non-NULL points to a string of some length less than or equal to 256
    String,

    /// signed long integer (4 bytes) Count of seconds since midnight
    Time,

    /// 1-byte unsigned integer Range: 0 - 255
    Byte,

    /// 2-byte unsigned integer Range: 0 - 65,535
    Word,

    /// 1-byte signed Integer Range: -128 - 127
    SByte,

    /// 2-byte signed integer Range: -32,768 - 32,767
    SWord,

    /// Allows up to 2048 characters per big string (up from 256 for STRING variables) May include CHR(0) characters in the middle of the big string (unlike STRING variables which may not)
    BigStr,

    /// 8-byte floating point number Range: +/-1.7E-308 - +/-1.7E+308 (15-digit precision)
    Double,

    Function,

    Procedure,

    /// Signed long integer for julian date. DDATE is for use with `DBase` date fields.
    /// It holds a long integer for julian dates.
    /// When coerced to string type it is in the format CCYYMMDD or 19940527
    DDate,

    Table,

    MessageAreaID,

    Password,

    UserData(u8),
}

impl From<u8> for VariableType {
    fn from(b: u8) -> Self {
        VariableType::from_byte(b)
    }
}

impl From<VariableType> for u8 {
    fn from(b: VariableType) -> u8 {
        match b {
            VariableType::Boolean => 0,
            VariableType::Unsigned => 1,
            VariableType::Date => 2,
            VariableType::EDate => 3,
            VariableType::Integer => 4,
            VariableType::Money => 5,
            VariableType::Float => 6,
            VariableType::String => 7,
            VariableType::Time => 8,
            VariableType::Byte => 9,
            VariableType::Word => 10,
            VariableType::SByte => 11,
            VariableType::SWord => 12,
            VariableType::BigStr => 13,
            VariableType::Double => 14,
            VariableType::Function => 15,
            VariableType::Procedure => 16,
            VariableType::DDate => 17,
            VariableType::Table => 18,
            VariableType::MessageAreaID => 19,
            VariableType::Password => 20,
            VariableType::UserData(b) => b,
            VariableType::None => 255,
        }
    }
}

impl VariableType {
    pub fn create_empty_value(&self) -> VariableValue {
        match self {
            VariableType::String | VariableType::BigStr => VariableValue::new_string(String::new()),
            _ => VariableValue::new(*self, VariableData::default()),
        }
    }

    pub(crate) fn from_byte(b: u8) -> VariableType {
        match b {
            0 => VariableType::Boolean,
            1 => VariableType::Unsigned,
            2 => VariableType::Date,
            3 => VariableType::EDate,
            4 => VariableType::Integer,
            5 => VariableType::Money,
            6 => VariableType::Float,
            7 => VariableType::String,
            8 => VariableType::Time,
            9 => VariableType::Byte,
            10 => VariableType::Word,
            11 => VariableType::SByte,
            12 => VariableType::SWord,
            13 => VariableType::BigStr,
            14 => VariableType::Double,
            15 => VariableType::Function,
            16 => VariableType::Procedure,
            17 => VariableType::DDate,
            18 => VariableType::Table,
            19 => VariableType::MessageAreaID,
            20 => VariableType::Password,
            _ => VariableType::UserData(b),
        }
    }

    pub fn get_signature(&self) -> Signature {
        let sig = match self {
            VariableType::Boolean => "BOOLEAN".to_string(),
            VariableType::Unsigned => "UNSIGNED".to_string(),
            VariableType::Date => "DATE".to_string(),
            VariableType::EDate => "EDATE".to_string(),
            VariableType::Integer => "INTEGER / SDWORD / LONG".to_string(),
            VariableType::Money => "MONEY".to_string(),
            VariableType::Float => "REAL / FLOAT".to_string(),
            VariableType::String => "STRING".to_string(),
            VariableType::Time => "TIME".to_string(),
            VariableType::Byte => "BYTE / UBYTE".to_string(),
            VariableType::Word => "WORD / UWORD".to_string(),
            VariableType::SByte => "SBYTE / SHORT".to_string(),
            VariableType::SWord => "SWORD / INT".to_string(),
            VariableType::BigStr => "BIGSTR".to_string(),
            VariableType::Double => "DREAL / DOUBLE".to_string(),
            VariableType::Function => "FUNCTION".to_string(),
            VariableType::Procedure => "PROCEDURE".to_string(),
            VariableType::DDate => "DDATE".to_string(),
            VariableType::Table => "TABLE".to_string(),
            VariableType::MessageAreaID => "MSGAREAID".to_string(),
            VariableType::Password => "PASSWORD".to_string(),
            VariableType::UserData(u) => format!("USERDATA({u})"),
            VariableType::None => "NONE".to_string(),
        };
        Signature::new(sig)
    }
}

impl fmt::Display for VariableType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VariableType::None => write!(f, "None"),
            VariableType::Boolean => write!(f, "Boolean"),         // BOOL 0 = false, 1 = true
            VariableType::Unsigned => write!(f, "Unsigned"),       // u32
            VariableType::Date => write!(f, "Date"),               // 2*u8 - julian date
            VariableType::EDate => write!(f, "EDate"),             // 2*u8 - julian date
            VariableType::Integer => write!(f, "Integer"),         // i32
            VariableType::Money => write!(f, "Money"),             // i32 - x/100 Dollar x%100 Cents
            VariableType::Float => write!(f, "Real"),              // f32
            VariableType::String => write!(f, "String"),           // String without \0 and maximum length of 255 (Pascal like)
            VariableType::Time => write!(f, "Time"),               // u32 - Seconds elapsed since midnight
            VariableType::Byte => write!(f, "Byte"),               // u8
            VariableType::Word => write!(f, "Word"),               // u16
            VariableType::SByte => write!(f, "SByte"),             // i8
            VariableType::SWord => write!(f, "SWord"),             // i16
            VariableType::BigStr => write!(f, "BigStr"),           // String (max 2kb)
            VariableType::Double => write!(f, "Double"),           // f65
            VariableType::Function => write!(f, "FUNC"),           // 2*u8
            VariableType::Procedure => write!(f, "PROC"),          // 2*u8
            VariableType::DDate => write!(f, "DDate"),             // i32
            VariableType::Table => write!(f, "Table"),             // Generic key-value table
            VariableType::MessageAreaID => write!(f, "MsgAreaID"), // 2*u8
            VariableType::Password => write!(f, "Password"),       // Password type
            VariableType::UserData(u) => write!(f, "UserData({u})"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StdStruct {
    pub lo: u32,
    pub hi: u32,
}

#[derive(Clone, Copy)]
pub union VariableData {
    pub unsigned_value: u64,
    pub date_value: u32,
    pub ddate_value: i32,
    pub edate_value: u32,
    pub int_value: i32,
    pub money_value: i32,
    pub float_value: f32,
    pub double_value: f64,
    pub time_value: i32,
    pub byte_value: u8,
    pub word_value: u16,
    pub sword_value: i16,
    pub sbyte_value: i8,
    pub u64_value: u64,
    pub function_value: FunctionValue,
    pub procedure_value: ProcedureValue,
    pub message_id_value: MsgAreaIdValue,
    pub std_struct: StdStruct,
}
unsafe impl Send for VariableData {}
unsafe impl Sync for VariableData {}

impl VariableData {
    pub fn from_int(r: i32) -> VariableData {
        let mut res = VariableData::default();
        res.int_value = r;
        res
    }

    pub fn from_bool(b: bool) -> VariableData {
        let mut res = VariableData::default();
        res.unsigned_value = u64::from(b);
        res
    }

    pub fn from_float(d: f64) -> VariableData {
        let mut res = VariableData::default();
        res.double_value = d;
        res
    }
}

impl Default for VariableData {
    fn default() -> Self {
        unsafe { std::mem::zeroed::<VariableData>() }
    }
}

impl fmt::Debug for VariableData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", unsafe { self.unsigned_value })
    }
}

#[derive(Default, Clone)]
pub enum GenericVariableData {
    #[default]
    None,
    String(String),

    Dim1(Vec<VariableValue>),
    Dim2(Vec<Vec<VariableValue>>),
    Dim3(Vec<Vec<Vec<VariableValue>>>),

    Table(PPLTable),

    Password(crate::icy_board::user_base::Password),

    /// The fields of a value whose type the program declared with TYPE/ENDTYPE.
    Record(Vec<VariableValue>),

    /// The object a member expression reads, kept alive by the values that name it.
    UserData(std::sync::Arc<dyn crate::compiler::user_data::UserDataValue>),
}

impl fmt::Debug for GenericVariableData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericVariableData::None => write!(f, "None"),
            GenericVariableData::String(s) => write!(f, "String({s:?})"),
            GenericVariableData::Dim1(data) => write!(f, "Dim1({data:?})"),
            GenericVariableData::Dim2(data) => write!(f, "Dim2({data:?})"),
            GenericVariableData::Dim3(data) => write!(f, "Dim3({data:?})"),
            GenericVariableData::Table(table) => write!(f, "Table({table:?})"),
            // A secret has no business in a log line.
            GenericVariableData::Password(_) => write!(f, "Password(******)"),
            GenericVariableData::Record(fields) => write!(f, "Record({fields:?})"),
            GenericVariableData::UserData(_) => write!(f, "UserData"),
        }
    }
}
unsafe impl Send for GenericVariableData {}
unsafe impl Sync for GenericVariableData {}
const MAX_ARRAY_SIZE: usize = 100_000_000;

impl GenericVariableData {
    pub(crate) fn create_array(base_value: VariableValue, dim: u8, vector_size: usize, matrix_size: usize, cube_size: usize) -> Option<GenericVariableData> {
        match dim {
            1 => {
                if vector_size > MAX_ARRAY_SIZE {
                    log::error!("Creating a large array of size: {vector_size} elements - probably file is corrupt.");
                    return None;
                }
                Some(GenericVariableData::Dim1(vec![base_value; vector_size + 1]))
            }
            2 => {
                if vector_size * matrix_size > MAX_ARRAY_SIZE {
                    log::error!(
                        "Creating a large array of size: {}x{}={} elements - probably file is corrupt.",
                        vector_size,
                        matrix_size,
                        vector_size * matrix_size
                    );
                    return None;
                }
                Some(GenericVariableData::Dim2(vec![vec![base_value; matrix_size + 1]; vector_size + 1]))
            }
            3 => {
                if vector_size * matrix_size * cube_size > MAX_ARRAY_SIZE {
                    log::error!(
                        "Creating a large array of size: {}x{}x{}={} elements - probably file is corrupt.",
                        vector_size,
                        matrix_size,
                        cube_size,
                        vector_size * matrix_size * cube_size
                    );
                    return None;
                }
                Some(GenericVariableData::Dim3(vec![
                    vec![vec![base_value; cube_size + 1]; matrix_size + 1];
                    vector_size + 1
                ]))
            }
            _ => panic!("Invalid dimension: {dim}"),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct VariableValue {
    pub vtype: VariableType,
    pub data: VariableData,
    pub generic_data: GenericVariableData,
}

unsafe impl Send for VariableValue {}
unsafe impl Sync for VariableValue {}

impl fmt::Display for VariableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.vtype {
                VariableType::Boolean => write!(f, "{}", self.as_bool()),
                VariableType::Unsigned => write!(f, "{}", self.data.unsigned_value),
                VariableType::Date | VariableType::DDate | VariableType::EDate => {
                    write!(f, "{}", IcbDate::from_pcboard(self.data.date_value))
                }
                VariableType::Integer => write!(f, "{}", self.data.int_value),
                VariableType::Money => write!(f, "{}", self.data.money_value),
                VariableType::Float => write!(f, "{}", self.data.float_value),
                VariableType::Double => write!(f, "{}", self.data.double_value),
                VariableType::Time => write!(f, "{}", IcbTime::from_pcboard(self.data.time_value)),
                VariableType::Byte => write!(f, "{}", self.data.byte_value),
                VariableType::Word => write!(f, "{}", self.data.word_value),
                VariableType::SByte => write!(f, "{}", self.data.sbyte_value),
                VariableType::SWord => write!(f, "{}", self.data.sword_value),

                VariableType::String | VariableType::BigStr => {
                    if let GenericVariableData::String(s) = &self.generic_data {
                        write!(f, "{s}")
                    } else {
                        write!(f, "")
                    }
                }
                _ => {
                    write!(f, "")
                }
            }
        }
    }
}

impl PartialEq for VariableValue {
    fn eq(&self, other: &Self) -> bool {
        if let (VariableType::UserData(left_type), VariableType::UserData(right_type)) = (self.vtype, other.vtype) {
            if left_type != right_type {
                return false;
            }
            return match (&self.generic_data, &other.generic_data) {
                (GenericVariableData::Record(left), GenericVariableData::Record(right)) => left == right,
                _ => false,
            };
        }
        let dest_type: VariableType = if self.vtype == VariableType::Password || other.vtype == VariableType::Password {
            VariableType::Password
        } else {
            promote_to(self.vtype, other.vtype)
        };
        unsafe {
            match dest_type {
                VariableType::Boolean => self.as_bool() == other.as_bool(),
                VariableType::Unsigned => self.data.unsigned_value == other.data.unsigned_value,
                VariableType::Date => self.data.date_value == other.data.date_value,
                VariableType::DDate => self.data.ddate_value == other.data.ddate_value,
                VariableType::EDate => self.data.edate_value == other.data.edate_value,

                VariableType::Integer => self.as_int() == other.as_int(),
                VariableType::Money => self.data.money_value == other.data.money_value,
                VariableType::String | VariableType::BigStr => {
                    /*log::info!(
                        "Comparing strings: '{}'({}) == '{}'({}) -> {}",
                        self.as_string(),
                        self.as_string().len(),
                        other.as_string(),
                        other.as_string().len(),
                        self.as_string() == other.as_string()
                    );*/
                    self.as_string() == other.as_string()
                }

                VariableType::Time => self.data.time_value == other.data.time_value,
                VariableType::Float => self.as_float() == other.as_float(),
                VariableType::Double => self.as_double() == other.as_double(),
                VariableType::Byte | VariableType::SByte => self.as_byte() == other.as_byte(),
                VariableType::Word | VariableType::SWord => self.as_word() == other.as_word(),

                VariableType::Password => {
                    // Convert both sides to Password and compare
                    let left_pwd = if self.vtype == VariableType::Password {
                        if let GenericVariableData::Password(ref pwd) = self.generic_data {
                            pwd.clone()
                        } else {
                            Password::PlainText(self.as_string().to_lowercase())
                        }
                    } else {
                        Password::PlainText(self.as_string().to_lowercase())
                    };

                    let right_pwd = if other.vtype == VariableType::Password {
                        if let GenericVariableData::Password(ref pwd) = other.generic_data {
                            pwd.clone()
                        } else {
                            Password::PlainText(other.as_string().to_lowercase())
                        }
                    } else {
                        Password::PlainText(other.as_string().to_lowercase())
                    };

                    left_pwd == right_pwd
                }

                _ => false,
            }
        }
    }
}

fn promote_to(l: VariableType, r: VariableType) -> VariableType {
    if l == r {
        return l;
    }
    if l == VariableType::String || l == VariableType::BigStr || r == VariableType::String || r == VariableType::BigStr {
        return VariableType::BigStr;
    }
    if l == VariableType::Float || l == VariableType::Double || r == VariableType::Float || r == VariableType::Double {
        return VariableType::Double;
    }
    VariableType::Integer
}

impl Add<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn add(self, other: VariableValue) -> Self {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean | VariableType::Date | VariableType::EDate | VariableType::Money | VariableType::Time | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let mut generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self.data.unsigned_value.wrapping_add(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.as_int().wrapping_add(other.as_int());
                }
                VariableType::Float => {
                    data.float_value = self.as_float() + other.as_float();
                }
                VariableType::Double => {
                    data.double_value = self.as_double() + other.as_double();
                }

                VariableType::String | VariableType::BigStr => {
                    let mut new_string = self.as_string();
                    new_string.push_str(&other.as_string());
                    generic_data = GenericVariableData::String(new_string);
                }

                VariableType::Byte => {
                    data.byte_value = self.as_byte().wrapping_add(other.as_byte());
                }
                VariableType::SByte => {
                    data.sbyte_value = self.as_sbyte().wrapping_add(other.as_sbyte());
                }
                VariableType::Word => {
                    data.word_value = self.as_word().wrapping_add(other.as_word());
                }
                VariableType::SWord => {
                    data.sword_value = self.as_sword().wrapping_add(other.as_sword());
                }

                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

impl Sub<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn sub(self, other: VariableValue) -> VariableValue {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean | VariableType::Date | VariableType::EDate | VariableType::Money | VariableType::Time | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                let r = other.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_sub(r)),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self.data.unsigned_value.wrapping_sub(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.as_int().wrapping_sub(other.as_int());
                }
                VariableType::Float => {
                    data.float_value = self.as_float() - other.as_float();
                }
                VariableType::Double => {
                    data.double_value = self.as_double() - other.as_double();
                }
                VariableType::Byte => {
                    data.byte_value = self.as_byte().wrapping_sub(other.as_byte());
                }
                VariableType::SByte => {
                    data.sbyte_value = self.as_sbyte().wrapping_sub(other.as_sbyte());
                }
                VariableType::Word => {
                    data.word_value = self.as_word().wrapping_sub(other.as_word());
                }
                VariableType::SWord => {
                    data.sword_value = self.as_sword().wrapping_sub(other.as_sword());
                }
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

impl Mul<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn mul(self, other: VariableValue) -> VariableValue {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean | VariableType::Date | VariableType::EDate | VariableType::Money | VariableType::Time | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_int();
                let r = other.as_int();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_mul(r)),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self.data.unsigned_value.wrapping_mul(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.as_int().wrapping_mul(other.as_int());
                }
                VariableType::Float => {
                    data.float_value = self.as_float() * other.as_float();
                }
                VariableType::Double => {
                    data.double_value = self.as_double() * other.as_double();
                }
                VariableType::Byte => {
                    data.byte_value = self.as_byte().wrapping_mul(other.as_byte());
                }
                VariableType::SByte => {
                    data.sbyte_value = self.as_sbyte().wrapping_mul(other.as_sbyte());
                }
                VariableType::Word => {
                    data.word_value = self.as_word().wrapping_mul(other.as_word());
                }
                VariableType::SWord => {
                    data.sword_value = self.as_sword().wrapping_mul(other.as_sword());
                }
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

impl Div<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn div(self, other: VariableValue) -> VariableValue {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);

        match dest_type {
            VariableType::Boolean | VariableType::Date | VariableType::EDate | VariableType::Money | VariableType::Time | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                let r = other.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_div(r)),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self.data.unsigned_value.wrapping_div(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.as_int().wrapping_div(other.as_int());
                }
                VariableType::Float => {
                    data.float_value = self.convert_to(VariableType::Float).data.float_value / other.convert_to(VariableType::Float).data.float_value;
                }
                VariableType::Double => {
                    data.double_value = self.as_double() / other.as_double();
                }
                VariableType::Byte => {
                    data.byte_value = self.as_byte().wrapping_div(other.as_byte());
                }
                VariableType::SByte => {
                    data.sbyte_value = self.as_sbyte().wrapping_div(other.as_sbyte());
                }
                VariableType::Word => {
                    data.word_value = self.as_word().wrapping_div(other.as_word());
                }
                VariableType::SWord => {
                    data.sword_value = self.as_sword().wrapping_div(other.as_sword());
                }
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

impl Rem<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn rem(self, other: VariableValue) -> VariableValue {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::DDate
            | VariableType::Float
            | VariableType::Double => {
                dest_type = VariableType::Integer;
            }

            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                let r = other.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_rem(r)),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self.data.unsigned_value.wrapping_rem(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.as_int().wrapping_rem(other.as_int());
                }
                VariableType::Byte => {
                    data.byte_value = self.as_byte().wrapping_rem(other.as_byte());
                }
                VariableType::SByte => {
                    data.sbyte_value = self.as_sbyte().wrapping_rem(other.as_sbyte());
                }
                VariableType::Word => {
                    data.word_value = self.as_word().wrapping_rem(other.as_word());
                }
                VariableType::SWord => {
                    data.sword_value = self.as_sword().wrapping_rem(other.as_sword());
                }
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

impl PartialOrd for VariableValue {
    fn partial_cmp(&self, other: &VariableValue) -> Option<Ordering> {
        let dest_type: VariableType = if self.vtype == VariableType::Password || other.vtype == VariableType::Password {
            VariableType::Password
        } else {
            promote_to(self.vtype, other.vtype)
        };
        unsafe {
            match dest_type {
                VariableType::Boolean => Some(self.as_bool().cmp(&other.as_bool())),
                VariableType::Unsigned => Some(self.data.unsigned_value.cmp(&other.data.unsigned_value)),
                VariableType::Date => Some(self.data.date_value.cmp(&other.data.date_value)),
                VariableType::DDate => Some(self.data.ddate_value.cmp(&other.data.ddate_value)),
                VariableType::EDate => Some(self.data.edate_value.cmp(&other.data.edate_value)),

                VariableType::Integer => Some(self.as_int().cmp(&other.as_int())),
                VariableType::Money => Some(self.data.money_value.cmp(&other.data.money_value)),
                VariableType::String | VariableType::BigStr => Some(self.as_string().cmp(&other.as_string())),

                VariableType::Time => Some(self.data.time_value.cmp(&other.data.time_value)),
                VariableType::Float => self.as_float().partial_cmp(&other.as_float()),
                VariableType::Double => self.as_double().partial_cmp(&other.as_double()),
                VariableType::Byte | VariableType::SByte => Some(self.as_byte().cmp(&other.as_byte())),
                VariableType::Word | VariableType::SWord => Some(self.as_word().cmp(&other.as_word())),

                VariableType::Password => {
                    // Passwords can only be equal or not equal, no ordering
                    // Use the same comparison logic as PartialEq
                    let left_pwd = if self.vtype == VariableType::Password {
                        if let GenericVariableData::Password(ref pwd) = self.generic_data {
                            pwd.clone()
                        } else {
                            Password::PlainText(self.as_string().to_lowercase())
                        }
                    } else {
                        Password::PlainText(self.as_string().to_lowercase())
                    };

                    let right_pwd = if other.vtype == VariableType::Password {
                        if let GenericVariableData::Password(ref pwd) = other.generic_data {
                            pwd.clone()
                        } else {
                            Password::PlainText(other.as_string().to_lowercase())
                        }
                    } else {
                        Password::PlainText(other.as_string().to_lowercase())
                    };

                    if left_pwd == right_pwd {
                        Some(Ordering::Equal)
                    } else {
                        None // Incomparable - passwords have no ordering beyond equality
                    }
                }

                _ => None,
            }
        }
    }
}

impl Neg for VariableValue {
    type Output = VariableValue;

    fn neg(self) -> VariableValue {
        let mut dest_type = self.vtype;
        match dest_type {
            VariableType::Unsigned
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::Byte
            | VariableType::Word
            | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(-l),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        match dest_type {
            VariableType::Boolean => data.unsigned_value = unsafe { u64::from(self.data.unsigned_value == 0) },
            VariableType::Integer => data.int_value = -self.as_int(),
            VariableType::SByte => data.sbyte_value = -self.as_sbyte(),
            VariableType::SWord => data.sword_value = -self.as_sword(),
            VariableType::Float => data.float_value = -self.as_float(),
            VariableType::Double => data.double_value = -self.as_double(),
            _ => {
                panic!("unsupported lvalue for add {self:?}");
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
impl VariableValue {
    /// The same value emptied out, keeping the fields a record is made of.
    #[must_use]
    pub fn emptied(&self) -> VariableValue {
        match &self.generic_data {
            GenericVariableData::Record(fields) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Record(fields.iter().map(VariableValue::emptied).collect()),
            },
            GenericVariableData::Dim1(values) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Dim1(values.iter().map(VariableValue::emptied).collect()),
            },
            GenericVariableData::Dim2(values) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Dim2(values.iter().map(|row| row.iter().map(VariableValue::emptied).collect()).collect()),
            },
            GenericVariableData::Dim3(values) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Dim3(
                    values
                        .iter()
                        .map(|plane| plane.iter().map(|row| row.iter().map(VariableValue::emptied).collect()).collect())
                        .collect(),
                ),
            },
            _ => self.vtype.create_empty_value(),
        }
    }

    pub fn new(vtype: VariableType, data: VariableData) -> Self {
        Self {
            vtype,
            data,
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_string(s: String) -> Self {
        Self {
            vtype: VariableType::String,
            data: VariableData::default(),
            generic_data: GenericVariableData::String(s),
        }
    }

    pub fn new_int(i: i32) -> Self {
        Self {
            vtype: VariableType::Integer,
            data: VariableData::from_int(i),
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_word(i: u16) -> Self {
        let mut data = VariableData::default();
        data.word_value = i;
        Self {
            vtype: VariableType::Word,
            data,
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_byte(i: u8) -> Self {
        let mut data = VariableData::default();
        data.byte_value = i;
        Self {
            vtype: VariableType::Byte,
            data,
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_bool(b: bool) -> Self {
        Self {
            vtype: VariableType::Boolean,
            data: VariableData::from_bool(b),
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_double(d: f64) -> Self {
        Self {
            vtype: VariableType::Double,
            data: VariableData { double_value: d },
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_unsigned(d: u64) -> Self {
        Self {
            vtype: VariableType::Unsigned,
            data: VariableData { unsigned_value: d },
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_msg_id(conference: i32, area: i32) -> Self {
        Self {
            vtype: VariableType::MessageAreaID,
            data: VariableData {
                message_id_value: MsgAreaIdValue { conference, area },
            },
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_vector(variable_type: VariableType, vec: Vec<VariableValue>) -> Self {
        Self {
            vtype: variable_type,
            data: VariableData::default(),
            generic_data: GenericVariableData::Dim1(vec),
        }
    }

    pub fn new_matrix(variable_type: VariableType, vec: Vec<Vec<VariableValue>>) -> Self {
        Self {
            vtype: variable_type,
            data: VariableData::default(),
            generic_data: GenericVariableData::Dim2(vec),
        }
    }

    pub fn new_cube(variable_type: VariableType, vec: Vec<Vec<Vec<VariableValue>>>) -> Self {
        Self {
            vtype: variable_type,
            data: VariableData::default(),
            generic_data: GenericVariableData::Dim3(vec),
        }
    }

    pub fn get_type(&self) -> VariableType {
        self.vtype
    }

    pub fn get_dimensions(&self) -> u8 {
        match self.generic_data {
            GenericVariableData::Dim1(_) => 1,
            GenericVariableData::Dim2(_) => 2,
            GenericVariableData::Dim3(_) => 3,
            _ => 0,
        }
    }

    pub fn get_vector_size(&self) -> usize {
        match &self.generic_data {
            GenericVariableData::Dim1(data) => data.len() - 1,
            GenericVariableData::Dim2(data) => data.len() - 1,
            GenericVariableData::Dim3(data) => data.len() - 1,
            _ => 0,
        }
    }

    pub fn get_matrix_size(&self) -> usize {
        match &self.generic_data {
            GenericVariableData::Dim2(data) => data[0].len() - 1,
            GenericVariableData::Dim3(data) => data[0].len() - 1,
            _ => 0,
        }
    }

    pub fn get_cube_size(&self) -> usize {
        match &self.generic_data {
            GenericVariableData::Dim3(data) => data[0][0].len() - 1,
            _ => 0,
        }
    }

    pub fn get_u64_value(&self) -> u64 {
        unsafe { self.data.u64_value }
    }

    /// .
    ///
    ///
    /// # Panics
    ///
    /// Panics if .
    #[must_use]
    pub fn pow(&self, other: VariableValue) -> VariableValue {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean
            | VariableType::Unsigned
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::Byte
            | VariableType::Word
            | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                let r = other.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_pow(r as u32)),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_pow(other.data.int_value as u32);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value.powf(other.data.float_value);
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value.powf(other.data.double_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_pow(other.data.sbyte_value as u32);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_pow(other.data.sword_value as u32);
                }
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }

    /// .
    ///
    ///
    /// # Panics
    ///
    /// Panics if .
    #[must_use]
    pub fn not(&self) -> VariableValue {
        Self {
            vtype: VariableType::Boolean,
            data: VariableData::from_bool(!self.as_bool()),
            generic_data: GenericVariableData::None,
        }
    }

    /// .
    ///
    ///
    /// # Panics
    ///
    /// Panics if .
    #[must_use]
    pub fn abs(&self) -> VariableValue {
        let mut dest_type: VariableType = self.vtype;
        match dest_type {
            VariableType::Boolean
            | VariableType::Unsigned
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::Byte
            | VariableType::Word
            | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.abs()),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Integer => data.int_value = self.data.int_value.abs(),
                VariableType::Float => data.float_value = self.data.float_value.abs(),
                VariableType::Double => data.double_value = self.data.double_value.abs(),
                VariableType::SByte => data.sbyte_value = self.data.sbyte_value.abs(),
                VariableType::SWord => data.sword_value = self.data.sword_value.abs(),
                _ => {
                    panic!("unsupported lvalue for add {self:?}");
                }
            }
        }
        Self {
            vtype: dest_type,
            data,
            generic_data,
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_bool(&self) -> bool {
        if self.vtype == VariableType::String || self.vtype == VariableType::BigStr {
            return self.as_int() != 0;
        }
        unsafe { self.data.unsigned_value != 0 }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_int(&self) -> i32 {
        if let GenericVariableData::String(s) = &self.generic_data {
            // PCBoard converts strings with atol(): skip leading whitespace, take an
            // optional sign, then digits up to the first non-digit. Verified against
            // PCBoard 15.4: "-5x" == -5, "12abc" == 12, " 7" == 7, "" == 0.
            let mut chars = s.chars().skip_while(|c| c.is_whitespace()).peekable();
            let negative = match chars.peek() {
                Some('-') => {
                    chars.next();
                    true
                }
                Some('+') => {
                    chars.next();
                    false
                }
                _ => false,
            };
            let mut res: i32 = 0;
            for c in chars {
                let Some(digit) = c.to_digit(10) else {
                    break;
                };
                res = res.wrapping_mul(10).wrapping_add(digit as i32);
            }
            return if negative { res.wrapping_neg() } else { res };
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1;
                }
                0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value as i32 },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer => unsafe { self.data.int_value },
            VariableType::Money => unsafe { self.data.money_value },
            VariableType::Float => unsafe { self.data.float_value as i32 },
            VariableType::Double => unsafe { self.data.double_value as i32 },
            VariableType::Time => unsafe { self.data.time_value },
            VariableType::Byte => unsafe { self.data.byte_value as i32 },
            VariableType::Word => unsafe { self.data.word_value as i32 },
            VariableType::SByte => unsafe { self.data.sbyte_value as i32 },
            VariableType::SWord => unsafe { self.data.sword_value as i32 },
            VariableType::MessageAreaID => unsafe { self.data.message_id_value.conference },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_unsigned(&self) -> u64 {
        if let GenericVariableData::String(s) = &self.generic_data {
            let mut res = 0;
            for c in s.chars() {
                if c.is_ascii_digit() {
                    if let Some(c) = c.to_digit(10) {
                        res = res * 10 + c as u64;
                    } else {
                        break;
                    }
                }
            }
            return res;
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1;
                }
                0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer => unsafe { self.data.int_value as u64 },
            VariableType::Money => unsafe { self.data.money_value as u64 },
            VariableType::Float => unsafe { self.data.float_value as u64 },
            VariableType::Double => unsafe { self.data.double_value as u64 },
            VariableType::Time => unsafe { self.data.time_value as u64 },
            VariableType::Byte => unsafe { self.data.byte_value as u64 },
            VariableType::Word => unsafe { self.data.word_value as u64 },
            VariableType::SByte => unsafe { self.data.sbyte_value as u64 },
            VariableType::SWord => unsafe { self.data.sword_value as u64 },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_double(&self) -> f64 {
        if let GenericVariableData::String(s) = &self.generic_data {
            return s.parse::<f64>().unwrap_or_default();
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1.0;
                }
                0.0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value as f64 },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer => unsafe { self.data.int_value as f64 },
            VariableType::Money => unsafe { self.data.money_value as f64 },
            VariableType::Float => unsafe { self.data.float_value as f64 },
            VariableType::Double => unsafe { self.data.double_value },
            VariableType::Time => unsafe { self.data.time_value as f64 },
            VariableType::Byte => unsafe { self.data.byte_value as f64 },
            VariableType::Word => unsafe { self.data.word_value as f64 },
            VariableType::SByte => unsafe { self.data.sbyte_value as f64 },
            VariableType::SWord => unsafe { self.data.sword_value as f64 },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_float(&self) -> f32 {
        if let GenericVariableData::String(s) = &self.generic_data {
            return s.parse::<f32>().unwrap_or_default();
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1.0;
                }
                0.0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value as f32 },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer => unsafe { self.data.int_value as f32 },
            VariableType::Money => unsafe { self.data.money_value as f32 },
            VariableType::Float => unsafe { self.data.float_value },
            VariableType::Double => unsafe { self.data.double_value as f32 },
            VariableType::Time => unsafe { self.data.time_value as f32 },
            VariableType::Byte => unsafe { self.data.byte_value as f32 },
            VariableType::Word => unsafe { self.data.word_value as f32 },
            VariableType::SByte => unsafe { self.data.sbyte_value as f32 },
            VariableType::SWord => unsafe { self.data.sword_value as f32 },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_byte(&self) -> u8 {
        if let GenericVariableData::String(s) = &self.generic_data {
            return s.parse::<u8>().unwrap_or_default();
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1;
                }
                0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value as u8 },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer => unsafe { self.data.int_value as u8 },
            VariableType::Money => unsafe { self.data.money_value as u8 },
            VariableType::Float => unsafe { self.data.float_value as u8 },
            VariableType::Double => unsafe { self.data.double_value as u8 },
            VariableType::Time => unsafe { self.data.time_value as u8 },
            VariableType::Byte => unsafe { self.data.byte_value },
            VariableType::Word => unsafe { self.data.word_value as u8 },
            VariableType::SByte => unsafe { self.data.sbyte_value as u8 },
            VariableType::SWord => unsafe { self.data.sword_value as u8 },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_sbyte(&self) -> i8 {
        self.as_byte() as i8
    }

    pub fn as_word(&self) -> u16 {
        if let GenericVariableData::String(s) = &self.generic_data {
            return s.parse::<u16>().unwrap_or_default();
        }

        match self.vtype {
            VariableType::Boolean => {
                if self.as_bool() {
                    return 1;
                }
                0
            }
            VariableType::Unsigned => unsafe { self.data.unsigned_value as u16 },
            VariableType::Date | VariableType::DDate | VariableType::EDate | VariableType::Integer | VariableType::Word => unsafe { self.data.word_value },
            VariableType::Money => unsafe { self.data.money_value as u16 },
            VariableType::Float => unsafe { self.data.float_value as u16 },
            VariableType::Double => unsafe { self.data.double_value as u16 },
            VariableType::Time => unsafe { self.data.time_value as u16 },
            VariableType::Byte => unsafe { self.data.byte_value as u16 },
            VariableType::SByte => unsafe { self.data.sbyte_value as u16 },
            VariableType::SWord => unsafe { self.data.sword_value as u16 },
            _ => {
                panic!("Unsupported type: {:?}", self.vtype);
            }
        }
    }

    pub fn as_sword(&self) -> i16 {
        self.as_word() as i16
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_string(&self) -> String {
        unsafe {
            match &self.generic_data {
                GenericVariableData::String(s) => s.clone(),
                GenericVariableData::Password(p) => match p {
                    Password::PlainText(s) => s.clone(),
                    _ => "******".to_string(),
                },
                _ => match self.vtype {
                    VariableType::Boolean => {
                        if self.as_bool() {
                            "1".to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    VariableType::Unsigned => self.data.unsigned_value.to_string(),
                    VariableType::Date => pcb_date_string(self.data.date_value),
                    VariableType::EDate => pcb_edate_string(self.data.edate_value),
                    VariableType::DDate => pcb_ddate_string(self.data.ddate_value),
                    VariableType::Integer => self.data.int_value.to_string(),
                    VariableType::Money => pcb_money_string(self.data.money_value),
                    VariableType::Float => self.data.float_value.to_string(),
                    VariableType::Double => self.data.double_value.to_string(),
                    VariableType::Time => {
                        format!("{}", IcbTime::from_pcboard(self.data.time_value))
                    }
                    VariableType::Byte => self.data.byte_value.to_string(),
                    VariableType::Word => self.data.word_value.to_string(),
                    VariableType::SByte => self.data.sbyte_value.to_string(),
                    VariableType::SWord => self.data.sword_value.to_string(),

                    _ => String::new(),
                },
            }
        }
    }

    pub fn as_date(&self) -> IcbDate {
        unsafe { IcbDate::from_pcboard(self.data.date_value) }
    }

    pub fn as_time(&self) -> IcbTime {
        unsafe { IcbTime::from_pcboard(self.data.time_value) }
    }

    /// Returns (conference, area) for a message id
    pub fn as_msg_id(&self) -> (i32, i32) {
        match self.vtype {
            VariableType::MessageAreaID => unsafe { (self.data.message_id_value.conference, self.data.message_id_value.area) },
            _ => (self.as_int(), 0),
        }
    }

    #[must_use]
    pub fn get_hour(&self) -> Self {
        VariableValue::new_int(unsafe { (self.data.time_value % (24 * 60 * 60)) / (60 * 60) })
    }
    #[must_use]
    pub fn get_minute(&self) -> Self {
        VariableValue::new_int(unsafe { (self.data.time_value % (60 * 60)) / 60 })
    }
    #[must_use]
    pub fn get_second(&self) -> Self {
        VariableValue::new_int(unsafe { self.data.time_value % 60 })
    }

    pub fn new_function(value: FunctionValue) -> VariableValue {
        VariableValue {
            vtype: VariableType::Function,
            data: value.to_data(),
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_procedure(value: ProcedureValue) -> VariableValue {
        VariableValue {
            vtype: VariableType::Procedure,
            data: value.to_data(),
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_date(reg_date: i32) -> VariableValue {
        VariableValue {
            vtype: VariableType::Date,
            data: VariableData::from_int(reg_date),
            generic_data: GenericVariableData::None,
        }
    }
    pub fn new_time(reg_date: i32) -> VariableValue {
        VariableValue {
            vtype: VariableType::Time,
            data: VariableData::from_int(reg_date),
            generic_data: GenericVariableData::None,
        }
    }

    #[must_use]
    pub fn get_array_value(&self, dim_1: usize, dim_2: usize, dim_3: usize) -> VariableValue {
        if let GenericVariableData::Dim1(data) = &self.generic_data {
            if dim_1 < data.len() {
                data[dim_1].clone()
            } else {
                log::error!("dim1 out of bounds: {} > {}", dim_1, data.len());
                self.vtype.create_empty_value()
            }
        } else if let GenericVariableData::Dim2(data) = &self.generic_data {
            if dim_1 < data.len() && dim_2 < data[dim_1].len() {
                data[dim_1][dim_2].clone()
            } else {
                if dim_1 < data.len() {
                    log::error!("dim1 out of bounds: {} > {}", dim_1, data.len());
                } else {
                    log::error!("dim2 out of bounds: {} > {}", dim_2, data[dim_1].len());
                }
                self.vtype.create_empty_value()
            }
        } else if let GenericVariableData::Dim3(data) = &self.generic_data {
            if dim_1 < data.len() && dim_2 < data[dim_1].len() && dim_3 < data[dim_1][dim_2].len() {
                data[dim_1][dim_2][dim_3].clone()
            } else {
                if dim_1 < data.len() {
                    if dim_2 < data[dim_1].len() {
                        log::error!("dim3 out of bounds: {} > {}", dim_3, data[dim_1][dim_2].len());
                    } else {
                        log::error!("dim2 out of bounds: {} > {}", dim_2, data[dim_1].len());
                    }
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim_1, data.len());
                }
                self.vtype.create_empty_value()
            }
        } else {
            self.vtype.create_empty_value()
        }
    }

    pub fn get_array_value_mut(&mut self, dim_1: usize, dim_2: usize, dim_3: usize) -> Option<&mut VariableValue> {
        match &mut self.generic_data {
            GenericVariableData::Dim1(data) => data.get_mut(dim_1),
            GenericVariableData::Dim2(data) => data.get_mut(dim_1)?.get_mut(dim_2),
            GenericVariableData::Dim3(data) => data.get_mut(dim_1)?.get_mut(dim_2)?.get_mut(dim_3),
            _ => None,
        }
    }

    pub fn redim(&mut self, dim: u8, vs: usize, ms: usize, cs: usize) {
        self.generic_data = GenericVariableData::create_array(self.vtype.create_empty_value(), dim, vs, ms, cs).unwrap_or(GenericVariableData::None);
    }

    pub fn set_array_value(&mut self, dim1: usize, dim2: usize, dim3: usize, val: VariableValue) -> Res<()> {
        match &mut self.generic_data {
            GenericVariableData::None => {
                return Err(Box::new(VMError::GenericDataNotSet));
            }
            GenericVariableData::Dim1(data) => {
                if dim1 < data.len() {
                    data[dim1] = val.convert_to(self.vtype);
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim1, data.len());
                }
            }
            GenericVariableData::Dim2(data) => {
                if dim1 < data.len() && dim2 < data[dim1].len() {
                    data[dim1][dim2] = val.convert_to(self.vtype);
                } else if dim1 < data.len() {
                    log::error!("dim2 out of bounds: {} > {}", dim2, data[dim1].len());
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim1, data.len());
                }
            }
            GenericVariableData::Dim3(data) => {
                if dim1 < data.len() && dim2 < data[dim1].len() && dim3 < data[dim1][dim2].len() {
                    data[dim1][dim2][dim3] = val.convert_to(self.vtype);
                } else if dim1 < data.len() {
                    if dim2 < data[dim1].len() {
                        log::error!("dim3 out of bounds: {} > {}", dim3, data[dim1][dim2].len());
                    } else {
                        log::error!("dim2 out of bounds: {} > {}", dim2, data[dim1].len());
                    }
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim1, data.len());
                }
            }
            _ => {
                if self.vtype == VariableType::String {
                    if let Some(ch) = val.as_string().chars().next() {
                        if let GenericVariableData::String(s) = &mut self.generic_data {
                            let mut v: Vec<char> = s.chars().collect();
                            v.resize(dim1 + 1, ' ');
                            v[dim1] = ch;
                            *s = v.iter().collect();
                        } else {
                            return Err(Box::new(VMError::NoStringVariable));
                        }
                    }
                    return Ok(());
                }
                return Err(Box::new(VMError::NoStringVariable));
            }
        }
        Ok(())
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    #[must_use]
    pub fn convert_to(self, convert_to_type: VariableType) -> VariableValue {
        if self.vtype == convert_to_type {
            return self;
        }

        let mut data = VariableData::default();

        match convert_to_type {
            VariableType::Boolean => {
                data.unsigned_value = u64::from(self.as_bool());
            }
            VariableType::Unsigned => {
                data.unsigned_value = self.as_int() as u64;
            }
            VariableType::Date => {
                data.date_value = match self.vtype {
                    VariableType::String | VariableType::BigStr => date_from_string(&self.as_string()),
                    _ => self.as_int() as u32,
                };
            }
            // An EDATE holds the same julian a DATE does, it only shows itself as YYMM.DD.
            // PCBoard does not read a date out of a string here, it answers 0.
            VariableType::EDate => {
                data.edate_value = match self.vtype {
                    VariableType::String | VariableType::BigStr => 0,
                    _ => self.as_int() as u32,
                };
            }
            VariableType::Integer | VariableType::MessageAreaID => {
                data.int_value = self.as_int();
            }
            VariableType::Money => {
                data.money_value = match self.vtype {
                    VariableType::String | VariableType::BigStr => money_from_string(&self.as_string()),
                    _ => self.as_int(),
                };
            }
            VariableType::String => return VariableValue::new_string(self.as_string()),
            VariableType::BigStr => {
                return VariableValue {
                    vtype: VariableType::BigStr,
                    data,
                    generic_data: GenericVariableData::String(self.as_string()),
                };
            }
            VariableType::Time => {
                data.time_value = match self.vtype {
                    VariableType::String | VariableType::BigStr => IcbTime::parse(&self.as_string()).to_pcboard_time(),
                    _ => self.as_int(),
                };
            }
            VariableType::Byte => {
                data.byte_value = self.as_byte();
            }
            VariableType::Word => {
                data.word_value = self.as_word();
            }
            VariableType::SByte => {
                data.sbyte_value = self.as_sbyte();
            }
            VariableType::SWord => {
                data.sword_value = self.as_sword();
            }
            VariableType::Float => {
                data.float_value = self.as_float();
            }
            VariableType::Double => {
                data.double_value = self.as_double();
            }
            VariableType::DDate => {
                // A DDATE holds the julian too; only its text form is CCYYMMDD.
                data.ddate_value = match self.vtype {
                    VariableType::String | VariableType::BigStr => ddate_from_string(&self.as_string()),
                    _ => self.as_int(),
                };
            }
            VariableType::Table => {
                panic!("Not supported for tables.")
            }
            VariableType::Function => {
                unsafe { data.function_value = self.data.function_value };
            }
            VariableType::Procedure => {
                unsafe { data.procedure_value = self.data.procedure_value };
            }
            VariableType::UserData(x) => {
                log::error!("can't convert {self:?} to user data type {x}");
                data.int_value = -1;
            }
            VariableType::Password => {
                return VariableValue::new_password(Password::PlainText(self.as_string().to_lowercase()));
            }
            VariableType::None => {
                panic!("Unknown variable type")
            }
        }
        VariableValue::new(convert_to_type, data)
    }

    pub(crate) fn new_password(password: crate::icy_board::user_base::Password) -> VariableValue {
        VariableValue {
            vtype: VariableType::Password,
            data: VariableData::default(),
            generic_data: GenericVariableData::Password(password),
        }
    }
}

/// `PCBoard` prints a date as MM/DD/YY and an empty one as 00/00/00.
fn pcb_date_string(date: u32) -> String {
    if date == 0 {
        return "00/00/00".to_string();
    }
    let date = IcbDate::from_pcboard(date);
    format!("{:02}/{:02}/{:02}", date.month(), date.day(), date.year() % 100)
}

/// Money is held in cents and printed as $D.CC.
fn pcb_money_string(cents: i32) -> String {
    format!("${}.{:02}", cents / 100, (cents % 100).abs())
}

/// A date a PPE hands over as text, or 0 when it is not one.
fn date_from_string(str: &str) -> u32 {
    IcbDate::try_parse(str).map_or(0, |date| date.to_pcboard_date().max(0) as u32)
}

/// `PCBoard` reads money as dollars and keeps cents, cutting off anything finer.
/// It drops the sign on the way in: "-1.50" is worth as much as "1.50".
fn money_from_string(str: &str) -> i32 {
    let mut dollars: i64 = 0;
    let mut cents: i64 = 0;
    let mut digits_after_dot = 0;
    let mut seen_dot = false;
    for ch in str.chars() {
        match ch {
            '.' if !seen_dot => seen_dot = true,
            '0'..='9' => {
                let digit = ch as i64 - '0' as i64;
                if seen_dot {
                    if digits_after_dot < 2 {
                        cents = cents * 10 + digit;
                        digits_after_dot += 1;
                    }
                } else {
                    dollars = dollars.saturating_mul(10).saturating_add(digit);
                }
            }
            _ => {}
        }
    }
    if digits_after_dot == 1 {
        cents *= 10;
    }
    dollars.saturating_mul(100).saturating_add(cents).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// An EDATE shows the same julian a DATE holds as YYMM.DD.
fn pcb_edate_string(date: u32) -> String {
    if date == 0 {
        return "0000.00".to_string();
    }
    let date = IcbDate::from_pcboard(date);
    format!("{:02}{:02}.{:02}", date.year() % 100, date.month(), date.day())
}

/// A DDATE shows the same julian as CCYYMMDD, with stars where the year will not fit.
fn pcb_ddate_string(date: i32) -> String {
    if date <= 0 {
        return " ".repeat(8);
    }
    let parsed = IcbDate::from_pcboard(date as u32);
    let year = full_year(&parsed);
    if !(0..=9999).contains(&year) {
        return "****0000".to_string();
    }
    format!("{year:04}{:02}{:02}", parsed.month(), parsed.day())
}

/// A DDATE reads a date out of CCYYMMDD text and keeps the julian for it.
fn ddate_from_string(str: &str) -> i32 {
    let digits: String = str.chars().filter(char::is_ascii_digit).collect();
    if digits.len() != 8 {
        return 0;
    }
    let year = digits[0..4].parse::<u16>().unwrap_or(0);
    let month = digits[4..6].parse::<u8>().unwrap_or(0);
    let day = digits[6..8].parse::<u8>().unwrap_or(0);
    if month == 0 || day == 0 {
        return 0;
    }
    IcbDate::new(month, day, year).to_pcboard_date()
}

/// A date unpacked from `PCBoard`'s format carries only two year digits.
fn full_year(date: &IcbDate) -> i32 {
    match date.year() {
        year if year >= 100 => year as i32,
        year if year < 79 => 2000 + year as i32,
        year => 1900 + year as i32,
    }
}

/// .
///
/// # Panics
///
/// Panics if .
pub fn convert_to(var_type: VariableType, value: &VariableValue) -> VariableValue {
    let mut res = value.clone();
    res.vtype = var_type;
    if var_type == VariableType::String || var_type == VariableType::BigStr {
        res.generic_data = GenericVariableData::String(value.as_string());
    }
    if var_type == VariableType::Password {
        if let GenericVariableData::Password(p) = &value.generic_data {
            res.generic_data = GenericVariableData::Password(p.clone());
        } else {
            res.generic_data = GenericVariableData::Password(Password::new_argon2(value.as_string()));
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use crate::executable::VariableData;

    #[test]
    fn check_variable_size() {
        assert_eq!(8, std::mem::size_of::<VariableData>());
    }
}

#[derive(Debug, Clone, Default)]
pub struct PPLTable {
    pub table: HashMap<VariableValue, VariableValue>,
}
