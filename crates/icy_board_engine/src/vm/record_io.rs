use std::io::{Cursor, Read};

use crate::executable::{GenericVariableData, VariableData, VariableTable, VariableType, VariableValue};

pub const MAX_RECORD_FRAME: usize = 16 * 1024 * 1024;

pub fn is_record(value: &VariableValue) -> bool {
    matches!(value.vtype, VariableType::UserData(id) if crate::parser::is_user_declared_type(id))
        && matches!(value.generic_data, GenericVariableData::Record(_))
}

pub fn ensure_supported(value: &VariableValue, table: &VariableTable) -> Result<(), String> {
    if !is_record(value) {
        return Err("a user-defined record is required".to_string());
    }
    if let VariableType::UserData(id) = value.vtype
        && table.record_io_unsupported_types.contains(&id)
    {
        return Err(format!("{} contains fields that cannot be stored in a record file", value.vtype));
    }
    Ok(())
}

pub fn encode_lines(value: &VariableValue, table: &VariableTable) -> Result<Vec<String>, String> {
    ensure_supported(value, table)?;
    let mut lines = Vec::new();
    walk_encode(value, &mut |leaf| lines.push(encode_text_scalar(leaf)))?;
    Ok(lines)
}

pub fn decode_lines(template: &VariableValue, lines: &[String], table: &VariableTable) -> Result<VariableValue, String> {
    ensure_supported(template, table)?;
    let mut lines = lines.iter();
    let value = decode_text_value(template, &mut lines, table)?;
    if lines.next().is_some() {
        return Err("record has too many fields".to_string());
    }
    Ok(value)
}

pub fn line_count(value: &VariableValue, table: &VariableTable) -> Result<usize, String> {
    ensure_supported(value, table)?;
    let mut count = 0usize;
    walk_encode(value, &mut |_| count += 1)?;
    Ok(count)
}

pub fn encode_binary(value: &VariableValue, table: &VariableTable) -> Result<Vec<u8>, String> {
    ensure_supported(value, table)?;
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

pub fn decode_binary(template: &VariableValue, payload: &[u8], table: &VariableTable) -> Result<VariableValue, String> {
    ensure_supported(template, table)?;
    let mut cursor = Cursor::new(payload);
    let value = decode_binary_value(template, &mut cursor, table)?;
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
        GenericVariableData::Enum(_) => leaf(value),
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
        GenericVariableData::None | GenericVariableData::String(_) | GenericVariableData::Enum(_) => return scalar(template),
        _ => return Err(format!("{} cannot be read from a record file", template.vtype)),
    };
    Ok(VariableValue {
        vtype: template.vtype,
        data: VariableData::default(),
        generic_data,
    })
}

