use std::collections::HashMap;

use crate::executable::VariableType;

static BUILT_IN_TYPE_LOOKUP: std::sync::LazyLock<HashMap<unicase::Ascii<String>, Vec<(VariableType, u16)>>> = std::sync::LazyLock::new(|| {
    let mut lookup = HashMap::new();
    for (name, variable_type, since) in BUILT_IN_TYPES {
        lookup
            .entry(unicase::Ascii::new((*name).to_string()))
            .or_insert_with(Vec::new)
            .push((*variable_type, *since));
    }
    lookup
});

/// Which language version gave each built-in type its name, from the PPL release
/// notes: 2.00 brought the numeric widths and the big string, 3.00 the `DBase` date.
static BUILT_IN_TYPES: &[(&str, VariableType, u16)] = &[
    ("INTEGER", VariableType::Integer, 100),
    ("STRING", VariableType::String, 100),
    ("STRING", VariableType::UnboundedString, 400),
    ("BOOLEAN", VariableType::Boolean, 100),
    ("DATE", VariableType::Date, 100),
    ("TIME", VariableType::Time, 100),
    ("MONEY", VariableType::Money, 100),
    ("SDWORD", VariableType::Integer, 200),
    ("LONG", VariableType::Integer, 200),
    ("LONG", VariableType::Long, 400),
    ("ULONG", VariableType::ULong, 400),
    ("BIGSTR", VariableType::BigStr, 200),
    ("EDATE", VariableType::EDate, 200),
    ("WORD", VariableType::Word, 200),
    ("UWORD", VariableType::Word, 200),
    ("SWORD", VariableType::SWord, 200),
    ("INT", VariableType::SWord, 200),
    ("BYTE", VariableType::Byte, 200),
    ("UBYTE", VariableType::Byte, 200),
    ("UNSIGNED", VariableType::Unsigned, 200),
    ("DWORD", VariableType::Unsigned, 200),
    ("UDWORD", VariableType::Unsigned, 200),
    ("SBYTE", VariableType::SByte, 200),
    ("SHORT", VariableType::SByte, 200),
    ("REAL", VariableType::Float, 200),
    ("FLOAT", VariableType::Float, 200),
    ("DOUBLE", VariableType::Double, 200),
    ("DREAL", VariableType::Double, 200),
    ("DDATE", VariableType::DDate, 300),
    ("MSGAREAID", VariableType::MessageAreaID, 400),
    ("BYTES", VariableType::Bytes, 400),
];

/// The built-in type that name stands for, or nothing if the language did not have
/// it yet.
pub fn built_in_type(name: &unicase::Ascii<String>, lang_version: u16) -> Option<VariableType> {
    BUILT_IN_TYPE_LOOKUP
        .get(name)
        .and_then(|versions| versions.iter().rev().find(|(_, since)| *since <= lang_version))
        .map(|(variable_type, _)| *variable_type)
}

/// The type names a program written for that language version may use.
pub fn built_in_type_names(lang_version: u16) -> Vec<&'static str> {
    let mut names: Vec<_> = BUILT_IN_TYPES
        .iter()
        .filter(|(_, _, since)| *since <= lang_version)
        .map(|(name, _, _)| *name)
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}
