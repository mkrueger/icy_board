//! The transfer limits PCBoard applied to a download.
//!
//! Faithful to `checkdlfiles`, `checkratio` and `checklimit` in the original source: each
//! file is judged on its own against everything already accepted into the batch, and a
//! file that fails is skipped while the rest of the batch still goes out.

use crate::icy_board::sec_levels::SecurityLevel;

/// What the caller is allowed, resolved from their PWRD entry when they log on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransferLimits {
    /// Bytes still available today; `None` is PCBoard's -1, an unlimited allowance.
    pub bytes_remaining: Option<i64>,
    /// Files the caller may download until the sysop resets the count. 0 disables it.
    pub total_file_limit: u64,
    /// Bytes the caller may download until the sysop resets the count. 0 disables it.
    pub total_byte_limit: u64,
    /// Download:upload file ratio in tenths. 0 disables it.
    pub file_ratio_tenths: u64,
    /// Download:upload byte ratio in tenths. 0 disables it.
    pub byte_ratio_tenths: u64,
    /// Counts as if the caller had uploaded this many files.
    pub file_credit: u64,
    /// Counts as if the caller had uploaded this many bytes.
    pub byte_credit: u64,
}

/// The caller's lifetime transfer counts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TransferHistory {
    pub num_uploads: u64,
    pub num_downloads: u64,
    pub total_upld_bytes: u64,
    pub total_dnld_bytes: u64,
}

/// What the batch has already taken, ignoring files that download for free.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BatchSoFar {
    pub files: u64,
    pub bytes: u64,
}

impl BatchSoFar {
    /// A free file still travels, but costs the caller nothing.
    pub fn accept(&mut self, size: u64, free: bool) {
        if !free {
            self.files += 1;
            self.bytes += size;
        }
    }
}

/// Why a file cannot go out. The figures are what the matching PCBTEXT prompts show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LimitVerdict {
    Allowed,
    DailyBytes { bytes_left: i64 },
    FileRatio { limit_tenths: u64, current_tenths: u64 },
    ByteRatio { limit_tenths: u64, current_tenths: u64 },
    FileLimit { limit: u64, downloaded: u64 },
    ByteLimit { limit: u64, downloaded: u64 },
}

impl LimitVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, LimitVerdict::Allowed)
    }
}

impl TransferLimits {
    /// Resolves the allowance from a PWRD entry. `bps` is the speed the caller connected
    /// at, which scales the daily allowance against the level's base baud rate.
    pub fn from_security_level(level: &SecurityLevel, bps: u32) -> Self {
        Self {
            bytes_remaining: daily_allowance(level, bps),
            total_file_limit: level.file_limit,
            total_byte_limit: level.file_kb_limit.saturating_mul(1024),
            file_ratio_tenths: level.uldl_ratio_tenths as u64,
            byte_ratio_tenths: level.uldl_kb_ratio_tenths as u64,
            file_credit: level.file_credit,
            byte_credit: level.file_kb_credit.saturating_mul(1024),
        }
    }

    /// Takes off what the caller has already used today, the way PCBoard does at logon.
    pub fn charge_todays_usage(&mut self, used_today: i64) {
        if let Some(remaining) = self.bytes_remaining {
            self.bytes_remaining = Some((remaining - used_today).max(0));
        }
    }

    /// Bytes left once the batch so far is paid for.
    pub fn bytes_left(&self, so_far: BatchSoFar) -> Option<i64> {
        self.bytes_remaining.map(|remaining| remaining - so_far.bytes as i64)
    }

