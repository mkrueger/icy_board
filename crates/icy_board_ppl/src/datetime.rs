use core::fmt;
use std::str::FromStr;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use serde::Deserialize;
use toml::value::{Date, Datetime};
#[derive(Debug, Clone, PartialEq)]
pub struct IcbDate {
    month: u8,
    day: u8,
    year: u16,
}

impl Default for IcbDate {
    fn default() -> Self {
        Self {
            month: 1,
            day: 1,
            year: Default::default(),
        }
    }
}

impl<'de> Deserialize<'de> for IcbDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Datetime::deserialize(deserializer).map(IcbDate::from)
    }
}

impl serde::Serialize for IcbDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        toml::value::Datetime {
            date: Some(Date {
                year: self.year,
                month: self.month.max(1),
                day: self.day.max(1),
            }),
            time: None,
            offset: None,
        }
        .serialize(serializer)
    }
}

impl From<NaiveDateTime> for IcbDate {
    fn from(date: NaiveDateTime) -> Self {
        Self {
            month: date.month() as u8,
            day: date.day() as u8,
            year: date.year() as u16,
        }
    }
}

impl From<NaiveDate> for IcbDate {
    fn from(date: NaiveDate) -> Self {
        Self {
            month: date.month() as u8,
            day: date.day() as u8,
            year: date.year() as u16,
        }
    }
}

impl From<Datetime> for IcbDate {
    fn from(datetime: Datetime) -> Self {
        let date = &datetime.date.unwrap();
        Self {
            month: date.month,
            day: date.day,
            year: date.year,
        }
    }
}

impl From<IcbDate> for Datetime {
    fn from(datetime: IcbDate) -> Datetime {
        Datetime {
            date: Some(Date {
                year: datetime.year,
                month: datetime.month,
                day: datetime.day,
            }),
            time: None,
            offset: None,
        }
    }
}

const DAYS: [[i64; 12]; 2] = [
    [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334],
    [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335],
];

pub fn pcboard_date_parts(value: &str) -> [u16; 3] {
    let mut parts = [0; 3];
    let mut field = 0;
    let mut digits = 0;
    for byte in value.bytes().take_while(|byte| *byte != 0) {
        if !byte.is_ascii_digit() {
            field += 1;
            digits = 0;
            if field == 3 {
                break;
            }
            continue;
        }
        if digits == if field == 2 { 4 } else { 2 } {
            field += 1;
            digits = 0;
            if field == 3 {
                break;
            }
        }
        parts[field] = parts[field] * 10 + u16::from(byte - b'0');
        digits += 1;
    }
    parts
}

pub fn pcboard_date_from_parts([month, day, year]: [u16; 3]) -> u16 {
    if month == 0 || day == 0 {
        return 0;
    }
    let year = i64::from(year)
        + if year < 79 {
            2000
        } else if year < 100 {
            1900
        } else {
            0
        };
    let scaled = 36525 * year;
    let adjustment = i64::from(scaled % 100 == 0 && month < 3);
    let Some(offset) = DAYS.iter().flatten().nth(usize::from(month - 1)) else {
        return 0;
    };
    (((scaled - adjustment - 1900 * 36525) / 100) + i64::from(day) + offset) as u16
}

pub fn pcboard_mkdate(year: i32, month: i32, day: i32) -> u16 {
    let relative_year = (year as u16).wrapping_sub(1900) as i16;
    let month = month as u16;
    let offsets = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let Some(offset) = offsets.get(usize::from(month)) else {
        return 0;
    };
    let leap = i32::from(relative_year > 0 && relative_year % 4 == 0 && month > 2);
    (i32::from(relative_year) * 365 + i32::from(relative_year.wrapping_sub(1) / 4) + offset + leap + i32::from(day as u16)) as u16
}

impl IcbDate {
    pub fn new(month: u8, day: u8, year: u16) -> Self {
        let month = month.clamp(1, 12);
        let day = day.clamp(1, 31);

        Self { month, day, year }
    }

