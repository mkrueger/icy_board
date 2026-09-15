use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, SecondsFormat, Timelike, Utc};

use super::{GenericVariableData, VariableType, VariableValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum TemporalOp {
    Today = 0,
    TimeNow = 1,
    TimestampNow = 2,
    ParseDate = 3,
    ParseTime = 4,
    ParseTimestamp = 5,
    CreateDate = 6,
    CreateTime = 7,
    FromUtc = 8,
    FromUnix = 9,
    IsEmpty = 10,
    Year = 11,
    Month = 12,
    Day = 13,
    DayOfWeek = 14,
    Hour = 15,
    Minute = 16,
    Second = 17,
    Nanosecond = 18,
    UtcDate = 19,
    UtcTime = 20,
    UnixSeconds = 21,
    Format = 22,
    WithYear = 23,
    AddDays = 24,
    AddSeconds = 25,
    DaysUntil = 26,
    SecondsUntil = 27,
    LegacyDate = 28,
    LegacyTime = 29,
}

pub const TEMPORAL_OPS: &[TemporalOp] = &[
    TemporalOp::Today,
    TemporalOp::TimeNow,
    TemporalOp::TimestampNow,
    TemporalOp::ParseDate,
    TemporalOp::ParseTime,
    TemporalOp::ParseTimestamp,
    TemporalOp::CreateDate,
    TemporalOp::CreateTime,
    TemporalOp::FromUtc,
    TemporalOp::FromUnix,
    TemporalOp::IsEmpty,
    TemporalOp::Year,
    TemporalOp::Month,
    TemporalOp::Day,
    TemporalOp::DayOfWeek,
    TemporalOp::Hour,
    TemporalOp::Minute,
    TemporalOp::Second,
    TemporalOp::Nanosecond,
    TemporalOp::UtcDate,
    TemporalOp::UtcTime,
    TemporalOp::UnixSeconds,
    TemporalOp::Format,
    TemporalOp::WithYear,
    TemporalOp::AddDays,
    TemporalOp::AddSeconds,
    TemporalOp::DaysUntil,
    TemporalOp::SecondsUntil,
    TemporalOp::LegacyDate,
    TemporalOp::LegacyTime,
];

pub struct TemporalMember {
    pub name: &'static str,
    pub operation: TemporalOp,
    pub arguments: &'static [VariableType],
    pub result: VariableType,
    pub property: bool,
    pub is_static: bool,
}

pub fn temporal_members(typ: VariableType) -> Vec<TemporalMember> {
    use TemporalOp as Op;
    use VariableType as V;
    let mut members = vec![
        TemporalMember {
            name: "IsEmpty",
            operation: Op::IsEmpty,
            arguments: &[],
            result: V::Boolean,
            property: true,
            is_static: false,
        },
        TemporalMember {
            name: "Format",
            operation: Op::Format,
            arguments: &[V::UnboundedString],
            result: V::UnboundedString,
            property: false,
            is_static: false,
        },
    ];
    let specific: &[(&str, Op, &[V], V, bool, bool)] = match typ {
        V::CalendarDate => &[
            ("ToLegacy", Op::LegacyDate, &[], V::Date, false, false),
            ("Today", Op::Today, &[], V::CalendarDate, false, true),
            ("Parse", Op::ParseDate, &[V::UnboundedString], V::CalendarDate, false, true),
            ("Create", Op::CreateDate, &[V::Integer, V::Integer, V::Integer], V::CalendarDate, false, true),
            ("Year", Op::Year, &[], V::Integer, true, false),
            ("Month", Op::Month, &[], V::Integer, true, false),
            ("Day", Op::Day, &[], V::Integer, true, false),
            ("DayOfWeek", Op::DayOfWeek, &[], V::Integer, true, false),
            ("WithYear", Op::WithYear, &[V::Integer], V::CalendarDate, false, false),
            ("AddDays", Op::AddDays, &[V::Long], V::CalendarDate, false, false),
            ("DaysUntil", Op::DaysUntil, &[V::CalendarDate], V::Long, false, false),
        ],
        V::ClockTime => &[
            ("ToLegacy", Op::LegacyTime, &[], V::Time, false, false),
            ("Now", Op::TimeNow, &[], V::ClockTime, false, true),
            ("Parse", Op::ParseTime, &[V::UnboundedString], V::ClockTime, false, true),
            ("Create", Op::CreateTime, &[V::Integer, V::Integer, V::Integer], V::ClockTime, false, true),
            ("Hour", Op::Hour, &[], V::Integer, true, false),
            ("Minute", Op::Minute, &[], V::Integer, true, false),
            ("Second", Op::Second, &[], V::Integer, true, false),
            ("Nanosecond", Op::Nanosecond, &[], V::Integer, true, false),
        ],
        V::Timestamp => &[
            ("Now", Op::TimestampNow, &[], V::Timestamp, false, true),
            ("Parse", Op::ParseTimestamp, &[V::UnboundedString], V::Timestamp, false, true),
            ("FromUtc", Op::FromUtc, &[V::CalendarDate, V::ClockTime], V::Timestamp, false, true),
            ("FromUnix", Op::FromUnix, &[V::Long], V::Timestamp, false, true),
            ("UtcDate", Op::UtcDate, &[], V::CalendarDate, true, false),
            ("UtcTime", Op::UtcTime, &[], V::ClockTime, true, false),
            ("UnixSeconds", Op::UnixSeconds, &[], V::Long, true, false),
            ("Nanosecond", Op::Nanosecond, &[], V::Integer, true, false),
            ("AddSeconds", Op::AddSeconds, &[V::Long], V::Timestamp, false, false),
            ("SecondsUntil", Op::SecondsUntil, &[V::Timestamp], V::Long, false, false),
        ],
        _ => return Vec::new(),
    };
    members.extend(
        specific
            .iter()
            .map(|&(name, operation, arguments, result, property, is_static)| TemporalMember {
                name,
                operation,
                arguments,
                result,
                property,
                is_static,
            }),
    );
    members
}

pub fn temporal_builtin(name: &str) -> Option<(TemporalOp, &'static [VariableType], VariableType)> {
    use TemporalOp as Op;
    use VariableType as V;
    Some(match name.to_ascii_uppercase().as_str() {
        "DATE" => (Op::Today, &[], V::CalendarDate),
        "TIME" => (Op::TimeNow, &[], V::ClockTime),
        "TIMESTAMP" => (Op::TimestampNow, &[], V::Timestamp),
        "TODATE" => (Op::ParseDate, &[V::None], V::CalendarDate),
        "TOTIME" => (Op::ParseTime, &[V::None], V::ClockTime),
        "MKDATE" => (Op::CreateDate, &[V::Integer, V::Integer, V::Integer], V::CalendarDate),
        "YEAR" => (Op::Year, &[V::CalendarDate], V::Integer),
        "MONTH" => (Op::Month, &[V::CalendarDate], V::Integer),
        "DAY" => (Op::Day, &[V::CalendarDate], V::Integer),
        "DOW" => (Op::DayOfWeek, &[V::CalendarDate], V::Integer),
        "HOUR" => (Op::Hour, &[V::ClockTime], V::Integer),
        "MIN" => (Op::Minute, &[V::ClockTime], V::Integer),
        "SEC" => (Op::Second, &[V::ClockTime], V::Integer),
        _ => return None,
    })
}

impl TemporalOp {
    pub fn evaluate(self, receiver: &VariableValue, args: &[VariableValue]) -> Result<VariableValue, String> {
        use TemporalValue as T;
        let arg = |index: usize| args.get(index).ok_or_else(|| "Missing temporal argument".to_string());
        let date = |value: &VariableValue| match value.temporal() {
            Some(T::Date(Some(value))) => Ok(value),
            _ => Err("A nonempty DATE is required".to_string()),
        };
        let time = |value: &VariableValue| match value.temporal() {
            Some(T::Time(Some(value))) => Ok(value),
            _ => Err("A nonempty TIME is required".to_string()),
        };
        let stamp = |value: &VariableValue| match value.temporal() {
            Some(T::Timestamp(Some(value))) => Ok(value),
            _ => Err("A nonempty TIMESTAMP is required".to_string()),
        };
        let number = |index| -> Result<i64, String> {
            let value = arg(index)?;
            if !matches!(
                value.vtype,
                VariableType::Integer | VariableType::Long | VariableType::Byte | VariableType::SByte | VariableType::Word | VariableType::SWord
            ) {
                return Err("An integer is required".into());
            }
            Ok(value.as_long())
        };
        let integer = |index| i32::try_from(number(index)?).map_err(|_| "Integer overflow".to_string());
        let unsigned = |index| u32::try_from(number(index)?).map_err(|_| "Invalid negative component".to_string());
        let part_receiver = if receiver.vtype.is_temporal() {
            receiver
        } else {
            args.first().unwrap_or(receiver)
        };
        let value = match self {
            Self::Today => T::Date(Some(chrono::Local::now().date_naive())),
            Self::TimeNow => T::Time(Some(chrono::Local::now().time())),
            Self::TimestampNow => T::Timestamp(Some(Utc::now())),
            Self::ParseDate => return arg(0)?.try_temporal_conversion(VariableType::CalendarDate),
            Self::ParseTime => return arg(0)?.try_temporal_conversion(VariableType::ClockTime),
            Self::ParseTimestamp => return arg(0)?.try_temporal_conversion(VariableType::Timestamp),
            Self::CreateDate => T::Date(Some(
                NaiveDate::from_ymd_opt(integer(0)?, unsigned(1)?, unsigned(2)?).ok_or("Invalid calendar date")?,
            )),
            Self::CreateTime => T::Time(Some(
                NaiveTime::from_hms_opt(unsigned(0)?, unsigned(1)?, unsigned(2)?).ok_or("Invalid clock time")?,
            )),
            Self::FromUtc => T::Timestamp(Some(date(arg(0)?)?.and_time(time(arg(1)?)?).and_utc())),
            Self::FromUnix => T::Timestamp(Some(DateTime::from_timestamp(number(0)?, 0).ok_or("Timestamp out of range")?)),
            Self::IsEmpty => return Ok(VariableValue::new_bool(receiver.temporal().ok_or("Temporal receiver required")?.is_empty())),
            Self::Year => return Ok(VariableValue::new_int(date(part_receiver)?.year())),
            Self::Month => return Ok(VariableValue::new_int(date(part_receiver)?.month() as i32)),
            Self::Day => return Ok(VariableValue::new_int(date(part_receiver)?.day() as i32)),
            Self::DayOfWeek => return Ok(VariableValue::new_int(date(part_receiver)?.weekday().num_days_from_sunday() as i32)),
            Self::Hour => return Ok(VariableValue::new_int(time(part_receiver)?.hour() as i32)),
            Self::Minute => return Ok(VariableValue::new_int(time(part_receiver)?.minute() as i32)),
            Self::Second => return Ok(VariableValue::new_int(time(part_receiver)?.second() as i32)),
            Self::Nanosecond => {
                return Ok(VariableValue::new_int(match receiver.temporal() {
                    Some(T::Time(Some(value))) => value.nanosecond(),
                    Some(T::Timestamp(Some(value))) => value.timestamp_subsec_nanos(),
                    _ => return Err("A nonempty TIME or TIMESTAMP is required".into()),
                } as i32));
            }
            Self::UtcDate => T::Date(Some(stamp(receiver)?.date_naive())),
            Self::UtcTime => T::Time(Some(stamp(receiver)?.time())),
            Self::UnixSeconds => return Ok(VariableValue::new_long(stamp(receiver)?.timestamp())),
            Self::Format => {
                use std::fmt::Write;
                let format = arg(0)?.as_string();
                if chrono::format::StrftimeItems::new(&format).any(|item| matches!(item, chrono::format::Item::Error)) {
                    return Err("Invalid date/time format".into());
                }
                let mut text = String::new();
                let result = match receiver.temporal() {
                    Some(T::Date(Some(value))) => write!(text, "{}", value.format(&format)),
                    Some(T::Time(Some(value))) => write!(text, "{}", value.format(&format)),
                    Some(T::Timestamp(Some(value))) => write!(text, "{}", value.format(&format)),
                    Some(value) if value.is_empty() => return Ok(VariableValue::new_unbounded_string(text)),
                    _ => return Err("Temporal receiver required".into()),
                };
                result.map_err(|_| "Format requires unavailable date/time components")?;
                return Ok(VariableValue::new_unbounded_string(text));
            }
            Self::WithYear => T::Date(Some(date(receiver)?.with_year(integer(0)?).ok_or("Invalid calendar date")?)),
            Self::AddDays => T::Date(Some(
                date(receiver)?
                    .checked_add_signed(chrono::TimeDelta::try_days(number(0)?).ok_or("Day count overflow")?)
                    .ok_or("Date out of range")?,
            )),
            Self::AddSeconds => T::Timestamp(Some(
                stamp(receiver)?
                    .checked_add_signed(chrono::TimeDelta::try_seconds(number(0)?).ok_or("Second count overflow")?)
                    .ok_or("Timestamp out of range")?,
            )),
            Self::DaysUntil => return Ok(VariableValue::new_long((date(arg(0)?)? - date(receiver)?).num_days())),
            Self::SecondsUntil => return Ok(VariableValue::new_long((stamp(arg(0)?)? - stamp(receiver)?).num_seconds())),
            Self::LegacyDate => {
                if receiver.temporal().is_some_and(TemporalValue::is_empty) {
                    return Ok(VariableValue::new_date(0));
                }
                let value = date(receiver)?;
                let raw = crate::datetime::IcbDate::new(
                    value.month() as u8,
                    value.day() as u8,
                    u16::try_from(value.year()).map_err(|_| "Legacy date out of range")?,
                )
                .to_pcboard_date();
                let legacy = VariableValue::new_date(raw);
                if legacy.try_temporal_conversion(VariableType::CalendarDate)? != *receiver {
                    return Err("Legacy date out of range".into());
                }
                return Ok(legacy);
            }
            Self::LegacyTime => {
                if receiver.temporal().is_some_and(TemporalValue::is_empty) {
                    return Ok(VariableValue::new_time(0));
                }
                let value = time(receiver)?;
                if value.nanosecond() != 0 {
                    return Err("Legacy TIME cannot preserve subsecond precision".into());
                }
                return Ok(VariableValue::new_time(value.num_seconds_from_midnight() as i32));
            }
        };
        Ok(VariableValue::new_temporal(value))
    }
}

impl VariableValue {
    pub fn checked_legacy_temporal_conversion(&self, target: VariableType) -> Result<Self, String> {
        if !self.vtype.is_temporal() {
            return self.clone().convert_to(target).map_err(|error| error.to_string());
        }
        let operation = match target {
            VariableType::Date | VariableType::EDate | VariableType::DDate => TemporalOp::LegacyDate,
            VariableType::Time => TemporalOp::LegacyTime,
            _ => return Err(format!("Cannot convert {} to {target}", self.vtype)),
        };
        operation.evaluate(self, &[])?.convert_to(target).map_err(|error| error.to_string())
    }

    pub fn checked_temporal_assignment(&self, target: VariableType) -> Result<Self, String> {
        let convert = |value: &Self| value.checked_temporal_assignment(target);
        let data = match &self.generic_data {
            GenericVariableData::Dim1(values) => GenericVariableData::Dim1(std::sync::Arc::new(values.iter().map(convert).collect::<Result<_, _>>()?)),
            GenericVariableData::Dim2(values) => GenericVariableData::Dim2(std::sync::Arc::new(
                values.iter().map(|row| row.iter().map(convert).collect()).collect::<Result<_, _>>()?,
            )),
            GenericVariableData::Dim3(values) => GenericVariableData::Dim3(std::sync::Arc::new(
                values
                    .iter()
                    .map(|plane| plane.iter().map(|row| row.iter().map(convert).collect()).collect())
                    .collect::<Result<_, _>>()?,
            )),
            _ => {
                if target.is_temporal() {
                    return self.try_temporal_conversion(target);
                }
                if matches!(
                    target,
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString | VariableType::Bytes
                ) {
                    return Ok(self.clone().convert_legacy(target));
                }
                return Err(format!("Cannot implicitly convert {} to {target}", self.vtype));
            }
        };
        Ok(Self {
            vtype: target,
            generic_data: data,
            ..Default::default()
        })
    }

    pub fn new_temporal(value: TemporalValue) -> Self {
        Self {
            vtype: match value {
                TemporalValue::Date(_) => VariableType::CalendarDate,
                TemporalValue::Time(_) => VariableType::ClockTime,
                TemporalValue::Timestamp(_) => VariableType::Timestamp,
            },
            generic_data: GenericVariableData::Temporal(value),
            ..Default::default()
        }
    }

    pub fn temporal(&self) -> Option<TemporalValue> {
        match self.generic_data {
            GenericVariableData::Temporal(value) => Some(value),
            GenericVariableData::None => self.vtype.empty_temporal(),
            _ => None,
        }
    }

    pub fn try_temporal_conversion(&self, target: VariableType) -> Result<Self, String> {
        let empty = target.empty_temporal().ok_or("Not a temporal type")?;
        if self.vtype == target {
            return Ok(self.clone());
        }
        let value = match (target, self.vtype) {
            (_, VariableType::Bytes) => empty.decode(self.as_byte_slice())?,
            (_, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) => empty.parse(&self.as_string())?,
            (VariableType::CalendarDate, VariableType::Date | VariableType::EDate | VariableType::DDate) => {
                let raw = self.as_int();
                if raw == 0 {
                    empty
                } else {
                    let date = crate::datetime::IcbDate::from_pcboard_full(raw as u32);
                    TemporalValue::Date(Some(
                        NaiveDate::from_ymd_opt(i32::from(date.year()), u32::from(date.month()), u32::from(date.day())).ok_or("Invalid legacy date")?,
                    ))
                }
            }
            (VariableType::ClockTime, VariableType::Time) => TemporalValue::Time(Some(
                u32::try_from(self.as_int())
                    .ok()
                    .and_then(|seconds| NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0))
                    .ok_or("Invalid legacy time")?,
            )),
            _ => return Err(format!("Cannot convert {} to {target}", self.vtype)),
        };
        Ok(Self::new_temporal(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemporalValue {
    Date(Option<NaiveDate>),
    Time(Option<NaiveTime>),
    Timestamp(Option<DateTime<Utc>>),
}

impl TemporalValue {
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Date(None) | Self::Time(None) | Self::Timestamp(None))
    }

    pub fn parse(self, text: &str) -> Result<Self, String> {
        if text.is_empty() {
            return Ok(match self {
                Self::Date(_) => Self::Date(None),
                Self::Time(_) => Self::Time(None),
                Self::Timestamp(_) => Self::Timestamp(None),
            });
        }
        match self {
            Self::Date(_) => NaiveDate::parse_from_str(text, "%Y-%m-%d").map(|value| Self::Date(Some(value))),
            Self::Time(_) => NaiveTime::parse_from_str(text, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M"))
                .map(|value| Self::Time(Some(value))),
            Self::Timestamp(_) => DateTime::parse_from_rfc3339(text).map(|value| Self::Timestamp(Some(value.with_timezone(&Utc)))),
        }
        .map_err(|error| error.to_string())
    }

    pub fn text(self) -> String {
        match self {
            Self::Date(Some(value)) => value.format("%Y-%m-%d").to_string(),
            Self::Time(Some(value)) => value.format("%H:%M:%S%.f").to_string(),
            Self::Timestamp(Some(value)) => value.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            _ => String::new(),
        }
    }

    pub fn encode(self) -> [u8; 13] {
        let (whole, nanos) = match self {
            Self::Date(Some(value)) => (i64::from(value.num_days_from_ce()), 0),
            Self::Time(Some(value)) => (i64::from(value.num_seconds_from_midnight()), value.nanosecond()),
            Self::Timestamp(Some(value)) => (value.timestamp(), value.timestamp_subsec_nanos()),
            _ => (0, 0),
        };
        let mut bytes = [0; 13];
        bytes[0] = u8::from(!self.is_empty());
        bytes[1..9].copy_from_slice(&whole.to_le_bytes());
        bytes[9..13].copy_from_slice(&nanos.to_le_bytes());
        bytes
    }

    pub fn decode(self, bytes: &[u8]) -> Result<Self, String> {
        let bytes: &[u8; 13] = bytes.try_into().map_err(|_| "Invalid temporal value length")?;
        let whole = i64::from_le_bytes(bytes[1..9].try_into().unwrap());
        let nanos = u32::from_le_bytes(bytes[9..13].try_into().unwrap());
        if bytes[0] == 0 && whole == 0 && nanos == 0 {
            return self.parse("");
        }
        if bytes[0] != 1 {
            return Err("Invalid temporal presence flag".into());
        }
        match self {
            Self::Date(_) if nanos == 0 => i32::try_from(whole)
                .ok()
                .and_then(NaiveDate::from_num_days_from_ce_opt)
                .map(|value| Self::Date(Some(value))),
            Self::Time(_) => u32::try_from(whole)
                .ok()
                .and_then(|value| NaiveTime::from_num_seconds_from_midnight_opt(value, nanos))
                .map(|value| Self::Time(Some(value))),
            Self::Timestamp(_) => DateTime::from_timestamp(whole, nanos).map(|value| Self::Timestamp(Some(value))),
            _ => None,
        }
        .ok_or_else(|| "Invalid temporal value".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_failed_conversions_preserve_destination_values() {
        use super::super::{EntryType, TableEntry, VarHeader, VariableTable};
        let date = VariableValue::new_string("1983-09-15".into()).convert_to(VariableType::CalendarDate).unwrap();
        for invalid in [
            VariableValue::new_string("2023-02-29".into()),
            VariableValue::new_int(1),
            VariableValue::new_bytes(vec![0]),
        ] {
            assert!(invalid.clone().convert_to(VariableType::CalendarDate).is_err());
            let mut array = VariableValue {
                vtype: VariableType::CalendarDate,
                generic_data: GenericVariableData::Dim1(std::sync::Arc::new(vec![date.clone()])),
                ..Default::default()
            };
            let original = array.clone();
            assert!(array.set_array_value(0, 0, 0, invalid.clone()).is_err());
            assert_eq!(array, original);
            let mut table = VariableTable::default();
            table.push(TableEntry::new(
                "value",
                VarHeader {
                    id: 1,
                    variable_type: date.vtype,
                    ..Default::default()
                },
                date.clone(),
                EntryType::Variable,
            ));
            assert!(table.set_value(1, invalid).is_err());
            assert_eq!(table.get_value(1), &date);
        }
        for target in [
            VariableType::Integer,
            VariableType::Long,
            VariableType::Double,
            VariableType::Date,
            VariableType::ClockTime,
        ] {
            assert!(date.clone().convert_to(target).is_err(), "{target}");
        }
        assert_eq!(date.clone().convert_to(VariableType::String).unwrap().as_string(), "1983-09-15");
        assert!(date.clone().convert_to(VariableType::Boolean).unwrap().as_bool());
        assert_eq!(date.clone().convert_to(VariableType::Bytes).unwrap().convert_to(date.vtype).unwrap(), date);
        assert!(
            VariableValue::new_string(String::new())
                .convert_to(date.vtype)
                .unwrap()
                .temporal()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn temporal_operation_numbers_are_stable() {
        let names = [
            "Today",
            "TimeNow",
            "TimestampNow",
            "ParseDate",
            "ParseTime",
            "ParseTimestamp",
            "CreateDate",
            "CreateTime",
            "FromUtc",
            "FromUnix",
            "IsEmpty",
            "Year",
            "Month",
            "Day",
            "DayOfWeek",
            "Hour",
            "Minute",
            "Second",
            "Nanosecond",
            "UtcDate",
            "UtcTime",
            "UnixSeconds",
            "Format",
            "WithYear",
            "AddDays",
            "AddSeconds",
            "DaysUntil",
            "SecondsUntil",
            "LegacyDate",
            "LegacyTime",
        ];
        assert_eq!(TEMPORAL_OPS.len(), names.len());
        for (number, (operation, name)) in TEMPORAL_OPS.iter().zip(names).enumerate() {
            assert_eq!(*operation as usize, number);
            assert_eq!(format!("{operation:?}"), name, "stored operation {number}");
        }
    }

    #[test]
    fn temporal_arrays_compare_elements_and_legacy_bridges_share_day_numbers() {
        let date = VariableValue::new_temporal(TemporalValue::Date(None).parse("1983-09-15").unwrap());
        let later = TemporalOp::AddDays.evaluate(&date, &[VariableValue::new_int(1)]).unwrap();
        let array = |value| VariableValue {
            vtype: VariableType::CalendarDate,
            generic_data: GenericVariableData::Dim1(std::sync::Arc::new(vec![value])),
            ..Default::default()
        };
        assert_eq!(array(date.clone()), array(date.clone()));
        assert_ne!(array(date.clone()), array(later));
        assert_ne!(array(date.clone()), VariableType::CalendarDate.create_empty_value());
        assert!(array(date.clone()).temporal().is_none());
        let legacy = TemporalOp::LegacyDate.evaluate(&date, &[]).unwrap();
        for typ in [VariableType::Date, VariableType::EDate, VariableType::DDate] {
            assert_eq!(
                legacy
                    .clone()
                    .convert_to(typ)
                    .unwrap()
                    .try_temporal_conversion(VariableType::CalendarDate)
                    .unwrap(),
                date
            );
            assert!(
                typ.create_empty_value()
                    .try_temporal_conversion(VariableType::CalendarDate)
                    .unwrap()
                    .temporal()
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn temporal_operations_preserve_types_and_check_invalid_changes() {
        let date = VariableValue::new_temporal(TemporalValue::Date(None).parse("2024-02-29").unwrap());
        assert!(TemporalOp::WithYear.evaluate(&date, &[VariableValue::new_int(1983)]).is_err());
        assert_eq!(
            TemporalOp::AddDays.evaluate(&date, &[VariableValue::new_int(1)]).unwrap().as_string(),
            "2024-03-01"
        );
        assert_eq!(
            TemporalOp::Format
                .evaluate(&date, &[VariableValue::new_string("%d-%m-%Y".into())])
                .unwrap()
                .as_string(),
            "29-02-2024"
        );
        assert!(TemporalOp::Format.evaluate(&date, &[VariableValue::new_string("%H".into())]).is_err());
        assert!(TemporalOp::AddDays.evaluate(&date, &[VariableValue::new_long(i64::MAX)]).is_err());
        let stamp = VariableValue::new_temporal(TemporalValue::Timestamp(None).parse("2024-12-31T23:59:59.123456789Z").unwrap());
        assert_eq!(
            TemporalOp::AddSeconds.evaluate(&stamp, &[VariableValue::new_int(1)]).unwrap().as_string(),
            "2025-01-01T00:00:00.123456789Z"
        );
    }

    #[test]
    fn temporal_values_roundtrip_without_losing_precision() {
        for (empty, text) in [
            (TemporalValue::Date(None), "1883-09-15"),
            (TemporalValue::Date(None), "2400-02-29"),
            (TemporalValue::Time(None), "00:00:00"),
            (TemporalValue::Time(None), "23:59:59.123456789"),
            (TemporalValue::Timestamp(None), "1969-12-31T23:59:59.123456789Z"),
            (TemporalValue::Timestamp(None), "1970-01-01T00:00:00Z"),
        ] {
            let value = empty.parse(text).unwrap();
            assert!(!value.is_empty());
            assert_eq!(value.text(), text);
            assert_eq!(empty.decode(&value.encode()).unwrap(), value);
            assert_eq!(empty.decode(&empty.encode()).unwrap(), empty);
            assert_ne!(empty.encode(), value.encode());
        }
    }

    #[test]
    fn temporal_parsing_validates_calendar_and_requires_timestamp_offset() {
        assert!(TemporalValue::Date(None).parse("1983-02-29").is_err());
        assert!(TemporalValue::Time(None).parse("24:00:00").is_err());
        assert!(TemporalValue::Timestamp(None).parse("2026-09-15T12:30:00").is_err());
        assert_eq!(
            TemporalValue::Timestamp(None).parse("2026-09-15T00:30:00+02:00").unwrap().text(),
            "2026-09-14T22:30:00Z"
        );
        assert!(TemporalValue::Date(None).decode(&[0; 12]).is_err());
        assert!(TemporalValue::Date(None).decode(&[2; 13]).is_err());
    }
}
