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

    /// 8-byte signed integer.
    Long,

    /// 8-byte unsigned integer.
    ULong,

    /// Contiguous, growable binary blob (`Vec<u8>`). Language extension (>=400) for binary data and fast I/O.
    Bytes,

    /// Unbounded Unicode text used by PPL 4.00 STRING declarations.
    UnboundedString,

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
            VariableType::Long => 21,
            VariableType::ULong => 22,
            VariableType::Bytes => 23,
            VariableType::UnboundedString => 24,
            VariableType::UserData(b) => b,
            VariableType::None => 255,
        }
    }
}

impl VariableType {
    pub fn create_empty_value(&self) -> VariableValue {
        match self {
            VariableType::String => VariableValue::new_string(String::new()),
            VariableType::BigStr => VariableValue {
                vtype: VariableType::BigStr,
                generic_data: GenericVariableData::String(std::sync::Arc::new(String::new())),
                ..Default::default()
            },
            VariableType::UnboundedString => VariableValue {
                vtype: VariableType::UnboundedString,
                generic_data: GenericVariableData::String(std::sync::Arc::new(String::new())),
                ..Default::default()
            },
            VariableType::Bytes => VariableValue::new_bytes(Vec::new()),
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
            21 => VariableType::Long,
            22 => VariableType::ULong,
            23 => VariableType::Bytes,
            24 => VariableType::UnboundedString,
            _ => VariableType::UserData(b),
        }
    }