    /// Judges one more file. `free` files bypass every limit, which is how PCBoard treats
    /// a download the FSEC file marked free.
    pub fn check_file(&self, history: &TransferHistory, so_far: BatchSoFar, size: u64, free: bool) -> LimitVerdict {
        if free {
            return LimitVerdict::Allowed;
        }

        if let Some(bytes_left) = self.bytes_left(so_far) {
            if size as i64 > bytes_left {
                return LimitVerdict::DailyBytes { bytes_left };
            }
        }

        if let Some(verdict) = check_ratio(
            self.file_ratio_tenths,
            history.num_uploads + self.file_credit,
            history.num_downloads + so_far.files,
            1,
        ) {
            return LimitVerdict::FileRatio {
                limit_tenths: self.file_ratio_tenths,
                current_tenths: verdict,
            };
        }

        if let Some(verdict) = check_ratio(
            self.byte_ratio_tenths,
            history.total_upld_bytes + self.byte_credit,
            history.total_dnld_bytes + so_far.bytes,
            size,
        ) {
            return LimitVerdict::ByteRatio {
                limit_tenths: self.byte_ratio_tenths,
                current_tenths: verdict,
            };
        }

        if exceeds_limit(self.total_file_limit, history.num_downloads, 1 + so_far.files) {
            return LimitVerdict::FileLimit {
                limit: self.total_file_limit,
                downloaded: history.num_downloads,
            };
        }

        let downloaded_bytes = history.total_dnld_bytes + so_far.bytes;
        if exceeds_limit(self.total_byte_limit, downloaded_bytes, size) {
            return LimitVerdict::ByteLimit {
                limit: self.total_byte_limit,
                downloaded: downloaded_bytes,
            };
        }

        LimitVerdict::Allowed
    }

    /// Bytes the caller may still download once the daily allowance, the total limit and
    /// the byte ratio have each had their say. `None` when nothing constrains them.
    pub fn bytes_available(&self, history: &TransferHistory, so_far: BatchSoFar) -> Option<i64> {
        let current = so_far.bytes as i64;
        let mut unlimited = self.bytes_remaining.is_none();
        let mut limit = self.bytes_remaining.map_or(i64::MAX, |remaining| remaining - current);

        if self.total_byte_limit > 0 {
            let left = self.total_byte_limit as i64 - history.total_dnld_bytes as i64 - current;
            if left < limit {
                limit = left;
                unlimited = false;
            }
        }

        if self.byte_ratio_tenths > 0 {
            let up = (history.total_upld_bytes + self.byte_credit) as i128;
            let allowed = (up * self.byte_ratio_tenths as i128) / 10;
            let left = (allowed - (history.total_dnld_bytes as i128 + current as i128)).max(0) as i64;
            if left < limit {
                limit = left;
                unlimited = false;
            }
        }

        if unlimited { None } else { Some(limit.max(0)) }
    }
}

/// The caller's standing as PCBoard prints it: always anchored on 1, so a caller who has
/// taken more than they gave reads "5.0:1" and one who gave more reads "1:5.0".
pub fn format_ratio(down: u64, up: u64) -> String {
    fn part(a: u64, b: u64) -> String {
        if b == 0 {
            format!("{:.1}", a as f64)
        } else {
            format!("{:.1}", a as f64 / b as f64)
        }
    }
    if down > up {
        format!("{}{}", part(down, up), if up == 0 { ":0" } else { ":1" })
    } else if down == up {
        if up == 0 { "0:0".to_string() } else { "1:1".to_string() }
    } else {
        format!("{}{}", if down == 0 { "0:" } else { "1:" }, part(up, down))
    }
}

/// PCBoard's daily allowance: 32767 K means unlimited, and the figure is scaled by how
/// far the caller's speed is from the level's base baud rate.
fn daily_allowance(level: &SecurityLevel, bps: u32) -> Option<i64> {
    const UNLIMITED_KB: u64 = 32767;
    if level.daily_file_kb_limit == UNLIMITED_KB {
        return None;
    }
    let mut bytes = (level.daily_file_kb_limit as i64).saturating_mul(1024);
    let base = level.base_baud_rate;
    if base != 0 {
        if bps >= base {
            bytes = bytes.saturating_mul((bps / base) as i64);
        } else if bps != 0 {
            bytes /= (base / bps) as i64;
        }
    }
    Some(bytes)
}

