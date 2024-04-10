use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

use crate::{
    datetime::{IcbDate, IcbTime},
    executable::{FunctionValue, ProcedureValue},
};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Eq, Hash)]
#[allow(dead_code)]
pub enum VariableType {
    /// unsigned character (1 byte) 0 = FALSE, non-0 = TRUE
    Boolean = 0,

    /// 4-byte unsigned integer Range: 0 - 4,294,967,295
    Unsigned = 1,

    /// unsigned integer (2 bytes) `PCBoard` julian date (count of days since 1/1/1900)
    Date = 2,

    /// Julian date in earth date format Deals with dates formatted YYMM.DD Range: Same as DATE
    EDate = 3,

    /// signed long integer (4 bytes) Range: -2,147,483,648 → +2,147,483,647
    #[default]
    Integer = 4,
    /// signed long integer (4 bytes) Range: -$21,474,836.48 → +$21,474,836.47
    Money = 5,
    ///  4-byte floating point number Range: +/-3.4E-38 - +/-3.4E+38 (7-digit precision)
    Float = 6,
    /// far character pointer (4 bytes) NULL is an empty string non-NULL points to a string of some length less than or equal to 256
    String = 7,

    /// signed long integer (4 bytes) Count of seconds since midnight
    Time = 8,

    /// 1-byte unsigned integer Range: 0 - 255
    Byte = 9,
    /// 2-byte unsigned integer Range: 0 - 65,535
    Word = 10,
    /// 1-byte signed Integer Range: -128 - 127
    SByte = 11,
    /// 2-byte signed integer Range: -32,768 - 32,767
    SWord = 12,

    /// Allows up to 2048 characters per big string (up from 256 for STRING variables) May include CHR(0) characters in the middle of the big string (unlike STRING variables which may not)
    BigStr = 13,
    /// 8-byte floating point number Range: +/-1.7E-308 - +/-1.7E+308 (15-digit precision)
    Double = 14,

    Function = 15,
    Procedure = 16,

    /// Signed long integer for julian date. DDATE is for use with `DBase` date fields.
    /// It holds a long integer for julian dates.
    /// When coerced to string type it is in the format CCYYMMDD or 19940527
    DDate = 17,

    Unknown = 255,
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
            _ => {
                log::warn!("Invalid variable type: {}", b);
                VariableType::Integer
            }
        }
    }
}

impl fmt::Display for VariableType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VariableType::Boolean => write!(f, "Boolean"), // BOOL 0 = false, 1 = true
            VariableType::Unsigned => write!(f, "Unsigned"), // u32
            VariableType::Date => write!(f, "Date"),       // 2*u8 - julian date
            VariableType::EDate => write!(f, "EDate"),     // 2*u8 - julian date
            VariableType::Integer => write!(f, "Integer"), // i32
            VariableType::Money => write!(f, "Money"),     // i32 - x/100 Dollar x%100 Cents
            VariableType::Float => write!(f, "Real"),      // f32
            VariableType::String => write!(f, "String"), // String without \0 and maximum length of 255 (Pascal like)
            VariableType::Time => write!(f, "Time"),     // u32 - Seconds elapsed since midnight
            VariableType::Byte => write!(f, "Byte"),     // u8
            VariableType::Word => write!(f, "Word"),     // u16
            VariableType::SByte => write!(f, "SByte"),   // i8
            VariableType::SWord => write!(f, "SWord"),   // i16
            VariableType::BigStr => write!(f, "BigStr"), // String (max 2kb)
            VariableType::Double => write!(f, "Double"), // f65
            VariableType::Function => write!(f, "FUNC"), // 2*u8
            VariableType::Procedure => write!(f, "PROC"), // 2*u8
            VariableType::DDate => write!(f, "DDate"),   // i32
            VariableType::Unknown => write!(f, "Unknown"),
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
    pub bool_value: bool,
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

    pub std_struct: StdStruct,
}

impl VariableData {
    pub fn from_int(r: i32) -> VariableData {
        let mut res = VariableData::default();
        res.int_value = r;
        res
    }