    pub fn today() -> Self {
        let now = chrono::Local::now();
        Self {
            month: now.month() as u8,
            day: now.day() as u8,
            year: now.year() as u16,
        }
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    /// Number of days from sunday
    pub fn day_of_week(&self) -> u8 {
        self.to_utc_date_time().weekday().num_days_from_sunday() as u8
    }

    pub fn is_empty(&self) -> bool {
        self.month == 1 && self.day == 1 && self.year == 0
    }

    pub fn parse(str: &str) -> Self {
        Self::try_parse(str).unwrap_or_default()
    }

    pub fn try_parse(str: &str) -> Option<Self> {
        if !str.is_ascii() {
            return None;
        }
        let [month, day, year] = pcboard_date_parts(str);
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        let year = year
            + if year < 79 {
                2000
            } else if year < 100 {
                1900
            } else {
                0
            };
        Some(Self::new(month as u8, day as u8, year))
    }

    pub fn to_pcb_str(&self) -> String {
        // PCBoard uses MM-DD-YY format (6 characters)
        // Year is stored as 2 digits:
        // 00-78 = 2000-2078
        // 79-99 = 1979-1999
        let year_2digit = if self.year >= 2000 {
            (self.year - 2000) % 100
        } else if self.year >= 1900 {
            self.year - 1900
        } else {
            self.year % 100
        };

        format!("{:02}-{:02}-{:02}", self.month, self.day, year_2digit)
    }

    pub fn from_pcboard(jd: u32) -> Self {
        juilian_to_date(jd as i64)
    }

    pub fn from_pcboard_full(jd: u32) -> Self {
        let jd = jd as u16;
        let mut date = Self::from_pcboard(u32::from(jd));
        if jd != 0 {
            date.year += if jd > 36524 { 2000 } else { 1900 };
        }
        date
    }

    pub fn to_pcboard_date(&self) -> i32 {
        if self.is_empty() {
            return 0;
        }
        i32::from(pcboard_date_from_parts([u16::from(self.month), u16::from(self.day), self.year]))
    }

    pub fn to_julian_date(&self) -> u64 {
        let year = self.year as i64;
        let mut res = 36525 * year;
        if res % 100 == 0 && self.month < 3 {
            res -= 1;
        }
        res = (res - (1900 * 36525)) / 100;
        res += self.day as i64 + DAYS[0][self.month as usize - 1];

        res as u64
    }

    pub fn to_country_date(&self) -> String {
        self.to_string()
    }

    pub fn to_utc_date_time(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        let first_day = NaiveDate::from_ymd_opt(self.year as i32, self.month.clamp(1, 12) as u32, 1).unwrap_or(NaiveDate::MIN);
        let date = first_day
            .checked_add_signed(chrono::Duration::days(i64::from(self.day.max(1) - 1)))
            .unwrap_or(first_day);
        chrono::prelude::DateTime::<Utc>::from_naive_utc_and_offset(NaiveDateTime::new(date, NaiveTime::MIN), Utc)
    }

    pub fn to_local_date_time(&self) -> chrono::prelude::DateTime<chrono::prelude::Local> {
        let utc = self.to_utc_date_time();
        Local
            .from_local_datetime(&utc.naive_utc())
            .earliest()
            .unwrap_or_else(|| utc.with_timezone(&Local))
    }

    /// `None` for the empty date, which is how `PCBoard` stored "no date given".
    pub fn to_naive_date(&self) -> Option<NaiveDate> {
        if self.is_empty() {
            return None;
        }
        Some(self.to_utc_date_time().date_naive())
    }

    pub fn from_utc(date_time: &chrono::prelude::DateTime<chrono::prelude::Utc>) -> Self {
        Self {
            month: date_time.month() as u8,
            day: date_time.day() as u8,
            year: date_time.year() as u16,
        }
    }
}

impl fmt::Display for IcbDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}-{:02}-{:02}", self.month, self.day, self.year)
    }
}

fn juilian_to_date(jd: i64) -> IcbDate {
    if jd == 0 {
        return IcbDate::new(0, 0, 0);
    }

    let mut year = 100 * jd / 36525;
    let mut jd = jd - (year * 36525) / 100;

    let tmp = year * 36525;
    let day_table = if tmp % 100 == 0 && (year != 0 && year != 1900) {
        jd += 1;
        DAYS[1]
    } else {
        DAYS[0]
    };

    let mut month = 0;
    for (m, day) in day_table.iter().enumerate() {
        if *day < jd {
            month = m;
        } else {
            break;
        }
    }
    let day = jd - day_table[month];

    if year >= 100 {
        year -= 100;
    }

    IcbDate::new(month as u8 + 1, day as u8, year as u16)
}

#[test]
fn test_to_julian_date() {
    let date = IcbDate::parse("12-30-1976");
    assert_eq!(28123, date.to_pcboard_date());
}

