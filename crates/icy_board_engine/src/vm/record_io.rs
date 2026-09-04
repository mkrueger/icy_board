use std::io::{Cursor, Read};

use crate::executable::{GenericVariableData, VariableData, VariableType, VariableValue};

pub const MAX_RECORD_FRAME: usize = 16 * 1024 * 1024;

pub fn is_record(value: &VariableValue) -> bool {
    matches!(value.vtype, VariableType::UserData(id) if crate::parser::is_user_declared_type(id))
        && matches!(value.generic_data, GenericVariableData::Record(_))
}

pub fn encode_lines(value: &VariableValue) -> Result<Vec<String>, String> {
    if !is_record(value) {
        return Err("a user-defined record is required".to_string());
    }
    let mut lines = Vec::new();
    walk_encode(value, &mut |leaf| lines.push(encode_text_scalar(leaf)))?;
    Ok(lines)
}

pub fn decode_lines(template: &VariableValue, lines: &[String]) -> Result<VariableValue, String> {
    if !is_record(template) {
        return Err("a user-defined record is required".to_string());
    }
    let mut lines = lines.iter();
    let value = decode_text_value(template, &mut lines)?;
    if lines.next().is_some() {
        return Err("record has too many fields".to_string());
    }
    Ok(value)
}

pub fn line_count(value: &VariableValue) -> Result<usize, String> {
    if !is_record(value) {
        return Err("a user-defined record is required".to_string());
    }
    let mut count = 0usize;
    walk_encode(value, &mut |_| count += 1)?;
    Ok(count)
}

pub fn encode_binary(value: &VariableValue) -> Result<Vec<u8>, String> {
    if !is_record(value) {
        return Err("a user-defined record is required".to_string());
    }
    let mut payload = Vec::new();
    walk_encode(value, &mut |leaf| encode_binary_scalar(leaf, &mut payload))?;
    if payload.len() > MAX_RECORD_FRAME {
        return Err("record exceeds the 16 MiB frame limit".to_string());
    }
    let mut framed = Vec::with_capacity(payload.len() + 4);
    framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    framed.extend_from_slice(&payload);
    Ok(framed)
}

pub fn decode_binary(template: &VariableValue, payload: &[u8]) -> Result<VariableValue, String> {
    if !is_record(template) {
        return Err("a user-defined record is required".to_string());
    }
    let mut cursor = Cursor::new(payload);
    let value = decode_binary_value(template, &mut cursor)?;
    if cursor.position() as usize != payload.len() {
        return Err("binary record has trailing payload bytes".to_string());
    }
    Ok(value)
}

fn walk_encode<E>(value: &VariableValue, leaf: &mut E) -> Result<(), String>
where
    E: FnMut(&VariableValue),
{
    match &value.generic_data {
        GenericVariableData::Record(values) => {
            for value in values.iter() {
                walk_encode(value, leaf)?;
            }
        }
        GenericVariableData::Dim1(values) => {
            for value in values.iter() {
                walk_encode(value, leaf)?;
            }
        }
        GenericVariableData::Dim2(values) => {
            for value in values.iter().flatten() {
                walk_encode(value, leaf)?;
            }
        }
        GenericVariableData::Dim3(values) => {
            for value in values.iter().flatten().flatten() {
                walk_encode(value, leaf)?;
            }
        }
        GenericVariableData::None | GenericVariableData::String(_) => {
            ensure_scalar_type(value.vtype)?;
            leaf(value);
        }
        _ => return Err(format!("{} cannot be stored in a record file", value.vtype)),
    }
    Ok(())
}

fn map_shape<F>(template: &VariableValue, scalar: &mut F) -> Result<VariableValue, String>
where
    F: FnMut(&VariableValue) -> Result<VariableValue, String>,
{
    let generic_data = match &template.generic_data {
        GenericVariableData::Record(values) => GenericVariableData::Record(std::sync::Arc::new(
            values.iter().map(|value| map_shape(value, scalar)).collect::<Result<_, _>>()?,
        )),
        GenericVariableData::Dim1(values) => GenericVariableData::Dim1(std::sync::Arc::new(
            values.iter().map(|value| map_shape(value, scalar)).collect::<Result<_, _>>()?,
        )),
        GenericVariableData::Dim2(values) => GenericVariableData::Dim2(std::sync::Arc::new(
            values
                .iter()
                .map(|row| row.iter().map(|value| map_shape(value, scalar)).collect::<Result<_, _>>())
                .collect::<Result<_, _>>()?,
        )),
        GenericVariableData::Dim3(values) => GenericVariableData::Dim3(std::sync::Arc::new(
            values
                .iter()
                .map(|plane| {
                    plane
                        .iter()
                        .map(|row| row.iter().map(|value| map_shape(value, scalar)).collect::<Result<_, _>>())
                        .collect::<Result<_, _>>()
                })
                .collect::<Result<_, _>>()?,
        )),
        GenericVariableData::None | GenericVariableData::String(_) => return scalar(template),
        _ => return Err(format!("{} cannot be read from a record file", template.vtype)),
    };
    Ok(VariableValue {
        vtype: template.vtype,
        data: VariableData::default(),
        generic_data,
    })
}