    pub fn from_bool(b: bool) -> VariableData {
        let mut res = VariableData::default();
        res.bool_value = b;
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

#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub enum GenericVariableData {
    #[default]
    None,
    String(String),

    Dim1(Vec<VariableValue>),
    Dim2(Vec<Vec<VariableValue>>),
    Dim3(Vec<Vec<Vec<VariableValue>>>),
}
impl GenericVariableData {
    pub(crate) fn create_array(
        base_value: VariableValue,
        dim: u8,
        vector_size: usize,
        matrix_size: usize,
        cube_size: usize,
    ) -> GenericVariableData {
        match dim {
            1 => GenericVariableData::Dim1(vec![base_value; vector_size + 1]),
            2 => {
                GenericVariableData::Dim2(vec![vec![base_value; matrix_size + 1]; vector_size + 1])
            }
            3 => {
                if vector_size * matrix_size * cube_size > 1_000_000_000 {
                    log::error!("Creating a large array of size: {}x{}x{}={} elements - probably file is corrupt.", vector_size, matrix_size, cube_size, vector_size * matrix_size * cube_size);
                }
                GenericVariableData::Dim3(vec![
                    vec![
                        vec![base_value; cube_size + 1];
                        matrix_size + 1
                    ];
                    vector_size + 1
                ])
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

impl fmt::Display for VariableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.vtype {
                VariableType::Boolean => write!(f, "{}", self.data.bool_value),
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
        let dest_type: VariableType = promote_to(self.vtype, other.vtype);
        unsafe {
            match dest_type {
                VariableType::Boolean => self.data.bool_value == other.data.bool_value,
                VariableType::Unsigned => self.data.unsigned_value == other.data.unsigned_value,
                VariableType::Date => self.data.date_value == other.data.date_value,
                VariableType::DDate => self.data.ddate_value == other.data.ddate_value,
                VariableType::EDate => self.data.edate_value == other.data.edate_value,

                VariableType::Integer => self.as_int() == other.as_int(),
                VariableType::Money => self.data.money_value == other.data.money_value,
                VariableType::String | VariableType::BigStr => {
                    self.as_string() == other.as_string()
                }

                VariableType::Time => self.data.time_value == other.data.time_value,
                VariableType::Float => self.data.float_value == other.data.float_value,
                VariableType::Double => self.data.double_value == other.data.double_value,
                VariableType::Byte => self.data.byte_value == other.data.byte_value,
                VariableType::SByte => self.data.sbyte_value == other.data.sbyte_value,
                VariableType::Word => self.data.word_value == other.data.word_value,
                VariableType::SWord => self.data.sword_value == other.data.sword_value,

                _ => false,
            }
        }
    }
}

fn promote_to(l: VariableType, r: VariableType) -> VariableType {
    if l == r {
        return l;
    }
    if (l == VariableType::String || l == VariableType::BigStr)
        && (r == VariableType::String || r == VariableType::BigStr)
    {
        return VariableType::BigStr;
    }
    if (l == VariableType::Float || l == VariableType::Double)
        && (r == VariableType::Float || r == VariableType::Double)
    {
        return VariableType::Double;
    }
    VariableType::Integer
}

impl Add<VariableValue> for VariableValue {
    type Output = VariableValue;

    fn add(self, other: VariableValue) -> Self {
        let mut dest_type: VariableType = promote_to(self.vtype, other.vtype);
        match dest_type {
            VariableType::Boolean
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let mut generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Unsigned => {
                    data.unsigned_value = self
                        .data
                        .unsigned_value
                        .wrapping_add(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_add(other.data.int_value);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value + other.data.float_value;
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value + other.data.double_value;
                }

                VariableType::String | VariableType::BigStr => {
                    generic_data = GenericVariableData::String(format!("{self}{other}"));
                }

                VariableType::Byte => {
                    data.byte_value = self.data.byte_value.wrapping_add(other.data.byte_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_add(other.data.sbyte_value);
                }
                VariableType::Word => {
                    data.word_value = self.data.word_value.wrapping_add(other.data.word_value);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_add(other.data.sword_value);
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
            VariableType::Boolean
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::DDate => {
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
                    data.unsigned_value = self
                        .data
                        .unsigned_value
                        .wrapping_sub(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_sub(other.data.int_value);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value - other.data.float_value;
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value - other.data.double_value;
                }
                VariableType::Byte => {
                    data.byte_value = self.data.byte_value.wrapping_sub(other.data.byte_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_sub(other.data.sbyte_value);
                }
                VariableType::Word => {
                    data.word_value = self.data.word_value.wrapping_sub(other.data.word_value);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_sub(other.data.sword_value);
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
            VariableType::Boolean
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::DDate => {
                dest_type = VariableType::Integer;
            }
            VariableType::String | VariableType::BigStr => {
                let l = self.as_string().parse::<i32>().unwrap_or_default();
                let r = other.as_string().parse::<i32>().unwrap_or_default();
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
                    data.unsigned_value = self
                        .data
                        .unsigned_value
                        .wrapping_mul(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_mul(other.data.int_value);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value * other.data.float_value;
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value * other.data.double_value;
                }
                VariableType::Byte => {
                    data.byte_value = self.data.byte_value.wrapping_mul(other.data.byte_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_mul(other.data.sbyte_value);
                }
                VariableType::Word => {
                    data.word_value = self.data.word_value.wrapping_mul(other.data.word_value);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_mul(other.data.sword_value);
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
            VariableType::Boolean
            | VariableType::Date
            | VariableType::EDate
            | VariableType::Money
            | VariableType::Time
            | VariableType::DDate => {
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
                    data.unsigned_value = self
                        .data
                        .unsigned_value
                        .wrapping_div(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_div(other.data.int_value);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value / other.data.float_value;
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value / other.data.double_value;
                }
                VariableType::Byte => {
                    data.byte_value = self.data.byte_value.wrapping_div(other.data.byte_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_div(other.data.sbyte_value);
                }
                VariableType::Word => {
                    data.word_value = self.data.word_value.wrapping_div(other.data.word_value);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_div(other.data.sword_value);
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
                    data.unsigned_value = self
                        .data
                        .unsigned_value
                        .wrapping_rem(other.data.unsigned_value);
                }
                VariableType::Integer => {
                    data.int_value = self.data.int_value.wrapping_rem(other.data.int_value);
                }
                VariableType::Byte => {
                    data.byte_value = self.data.byte_value.wrapping_rem(other.data.byte_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self.data.sbyte_value.wrapping_rem(other.data.sbyte_value);
                }
                VariableType::Word => {
                    data.word_value = self.data.word_value.wrapping_rem(other.data.word_value);
                }
                VariableType::SWord => {
                    data.sword_value = self.data.sword_value.wrapping_rem(other.data.sword_value);
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
        let dest_type: VariableType = promote_to(self.vtype, other.vtype);
        unsafe {
            match dest_type {
                VariableType::Boolean => Some(self.data.bool_value.cmp(&other.data.bool_value)),
                VariableType::Unsigned => {
                    Some(self.data.unsigned_value.cmp(&other.data.unsigned_value))
                }
                VariableType::Date => Some(self.data.date_value.cmp(&other.data.date_value)),
                VariableType::DDate => Some(self.data.ddate_value.cmp(&other.data.ddate_value)),
                VariableType::EDate => Some(self.data.edate_value.cmp(&other.data.edate_value)),

                VariableType::Integer => Some(self.as_int().cmp(&other.as_int())),
                VariableType::Money => Some(self.data.money_value.cmp(&other.data.money_value)),
                VariableType::String | VariableType::BigStr => {
                    Some(self.as_string().cmp(&other.as_string()))
                }

                VariableType::Time => Some(self.data.time_value.cmp(&other.data.time_value)),
                VariableType::Float => self.data.float_value.partial_cmp(&other.data.float_value),
                VariableType::Double => {
                    self.data.double_value.partial_cmp(&other.data.double_value)
                }
                VariableType::Byte => Some(self.data.byte_value.cmp(&other.data.byte_value)),
                VariableType::SByte => Some(self.data.sbyte_value.cmp(&other.data.sbyte_value)),
                VariableType::Word => Some(self.data.word_value.cmp(&other.data.word_value)),
                VariableType::SWord => Some(self.data.sword_value.cmp(&other.data.sword_value)),

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
                    data: VariableData::from_int(-l),
                    generic_data: GenericVariableData::None,
                };
            }
            _ => {}
        }
        let mut data = VariableData::default();
        let generic_data = GenericVariableData::None;
        unsafe {
            match dest_type {
                VariableType::Integer => data.int_value = -self.data.int_value,
                VariableType::SByte => data.sbyte_value = -self.data.sbyte_value,
                VariableType::SWord => data.sword_value = -self.data.sword_value,
                VariableType::Float => data.float_value = -self.data.float_value,
                VariableType::Double => data.double_value = -self.data.double_value,
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

#[allow(clippy::needless_pass_by_value)]
impl VariableValue {
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
                    data.int_value = self
                        .data
                        .int_value
                        .wrapping_pow(other.data.int_value as u32);
                }
                VariableType::Float => {
                    data.float_value = self.data.float_value.powf(other.data.float_value);
                }
                VariableType::Double => {
                    data.double_value = self.data.double_value.powf(other.data.double_value);
                }
                VariableType::SByte => {
                    data.sbyte_value = self
                        .data
                        .sbyte_value
                        .wrapping_pow(other.data.sbyte_value as u32);
                }
                VariableType::SWord => {
                    data.sword_value = self
                        .data
                        .sword_value
                        .wrapping_pow(other.data.sword_value as u32);
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
        unsafe {
            Self {
                vtype: VariableType::Boolean,
                data: VariableData::from_bool(!self.data.bool_value),
                generic_data: GenericVariableData::None,
            }
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
        unsafe { self.data.bool_value }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_int(&self) -> i32 {
        if let GenericVariableData::String(s) = &self.generic_data {
            return s.parse::<i32>().unwrap_or_default();
        }
        unsafe { self.data.int_value }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn as_string(&self) -> String {
        unsafe {
            match &self.generic_data {
                GenericVariableData::String(s) => s.to_string(),
                _ => match self.vtype {
                    VariableType::Boolean => {
                        if self.data.bool_value {
                            "1".to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    VariableType::Unsigned => self.data.unsigned_value.to_string(),
                    VariableType::Date | VariableType::DDate | VariableType::EDate => {
                        format!("{}", IcbDate::from_pcboard(self.data.date_value))
                    }
                    VariableType::Integer => self.data.int_value.to_string(),
                    VariableType::Money => self.data.money_value.to_string(),
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
                self.vtype.create_empty_value()
            }
        } else if let GenericVariableData::Dim2(data) = &self.generic_data {
            if dim_1 < data.len() && dim_2 < data[dim_1].len() {
                data[dim_1][dim_2].clone()
            } else {
                self.vtype.create_empty_value()
            }
        } else if let GenericVariableData::Dim3(data) = &self.generic_data {
            if dim_1 < data.len() && dim_2 < data[dim_1].len() && dim_3 < data[dim_1][dim_2].len() {
                data[dim_1][dim_2][dim_3].clone()
            } else {
                self.vtype.create_empty_value()
            }
        } else {
            self.vtype.create_empty_value()
        }
    }

    pub fn redim(&mut self, dim: u8, vs: usize, ms: usize, cs: usize) {
        self.generic_data =
            GenericVariableData::create_array(self.vtype.create_empty_value(), dim, vs, ms, cs);
    }

    pub fn set_array_value(&mut self, dim1: usize, dim2: usize, dim3: usize, val: VariableValue) {
        match &mut self.generic_data {
            GenericVariableData::Dim1(data) => {
                if dim1 < data.len() {
                    data[dim1] = val.convert_to(self.vtype);
                }
            }
            GenericVariableData::Dim2(data) => {
                if dim1 < data.len() && dim2 < data[dim1].len() {
                    data[dim2][dim1] = val.convert_to(self.vtype);
                }
            }
            GenericVariableData::Dim3(data) => {
                if dim1 < data.len() && dim2 < data[dim1].len() && dim3 < data[dim1][dim2].len() {
                    data[dim3][dim2][dim1] = val.convert_to(self.vtype);
                }
            }
            _ => log::error!("no array variable: {self}"),
        }
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
                if self.vtype == VariableType::String {
                    return VariableValue::new_bool(self.as_string() == "0");
                }
                data.bool_value = self.as_int() != 0;
            }
            VariableType::Unsigned => {
                data.unsigned_value = self.as_int() as u64;
            }
            VariableType::Date => {
                data.date_value = self.as_int() as u32;
            }
            VariableType::EDate => {
                data.edate_value = self.as_int() as u32;
            }
            VariableType::Integer => {
                data.int_value = self.as_int();
            }
            VariableType::Money => {
                data.money_value = self.as_int();
            }
            VariableType::String => return VariableValue::new_string(self.as_string()),
            VariableType::BigStr => {
                return VariableValue {
                    vtype: VariableType::BigStr,
                    data,
                    generic_data: GenericVariableData::String(self.as_string()),
                }
            }
            VariableType::Time => {
                data.time_value = self.as_int();
            }
            VariableType::Byte => {
                data.byte_value = self.as_int() as u8;
            }
            VariableType::Word => {
                data.word_value = self.as_int() as u16;
            }
            VariableType::SByte => {
                data.sbyte_value = self.as_int() as i8;
            }
            VariableType::SWord => {
                data.sword_value = self.as_int() as i16;
            }
            VariableType::Float => {
                data.float_value = self.as_string().parse().unwrap();
            }
            VariableType::Double => {
                data.double_value = self.as_string().parse().unwrap();
            }
            VariableType::DDate => {
                data.ddate_value = self.as_int();
            }
            VariableType::Function | VariableType::Procedure | VariableType::Unknown => {
                panic!("Unknown variable type")
            }
        }

        VariableValue::new(convert_to_type, data)
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