#[test]
fn test_pcb_date() {
    let date = IcbDate::from_pcboard(64648);
    assert_eq!(format!("{date}"), "12-30-76");
}

/// The numbers `PCBoard` 15.4/M answered for a file stamped 03-15-1996 14:22:36.
#[test]
fn test_file_stamp_matches_pcboard() {
    assert_eq!(35138, IcbDate::new(3, 15, 1996).to_pcboard_date());
    assert_eq!(51756, IcbTime::new(14, 22, 36).to_pcboard_time());
}

#[test]
fn test_parse_date() {
    let date = IcbDate::parse("12-30-1976");
    assert_eq!(format!("{date}"), "12-30-1976");

    let date = IcbDate::parse("12/30/1976");
    assert_eq!(format!("{date}"), "12-30-1976");

    let date = IcbDate::parse("12301976");
    assert_eq!(format!("{date}"), "12-30-1976");
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct IcbTime {
    hour: u8,
    minute: u8,
    second: u8,
}

impl IcbTime {
    pub fn parse_pcboard(value: &str) -> i32 {
        let mut parts = value.split(':').filter(|part| !part.is_empty());
        let mut result = 0i32;
        for (index, multiplier) in [3600, 60, 1].into_iter().enumerate() {
            let Some(part) = parts.next() else {
                return if index == 2 { result % 86400 } else { 0 };
            };
            if !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return 0;
            }
            let number = part
                .bytes()
                .fold(0i32, |number, digit| number.wrapping_mul(10).wrapping_add(i32::from(digit - b'0')));
            result = result.wrapping_add(number.wrapping_mul(multiplier));
        }
        result % 86400
    }

    pub fn format_pcboard(value: i32) -> String {
        let value = value % 86400;
        [value / 3600, value % 3600 / 60, value % 60]
            .map(|part| if part < 0 { format!("-{:02}", -part) } else { format!("{part:02}") })
            .join(":")
    }

    pub fn is_valid_pcboard(value: &str) -> bool {
        let (hour, rest) = signed_decimal(value.as_bytes());
        let Some(rest) = rest.strip_prefix(b":") else { return false };
        let (minute, rest) = signed_decimal(rest);
        let (second, rest) = if let Some(rest) = rest.strip_prefix(b":") {
            signed_decimal(rest)
        } else {
            (0, rest)
        };
        rest.is_empty() && (0..24).contains(&hour) && (0..60).contains(&minute) && (0..60).contains(&second)
    }

    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self { hour, minute, second }
    }

    pub fn now() -> Self {
        let now = chrono::Local::now();
        Self {
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: now.second() as u8,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hour == 0 && self.minute == 0 && self.second == 0
    }

    pub fn get_hour(&self) -> u8 {
        self.hour
    }

    pub fn get_minute(&self) -> u8 {
        self.minute
    }

    pub fn get_second(&self) -> u8 {
        self.second
    }

    pub fn parse(str: &str) -> Self {
        let parts = str.split([':', ' ']).map(|c| c.parse::<i32>().unwrap_or_default()).collect::<Vec<i32>>();
        if parts.len() != 3 {
            return IcbTime::new(0, 0, 0);
        }
        let hour = parts[0];
        let minute = parts[1];
        let second = parts[2];

        Self {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
        }
    }

    pub fn to_pcb_str(&self) -> String {
        // PCBoard uses HH:MM format (5 characters) for time fields
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    pub fn from_pcboard(time: i32) -> Self {
        let hour = time / 3600;
        let minute = (time % 3600) / 60;
        let second = time % 60;
        Self {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
        }
    }
    pub fn to_pcboard_time(&self) -> i32 {
        self.hour as i32 * 60 * 60 + self.minute as i32 * 60 + self.second as i32
    }

    pub fn from_naive(date_time: NaiveDateTime) -> IcbTime {
        IcbTime {
            hour: date_time.hour() as u8,
            minute: date_time.minute() as u8,
            second: date_time.second() as u8,
        }
    }
}

pub(crate) fn signed_decimal(value: &[u8]) -> (i32, &[u8]) {
    let mut position = 0;
    while value.get(position).is_some_and(u8::is_ascii_whitespace) {
        position += 1;
    }
    let negative = value.get(position) == Some(&b'-');
    if negative || value.get(position) == Some(&b'+') {
        position += 1;
    }
    let start = position;
    let mut number = 0i64;
    while let Some(digit) = value.get(position).filter(|digit| digit.is_ascii_digit()) {
        number = (number * 10 + i64::from(*digit - b'0')).min(i64::from(i32::MAX) + 1);
        position += 1;
    }
    if position == start {
        return (0, value);
    }
    let number = if negative { -number } else { number };
    (number.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32, &value[position..])
}

impl fmt::Display for IcbTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

impl<'de> Deserialize<'de> for IcbTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Datetime::deserialize(deserializer).map(IcbTime::from)
    }
}