    pub fn get_signature(&self) -> Signature {
        let sig = match self {
            VariableType::Boolean => "BOOLEAN".to_string(),
            VariableType::Unsigned => "UNSIGNED".to_string(),
            VariableType::Date => "DATE".to_string(),
            VariableType::EDate => "EDATE".to_string(),
            VariableType::Integer => "INTEGER / SDWORD".to_string(),
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
            VariableType::Long => "LONG".to_string(),
            VariableType::ULong => "ULONG".to_string(),
            VariableType::Bytes => "BYTES".to_string(),
            VariableType::UnboundedString => "STRING".to_string(),
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
            VariableType::String => write!(f, "String"),           // String without \0 and maximum length of 256
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
            VariableType::Long => write!(f, "Long"),               // i64
            VariableType::ULong => write!(f, "ULong"),             // u64
            VariableType::Bytes => write!(f, "Bytes"),             // Vec<u8>
            VariableType::UnboundedString => write!(f, "String"),
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
    pub long_value: i64,
    pub ulong_value: u64,
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
    String(std::sync::Arc<String>),

    /// Contiguous binary payload for `VariableType::Bytes`.
    Bytes(Vec<u8>),

    Dim1(std::sync::Arc<Vec<VariableValue>>),
    Dim2(std::sync::Arc<Vec<Vec<VariableValue>>>),
    Dim3(std::sync::Arc<Vec<Vec<Vec<VariableValue>>>>),

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
            GenericVariableData::Bytes(data) => write!(f, "Bytes({} bytes)", data.len()),
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
pub(crate) const MAX_ARRAY_SIZE: usize = 100_000_000;

impl GenericVariableData {
    pub(crate) fn create_array(base_value: VariableValue, dim: u8, vector_size: usize, matrix_size: usize, cube_size: usize) -> Option<GenericVariableData> {
        match dim {
            1 => {
                if vector_size > MAX_ARRAY_SIZE {
                    log::error!("Creating a large array of size: {vector_size} elements - probably file is corrupt.");
                    return None;
                }
                Some(GenericVariableData::Dim1(std::sync::Arc::new(vec![base_value; vector_size + 1])))
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
                Some(GenericVariableData::Dim2(std::sync::Arc::new(vec![
                    vec![base_value; matrix_size + 1];
                    vector_size + 1
                ])))
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
                Some(GenericVariableData::Dim3(std::sync::Arc::new(vec![
                    vec![
                        vec![base_value; cube_size + 1];
                        matrix_size + 1
                    ];
                    vector_size + 1
                ])))
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
                VariableType::Long => write!(f, "{}", self.data.long_value),
                VariableType::ULong => write!(f, "{}", self.data.ulong_value),
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

                VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                    if let GenericVariableData::String(s) = &self.generic_data {
                        write!(f, "{s}")
                    } else {
                        write!(f, "")
                    }
                }
                VariableType::Bytes => {
                    if let GenericVariableData::Bytes(data) = &self.generic_data {
                        write!(f, "{}", bytes_to_hex(data))
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
        match (&self.generic_data, &other.generic_data) {
            (GenericVariableData::Dim1(left), GenericVariableData::Dim1(right)) => return self.vtype == other.vtype && left == right,
            (GenericVariableData::Dim2(left), GenericVariableData::Dim2(right)) => return self.vtype == other.vtype && left == right,
            (GenericVariableData::Dim3(left), GenericVariableData::Dim3(right)) => return self.vtype == other.vtype && left == right,
            (GenericVariableData::Bytes(left), GenericVariableData::Bytes(right)) => return left == right,
            _ => {}
        }
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
                VariableType::Long => self.as_long() == other.as_long(),
                VariableType::ULong => self.as_ulong() == other.as_ulong(),
                VariableType::Date => self.data.date_value == other.data.date_value,
                VariableType::DDate => self.data.ddate_value == other.data.ddate_value,
                VariableType::EDate => self.data.edate_value == other.data.edate_value,

                VariableType::Integer => self.as_int() == other.as_int(),
                VariableType::Money => self.data.money_value == other.data.money_value,
                VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
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
    if matches!(l, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
        && matches!(r, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
    {
        if l == VariableType::UnboundedString || r == VariableType::UnboundedString {
            return VariableType::UnboundedString;
        }
        return VariableType::BigStr;
    }
    if l == VariableType::Float || l == VariableType::Double || r == VariableType::Float || r == VariableType::Double {
        return VariableType::Double;
    }
    if l == VariableType::ULong || r == VariableType::ULong {
        return VariableType::ULong;
    }
    if l == VariableType::Long || r == VariableType::Long {
        return VariableType::Long;
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
                VariableType::Long => {
                    data.long_value = self.as_long().wrapping_add(other.as_long());
                }
                VariableType::ULong => {
                    data.ulong_value = self.as_ulong().wrapping_add(other.as_ulong());
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

                VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                    let mut new_string = self.as_string();
                    new_string.push_str(&other.as_string());
                    generic_data = GenericVariableData::String(std::sync::Arc::new(new_string));
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
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                let r = other.as_int();
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
                VariableType::Long => {
                    data.long_value = self.as_long().wrapping_sub(other.as_long());
                }
                VariableType::ULong => {
                    data.ulong_value = self.as_ulong().wrapping_sub(other.as_ulong());
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
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
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
                VariableType::Long => {
                    data.long_value = self.as_long().wrapping_mul(other.as_long());
                }
                VariableType::ULong => {
                    data.ulong_value = self.as_ulong().wrapping_mul(other.as_ulong());
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
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                let r = other.as_int();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(if r == 0 { 0 } else { l.wrapping_div(r) }),
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
                    data.unsigned_value = if other.data.unsigned_value == 0 {
                        0
                    } else {
                        self.data.unsigned_value.wrapping_div(other.data.unsigned_value)
                    };
                }
                VariableType::Long => {
                    let divisor = other.as_long();
                    data.long_value = if divisor == 0 { 0 } else { self.as_long().wrapping_div(divisor) };
                }
                VariableType::ULong => {
                    let divisor = other.as_ulong();
                    data.ulong_value = if divisor == 0 { 0 } else { self.as_ulong().wrapping_div(divisor) };
                }
                VariableType::Integer => {
                    let divisor = other.as_int();
                    data.int_value = if divisor == 0 { 0 } else { self.as_int().wrapping_div(divisor) };
                }
                VariableType::Float => {
                    let dividend = self.convert_to(VariableType::Float).data.float_value;
                    let divisor = other.convert_to(VariableType::Float).data.float_value;
                    data.float_value = if divisor == 0.0 { 0.0 } else { dividend / divisor };
                }
                VariableType::Double => {
                    let divisor = other.as_double();
                    data.double_value = if divisor == 0.0 { 0.0 } else { self.as_double() / divisor };
                }
                VariableType::Byte => {
                    let divisor = other.as_byte();
                    data.byte_value = if divisor == 0 { 0 } else { self.as_byte().wrapping_div(divisor) };
                }
                VariableType::SByte => {
                    let divisor = other.as_sbyte();
                    data.sbyte_value = if divisor == 0 { 0 } else { self.as_sbyte().wrapping_div(divisor) };
                }
                VariableType::Word => {
                    let divisor = other.as_word();
                    data.word_value = if divisor == 0 { 0 } else { self.as_word().wrapping_div(divisor) };
                }
                VariableType::SWord => {
                    let divisor = other.as_sword();
                    data.sword_value = if divisor == 0 { 0 } else { self.as_sword().wrapping_div(divisor) };
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

            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                let r = other.as_int();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(if r == 0 { 0 } else { l.wrapping_rem(r) }),
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
                    data.unsigned_value = if other.data.unsigned_value == 0 {
                        0
                    } else {
                        self.data.unsigned_value.wrapping_rem(other.data.unsigned_value)
                    };
                }
                VariableType::Long => {
                    let divisor = other.as_long();
                    data.long_value = if divisor == 0 { 0 } else { self.as_long().wrapping_rem(divisor) };
                }
                VariableType::ULong => {
                    let divisor = other.as_ulong();
                    data.ulong_value = if divisor == 0 { 0 } else { self.as_ulong().wrapping_rem(divisor) };
                }
                VariableType::Integer => {
                    let divisor = other.as_int();
                    data.int_value = if divisor == 0 { 0 } else { self.as_int().wrapping_rem(divisor) };
                }
                VariableType::Byte => {
                    let divisor = other.as_byte();
                    data.byte_value = if divisor == 0 { 0 } else { self.as_byte().wrapping_rem(divisor) };
                }
                VariableType::SByte => {
                    let divisor = other.as_sbyte();
                    data.sbyte_value = if divisor == 0 { 0 } else { self.as_sbyte().wrapping_rem(divisor) };
                }
                VariableType::Word => {
                    let divisor = other.as_word();
                    data.word_value = if divisor == 0 { 0 } else { self.as_word().wrapping_rem(divisor) };
                }
                VariableType::SWord => {
                    let divisor = other.as_sword();
                    data.sword_value = if divisor == 0 { 0 } else { self.as_sword().wrapping_rem(divisor) };
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
                VariableType::Long => Some(self.as_long().cmp(&other.as_long())),
                VariableType::ULong => Some(self.as_ulong().cmp(&other.as_ulong())),
                VariableType::Date => Some(self.data.date_value.cmp(&other.data.date_value)),
                VariableType::DDate => Some(self.data.ddate_value.cmp(&other.data.ddate_value)),
                VariableType::EDate => Some(self.data.edate_value.cmp(&other.data.edate_value)),

                VariableType::Integer => Some(self.as_int().cmp(&other.as_int())),
                VariableType::Money => Some(self.data.money_value.cmp(&other.data.money_value)),
                VariableType::String | VariableType::BigStr | VariableType::UnboundedString => Some(self.as_string().cmp(&other.as_string())),

                VariableType::Time => Some(self.data.time_value.cmp(&other.data.time_value)),
                VariableType::Float => self.as_float().partial_cmp(&other.as_float()),
                VariableType::Double => self.as_double().partial_cmp(&other.as_double()),
                VariableType::Byte => Some(self.as_byte().cmp(&other.as_byte())),
                VariableType::SByte => Some(self.as_sbyte().cmp(&other.as_sbyte())),
                VariableType::Word => Some(self.as_word().cmp(&other.as_word())),
                VariableType::SWord => Some(self.as_sword().cmp(&other.as_sword())),

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
            VariableType::ULong => {
                dest_type = VariableType::Long;
            }
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
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_neg()),
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
            VariableType::Long => data.long_value = self.as_long().wrapping_neg(),
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
    pub(crate) fn remap_user_types(&mut self, remap: &std::collections::HashMap<u8, u8>) {
        if let VariableType::UserData(type_id) = self.vtype
            && let Some(new_id) = remap.get(&type_id)
        {
            self.vtype = VariableType::UserData(*new_id);
        }
        match &mut self.generic_data {
            GenericVariableData::Dim1(values) => {
                for value in std::sync::Arc::make_mut(values) {
                    value.remap_user_types(remap);
                }
            }
            GenericVariableData::Dim2(values) => {
                for value in std::sync::Arc::make_mut(values).iter_mut().flatten() {
                    value.remap_user_types(remap);
                }
            }
            GenericVariableData::Dim3(values) => {
                for value in std::sync::Arc::make_mut(values).iter_mut().flatten().flatten() {
                    value.remap_user_types(remap);
                }
            }
            GenericVariableData::Record(fields) => {
                for field in fields {
                    field.remap_user_types(remap);
                }
            }
            _ => {}
        }
    }

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
                generic_data: GenericVariableData::Dim1(std::sync::Arc::new(values.iter().map(VariableValue::emptied).collect())),
            },
            GenericVariableData::Dim2(values) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Dim2(std::sync::Arc::new(
                    values.iter().map(|row| row.iter().map(VariableValue::emptied).collect()).collect(),
                )),
            },
            GenericVariableData::Dim3(values) => VariableValue {
                vtype: self.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::Dim3(std::sync::Arc::new(
                    values
                        .iter()
                        .map(|plane| plane.iter().map(|row| row.iter().map(VariableValue::emptied).collect()).collect())
                        .collect(),
                )),
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
            generic_data: GenericVariableData::String(std::sync::Arc::new(s)),
        }
    }

    pub fn new_unbounded_string(s: String) -> Self {
        Self {
            vtype: VariableType::UnboundedString,
            data: VariableData::default(),
            generic_data: GenericVariableData::String(std::sync::Arc::new(s)),
        }
    }

    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        Self {
            vtype: VariableType::Bytes,
            data: VariableData::default(),
            generic_data: GenericVariableData::Bytes(bytes),
        }
    }

    /// The raw binary payload of a `Bytes` value, or an empty slice for any other type.
    pub fn as_byte_slice(&self) -> &[u8] {
        if let GenericVariableData::Bytes(data) = &self.generic_data {
            data
        } else {
            &[]
        }
    }

    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        if let GenericVariableData::Bytes(data) = &self.generic_data {
            return Some(data.clone());
        }
        if let GenericVariableData::String(value) = &self.generic_data {
            return Some(value.as_bytes().to_vec());
        }
        unsafe {
            match self.vtype {
                VariableType::Boolean => Some(vec![u8::from(self.as_bool())]),
                VariableType::Byte => Some(vec![self.data.byte_value]),
                VariableType::SByte => Some(self.data.sbyte_value.to_le_bytes().to_vec()),
                VariableType::Word => Some(self.data.word_value.to_le_bytes().to_vec()),
                VariableType::SWord => Some(self.data.sword_value.to_le_bytes().to_vec()),
                VariableType::Integer => Some(self.data.int_value.to_le_bytes().to_vec()),
                VariableType::Unsigned => Some((self.data.unsigned_value as u32).to_le_bytes().to_vec()),
                VariableType::Long => Some(self.data.long_value.to_le_bytes().to_vec()),
                VariableType::ULong => Some(self.data.ulong_value.to_le_bytes().to_vec()),
                VariableType::Float => Some(self.data.float_value.to_le_bytes().to_vec()),
                VariableType::Double => Some(self.data.double_value.to_le_bytes().to_vec()),
                VariableType::Money => Some(self.data.money_value.to_le_bytes().to_vec()),
                VariableType::Date => Some(self.data.date_value.to_le_bytes().to_vec()),
                VariableType::EDate => Some(self.data.edate_value.to_le_bytes().to_vec()),
                VariableType::DDate => Some(self.data.ddate_value.to_le_bytes().to_vec()),
                VariableType::Time => Some(self.data.time_value.to_le_bytes().to_vec()),
                VariableType::MessageAreaID => {
                    let mut result = self.data.message_id_value.conference.to_le_bytes().to_vec();
                    result.extend_from_slice(&self.data.message_id_value.area.to_le_bytes());
                    Some(result)
                }
                _ => None,
            }
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

    pub fn new_long(value: i64) -> Self {
        Self {
            vtype: VariableType::Long,
            data: VariableData { long_value: value },
            generic_data: GenericVariableData::None,
        }
    }

    pub fn new_ulong(value: u64) -> Self {
        Self {
            vtype: VariableType::ULong,
            data: VariableData { ulong_value: value },
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
            generic_data: GenericVariableData::Dim1(std::sync::Arc::new(vec)),
        }
    }

    pub fn new_matrix(variable_type: VariableType, vec: Vec<Vec<VariableValue>>) -> Self {
        Self {
            vtype: variable_type,
            data: VariableData::default(),
            generic_data: GenericVariableData::Dim2(std::sync::Arc::new(vec)),
        }
    }

    pub fn new_cube(variable_type: VariableType, vec: Vec<Vec<Vec<VariableValue>>>) -> Self {
        Self {
            vtype: variable_type,
            data: VariableData::default(),
            generic_data: GenericVariableData::Dim3(std::sync::Arc::new(vec)),
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
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                let r = other.as_int();
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
                VariableType::Long => {
                    data.long_value = self.as_long().wrapping_pow(other.as_ulong() as u32);
                }
                VariableType::ULong => {
                    data.ulong_value = self.as_ulong().wrapping_pow(other.as_ulong() as u32);
                }
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
            VariableType::ULong => return self.clone(),
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
                let l = self.as_int();
                return Self {
                    vtype: VariableType::Integer,
                    data: VariableData::from_int(l.wrapping_abs()),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Long => data.long_value = self.as_long().wrapping_abs(),
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
        if matches!(self.vtype, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
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
            VariableType::Long => unsafe { self.data.long_value as i32 },
            VariableType::ULong => unsafe { self.data.ulong_value as i32 },
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

    /// The same as [`VariableValue::as_int`], for a value a corrupt PPE may have typed
    /// as something that has no integer form at all.
    pub fn try_as_int(&self) -> Option<i32> {
        if matches!(self.generic_data, GenericVariableData::String(_)) {
            return Some(self.as_int());
        }
        match self.vtype {
            VariableType::Boolean
            | VariableType::Unsigned
            | VariableType::Long
            | VariableType::ULong
            | VariableType::Date
            | VariableType::DDate
            | VariableType::EDate
            | VariableType::Integer
            | VariableType::Money
            | VariableType::Float
            | VariableType::Double
            | VariableType::Time
            | VariableType::Byte
            | VariableType::Word
            | VariableType::SByte
            | VariableType::SWord
            | VariableType::MessageAreaID => Some(self.as_int()),
            _ => None,
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
            VariableType::Long => unsafe { self.data.long_value as u64 },
            VariableType::ULong => unsafe { self.data.ulong_value },
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

    pub fn as_long(&self) -> i64 {
        if let GenericVariableData::String(value) = &self.generic_data {
            return value.trim().parse().unwrap_or_default();
        }
        match self.vtype {
            VariableType::Long => unsafe { self.data.long_value },
            VariableType::ULong | VariableType::Unsigned => unsafe { self.data.ulong_value as i64 },
            _ => self.as_int() as i64,
        }
    }

    pub fn as_ulong(&self) -> u64 {
        if let GenericVariableData::String(value) = &self.generic_data {
            return value.trim().parse().unwrap_or_default();
        }
        match self.vtype {
            VariableType::Long => unsafe { self.data.long_value as u64 },
            VariableType::ULong | VariableType::Unsigned => unsafe { self.data.ulong_value },
            _ => self.as_int() as u64,
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
            VariableType::Long => unsafe { self.data.long_value as f64 },
            VariableType::ULong => unsafe { self.data.ulong_value as f64 },
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
            VariableType::Long => unsafe { self.data.long_value as f32 },
            VariableType::ULong => unsafe { self.data.ulong_value as f32 },
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
            VariableType::Long => unsafe { self.data.long_value as u8 },
            VariableType::ULong => unsafe { self.data.ulong_value as u8 },
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
            VariableType::Long => unsafe { self.data.long_value as u16 },
            VariableType::ULong => unsafe { self.data.ulong_value as u16 },
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

    pub fn as_str(&self) -> Option<&str> {
        match &self.generic_data {
            GenericVariableData::String(value) => Some(value.as_str()),
            GenericVariableData::Password(Password::PlainText(value)) => Some(value),
            _ => None,
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_string(&self) -> String {
        unsafe {
            match &self.generic_data {
                GenericVariableData::String(s) => s.as_ref().clone(),
                GenericVariableData::Bytes(data) => bytes_to_hex(data),
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
                    VariableType::Long => self.data.long_value.to_string(),
                    VariableType::ULong => self.data.ulong_value.to_string(),
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
            GenericVariableData::Dim1(data) => std::sync::Arc::make_mut(data).get_mut(dim_1),
            GenericVariableData::Dim2(data) => std::sync::Arc::make_mut(data).get_mut(dim_1)?.get_mut(dim_2),
            GenericVariableData::Dim3(data) => std::sync::Arc::make_mut(data).get_mut(dim_1)?.get_mut(dim_2)?.get_mut(dim_3),
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
                let data = std::sync::Arc::make_mut(data);
                if dim1 < data.len() {
                    data[dim1] = val.convert_to(self.vtype);
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim1, data.len());
                }
            }
            GenericVariableData::Dim2(data) => {
                let data = std::sync::Arc::make_mut(data);
                if dim1 < data.len() && dim2 < data[dim1].len() {
                    data[dim1][dim2] = val.convert_to(self.vtype);
                } else if dim1 < data.len() {
                    log::error!("dim2 out of bounds: {} > {}", dim2, data[dim1].len());
                } else {
                    log::error!("dim1 out of bounds: {} > {}", dim1, data.len());
                }
            }
            GenericVariableData::Dim3(data) => {
                let data = std::sync::Arc::make_mut(data);
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
                            let s = std::sync::Arc::make_mut(s);
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
        // The PPL 4.00 string is unbounded. Legacy strings still fall through so
        // their character limits are enforced on every assignment.
        if self.vtype == convert_to_type && convert_to_type == VariableType::UnboundedString {
            return self;
        }
        if convert_to_type == VariableType::Bytes {
            if let GenericVariableData::Dim1(values) = &self.generic_data {
                return VariableValue::new_bytes(values.iter().map(VariableValue::as_byte).collect());
            }
            return VariableValue::new_bytes(self.to_bytes().unwrap_or_default());
        }
        if matches!(convert_to_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
            match self.generic_data {
                GenericVariableData::Dim1(values) => {
                    let values = std::sync::Arc::unwrap_or_clone(values);
                    return VariableValue {
                        vtype: convert_to_type,
                        generic_data: GenericVariableData::Dim1(std::sync::Arc::new(
                            values.into_iter().map(|value| value.convert_to(convert_to_type)).collect(),
                        )),
                        ..Default::default()
                    };
                }
                GenericVariableData::Dim2(values) => {
                    let values = std::sync::Arc::unwrap_or_clone(values);
                    return VariableValue {
                        vtype: convert_to_type,
                        generic_data: GenericVariableData::Dim2(std::sync::Arc::new(
                            values
                                .into_iter()
                                .map(|row| row.into_iter().map(|value| value.convert_to(convert_to_type)).collect())
                                .collect(),
                        )),
                        ..Default::default()
                    };
                }
                GenericVariableData::Dim3(values) => {
                    let values = std::sync::Arc::unwrap_or_clone(values);
                    return VariableValue {
                        vtype: convert_to_type,
                        generic_data: GenericVariableData::Dim3(std::sync::Arc::new(
                            values
                                .into_iter()
                                .map(|plane| {
                                    plane
                                        .into_iter()
                                        .map(|row| row.into_iter().map(|value| value.convert_to(convert_to_type)).collect())
                                        .collect()
                                })
                                .collect(),
                        )),
                        ..Default::default()
                    };
                }
                generic_data => {
                    let mut value = VariableValue { generic_data, ..self }.as_string();
                    if convert_to_type == VariableType::String {
                        if let Some((byte_index, _)) = value.char_indices().nth(256) {
                            value.truncate(byte_index);
                        }
                    } else if convert_to_type == VariableType::BigStr
                        && let Some((byte_index, _)) = value.char_indices().nth(2048)
                    {
                        value.truncate(byte_index);
                    }
                    return VariableValue {
                        vtype: convert_to_type,
                        generic_data: GenericVariableData::String(std::sync::Arc::new(value)),
                        ..Default::default()
                    };
                }
            }
        }
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
            VariableType::Long => {
                data.long_value = self.as_long();
            }
            VariableType::ULong => {
                data.ulong_value = self.as_ulong();
            }
            VariableType::Date => {
                data.date_value = match self.vtype {
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString => date_from_string(&self.as_string()),
                    _ => self.as_int() as u32,
                };
            }
            // An EDATE holds the same julian a DATE does, it only shows itself as YYMM.DD.
            // PCBoard does not read a date out of a string here, it answers 0.
            VariableType::EDate => {
                data.edate_value = match self.vtype {
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString => 0,
                    _ => self.as_int() as u32,
                };
            }
            VariableType::Integer | VariableType::MessageAreaID => {
                data.int_value = self.as_int();
            }
            VariableType::Money => {
                data.money_value = match self.vtype {
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString => money_from_string(&self.as_string()),
                    _ => self.as_int(),
                };
            }
            VariableType::String | VariableType::BigStr | VariableType::UnboundedString => unreachable!(),
            VariableType::Bytes => unreachable!(),
            VariableType::Time => {
                data.time_value = match self.vtype {
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString => IcbTime::parse(&self.as_string()).to_pcboard_time(),
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
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString => ddate_from_string(&self.as_string()),
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

/// Uppercase, separator-free hex rendering of a byte blob (e.g. `48656C6C6F`).
fn bytes_to_hex(data: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        let _ = write!(s, "{b:02X}");
    }
    s
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
    if matches!(var_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
        res.generic_data = GenericVariableData::String(std::sync::Arc::new(value.as_string()));
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
    use unicase::Ascii;

    use crate::{
        executable::{VariableData, VariableType, VariableValue},
        parser::built_in_type,
    };

    #[test]
    fn check_variable_size() {
        assert_eq!(8, std::mem::size_of::<VariableData>());
    }

    #[test]
    fn bytes_round_trips_its_binary_payload() {
        let value = VariableValue::new_bytes(vec![0x00, 0x48, 0xFF, 0x10]);
        assert_eq!(VariableType::Bytes, value.get_type());
        assert_eq!(&[0x00, 0x48, 0xFF, 0x10], value.as_byte_slice());
    }

    #[test]
    fn bytes_render_as_uppercase_hex() {
        assert_eq!("48656C6C6F", VariableValue::new_bytes(b"Hello".to_vec()).as_string());
        assert_eq!("", VariableValue::new_bytes(Vec::new()).as_string());
    }

    #[test]
    fn bytes_compare_by_content_not_identity() {
        assert_eq!(VariableValue::new_bytes(vec![1, 2, 3]), VariableValue::new_bytes(vec![1, 2, 3]));
        assert_ne!(VariableValue::new_bytes(vec![1, 2, 3]), VariableValue::new_bytes(vec![1, 2, 4]));
    }

    #[test]
    fn cloned_arrays_detach_on_write() {
        let original = VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(1), VariableValue::new_int(2)]);
        let mut clone = original.clone();

        clone.set_array_value(0, 0, 0, VariableValue::new_int(9)).unwrap();

        assert_eq!(1, original.get_array_value(0, 0, 0).as_int());
        assert_eq!(9, clone.get_array_value(0, 0, 0).as_int());
    }

    #[test]
    fn a_string_converts_to_its_utf8_bytes() {
        let value = VariableValue::new_string("Grüße".to_string()).convert_to(VariableType::Bytes);
        assert_eq!(VariableType::Bytes, value.get_type());
        assert_eq!("Grüße".as_bytes(), value.as_byte_slice());
    }

    #[test]
    fn string_conversion_matches_pcboard_capacity() {
        let text = "ä".repeat(3000);

        assert_eq!(
            256,
            VariableValue::new_string(text.clone())
                .convert_to(VariableType::String)
                .as_string()
                .chars()
                .count()
        );
        assert_eq!(
            2048,
            VariableValue::new_string(text.clone())
                .convert_to(VariableType::BigStr)
                .convert_to(VariableType::BigStr)
                .as_string()
                .chars()
                .count()
        );
        assert_eq!(
            3000,
            VariableValue::new_string(text)
                .convert_to(VariableType::UnboundedString)
                .as_string()
                .chars()
                .count()
        );
    }

    #[test]
    fn string_type_ids_and_version_mapping_are_stable() {
        assert_eq!(7, u8::from(VariableType::String));
        assert_eq!(13, u8::from(VariableType::BigStr));
        assert_eq!(24, u8::from(VariableType::UnboundedString));
        assert_eq!(VariableType::String, VariableType::from(7));
        assert_eq!(VariableType::BigStr, VariableType::from(13));
        assert_eq!(VariableType::UnboundedString, VariableType::from(24));

        let name = Ascii::new("STRING".to_string());
        assert_eq!(Some(VariableType::String), built_in_type(&name, 340));
        assert_eq!(Some(VariableType::UnboundedString), built_in_type(&name, 400));
    }

    #[test]
    fn long_keeps_its_historical_meaning_before_runtime_400() {
        let name = Ascii::new("LONG".to_string());
        assert_eq!(Some(VariableType::Integer), built_in_type(&name, 340));
        assert_eq!(Some(VariableType::Long), built_in_type(&name, 400));
    }

    #[test]
    fn long_arithmetic_keeps_integer_literals_at_64_bits() {
        let value = VariableValue::new_long(i32::MAX as i64 + 1);
        let one = VariableValue::new_int(1);

        assert_eq!("2147483649", (value.clone() + one.clone()).as_string());
        assert_eq!("2147483647", (value.clone() - one).as_string());
        assert!(value > VariableValue::new_int(i32::MAX));
    }

    #[test]
    fn ulong_arithmetic_uses_the_full_unsigned_width() {
        let value = VariableValue::new_ulong(u64::MAX);

        assert_eq!(u64::MAX.to_string(), value.as_string());
        assert_eq!("0", (value + VariableValue::new_ulong(1)).as_string());
    }

    #[test]
    fn negating_ulong_produces_a_long() {
        let value = -VariableValue::new_ulong(1);

        assert_eq!(VariableType::Long, value.vtype);
        assert_eq!(-1, value.as_long());
    }
}

#[derive(Debug, Clone, Default)]
pub struct PPLTable {
    pub table: HashMap<VariableValue, VariableValue>,
}