/// Returns the caller's current ratio in tenths when one more transfer would break the
/// allowed one, mirroring `checkratio`: `up * ratio / 10 - down` has to cover `new`.
fn check_ratio(ratio_tenths: u64, up: u64, down: u64, new: u64) -> Option<u64> {
    if ratio_tenths == 0 {
        return None;
    }
    // PCBoard substitutes 1 rather than dividing by zero, so a caller who has never
    // uploaded still gets the ratio's worth of downloads.
    let up = up.max(1);
    let allowed = (up as u128 * ratio_tenths as u128) / 10;
    let left = allowed as i128 - down as i128;
    if left < new as i128 {
        return Some(((down as u128 * 10) / up as u128) as u64);
    }
    None
}

fn exceeds_limit(limit: u64, downloaded: u64, new: u64) -> bool {
    limit != 0 && downloaded + new > limit
}

/// Seconds PCBoard reckons a transfer needs: the raw time at the caller's speed, seven
/// percent for protocol overhead, and ten seconds so every transfer costs something.
pub fn seconds_for_transfer(size: u64, bps: u32) -> i64 {
    let cps = (bps / 10).max(1) as u64;
    ((size / cps) * 107 / 100) as i64 + 10
}

/// Minutes the caller gets this session, with earlier calls today taken off when the
/// board enforces a daily limit rather than a per-session one.
///
/// Never returns zero, which elsewhere means an unlimited session: a caller who has
/// already used the day up is given a minute and then hung up on, which is close to what
/// PCBoard does with its own one minute of slack.
pub fn session_time_limit(time_per_day: u32, minutes_used_today: u16, enforce_daily: bool) -> i32 {
    if !enforce_daily {
        return time_per_day as i32;
    }
    (time_per_day as i32 - minutes_used_today as i32).max(1)
}