fn decode_text_value<'a, I>(template: &VariableValue, lines: &mut I) -> Result<VariableValue, String>
where
    I: Iterator<Item = &'a String>,
{
    map_shape(template, &mut |leaf| {
        let line = lines.next().ok_or_else(|| "record is truncated".to_string())?;
        decode_text_scalar(leaf, line)
    })
}

fn escape_text(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\r' => result.push_str("\\r"),
            '\n' => result.push_str("\\n"),
            '\0' => result.push_str("\\0"),
            _ => result.push(ch),
        }
    }
    result
}

fn unescape_text(value: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => result.push('\\'),
            Some('r') => result.push('\r'),
            Some('n') => result.push('\n'),
            Some('0') => result.push('\0'),
            Some(other) => return Err(format!("unknown escape \\{other}")),
            None => return Err("trailing backslash in string field".to_string()),
        }
    }
    Ok(result)
}

fn encode_text_scalar(value: &VariableValue) -> String {
    match value.vtype {
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => escape_text(&value.as_string()),
        VariableType::MessageAreaID => {
            let (conference, area) = value.as_msg_id();
            format!("{conference},{area}")
        }
        VariableType::Float => value.as_float().to_string(),
        VariableType::Double => value.as_double().to_string(),
        VariableType::Unsigned => value.as_unsigned().to_string(),
        VariableType::Long => value.as_long().to_string(),
        VariableType::ULong => value.as_ulong().to_string(),
        _ => value.as_int().to_string(),
    }
}