impl serde::Serialize for IcbTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        toml::value::Datetime {
            date: None,
            time: Some(toml::value::Time {
                hour: self.hour,
                minute: self.minute,
                second: Some(self.second),
                nanosecond: Some(0),
            }),
            offset: None,
        }
        .serialize(serializer)
    }
}

impl From<Datetime> for IcbTime {
    fn from(datetime: Datetime) -> Self {
        Self {
            hour: datetime.time.unwrap().hour,
            minute: datetime.time.unwrap().minute,
            second: datetime.time.unwrap().second.unwrap_or(0),
        }
    }
}

impl From<IcbTime> for Datetime {
    fn from(datetime: IcbTime) -> Datetime {
        Datetime {
            date: None,
            time: Some(toml::value::Time {
                hour: datetime.hour,
                minute: datetime.minute,
                second: Some(datetime.second),
                nanosecond: Some(0),
            }),
            offset: None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct IcbDoW {
    dow: u8,
}

impl IcbDoW {
    pub fn new(day: u8) -> Self {
        Self { dow: day }
    }

    pub fn all() -> Self {
        Self { dow: 0b0111_1111 }
    }

    pub fn is_empty(&self) -> bool {
        self.dow.trailing_zeros() >= 7
    }

    pub fn contains(&self, day: chrono::Weekday) -> bool {
        self.dow & (1 << day.num_days_from_sunday()) != 0
    }
}

impl fmt::Display for IcbDoW {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for i in 0..7 {
            if self.dow & (1 << i) != 0 {
                s.push('Y');
            } else {
                s.push('N');
            }
        }
        write!(f, "{s}")
    }
}

impl FromStr for IcbDoW {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut dow = 0;
        for (i, c) in s.chars().enumerate() {
            if c == 'Y' {
                dow |= 1 << i;
            }
        }
        Ok(Self { dow })
    }
}

impl From<String> for IcbDoW {
    fn from(datetime: String) -> IcbDoW {
        IcbDoW::from_str(&datetime).unwrap()
    }
}

impl<'de> Deserialize<'de> for IcbDoW {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(IcbDoW::from)
    }
}

impl serde::Serialize for IcbDoW {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn pcboard_datetime_full_year_roundtrip() {
        for raw in 1..=u16::MAX {
            let date = super::IcbDate::from_pcboard_full(u32::from(raw));
            assert_eq!(date.to_pcboard_date(), i32::from(raw), "{raw}: {date:?}");
            assert_eq!(date.day_of_week(), (raw % 7) as u8);
        }
    }

    #[test]
    fn pcboard_datetime_text_and_invalid_calendar_are_safe() {
        use chrono::Datelike;
        assert_eq!(super::IcbDate::parse("03-15-96"), super::IcbDate::new(3, 15, 1996));
        assert_eq!(super::IcbDate::parse("010224"), super::IcbDate::new(1, 2, 2024));
        assert!(super::IcbDate::try_parse("1\u{20ac}2345").is_none());
        assert!(super::IcbDate::parse("1\u{20ac}2345").is_empty());
        let normalized = super::IcbDate::new(2, 31, 2024).to_utc_date_time();
        assert_eq!((normalized.year(), normalized.month(), normalized.day()), (2024, 3, 2));
        let leap = super::IcbDate::new(2, 29, 2024).to_local_date_time();
        assert_eq!((leap.year(), leap.month(), leap.day()), (2024, 2, 29));
    }

    #[test]
    fn test_date_parse() {
        let date = super::IcbDate::parse("12-30-1976");
        assert_eq!(date.to_country_date(), "12-30-1976");
    }

    #[test]
    fn test_time_parse() {
        let time = super::IcbTime::parse("12:30:01");
        assert_eq!(time.to_string(), "12:30:01");
    }

    #[test]
    fn test_utc_date_conversion() {
        let date = super::IcbDate::parse("12-30-1976");
        let utc = date.to_utc_date_time();

        let date = super::IcbDate::from_utc(&utc);
        assert_eq!(utc, date.to_utc_date_time());
    }
}