fn decode_text_value<'a, I>(template: &VariableValue, lines: &mut I, table: &VariableTable) -> Result<VariableValue, String>
where
    I: Iterator<Item = &'a String>,
{
    map_shape(template, &mut |leaf| {
        let line = lines.next().ok_or_else(|| "record is truncated".to_string())?;
        decode_text_scalar(leaf, line, table)
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

fn decode_text_scalar(template: &VariableValue, text: &str, table: &VariableTable) -> Result<VariableValue, String> {
    let invalid = || format!("invalid {} value {text:?}", template.vtype);
    if matches!(template.generic_data, GenericVariableData::Enum(_)) {
        let number = text.parse::<i32>().map_err(|_| invalid())?;
        return decode_enum_scalar(template, number, table);
    }
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
        VariableType::Word | VariableType::Date | VariableType::EDate => VariableData::from_int(i32::from(text.parse::<u16>().map_err(|_| invalid())?)),
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
    if matches!(value.generic_data, GenericVariableData::Enum(_)) {
        output.extend_from_slice(&value.as_int().to_le_bytes());
        return;
    }
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

fn decode_binary_value(template: &VariableValue, input: &mut Cursor<&[u8]>, table: &VariableTable) -> Result<VariableValue, String> {
    map_shape(template, &mut |leaf| decode_binary_scalar(leaf, input, table))
}

fn decode_enum_scalar(template: &VariableValue, number: i32, table: &VariableTable) -> Result<VariableValue, String> {
    // Restore each leaf's nominal type and default before publishing the record.
    if !table.is_enum(template.vtype) {
        return Err(format!("missing enum metadata for {}", template.vtype));
    }
    table
        .checked_enum_value(template.vtype, VariableValue::new_int(number))
        .map_err(|error| error.to_string())
}

fn read_exact<const N: usize>(input: &mut Cursor<&[u8]>) -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    input.read_exact(&mut bytes).map_err(|_| "binary record is truncated".to_string())?;
    Ok(bytes)
}

fn decode_binary_scalar(template: &VariableValue, input: &mut Cursor<&[u8]>, table: &VariableTable) -> Result<VariableValue, String> {
    if matches!(template.generic_data, GenericVariableData::Enum(_)) {
        return decode_enum_scalar(template, i32::from_le_bytes(read_exact(input)?), table);
    }
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
        VariableType::Word | VariableType::Date | VariableType::EDate => VariableData::from_int(i32::from(u16::from_le_bytes(read_exact(input)?))),
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

    fn enum_record() -> (VariableTable, VariableValue) {
        let record_id = crate::parser::FIRST_USER_TYPE_ID as u32;
        let enum_id = record_id + 1;
        let mut table = VariableTable::default();
        table.enums.insert(enum_id, vec![7, -3, 7]);
        let value = VariableValue {
            vtype: VariableType::UserData(record_id),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(std::sync::Arc::new(vec![VariableValue::new_enum(VariableType::UserData(enum_id), -3, 7)])),
        };
        (table, value)
    }

    #[test]
    fn enum_record_wire_format_is_signed_i32_and_decoding_restores_the_default() {
        let (table, value) = enum_record();
        assert_eq!(1, line_count(&value, &table).unwrap());
        assert_eq!(vec!["-3"], encode_lines(&value, &table).unwrap());
        assert_eq!(vec![4, 0, 0, 0, 253, 255, 255, 255], encode_binary(&value, &table).unwrap());
        for decoded in [
            decode_lines(&value, &["-3".to_string()], &table).unwrap(),
            decode_binary(&value, &[253, 255, 255, 255], &table).unwrap(),
        ] {
            let GenericVariableData::Record(fields) = decoded.generic_data else {
                panic!("expected a record")
            };
            assert_eq!(-3, fields[0].as_int());
            assert!(matches!(fields[0].generic_data, GenericVariableData::Enum(7)));
            assert_eq!(7, fields[0].emptied().as_int());
        }
    }

    #[test]
    fn enum_record_decoding_requires_type_metadata_and_well_formed_leaves() {
        let (table, value) = enum_record();
        for number in [0i32, 8, i32::MIN, i32::MAX] {
            let lines = vec![number.to_string()];
            let decoded = decode_lines(&value, &lines, &table).unwrap();
            assert_eq!(lines, encode_lines(&decoded, &table).unwrap());
            let decoded = decode_binary(&value, &number.to_le_bytes(), &table).unwrap();
            assert_eq!(lines, encode_lines(&decoded, &table).unwrap());
        }
        for text in ["2147483648", "-2147483649", "1.5", "Shade.First"] {
            assert!(decode_lines(&value, &[text.to_string()], &table).is_err(), "{text}");
        }
        for payload in [&[253, 255, 255][..], &[253, 255, 255, 255, 0]] {
            assert!(decode_binary(&value, payload, &table).is_err(), "{payload:?}");
        }
        let missing = VariableTable::default();
        assert!(
            decode_lines(&value, &["-3".to_string()], &missing)
                .unwrap_err()
                .contains("missing enum metadata")
        );
        assert!(
            decode_binary(&value, &[253, 255, 255, 255], &missing)
                .unwrap_err()
                .contains("missing enum metadata")
        );
    }

    #[test]
    fn message_area_ids_round_trip_through_both_codecs() {
        let template = VariableValue::new_msg_id(0, 0);
        let value = VariableValue::new_msg_id(2, 3);
        let record = |field| VariableValue {
            vtype: VariableType::UserData(crate::parser::FIRST_USER_TYPE_ID as u32),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(std::sync::Arc::new(vec![field])),
        };

        let source = record(value);
        let empty = record(template);
        let lines = encode_lines(&source, &VariableTable::default()).unwrap();
        let from_text = decode_lines(&empty, &lines, &VariableTable::default()).unwrap();
        let binary = encode_binary(&source, &VariableTable::default()).unwrap();
        let from_binary = decode_binary(&empty, &binary[4..], &VariableTable::default()).unwrap();

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
    fn s1_layout_guards_reject_host_and_dynamic_fields_even_without_leaves() {
        use crate::executable::{RecordField, create_record_value};

        for field in [
            RecordField::scalar(VariableType::UserData(crate::parser::CONTACT_ID as u32)),
            RecordField::scalar(VariableType::UserData(30)),
            RecordField {
                dim: 1,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
            RecordField {
                dim: 2,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
            RecordField {
                dim: 3,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
        ] {
            let layouts = vec![vec![field], vec![RecordField::scalar(VariableType::UserData(100))]];
            let mut table = VariableTable::default();
            table.fill_in_records(&layouts);
            for type_id in [100, 101] {
                assert!(table.record_io_unsupported_types.contains(&type_id));
                let value = create_record_value(type_id, &layouts, &table.enums).unwrap();
                // A malformed empty record must not hide its declared layout either.
                let empty = VariableValue {
                    generic_data: GenericVariableData::Record(std::sync::Arc::new(Vec::new())),
                    ..value.clone()
                };
                for value in [value, empty] {
                    assert!(ensure_supported(&value, &table).unwrap_err().contains("cannot be stored"));
                    assert!(line_count(&value, &table).is_err());
                    assert!(encode_lines(&value, &table).is_err());
                    assert!(encode_binary(&value, &table).is_err());
                    assert!(decode_lines(&value, &[], &table).is_err());
                    assert!(decode_binary(&value, &[], &table).is_err());
                }
            }
        }
    }

    #[test]
    fn binary_booleans_accept_only_zero_or_one() {
        let template = VariableValue {
            vtype: VariableType::UserData(crate::parser::FIRST_USER_TYPE_ID as u32),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(std::sync::Arc::new(vec![VariableValue::new_bool(false)])),
        };

        assert_eq!(
            decode_binary(&template, &[2], &VariableTable::default()).unwrap_err(),
            "invalid BOOLEAN value 2"
        );
    }
}