fn decode_text_scalar(template: &VariableValue, text: &str) -> Result<VariableValue, String> {
    let invalid = || format!("invalid {} value {text:?}", template.vtype);
    let data = match template.vtype {
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
            return Ok(VariableValue {
                vtype: template.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::String(std::sync::Arc::new(unescape_text(text)?)),
            });
        }
        VariableType::Boolean => VariableData::from_bool(match text {
            "0" => false,
            "1" => true,
            _ => return Err(invalid()),
        }),
        VariableType::Unsigned => VariableData {
            unsigned_value: text.parse::<u32>().map_err(|_| invalid())? as u64,
        },
        VariableType::Long => VariableData {
            long_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::ULong => VariableData {
            ulong_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::Float => VariableData {
            float_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::Double => VariableData {
            double_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::Byte => VariableData {
            byte_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::SByte => VariableData {
            sbyte_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::Word | VariableType::Date | VariableType::EDate => VariableData {
            word_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::SWord => VariableData {
            sword_value: text.parse().map_err(|_| invalid())?,
        },
        VariableType::Integer | VariableType::Money | VariableType::Time | VariableType::DDate => VariableData::from_int(text.parse().map_err(|_| invalid())?),
        VariableType::MessageAreaID => {
            let (conference, area) = text.split_once(',').ok_or_else(invalid)?;
            VariableData {
                message_id_value: crate::executable::MsgAreaIdValue {
                    conference: conference.parse().map_err(|_| invalid())?,
                    area: area.parse().map_err(|_| invalid())?,
                },
            }
        }
        other => return Err(format!("{other} cannot be read from a record file")),
    };
    Ok(VariableValue::new(template.vtype, data))
}

fn encode_binary_scalar(value: &VariableValue, output: &mut Vec<u8>) {
    match value.vtype {
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
            let bytes = value.as_string().into_bytes();
            output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            output.extend_from_slice(&bytes);
        }
        VariableType::Boolean | VariableType::Byte => output.push(value.as_byte()),
        VariableType::SByte => output.push(value.as_sbyte() as u8),
        VariableType::Word | VariableType::Date | VariableType::EDate => output.extend_from_slice(&value.as_word().to_le_bytes()),
        VariableType::SWord => output.extend_from_slice(&value.as_sword().to_le_bytes()),
        VariableType::Unsigned => output.extend_from_slice(&(value.as_unsigned() as u32).to_le_bytes()),
        VariableType::Integer | VariableType::Money | VariableType::Time | VariableType::DDate => output.extend_from_slice(&value.as_int().to_le_bytes()),
        VariableType::Float => output.extend_from_slice(&value.as_float().to_le_bytes()),
        VariableType::Double => output.extend_from_slice(&value.as_double().to_le_bytes()),
        VariableType::Long => output.extend_from_slice(&value.as_long().to_le_bytes()),
        VariableType::ULong => output.extend_from_slice(&value.as_ulong().to_le_bytes()),
        VariableType::MessageAreaID => {
            let (conference, area) = value.as_msg_id();
            output.extend_from_slice(&conference.to_le_bytes());
            output.extend_from_slice(&area.to_le_bytes());
        }
        _ => unreachable!("scalar type was validated before encoding"),
    }
}

fn decode_binary_value(template: &VariableValue, input: &mut Cursor<&[u8]>) -> Result<VariableValue, String> {
    map_shape(template, &mut |leaf| decode_binary_scalar(leaf, input))
}

fn read_exact<const N: usize>(input: &mut Cursor<&[u8]>) -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    input.read_exact(&mut bytes).map_err(|_| "binary record is truncated".to_string())?;
    Ok(bytes)
}

fn decode_binary_scalar(template: &VariableValue, input: &mut Cursor<&[u8]>) -> Result<VariableValue, String> {
    let data = match template.vtype {
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
            let length = u32::from_le_bytes(read_exact(input)?) as usize;
            if length > MAX_RECORD_FRAME {
                return Err("string exceeds the 16 MiB frame limit".to_string());
            }
            let mut bytes = vec![0; length];
            input.read_exact(&mut bytes).map_err(|_| "binary string is truncated".to_string())?;
            let text = String::from_utf8(bytes).map_err(|_| "binary string is not UTF-8".to_string())?;
            return Ok(VariableValue {
                vtype: template.vtype,
                data: VariableData::default(),
                generic_data: GenericVariableData::String(std::sync::Arc::new(text)),
            });
        }
        VariableType::Boolean => {
            let value = read_exact::<1>(input)?[0];
            if value > 1 {
                return Err(format!("invalid BOOLEAN value {value}"));
            }
            VariableData::from_bool(value != 0)
        }
        VariableType::Byte => VariableData {
            byte_value: read_exact::<1>(input)?[0],
        },
        VariableType::SByte => VariableData {
            sbyte_value: read_exact::<1>(input)?[0] as i8,
        },
        VariableType::Word | VariableType::Date | VariableType::EDate => VariableData {
            word_value: u16::from_le_bytes(read_exact(input)?),
        },
        VariableType::SWord => VariableData {
            sword_value: i16::from_le_bytes(read_exact(input)?),
        },
        VariableType::Unsigned => VariableData {
            unsigned_value: u32::from_le_bytes(read_exact(input)?) as u64,
        },
        VariableType::Integer | VariableType::Money | VariableType::Time | VariableType::DDate => {
            VariableData::from_int(i32::from_le_bytes(read_exact(input)?))
        }
        VariableType::Float => VariableData {
            float_value: f32::from_le_bytes(read_exact(input)?),
        },
        VariableType::Double => VariableData {
            double_value: f64::from_le_bytes(read_exact(input)?),
        },
        VariableType::Long => VariableData {
            long_value: i64::from_le_bytes(read_exact(input)?),
        },
        VariableType::ULong => VariableData {
            ulong_value: u64::from_le_bytes(read_exact(input)?),
        },
        VariableType::MessageAreaID => VariableData {
            message_id_value: crate::executable::MsgAreaIdValue {
                conference: i32::from_le_bytes(read_exact(input)?),
                area: i32::from_le_bytes(read_exact(input)?),
            },
        },
        other => return Err(format!("{other} cannot be read from a binary record")),
    };
    Ok(VariableValue::new(template.vtype, data))
}

fn ensure_scalar_type(variable_type: VariableType) -> Result<(), String> {
    match variable_type {
        VariableType::Boolean
        | VariableType::Unsigned
        | VariableType::Date
        | VariableType::EDate
        | VariableType::Integer
        | VariableType::Money
        | VariableType::Float
        | VariableType::String
        | VariableType::Time
        | VariableType::Byte
        | VariableType::Word
        | VariableType::SByte
        | VariableType::SWord
        | VariableType::BigStr
        | VariableType::UnboundedString
        | VariableType::Double
        | VariableType::DDate
        | VariableType::MessageAreaID
        | VariableType::Long
        | VariableType::ULong => Ok(()),
        _ => Err(format!("{variable_type} cannot be stored in a record file")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_area_ids_round_trip_through_both_codecs() {
        let template = VariableValue::new_msg_id(0, 0);
        let value = VariableValue::new_msg_id(2, 3);
        let record = |field| VariableValue {
            vtype: VariableType::UserData(crate::parser::FIRST_USER_TYPE_ID as u8),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(std::sync::Arc::new(vec![field])),
        };

        let source = record(value);
        let empty = record(template);
        let lines = encode_lines(&source).unwrap();
        let from_text = decode_lines(&empty, &lines).unwrap();
        let binary = encode_binary(&source).unwrap();
        let from_binary = decode_binary(&empty, &binary[4..]).unwrap();

        let GenericVariableData::Record(text_fields) = from_text.generic_data else {
            panic!("record expected");
        };
        let GenericVariableData::Record(binary_fields) = from_binary.generic_data else {
            panic!("record expected");
        };
        assert_eq!(text_fields[0].as_msg_id(), (2, 3));
        assert_eq!(binary_fields[0].as_msg_id(), (2, 3));
    }

    #[test]
    fn binary_booleans_accept_only_zero_or_one() {
        let template = VariableValue {
            vtype: VariableType::UserData(crate::parser::FIRST_USER_TYPE_ID as u8),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(std::sync::Arc::new(vec![VariableValue::new_bool(false)])),
        };

        assert_eq!(decode_binary(&template, &[2]).unwrap_err(), "invalid BOOLEAN value 2");
    }
}