/// Whether the caller's session time is gone. A limit of zero is an unlimited session,
/// which is what `@TIMELEFT@` has always reported it as.
pub fn session_expired(time_limit: i32, minutes_online: i64) -> bool {
    time_limit != 0 && minutes_online >= time_limit as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> TransferLimits {
        TransferLimits {
            bytes_remaining: None,
            ..Default::default()
        }
    }

    fn level(daily_kb: u64, base_baud: u32) -> SecurityLevel {
        SecurityLevel {
            daily_file_kb_limit: daily_kb,
            base_baud_rate: base_baud,
            ..Default::default()
        }
    }

    #[test]
    fn nothing_is_refused_when_no_limit_is_configured() {
        let history = TransferHistory {
            num_downloads: 900,
            total_dnld_bytes: 9_000_000,
            ..Default::default()
        };
        assert_eq!(limits().check_file(&history, BatchSoFar::default(), 100_000, false), LimitVerdict::Allowed);
    }

    // --- daily allowance -------------------------------------------------------

    #[test]
    fn the_unlimited_marker_lifts_the_daily_allowance() {
        assert_eq!(TransferLimits::from_security_level(&level(32767, 0), 2400).bytes_remaining, None);
    }

    #[test]
    fn a_daily_allowance_is_kilobytes() {
        assert_eq!(TransferLimits::from_security_level(&level(400, 0), 2400).bytes_remaining, Some(400 * 1024));
    }

    /// A caller at four times the base rate gets four times the bytes.
    #[test]
    fn a_faster_caller_gets_a_bigger_allowance() {
        assert_eq!(TransferLimits::from_security_level(&level(400, 2400), 9600).bytes_remaining, Some(1600 * 1024));
    }

    #[test]
    fn a_slower_caller_gets_a_smaller_allowance() {
        assert_eq!(TransferLimits::from_security_level(&level(400, 9600), 2400).bytes_remaining, Some(100 * 1024));
    }

    #[test]
    fn todays_usage_comes_off_the_allowance() {
        let mut limits = TransferLimits::from_security_level(&level(100, 0), 2400);
        limits.charge_todays_usage(60 * 1024);
        assert_eq!(limits.bytes_remaining, Some(40 * 1024));
    }

    /// Uploading earns credit, which PCBoard records as a negative daily figure.
    #[test]
    fn upload_credit_raises_the_allowance() {
        let mut limits = TransferLimits::from_security_level(&level(100, 0), 2400);
        limits.charge_todays_usage(-20 * 1024);
        assert_eq!(limits.bytes_remaining, Some(120 * 1024));
    }

    #[test]
    fn a_spent_allowance_never_goes_negative() {
        let mut limits = TransferLimits::from_security_level(&level(100, 0), 2400);
        limits.charge_todays_usage(500 * 1024);
        assert_eq!(limits.bytes_remaining, Some(0));
    }

    #[test]
    fn a_file_bigger_than_the_allowance_is_refused() {
        let limits = TransferLimits {
            bytes_remaining: Some(1000),
            ..Default::default()
        };
        let verdict = limits.check_file(&TransferHistory::default(), BatchSoFar::default(), 1001, false);
        assert_eq!(verdict, LimitVerdict::DailyBytes { bytes_left: 1000 });
    }

    #[test]
    fn a_file_that_exactly_fits_the_allowance_goes_out() {
        let limits = TransferLimits {
            bytes_remaining: Some(1000),
            ..Default::default()
        };
        assert!(limits.check_file(&TransferHistory::default(), BatchSoFar::default(), 1000, false).is_allowed());
    }

    /// The allowance is spent by the batch as it is assembled, not just by past days.
    #[test]
    fn the_batch_so_far_eats_into_the_allowance() {
        let limits = TransferLimits {
            bytes_remaining: Some(1000),
            ..Default::default()
        };
        let so_far = BatchSoFar { files: 1, bytes: 600 };
        let verdict = limits.check_file(&TransferHistory::default(), so_far, 500, false);
        assert_eq!(verdict, LimitVerdict::DailyBytes { bytes_left: 400 });
    }

    #[test]
    fn a_free_file_ignores_every_limit() {
        let limits = TransferLimits {
            bytes_remaining: Some(0),
            file_ratio_tenths: 10,
            total_file_limit: 1,
            ..Default::default()
        };
        assert!(
            limits
                .check_file(&TransferHistory::default(), BatchSoFar::default(), 999_999, true)
                .is_allowed()
        );
    }

    #[test]
    fn a_free_file_does_not_spend_the_allowance() {
        let mut so_far = BatchSoFar::default();
        so_far.accept(5000, true);
        so_far.accept(300, false);
        assert_eq!(so_far, BatchSoFar { files: 1, bytes: 300 });
    }

    // --- ratios ----------------------------------------------------------------

    #[test]
    fn a_ratio_of_zero_is_disabled() {
        let limits = TransferLimits {
            file_ratio_tenths: 0,
            ..Default::default()
        };
        let history = TransferHistory {
            num_downloads: 500,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());
    }

    /// A 5.0:1 ratio lets five files out for each one uploaded.
    #[test]
    fn the_file_ratio_allows_its_share() {
        let limits = TransferLimits {
            file_ratio_tenths: 50,
            ..Default::default()
        };
        let history = TransferHistory {
            num_uploads: 2,
            num_downloads: 9,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());
    }

    #[test]
    fn the_file_ratio_refuses_one_too_many() {
        let limits = TransferLimits {
            file_ratio_tenths: 50,
            ..Default::default()
        };
        let history = TransferHistory {
            num_uploads: 2,
            num_downloads: 10,
            ..Default::default()
        };
        let verdict = limits.check_file(&history, BatchSoFar::default(), 1, false);
        assert_eq!(
            verdict,
            LimitVerdict::FileRatio {
                limit_tenths: 50,
                current_tenths: 50
            }
        );
    }

    /// Without the substitution a caller who never uploaded would divide by zero; PCBoard
    /// counts them as having uploaded one.
    #[test]
    fn a_caller_who_never_uploaded_still_gets_the_ratios_worth() {
        let limits = TransferLimits {
            file_ratio_tenths: 30,
            ..Default::default()
        };
        let history = TransferHistory::default();
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());

        let spent = TransferHistory {
            num_downloads: 3,
            ..Default::default()
        };
        assert!(!limits.check_file(&spent, BatchSoFar::default(), 1, false).is_allowed());
    }

    /// Credits stand in for uploads the caller has not made yet.
    #[test]
    fn file_credit_buys_a_grace_period() {
        let limits = TransferLimits {
            file_ratio_tenths: 10,
            file_credit: 10,
            ..Default::default()
        };
        let history = TransferHistory {
            num_downloads: 9,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());

        let spent = TransferHistory {
            num_downloads: 10,
            ..Default::default()
        };
        assert!(!limits.check_file(&spent, BatchSoFar::default(), 1, false).is_allowed());
    }

    #[test]
    fn the_byte_ratio_weighs_the_file_being_asked_for() {
        let limits = TransferLimits {
            byte_ratio_tenths: 100,
            ..Default::default()
        };
        let history = TransferHistory {
            total_upld_bytes: 1000,
            total_dnld_bytes: 9000,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1000, false).is_allowed());
        assert!(!limits.check_file(&history, BatchSoFar::default(), 1001, false).is_allowed());
    }

    #[test]
    fn byte_credit_is_counted_in_kilobytes() {
        let level = SecurityLevel {
            file_kb_credit: 20,
            ..Default::default()
        };
        assert_eq!(TransferLimits::from_security_level(&level, 2400).byte_credit, 20 * 1024);
    }

    // --- total limits ----------------------------------------------------------

    #[test]
    fn a_total_limit_of_zero_is_disabled() {
        let limits = TransferLimits {
            total_file_limit: 0,
            ..Default::default()
        };
        let history = TransferHistory {
            num_downloads: 10_000,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());
    }

    #[test]
    fn the_total_file_limit_stops_the_last_one() {
        let limits = TransferLimits {
            total_file_limit: 10,
            ..Default::default()
        };
        let history = TransferHistory {
            num_downloads: 9,
            ..Default::default()
        };
        assert!(limits.check_file(&history, BatchSoFar::default(), 1, false).is_allowed());

        let history = TransferHistory {
            num_downloads: 10,
            ..Default::default()
        };
        assert_eq!(
            limits.check_file(&history, BatchSoFar::default(), 1, false),
            LimitVerdict::FileLimit { limit: 10, downloaded: 10 }
        );
    }

    #[test]
    fn the_total_byte_limit_counts_the_batch_so_far() {
        let limits = TransferLimits {
            total_byte_limit: 10_000,
            ..Default::default()
        };
        let history = TransferHistory {
            total_dnld_bytes: 8_000,
            ..Default::default()
        };
        let so_far = BatchSoFar { files: 1, bytes: 1_500 };
        assert!(limits.check_file(&history, so_far, 500, false).is_allowed());
        assert_eq!(
            limits.check_file(&history, so_far, 501, false),
            LimitVerdict::ByteLimit {
                limit: 10_000,
                downloaded: 9_500
            }
        );
    }

    /// The daily allowance is answered before the ratio, so the caller is told the most
    /// immediate reason first.
    #[test]
    fn the_daily_allowance_is_reported_before_a_ratio() {
        let limits = TransferLimits {
            bytes_remaining: Some(10),
            file_ratio_tenths: 10,
            ..Default::default()
        };
        let history = TransferHistory {
            num_downloads: 500,
            ..Default::default()
        };
        assert_eq!(
            limits.check_file(&history, BatchSoFar::default(), 100, false),
            LimitVerdict::DailyBytes { bytes_left: 10 }
        );
    }

    // --- what is left ----------------------------------------------------------

    #[test]
    fn nothing_constrains_a_caller_without_limits() {
        assert_eq!(limits().bytes_available(&TransferHistory::default(), BatchSoFar::default()), None);
    }

    #[test]
    fn the_tightest_limit_decides_what_is_left() {
        let limits = TransferLimits {
            bytes_remaining: Some(50_000),
            total_byte_limit: 30_000,
            ..Default::default()
        };
        let history = TransferHistory {
            total_dnld_bytes: 25_000,
            ..Default::default()
        };
        assert_eq!(limits.bytes_available(&history, BatchSoFar::default()), Some(5_000));
    }

    #[test]
    fn a_ratio_can_be_the_tightest_limit() {
        let limits = TransferLimits {
            bytes_remaining: Some(1_000_000),
            byte_ratio_tenths: 10,
            ..Default::default()
        };
        let history = TransferHistory {
            total_upld_bytes: 5_000,
            total_dnld_bytes: 4_000,
            ..Default::default()
        };
        assert_eq!(limits.bytes_available(&history, BatchSoFar::default()), Some(1_000));
    }

    #[test]
    fn what_is_left_never_goes_negative() {
        let limits = TransferLimits {
            bytes_remaining: Some(100),
            ..Default::default()
        };
        let so_far = BatchSoFar { files: 1, bytes: 500 };
        assert_eq!(limits.bytes_available(&TransferHistory::default(), so_far), Some(0));
    }

    // --- how a ratio reads -----------------------------------------------------

    #[test]
    fn a_ratio_is_anchored_on_one() {
        assert_eq!(format_ratio(10_000, 2_000), "5.0:1");
        assert_eq!(format_ratio(2_000, 10_000), "1:5.0");
        assert_eq!(format_ratio(100, 100), "1:1");
    }

    #[test]
    fn a_ratio_reads_sensibly_with_nothing_on_one_side() {
        assert_eq!(format_ratio(0, 0), "0:0");
        assert_eq!(format_ratio(500, 0), "500.0:0");
        assert_eq!(format_ratio(0, 500), "0:500.0");
    }

    // --- how long a transfer takes ---------------------------------------------

    /// 2400 bps is 240 characters a second, so 24000 bytes is 100 seconds before the
    /// overhead and the flat ten seconds are added.
    #[test]
    fn a_transfer_is_costed_at_the_callers_speed() {
        assert_eq!(seconds_for_transfer(24_000, 2400), 117);
    }

    #[test]
    fn a_faster_caller_is_charged_less_time() {
        assert!(seconds_for_transfer(24_000, 9600) < seconds_for_transfer(24_000, 2400));
    }

    /// Even an empty file costs the handshake, so a batch cannot be free of time.
    #[test]
    fn every_transfer_costs_at_least_the_handshake() {
        assert_eq!(seconds_for_transfer(0, 2400), 10);
    }

    #[test]
    fn a_local_caller_without_a_speed_is_still_costed() {
        assert!(seconds_for_transfer(1_000_000, 0) > 0);
    }

    // --- how much of the day is left -------------------------------------------

    /// A per-session limit hands out the whole allowance on every call.
    #[test]
    fn a_session_limit_ignores_earlier_calls() {
        assert_eq!(session_time_limit(60, 45, false), 60);
    }

    #[test]
    fn a_daily_limit_counts_earlier_calls() {
        assert_eq!(session_time_limit(60, 45, true), 15);
    }

    /// Zero would read as an unlimited session everywhere else, so a caller who has spent
    /// the day gets a minute rather than the run of the board.
    #[test]
    fn a_spent_day_does_not_turn_into_unlimited_time() {
        assert_eq!(session_time_limit(60, 60, true), 1);
        assert_eq!(session_time_limit(60, 500, true), 1);
    }

    #[test]
    fn a_session_ends_when_its_minutes_are_gone() {
        assert!(!session_expired(30, 29));
        assert!(session_expired(30, 30));
        assert!(session_expired(30, 31));
    }

    #[test]
    fn an_unlimited_session_never_ends() {
        assert!(!session_expired(0, 100_000));
    }
}
