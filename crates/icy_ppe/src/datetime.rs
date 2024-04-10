use core::fmt;

use chrono::{Datelike, Timelike};
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

    pub fn get_month(&self) -> u8 {
        self.month
    }

    pub fn get_day(&self) -> u8 {
        self.day
    }

    pub fn get_year(&self) -> u16 {
        self.year
    }

    pub fn is_empty(&self) -> bool {
        self.month == 1 && self.day == 1 && self.year == 0
    }

    pub fn parse(str: &str) -> Self {
        let (month, day, year) = if str.len() == 8 {
            let month = str[0..2].parse::<i32>().unwrap_or_default();
            let day = str[2..4].parse::<i32>().unwrap_or_default();
            let year = str[4..].parse::<i32>().unwrap_or_default();
            (month, day, year)
        } else if str.len() == 6 {
            let month = str[0..2].parse::<i32>().unwrap_or_default();
            let day = str[2..4].parse::<i32>().unwrap_or_default();
            let year = 1900 + str[4..].parse::<i32>().unwrap_or_default();
            (month, day, year)
        } else {
            let parts = str
                .split(|c| c == '-' || c == '/' || c == '.' || c == ' ')
                .map(|c| c.parse::<i32>().unwrap_or_default())
                .collect::<Vec<i32>>();
            if parts.len() != 3 || parts[0] == 0 || parts[1] == 0 {
                return IcbDate::new(0, 0, 0);
            }
            let month = parts[0];
            let day = parts[1];
            let mut year = parts[2];
            if year < 100 {
                if year < 79 {
                    year += 2000;
                } else {
                    year += 1900;
                }
            }
            (month, day, year)
        };
        Self::new(month as u8, day as u8, year as u16)
    }

    pub fn from_pcboard(jd: u32) -> Self {
        juilian_to_date(jd as i64)
    }

    pub fn to_pcboard_date(&self) -> i32 {
        let mut year = self.year as i64;
        // correct pcboard design decision
        if (1900..1979).contains(&year) {
            year = year.saturating_add(100);
        }

        let mut res = 36525 * year;
        if res % 100 == 0 && self.month < 3 {
            res = res.saturating_sub(1);
        }
        res = (res.saturating_sub(1900 * 36525)) / 100;
        res += self.day as i64 + DAYS[0][(self.month as usize).saturating_sub(1)];

        res as i32
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
    assert_eq!(64648, date.to_pcboard_date());
}

#[test]
fn test_pcb_date() {
    let date = IcbDate::from_pcboard(64648);
    assert_eq!(format!("{date}"), "12-30-76");
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
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
        }
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
        let parts = str
            .split(|c| c == ':' || c == ' ')
            .map(|c| c.parse::<i32>().unwrap_or_default())
            .collect::<Vec<i32>>();
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
                second: self.second,
                nanosecond: 0,
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
            second: datetime.time.unwrap().second,
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
                second: datetime.second,
                nanosecond: 0,
            }),
            offset: None,
        }
    }
}
