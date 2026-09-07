//! Accounting arithmetic, calendar billing and serialized audit output.
//!
//! Board/session policy (security changes, exemptions and when to post charges) is
//! deliberately outside this module. This is an independent implementation of the
//! observed PCBoard accounting behavior, not a translation of its source.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike, Utc};

use crate::{
    Res,
    vm::dbase::file::{DbaseFile, FieldInfo, parse_field_info},
};

use super::{accounting_cfg::AccountingConfig, icb_config::AccountingOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccountingMode {
    #[default]
    Disabled,
    Tracking,
    Enforced,
}

/// Deterministic display until host currency/locale settings are available.
/// Credits retain up to six decimals with trailing zeros trimmed, like legacy
/// dcomma; money uses a fixed dollar prefix and two decimals, without locale lookup.
/// Non-finite input is displayed as "invalid", never as a plausible balance.
pub fn format_credit(value: f64, use_money: bool) -> String {
    if !value.is_finite() {
        return "invalid".into();
    }
    let formatted = if use_money { format!("{value:.2}") } else { format!("{value:.6}") };
    let (integer, fraction) = formatted.split_once('.').unwrap_or((&formatted, ""));
    let fraction = if use_money { fraction } else { fraction.trim_end_matches('0') };
    let digits = integer.strip_prefix('-').unwrap_or(integer);
    let mut output = String::new();
    if use_money {
        output.push('$');
    }
    if integer.starts_with('-') {
        output.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(digit);
    }
    if !fraction.is_empty() {
        output.push('.');
        output.push_str(fraction);
    }
    output
}

/// Peak periods use the local date, a Sunday-first day mask and inclusive HH:MM
/// endpoints. Equal endpoints mean all day, as with the legacy midnight-crossing
/// rule. A holiday suppresses peak billing for its entire local calendar date.
pub fn peak_at(options: &AccountingOptions, holidays: &[String], when: DateTime<Local>) -> bool {
    peak_in_timezone(options, holidays, when)
}

fn peak_in_timezone<Tz: TimeZone>(options: &AccountingOptions, holidays: &[String], when: DateTime<Tz>) -> bool {
    if !options.enabled || !options.peak_days_of_week.contains(when.weekday()) {
        return false;
    }
    let start = &options.peak_usage_start;
    let end = &options.peak_usage_end;
    if start.get_hour() > 23 || end.get_hour() > 23 || start.get_minute() > 59 || end.get_minute() > 59 {
        return false;
    }
    let start = u32::from(start.get_hour()) * 60 + u32::from(start.get_minute());
    let end = u32::from(end.get_hour()) * 60 + u32::from(end.get_minute());
    let minute = when.hour() * 60 + when.minute();
    let inside = if start < end {
        (start..=end).contains(&minute)
    } else {
        minute >= start || minute <= end
    };
    if !inside {
        return false;
    }
    let date = format!("{:02}-{:02}-{:02}", when.month(), when.day(), when.year().rem_euclid(100));
    !holidays.iter().any(|holiday| {
        let pattern = holiday.trim().as_bytes();
        pattern.len() == 8
            && pattern
                .iter()
                .zip(date.bytes())
                .all(|(&pattern, actual)| pattern == actual || (pattern == b'X' && actual.is_ascii_digit()))
    })
}

/// Bill calendar-minute boundaries, not rounded elapsed session minutes.
/// For example 12:00:59..12:01:00 bills the 12:00 minute; 12:00:00..12:00:59
/// bills nothing. Iteration is UTC so DST gaps/repeats are handled exactly once
/// per elapsed minute, with each minute classified against its local date/time.
/// This intentionally fixes the legacy midnight peak-calculation bug rather
/// than reproducing it; day masks and holidays follow each minute's local date.
/// Preview (`grace`) omits one normal minute, or one peak minute if none is normal.
/// Disabled accounting/reversed intervals cost zero. Invalid rates return NaN.
pub fn time_charge(rates: &AccountingConfig, options: &AccountingOptions, holidays: &[String], start: DateTime<Utc>, end: DateTime<Utc>, grace: bool) -> f64 {
    if !options.enabled || end <= start {
        return 0.0;
    }
    if rates.validate().is_err() {
        return f64::NAN;
    }
    charge_minutes(rates, start, end, grace, |minute| peak_at(options, holidays, minute.with_timezone(&Local)))
}

fn charge_minutes(rates: &AccountingConfig, start: DateTime<Utc>, end: DateTime<Utc>, grace: bool, mut is_peak: impl FnMut(DateTime<Utc>) -> bool) -> f64 {
    let first = start.timestamp().div_euclid(60);
    let last = end.timestamp().div_euclid(60);
    let mut normal = 0i64;
    let mut peak = 0i64;
    for minute in first..last {
        let Some(at) = DateTime::from_timestamp(minute * 60, 0) else {
            return f64::NAN;
        };
        if is_peak(at) {
            peak += 1;
        } else {
            normal += 1;
        }
    }
    if grace {
        if normal > 0 {
            normal -= 1;
        } else if peak > 0 {
            peak -= 1;
        }
    }
    rates.charge_per_time * normal as f64 + rates.charge_per_peak_time * peak as f64
}

/// Activity/conference elapsed time uses nearest-minute rounding instead of the
/// global calendar-minute rule. Negative durations count as zero.
pub fn minutes_used(duration: Duration) -> i64 {
    let seconds = duration.num_seconds().max(0);
    seconds / 60 + i64::from(seconds % 60 >= 30)
}

#[derive(Clone, Debug)]
pub struct TrackingEntry {
    pub at: DateTime<Local>,
    pub user: String,
    pub node: u16,
    pub conference: u16,
    pub activity: String,
    pub sub_activity: String,
    pub unit_cost: f64,
    pub quantity: i64,
    pub value: f64,
}

const DBF_FIELDS: [&str; 10] = [
    "Date,D,8,0",
    "Time,C,5,0",
    "Name,C,25,0",
    "NodeNumber,N,5,0",
    "ConfNumber,N,5,0",
    "Activity,C,15,0",
    "SubAct,C,25,0",
    "UnitCost,N,14,4",
    "Quantity,N,9,0",
    "Value,N,14,4",
];

fn ascii_field(value: &str, width: usize) -> String {
    value
        .chars()
        .take(width)
        .map(|ch| {
            if ch.is_ascii_control() {
                ' '
            } else if ch.is_ascii() {
                ch
            } else {
                '?'
            }
        })
        .collect()
}

fn tracking_values(entry: &TrackingEntry, dbf: bool) -> Res<[String; 10]> {
    if !entry.unit_cost.is_finite() || !entry.value.is_finite() {
        return Err("Accounting tracking amounts must be finite".into());
    }
    // dBase date fields have four year digits. Do not silently corrupt exotic dates.
    if !(1..=9999).contains(&entry.at.year()) {
        return Err("Accounting tracking date is outside years 1..=9999".into());
    }
    let unit_cost = format!("{:14.4}", entry.unit_cost);
    let quantity = format!("{:9}", entry.quantity);
    let value = format!("{:14.4}", entry.value);
    if unit_cost.len() > 14 || quantity.len() > 9 || value.len() > 14 {
        return Err("Accounting tracking number exceeds its fixed-width field".into());
    }
    Ok([
        entry.at.format(if dbf { "%Y%m%d" } else { "%m-%d-%y" }).to_string(),
        entry.at.format("%H:%M").to_string(),
        format!("{:<25}", ascii_field(&entry.user, 25)),
        format!("{:5}", entry.node),
        format!("{:5}", entry.conference),
        format!("{:<15}", ascii_field(&entry.activity, 15)),
        format!("{:<25}", ascii_field(&entry.sub_activity, 25)),
        unit_cost,
        quantity,
        value,
    ])
}

/// Append one PCBoard-shaped ASCII (CRLF) or dBase III record (`.dbf`, case
/// insensitive). Numeric overflow is rejected rather than widening/truncating fields.
/// Controls become spaces and non-ASCII characters become '?' in both formats.
///
/// A persistent sibling `<filename>.lock` uses OS file locks (Rust >= 1.89),
/// serializing cooperating threads/processes through creation, validation and append.
/// Do not delete/replace that lock file while writers run. Other DBF editors must
/// honor the same lock protocol; this does not lock out unrelated PPE DBF writers.
/// No record is rewritten on success, and corrupt/incompatible DBFs are rejected.
/// I/O errors propagate; this is not a crash-atomic database transaction.
pub fn append_tracking(path: &Path, entry: &TrackingEntry) -> Res<()> {
    let dbf = path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("dbf"));
    let values = tracking_values(entry, dbf)?;
    let mut lock_name = path.as_os_str().to_os_string();
    lock_name.push(".lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(Path::new(&lock_name))?;
    lock.lock()?;

    if dbf {
        append_dbf(path, &values)?;
    } else {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let line = format!("{}\r\n", values.join(" "));
        file.write_all(line.as_bytes())?;
        file.sync_data()?;
    }
    // Drop also releases the OS lock on every error path.
    lock.unlock()?;
    Ok(())
}

fn append_dbf(path: &Path, values: &[String; 10]) -> Res<()> {
    let fields = DBF_FIELDS
        .iter()
        .map(|spec| parse_field_info(spec).ok_or_else(|| "Invalid tracking DBF schema".into()))
        .collect::<Res<Vec<FieldInfo>>>()?;
    let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
    let mut db = if file.metadata()?.len() == 0 {
        DbaseFile::create(path, &fields)?
    } else {
        validate_dbf_layout(&mut file, &fields)?;
        let db = DbaseFile::open(path)?;
        if db.fields().len() != fields.len()
            || db.fields().iter().zip(&fields).any(|(actual, expected)| {
                !actual.name.eq_ignore_ascii_case(&expected.name)
                    || actual.field_type != expected.field_type
                    || actual.length != expected.length
                    || actual.decimals != expected.decimals
            })
        {
            return Err("Accounting tracking DBF has an incompatible schema".into());
        }
        db
    };
    db.begin_new()?;
    for (index, value) in values.iter().enumerate() {
        db.set_field(index, value.as_bytes());
    }
    db.append()?;
    db.flush()?;
    file.sync_data()?;
    Ok(())
}

// The existing general-purpose DBF reader trusts record layout. Check it before
// calling its fixed-field accessors, including truncated headers/records and EOF.
fn validate_dbf_layout(file: &mut File, fields: &[FieldInfo]) -> Res<()> {
    let mut header = [0u8; 32];
    file.read_exact(&mut header)?;
    let first = u64::from(u16::from_le_bytes([header[8], header[9]]));
    let size = u64::from(u16::from_le_bytes([header[10], header[11]]));
    let count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    if header[0] != 3
        || first != (33 + 32 * fields.len()) as u64
        || size != 1 + fields.iter().map(|field| field.length as u64).sum::<u64>()
        || count == u32::MAX
    {
        return Err("Accounting tracking DBF has an invalid header".into());
    }
    let end = first + u64::from(count) * size;
    let length = file.metadata()?.len();
    if length < end || length > end + 1 {
        return Err("Accounting tracking DBF has truncated or trailing records".into());
    }
    let mut marker = [0];
    file.seek(SeekFrom::Start(first - 1))?;
    file.read_exact(&mut marker)?;
    if marker[0] != 0x0d {
        return Err("Accounting tracking DBF has no header terminator".into());
    }
    if length == end + 1 {
        file.seek(SeekFrom::Start(end))?;
        file.read_exact(&mut marker)?;
        if marker[0] != 0x1a {
            return Err("Accounting tracking DBF has an invalid EOF marker".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        datetime::{IcbDoW, IcbTime},
        icy_board::{PCBoardBinImporter, icb_config::IcbConfig, pcb::user_inf::AccountUserInf},
    };
    use chrono::FixedOffset;

    fn options() -> AccountingOptions {
        let mut options = IcbConfig::default().accounting;
        options.enabled = true;
        options.peak_days_of_week = IcbDoW::all();
        options.peak_usage_start = IcbTime::new(9, 0, 0);
        options.peak_usage_end = IcbTime::new(17, 0, 0);
        options
    }

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text).unwrap().with_timezone(&Utc)
    }

    fn entry() -> TrackingEntry {
        TrackingEntry {
            at: utc("2026-09-06T12:34:00Z").with_timezone(&Local),
            user: "Alice".into(),
            node: 3,
            conference: 42,
            activity: "MSG READ".into(),
            sub_activity: "General".into(),
            unit_cost: 1.25,
            quantity: 2,
            value: 2.5,
        }
    }

    #[test]
    fn balance_sequential_concurrent_and_pending_time() {
        let mut account = AccountUserInf {
            starting_balance: 100.0,
            start_this_session: 999.0,
            ..Default::default()
        };
        account.apply_charge(2, 20.0).unwrap();
        account.apply_charge(3, 10.0).unwrap();
        account.apply_charge(9, 30.0).unwrap();
        account.apply_charge(14, 5.0).unwrap();
        account.apply_charge(15, 2.0).unwrap();
        account.apply_charge(16, 3.0).unwrap();
        assert_eq!(account.balance(false, 5.0), 45.0);
        assert_eq!(account.balance(true, 5.0), 80.0);
        assert_eq!(account.balance(true, 25.0), 75.0);
        account.apply_charge(2, -10.0).unwrap();
        account.apply_charge(16, -4.0).unwrap();
        assert_eq!(account.balance(false, 0.0), 56.0);
        assert_eq!(AccountingMode::default(), AccountingMode::Disabled);
    }

    #[test]
    fn every_charge_field_and_nonfinite_validation() {
        let mut account = AccountUserInf {
            starting_balance: 100.0,
            ..Default::default()
        };
        for field in 2..=16 {
            account.apply_charge(field, field as f64).unwrap();
        }
        assert_eq!(account.balance(false, 0.0), 55.0); // debits 2..13 = 90; credits 14..16 = 45
        assert_eq!(account.balance(true, 0.0), 132.0);
        for field in [0, 1, 17, usize::MAX] {
            assert!(account.apply_charge(field, 1.0).is_err());
        }
        for amount in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(account.apply_charge(2, amount).is_err());
            assert_eq!(account.debit_call, 2.0);
            assert!(account.balance(false, amount).is_nan());
        }
        account.debit_call = f64::MAX;
        assert!(account.apply_charge(2, f64::MAX).is_err());
        assert_eq!(account.debit_call, f64::MAX);
        account.credit_special = f64::NAN;
        assert!(account.validate().is_err());
        assert!(account.apply_charge(3, 0.0).is_err());
        assert!(account.balance(true, 0.0).is_nan());
        let mut raw = vec![0; 137];
        raw[16..24].copy_from_slice(&f64::INFINITY.to_le_bytes());
        assert!(AccountUserInf::read(&raw).is_err());
    }

    #[test]
    fn balance_rejects_overflow_without_masking_pending_time() {
        for amount in [f64::MAX, -f64::MAX] {
            let account = AccountUserInf {
                debit_time: amount,
                ..Default::default()
            };
            for concurrent in [false, true] {
                assert!(account.balance(concurrent, amount).is_nan());
            }
            let account = AccountUserInf {
                debit_call: amount,
                debit_time: amount,
                ..Default::default()
            };
            assert!(account.balance(false, 0.0).is_nan());
            // Maximum-category billing must not sum otherwise finite categories.
            assert_eq!(account.balance(true, 0.0), if amount > 0.0 { -amount } else { 0.0 });

            let account = AccountUserInf {
                starting_balance: amount,
                credit_special: amount,
                ..Default::default()
            };
            for concurrent in [false, true] {
                assert!(account.balance(concurrent, 0.0).is_nan());
            }
        }
    }

    #[test]
    fn all_config_fields_reject_nonfinite_but_accept_negative() {
        let raw = (-1.0f64).to_le_bytes().repeat(15);
        let rates = AccountingConfig::import_data(&raw).unwrap();
        rates.validate().unwrap();
        assert_eq!(rates.export_pcboard(), raw);
        for field in 0..15 {
            for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut raw = raw.clone();
                raw[field * 8..field * 8 + 8].copy_from_slice(&invalid.to_le_bytes());
                assert!(AccountingConfig::import_data(&raw).is_err());
            }
        }
        for length in [0, 119, 121] {
            assert!(AccountingConfig::import_data(&vec![0; length]).is_err());
        }
        let rates = AccountingConfig {
            warn_level: f64::NAN,
            ..Default::default()
        };
        assert!(rates.validate().is_err());
    }

    #[test]
    fn deterministic_credit_formatting() {
        assert_eq!(format_credit(1234567.5, false), "1,234,567.5");
        assert_eq!(format_credit(-1234.5, false), "-1,234.5");
        assert_eq!(format_credit(1234.0, false), "1,234");
        assert_eq!(format_credit(1234.1234564, false), "1,234.123456");
        assert_eq!(format_credit(1234.1234567, false), "1,234.123457");
        assert_eq!(format_credit(-1234.1234567, false), "-1,234.123457");
        assert_eq!(format_credit(999.9999999, false), "1,000");
        assert_eq!(format_credit(0.000001, false), "0.000001");
        assert_eq!(format_credit(0.0000001, false), "0");
        assert_eq!(format_credit(0.0, false), "0");
        assert_eq!(format_credit(-0.0, false), "-0");
        assert_eq!(format_credit(1234.5, true), "$1,234.50");
        assert_eq!(format_credit(-1234.5, true), "$-1,234.50");
        assert_eq!(format_credit(1234.567, true), "$1,234.57");
        assert_eq!(format_credit(0.0, true), "$0.00");
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(format_credit(value, false), "invalid");
            assert_eq!(format_credit(value, true), "invalid");
        }
    }

    #[test]
    fn peak_inclusive_endpoints_days_and_holidays() {
        let mut options = options();
        options.peak_days_of_week = IcbDoW::new(1); // Sunday only
        for (time, expected) in [("08:59:59", false), ("09:00:00", true), ("17:00:59", true), ("17:01:00", false)] {
            assert_eq!(peak_in_timezone(&options, &[], utc(&format!("2026-09-06T{time}Z"))), expected);
        }
        let sunday = utc("2026-09-06T12:00:00Z");
        assert!(!peak_in_timezone(&options, &[], utc("2026-09-07T12:00:00Z")));
        for holiday in ["09-06-26", "09-06-XX", "XX-XX-XX", "09-0X-26\r\n"] {
            assert!(!peak_in_timezone(&options, &[holiday.into()], sunday));
        }
        for non_holiday in ["09-06-25", "09/06/26", "09-06", "XXXXXXXX", "09-07-XX"] {
            assert!(peak_in_timezone(&options, &[non_holiday.into()], sunday));
        }
        options.peak_days_of_week = IcbDoW::all();
        assert!(!peak_in_timezone(&options, &["02-29-XX".into()], utc("2024-02-29T12:00:00Z")));
        options.enabled = false;
        assert!(!peak_at(&options, &[], sunday.with_timezone(&Local)));
    }

    #[test]
    fn peak_crosses_midnight_and_equal_endpoints_mean_all_day() {
        let mut options = options();
        options.peak_usage_start = IcbTime::new(22, 0, 0);
        options.peak_usage_end = IcbTime::new(2, 0, 0);
        for (time, expected) in [
            ("21:59:00", false),
            ("22:00:00", true),
            ("00:00:00", true),
            ("02:00:59", true),
            ("02:01:00", false),
        ] {
            assert_eq!(peak_in_timezone(&options, &[], utc(&format!("2026-09-06T{time}Z"))), expected);
        }
        options.peak_days_of_week = IcbDoW::new(1);
        assert!(!peak_in_timezone(&options, &[], utc("2026-09-07T01:00:00Z"))); // Monday, not Sunday's continuation
        options.peak_usage_start = IcbTime::new(0, 0, 0);
        options.peak_usage_end = IcbTime::new(0, 0, 0);
        assert!(peak_in_timezone(&options, &[], utc("2026-09-06T12:00:00Z")));
        options.peak_usage_start = IcbTime::new(24, 0, 0);
        assert!(!peak_in_timezone(&options, &[], utc("2026-09-06T12:00:00Z")));
    }

    #[test]
    fn calendar_minutes_and_preview_grace() {
        let rates = AccountingConfig {
            charge_per_time: 2.0,
            charge_per_peak_time: 5.0,
            ..Default::default()
        };
        let mut options = options();
        options.peak_days_of_week = IcbDoW::default();
        let start = utc("2026-09-06T12:00:59Z");
        let end = utc("2026-09-06T12:01:00Z");
        assert_eq!(time_charge(&rates, &options, &[], start, end, false), 2.0);
        assert_eq!(time_charge(&rates, &options, &[], start, end, true), 0.0);
        assert_eq!(time_charge(&rates, &options, &[], end, start, false), 0.0);
        assert_eq!(time_charge(&rates, &options, &[], end, end + Duration::seconds(59), false), 0.0);
        let start = utc("2026-09-06T08:59:59Z");
        let end = utc("2026-09-06T09:02:00Z");
        let options = self::options();
        let classify = |at| peak_in_timezone(&options, &[], at);
        assert_eq!(charge_minutes(&rates, start, end, false, classify), 12.0);
        assert_eq!(charge_minutes(&rates, start, end, true, classify), 10.0); // normal, not last peak minute, is free
        let start = utc("2026-09-06T09:00:00Z");
        assert_eq!(charge_minutes(&rates, start, end, true, |_| true), 5.0);
        assert_eq!(minutes_used(Duration::milliseconds(29999)), 0);
        assert_eq!(minutes_used(Duration::seconds(30)), 1);
        assert_eq!(minutes_used(Duration::seconds(89)), 1);
        assert_eq!(minutes_used(Duration::seconds(90)), 2);
        assert_eq!(minutes_used(Duration::seconds(-90)), 0);
    }

    #[test]
    fn utc_iteration_handles_midnight_holidays_and_dst_transitions() {
        let rates = AccountingConfig {
            charge_per_time: 2.0,
            charge_per_peak_time: 5.0,
            ..Default::default()
        };
        let mut options = options();
        options.peak_usage_start = IcbTime::new(0, 0, 0);
        options.peak_usage_end = IcbTime::new(0, 0, 0);
        let start = utc("2026-12-31T23:59:40Z");
        let end = utc("2027-01-01T00:02:00Z");
        assert_eq!(
            charge_minutes(&rates, start, end, false, |at| peak_in_timezone(&options, &["01-01-XX".into()], at)),
            9.0
        );

        // Explicit offsets make this test independent of the test runner's TZ and
        // avoid mutating process-global environment while tests run concurrently.
        options.peak_usage_start = IcbTime::new(1, 0, 0);
        options.peak_usage_end = IcbTime::new(1, 59, 0);
        let transition = utc("2026-11-01T06:00:00Z");
        let classify = |at: DateTime<Utc>| {
            let offset = FixedOffset::west_opt(if at < transition { 4 * 3600 } else { 5 * 3600 }).unwrap();
            peak_in_timezone(&options, &[], at.with_timezone(&offset))
        };
        assert_eq!(
            charge_minutes(&rates, utc("2026-11-01T05:00:00Z"), utc("2026-11-01T07:00:00Z"), false, classify),
            600.0
        );
        options.peak_usage_start = IcbTime::new(2, 0, 0);
        options.peak_usage_end = IcbTime::new(2, 59, 0);
        let transition = utc("2026-03-08T07:00:00Z");
        let classify = |at: DateTime<Utc>| {
            let offset = FixedOffset::west_opt(if at < transition { 5 * 3600 } else { 4 * 3600 }).unwrap();
            peak_in_timezone(&options, &[], at.with_timezone(&offset))
        };
        assert_eq!(
            charge_minutes(&rates, utc("2026-03-08T06:59:00Z"), utc("2026-03-08T07:01:00Z"), false, classify),
            4.0
        );
    }

    #[test]
    fn tracking_ascii_and_dbf_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let ascii = dir.path().join("account.log");
        let dbf = dir.path().join("account.DBF");
        let mut entry = entry();
        entry.user = "A\r\nBé\t".into();
        entry.activity = "12345678901234567890".into();
        entry.sub_activity = "界".repeat(30);
        entry.unit_cost = -1.25;
        entry.value = -2.5;
        append_tracking(&ascii, &entry).unwrap();
        append_tracking(&dbf, &entry).unwrap();
        append_tracking(&dbf, &entry).unwrap();
        let bytes = std::fs::read(&ascii).unwrap();
        assert!(bytes.is_ascii());
        assert_eq!(bytes.len(), 136);
        assert!(bytes.ends_with(b"\r\n"));
        assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
        assert_eq!(&bytes[15..40], b"A  B?                    ");
        let mut db = DbaseFile::open(&dbf).unwrap();
        assert_eq!(db.record_count(), 2);
        assert!(db.goto(2).unwrap());
        let values = tracking_values(&entry, true).unwrap();
        for (index, value) in values.iter().enumerate() {
            assert_eq!(db.get_field(index), value.as_bytes());
        }
        assert_eq!(db.get_field(0), entry.at.format("%Y%m%d").to_string().as_bytes());
    }

    #[test]
    fn tracking_rejects_invalid_numbers_and_corrupt_dbf_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["account.log", "account.dbf"] {
            let path = dir.path().join(name);
            append_tracking(&path, &entry()).unwrap();
            let before = std::fs::read(&path).unwrap();
            for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
                let mut entry = entry();
                entry.value = value;
                assert!(append_tracking(&path, &entry).is_err());
                entry.value = 0.0;
                entry.unit_cost = value;
                assert!(append_tracking(&path, &entry).is_err());
            }
            let mut huge = entry();
            huge.quantity = i64::MAX;
            assert!(append_tracking(&path, &huge).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
        let path = dir.path().join("account.dbf");
        let valid = std::fs::read(&path).unwrap();
        for index in [0, 8, 10, 32, 48, 352] {
            let mut corrupt = valid.clone();
            corrupt[index] = 0;
            std::fs::write(&path, &corrupt).unwrap();
            assert!(append_tracking(&path, &entry()).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        }
        std::fs::write(&path, &valid[..valid.len() - 2]).unwrap();
        assert!(append_tracking(&path, &entry()).is_err());
        assert!(append_tracking(&dir.path().join("missing/account.log"), &entry()).is_err());
    }

    fn record_count(path: &Path) -> usize {
        if path.extension().unwrap() == "dbf" {
            DbaseFile::open(path).unwrap().record_count()
        } else {
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(bytes.len() % 136, 0);
            assert!(bytes.chunks(136).all(|line| line.ends_with(b"\r\n")));
            bytes.len() / 136
        }
    }

    #[test]
    fn tracking_serializes_threads() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["account.log", "account.dbf"] {
            let path = dir.path().join(name);
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    let path = &path;
                    scope.spawn(move || {
                        for _ in 0..12 {
                            append_tracking(path, &entry()).unwrap();
                        }
                    });
                }
            });
            assert_eq!(record_count(&path), 48);
            if name.ends_with("dbf") {
                let mut db = DbaseFile::open(&path).unwrap();
                for number in 1..=48 {
                    assert!(db.goto(number).unwrap());
                    assert_eq!(db.get_field(2), b"Alice                    ");
                }
            }
        }
    }

    #[test]
    #[ignore = "subprocess helper invoked by tracking_serializes_processes"]
    fn tracking_process_worker() {
        let Some(path) = std::env::var_os("ICY_ACCOUNTING_TRACKING_TEST_PATH") else {
            return;
        };
        for _ in 0..10 {
            append_tracking(Path::new(&path), &entry()).unwrap();
        }
    }

    #[test]
    fn tracking_serializes_processes() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["account.log", "account.dbf"] {
            let path = dir.path().join(name);
            let mut children = Vec::new();
            for _ in 0..3 {
                children.push(
                    std::process::Command::new(std::env::current_exe().unwrap())
                        .args(["--exact", "icy_board::accounting::tests::tracking_process_worker", "--ignored"])
                        .env("ICY_ACCOUNTING_TRACKING_TEST_PATH", &path)
                        .spawn()
                        .unwrap(),
                );
            }
            for mut child in children {
                assert!(child.wait().unwrap().success());
            }
            assert_eq!(record_count(&path), 30);
        }
    }
}
