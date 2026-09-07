//! Event scheduling contract (local wall time, second resolution).
//!
//! * No interval: the original one-start-per-selected-day behavior. Positive
//!   intervals enumerate `time + n * interval_minutes` on selected weekdays,
//!   bounded by the inclusive `end_time` second or the end of that calendar day.
//! * Overnight windows are rejected, not inferred. DST gaps are skipped; ambiguous
//!   slots use the first fold. `end_time` limits starting, never running duration.
//! * New records get UUID-v4 strings using existing rand_core OS randomness.
//!   Missing IDs use SHA-256 of the canonical fully defaulted serialized record
//!   with an empty ID. Load does not write the schedule; the next editor save
//!   persists this fallback. Before that save, changing any record field changes
//!   its legacy identity. Reordering/whitespace/default spelling does not.
//!   Identical legacy records collide deliberately and must be assigned distinct
//!   IDs explicitly. Never assign fresh random IDs during deserialization.
//! * An edit/clone preserves ID. A duplicate/new record must use `new_id()`.
//! * Online is untrusted admin shell execution, not Fido-safe maintenance. It must
//!   not modify live board data and must remain foreground with no interactive
//!   input/background descendants. Fixed Online runs with callers; Slide Online
//!   waits for no callers without closing admission; Idle Online skips when busy.
//! * Explicit manual requests may run disabled records with global scheduling off
//!   and override day/start/end restrictions. Maintenance Fixed drains immediately,
//!   Slide closes admission and waits naturally, Idle skips online callers.
//!
//! History is exposed through `events::event_history::EventHistory::read_entries`.

use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[path = "event_history.rs"]
pub mod event_history;

use crate::datetime::{IcbDoW, IcbTime};

use super::{IcyBoardSerializer, icb_config::EventOptions};

/// What happens to the callers that are still online when the clock reaches an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventMode {
    /// Log everybody off and run on time.
    #[default]
    Fixed,
    /// Let the last caller finish the session, then run.
    Slide,
    /// Skip this occurrence when somebody is online.
    Idle,
}

/// Online is an administrator-supplied shell command, NOT a Fido-safe operation.
/// It must not modify live board data, fork background work, or require input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventExecution {
    #[default]
    Maintenance,
    Online,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EventFields")]
pub struct BoardEvent {
    /// Persist on save; cloning for editing preserves this, duplicating must renew it.
    pub id: String,
    #[serde(default)]
    pub description: String,

    #[serde(default = "enabled_by_default")]
    pub enabled: bool,

    #[serde(default)]
    pub time: IcbTime,

    #[serde(default = "IcbDoW::all")]
    pub days: IcbDoW,

    #[serde(default)]
    pub mode: EventMode,

    /// Handed to the shell when the event fires; nothing is run when it is empty.
    #[serde(default)]
    pub command: String,

    /// Inclusive latest local start, same day only. Earlier than `time` is invalid.
    pub end_time: Option<IcbTime>,
    /// Positive wall-clock minutes from `time`, not from the previous completion.
    pub interval_minutes: Option<u32>,
    /// Warn only: never kill a shell or its descendants on a timer.
    pub warning_minutes: Option<u32>,
    pub execution: EventExecution,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventFields {
    id: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(default, deserialize_with = "read_time")]
    time: IcbTime,
    #[serde(default = "IcbDoW::all", deserialize_with = "read_days")]
    days: IcbDoW,
    #[serde(default)]
    mode: EventMode,
    #[serde(default)]
    command: String,
    #[serde(default, deserialize_with = "read_optional_time")]
    end_time: Option<IcbTime>,
    interval_minutes: Option<u32>,
    warning_minutes: Option<u32>,
    #[serde(default)]
    execution: EventExecution,
}

fn checked_time<E: serde::de::Error>(value: toml::value::Datetime) -> Result<IcbTime, E> {
    let time = value.time.ok_or_else(|| E::custom("event time must be a local time"))?;
    if value.date.is_some() || value.offset.is_some() || time.nanosecond.unwrap_or(0) != 0 {
        return Err(E::custom("event time must be a local time with whole seconds"));
    }
    let time = IcbTime::new(time.hour, time.minute, time.second.unwrap_or(0));
    if time.get_hour() > 23 || time.get_minute() > 59 || time.get_second() > 59 {
        return Err(E::custom("invalid event time"));
    }
    Ok(time)
}

fn read_time<'de, D: serde::Deserializer<'de>>(d: D) -> Result<IcbTime, D::Error> {
    checked_time(toml::value::Datetime::deserialize(d)?)
}

fn read_days<'de, D: serde::Deserializer<'de>>(d: D) -> Result<IcbDoW, D::Error> {
    let days = String::deserialize(d)?;
    if days.len() != 7 || !days.bytes().all(|b| b == b'Y' || b == b'N') {
        return Err(serde::de::Error::custom("event days must contain seven Y/N characters, Sunday first"));
    }
    Ok(IcbDoW::from(days))
}

fn read_optional_time<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<IcbTime>, D::Error> {
    Option::<toml::value::Datetime>::deserialize(d)?.map(checked_time).transpose()
}

impl TryFrom<EventFields> for BoardEvent {
    type Error = String;

    fn try_from(fields: EventFields) -> Result<Self, Self::Error> {
        let legacy = fields.id.is_none();
        let mut event = Self {
            id: fields.id.unwrap_or_default(),
            description: fields.description,
            enabled: fields.enabled,
            time: fields.time,
            days: fields.days,
            mode: fields.mode,
            command: fields.command,
            end_time: fields.end_time,
            interval_minutes: fields.interval_minutes,
            warning_minutes: fields.warning_minutes,
            execution: fields.execution,
        };
        if legacy {
            // Hash the canonical, fully defaulted record (empty ID), never file order
            // or randomness on load. Identical legacy records deliberately share an ID.
            let canonical = toml::to_string(&event).map_err(|e| e.to_string())?;
            event.id = format!("legacy-{:x}", Sha256::digest(canonical.as_bytes()));
        }
        event.validate()?;
        Ok(event)
    }
}

impl BoardEvent {
    /// UUID v4 using the existing OS-random dependency; no UUID crate is required.
    pub fn new_id() -> String {
        use rand_core::RngCore;
        let mut bytes = [0u8; 16];
        rand_core::OsRng.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || self.id.len() > 128 || !self.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
            return Err("event id must contain 1..128 ASCII letters, digits, '-' or '_'".into());
        }
        for time in std::iter::once(&self.time).chain(self.end_time.iter()) {
            if time.get_hour() > 23 || time.get_minute() > 59 || time.get_second() > 59 {
                return Err("invalid event time".into());
            }
        }
        if self.end_time.as_ref().is_some_and(|end| end.to_pcboard_time() < self.time.to_pcboard_time()) {
            return Err("overnight event windows are unsupported; end_time must be >= time".into());
        }
        if self.interval_minutes == Some(0) || self.warning_minutes == Some(0) {
            return Err("interval_minutes and warning_minutes must be positive when present".into());
        }
        Ok(())
    }

    /// Enumerate local wall-clock slots, choosing the first fold and skipping DST gaps.
    pub fn next_occurrence(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
        if !self.enabled || self.days.is_empty() || self.validate().is_err() {
            return None;
        }
        let start = i64::from(self.time.to_pcboard_time());
        let end = self.end_time.as_ref().map_or(86399, |t| i64::from(t.to_pcboard_time()));
        let step = self.interval_minutes.map_or(86400, |m| i64::from(m) * 60);
        for offset in 0..8 {
            let date = now.date_naive() + Duration::days(offset);
            if !self.days.contains(date.weekday()) {
                continue;
            }
            let mut seconds = start;
            while seconds <= end {
                let naive = date.and_hms_opt(0, 0, 0)? + Duration::seconds(seconds);
                if let Some(at) = Local.from_local_datetime(&naive).earliest() {
                    if at > *now {
                        return Some(at);
                    }
                }
                seconds += step;
            }
        }
        None
    }

    /// A retained occurrence may wait beyond midnight only when no end is specified.
    pub fn expired(&self, scheduled_for: DateTime<Local>, now: DateTime<Local>) -> bool {
        self.end_time.as_ref().is_some_and(|end| {
            now.date_naive() > scheduled_for.date_naive()
                || (now.date_naive() == scheduled_for.date_naive() && now.time().num_seconds_from_midnight() > end.to_pcboard_time() as u32)
        })
    }
}

fn enabled_by_default() -> bool {
    true
}

impl Default for BoardEvent {
    fn default() -> Self {
        Self {
            id: Self::new_id(),
            description: String::new(),
            enabled: true,
            time: IcbTime::default(),
            days: IcbDoW::all(),
            mode: EventMode::default(),
            command: String::new(),
            end_time: None,
            interval_minutes: None,
            warning_minutes: None,
            execution: EventExecution::Maintenance,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EventListFields")]
pub struct EventList {
    #[serde(rename = "event", default)]
    events: Vec<BoardEvent>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventListFields {
    #[serde(rename = "event", default)]
    events: Vec<BoardEvent>,
}

impl TryFrom<EventListFields> for EventList {
    type Error = String;
    fn try_from(fields: EventListFields) -> Result<Self, Self::Error> {
        let list = Self { events: fields.events };
        list.validate()?;
        Ok(list)
    }
}

impl IcyBoardSerializer for EventList {
    const FILE_TYPE: &'static str = "events";

    fn save<P: AsRef<std::path::Path>>(&self, path: &P) -> crate::Res<()> {
        self.validate()?;
        super::save_internal(self, path)
    }
}

impl std::ops::Deref for EventList {
    type Target = Vec<BoardEvent>;
    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl std::ops::DerefMut for EventList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

impl EventList {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::HashSet::new();
        for event in &self.events {
            event.validate()?;
            if !ids.insert(&event.id) {
                return Err(format!("duplicate event ID '{}'; duplicated events need a new ID", event.id));
            }
        }
        Ok(())
    }

    /// The event that fires next, and when. Ties are broken by the order in the file.
    pub fn next_occurrence(&self, now: &DateTime<Local>) -> Option<(&BoardEvent, DateTime<Local>)> {
        let mut best: Option<(&BoardEvent, DateTime<Local>)> = None;
        for event in &self.events {
            if let Some(at) = event.next_occurrence(now) {
                if best.as_ref().is_none_or(|(_, best_at)| at < *best_at) {
                    best = Some((event, at));
                }
            }
        }
        best
    }
}

/// The next event together with the moments the board starts turning callers away.
#[derive(Clone, Debug, PartialEq)]
pub struct EventWindow {
    pub event: BoardEvent,
    pub run_at: DateTime<Local>,
    pub suspend_at: DateTime<Local>,
    pub uploads_stop_at: Option<DateTime<Local>>,
}

impl EventWindow {
    /// Nobody may log on any more and the callers still online are asked to leave.
    pub fn is_suspended(&self, now: &DateTime<Local>) -> bool {
        self.event.execution == EventExecution::Maintenance && self.event.mode == EventMode::Fixed && *now >= self.suspend_at
    }

    pub fn uploads_blocked(&self, now: &DateTime<Local>) -> bool {
        self.event.execution == EventExecution::Maintenance && self.event.mode == EventMode::Fixed && self.uploads_stop_at.is_some_and(|stop| *now >= stop)
    }

    /// Minutes a session started now may still last.
    pub fn minutes_until_suspend(&self, now: &DateTime<Local>) -> i64 {
        if self.event.execution == EventExecution::Online || self.event.mode != EventMode::Fixed {
            // Session APIs historically narrow minutes to i32.
            return i64::from(i32::MAX);
        }
        (self.suspend_at - *now).num_minutes().max(0)
    }
}

/// Admission/session window, maintenance only. Use EventList::next_occurrence for
/// a display of ALL executions; an Online event must not hide a later Fixed cap.
pub fn next_window(options: &EventOptions, events: &EventList, now: &DateTime<Local>) -> Option<EventWindow> {
    if !options.enabled {
        return None;
    }
    let (event, run_at) = events
        .iter()
        .filter(|event| event.execution == EventExecution::Maintenance)
        .filter_map(|event| event.next_occurrence(now).map(|at| (event, at)))
        .min_by_key(|(_, at)| *at)?;
    Some(event_window(options, event.clone(), run_at))
}

/// Construct a retained occurrence, including one whose scheduled time has passed.
pub fn event_window(options: &EventOptions, event: BoardEvent, run_at: DateTime<Local>) -> EventWindow {
    let uploads_stop_at = if options.disallow_uploads {
        Some(run_at - Duration::minutes(options.minutes_uploads_disallowed as i64))
    } else {
        None
    };
    EventWindow {
        event,
        run_at,
        suspend_at: run_at - Duration::minutes(options.suspend_minutes as i64),
        uploads_stop_at,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(year, month, day, hour, minute, 0).unwrap()
    }

    fn daily(time: &str) -> BoardEvent {
        BoardEvent {
            time: IcbTime::parse(time),
            ..Default::default()
        }
    }

    #[test]
    fn intervals_are_start_anchored_bounded_and_reset_on_matching_days() {
        let event = BoardEvent {
            interval_minutes: Some(17),
            end_time: Some(IcbTime::parse("04:00:00")),
            days: IcbDoW::from("NYNNNNN".to_string()),
            ..daily("03:05:00")
        };
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 3, 6)), Some(at(2024, 6, 3, 3, 22)));
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 3, 40)), Some(at(2024, 6, 3, 3, 56)));
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 3, 56)), Some(at(2024, 6, 10, 3, 5)));
        assert!(!event.expired(at(2024, 6, 3, 3, 5), at(2024, 6, 3, 4, 0)));
        assert!(event.expired(at(2024, 6, 3, 3, 5), at(2024, 6, 3, 4, 1)));
        assert!(event.expired(at(2024, 6, 3, 3, 5), at(2024, 6, 4, 0, 0)));
    }

    #[test]
    fn end_is_inclusive_and_unbounded_intervals_do_not_roll_into_next_day() {
        let mut event = BoardEvent {
            interval_minutes: Some(30),
            end_time: Some(IcbTime::parse("03:30:00")),
            ..daily("03:00:00")
        };
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 3, 0)), Some(at(2024, 6, 3, 3, 30)));
        event.time = IcbTime::parse("23:45:00");
        event.end_time = None;
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 23, 45)), Some(at(2024, 6, 4, 23, 45)));
        event.interval_minutes = Some(u32::MAX);
        assert_eq!(event.next_occurrence(&at(2024, 6, 3, 23, 45)), Some(at(2024, 6, 4, 23, 45)));
    }

    #[test]
    fn serde_rejects_invalid_windows_durations_times_days_and_unknown_fields() {
        for source in [
            "interval_minutes = 0",
            "interval_minutes = -1",
            "interval_minutes = 1.5",
            "warning_minutes = 0",
            "time = 23:00:00\nend_time = 02:00:00",
            "time = 2024-06-03",
            "time = 2024-06-03T03:00:00Z",
            "time = 03:00:00.001",
            "days = 'YYYYYYYYYYYY'",
            "days = 'yes'",
            "execution = 'fido'",
            "typo = 1",
            "id = ''",
            "id = '../escape'",
        ] {
            assert!(toml::from_str::<BoardEvent>(source).is_err(), "accepted {source}");
        }
        let event: BoardEvent =
            toml::from_str("time = 03:00:00\nend_time = 03:00:00\ninterval_minutes = 1\nwarning_minutes = 5\nexecution = 'online'").unwrap();
        assert_eq!(event.execution, EventExecution::Online);
        assert_eq!(event.end_time, Some(IcbTime::parse("03:00:00")));
    }

    #[test]
    fn legacy_ids_are_deterministic_canonical_and_saved_edits_preserve_identity() {
        let mut event: BoardEvent = toml::from_str("command = 'true'\ntime = 03:00:00").unwrap();
        let reordered: BoardEvent = toml::from_str("time = 03:00:00\ncommand = 'true'\nenabled = true").unwrap();
        assert_eq!(event.id, reordered.id);
        assert!(event.id.starts_with("legacy-"));
        let id = event.id.clone();
        event.command = "new command".into();
        assert_eq!(toml::from_str::<BoardEvent>(&toml::to_string(&event).unwrap()).unwrap().id, id);
        let new = BoardEvent::default();
        assert_ne!(new.id, BoardEvent::default().id);
        assert_eq!(new.id.len(), 36);
        assert_eq!(&new.id[14..15], "4");
        assert_eq!(toml::from_str::<BoardEvent>(&toml::to_string(&new).unwrap()).unwrap(), new);
    }

    #[test]
    fn duplicate_ids_are_not_silently_assigned_new_history_identity() {
        let mut list = EventList::default();
        let event = daily("03:00:00");
        list.extend([event.clone(), event]);
        assert!(list.validate().is_err());
        assert!(toml::from_str::<EventList>(&toml::to_string(&list).unwrap()).is_err());
        list[1].id = BoardEvent::new_id();
        assert!(list.validate().is_ok());
        assert!(toml::from_str::<EventList>(&toml::to_string(&list).unwrap()).is_ok());
    }

    #[test]
    fn online_fixed_never_restricts_sessions_or_masks_a_maintenance_window() {
        let options = EventOptions {
            enabled: true,
            disallow_uploads: true,
            suspend_minutes: 10,
            ..Default::default()
        };
        let event = BoardEvent {
            execution: EventExecution::Online,
            ..daily("03:00:00")
        };
        let window = event_window(&options, event.clone(), at(2024, 6, 3, 3, 0));
        assert!(!window.is_suspended(&at(2024, 6, 3, 4, 0)));
        assert!(!window.uploads_blocked(&at(2024, 6, 3, 4, 0)));
        assert_eq!(window.minutes_until_suspend(&at(2024, 6, 3, 4, 0)), i64::from(i32::MAX));
        let mut list = EventList::default();
        list.push(event);
        assert!(next_window(&options, &list, &at(2024, 6, 3, 1, 0)).is_none());
        list.push(daily("04:00:00"));
        assert_eq!(next_window(&options, &list, &at(2024, 6, 3, 1, 0)).unwrap().run_at, at(2024, 6, 3, 4, 0));
        assert_eq!(list.next_occurrence(&at(2024, 6, 3, 1, 0)).unwrap().1, at(2024, 6, 3, 3, 0));
    }

    #[test]
    fn test_a_daily_event_still_ahead_today_fires_today() {
        let mut list = EventList::default();
        list.push(daily("03:00:00"));
        // 2024-06-03 is a Monday.
        let (_, run_at) = list.next_occurrence(&at(2024, 6, 3, 1, 0)).unwrap();
        assert_eq!(at(2024, 6, 3, 3, 0), run_at);
    }

    #[test]
    fn test_a_daily_event_already_past_fires_tomorrow() {
        let mut list = EventList::default();
        list.push(daily("03:00:00"));
        let (_, run_at) = list.next_occurrence(&at(2024, 6, 3, 4, 0)).unwrap();
        assert_eq!(at(2024, 6, 4, 3, 0), run_at);
    }

    #[test]
    fn test_the_day_mask_moves_the_event_to_the_next_matching_weekday() {
        let mut list = EventList::default();
        list.push(BoardEvent {
            days: IcbDoW::from(String::from("NNNNNNY")),
            ..daily("03:00:00")
        });
        // Monday -> the next Saturday.
        let (_, run_at) = list.next_occurrence(&at(2024, 6, 3, 4, 0)).unwrap();
        assert_eq!(at(2024, 6, 8, 3, 0), run_at);
    }

    #[test]
    fn test_a_disabled_event_never_fires() {
        let mut list = EventList::default();
        list.push(BoardEvent {
            enabled: false,
            ..daily("03:00:00")
        });
        assert!(list.next_occurrence(&at(2024, 6, 3, 1, 0)).is_none());
    }

    #[test]
    fn test_the_earliest_of_several_events_wins() {
        let mut list = EventList::default();
        list.push(daily("23:00:00"));
        list.push(daily("04:00:00"));
        let (event, run_at) = list.next_occurrence(&at(2024, 6, 3, 1, 0)).unwrap();
        assert_eq!(at(2024, 6, 3, 4, 0), run_at);
        assert_eq!(IcbTime::parse("04:00:00"), event.time);
    }

    #[test]
    fn test_the_board_is_suspended_once_the_suspend_period_has_begun() {
        let mut list = EventList::default();
        list.push(daily("03:00:00"));
        let options = EventOptions {
            enabled: true,
            suspend_minutes: 10,
            ..EventOptions::default()
        };
        let window = next_window(&options, &list, &at(2024, 6, 3, 2, 45)).unwrap();
        assert_eq!(at(2024, 6, 3, 2, 50), window.suspend_at);
        assert!(!window.is_suspended(&at(2024, 6, 3, 2, 45)));
        assert_eq!(5, window.minutes_until_suspend(&at(2024, 6, 3, 2, 45)));
        assert!(window.is_suspended(&at(2024, 6, 3, 2, 55)));
    }

    #[test]
    fn test_uploads_stop_earlier_than_the_board_when_asked_for() {
        let mut list = EventList::default();
        list.push(daily("03:00:00"));
        let options = EventOptions {
            enabled: true,
            suspend_minutes: 10,
            disallow_uploads: true,
            minutes_uploads_disallowed: 30,
            ..EventOptions::default()
        };
        let window = next_window(&options, &list, &at(2024, 6, 3, 1, 0)).unwrap();
        assert!(!window.uploads_blocked(&at(2024, 6, 3, 2, 25)));
        assert!(window.uploads_blocked(&at(2024, 6, 3, 2, 35)));
    }

    #[test]
    fn test_switching_events_off_hides_the_window() {
        let mut list = EventList::default();
        list.push(daily("03:00:00"));
        assert!(next_window(&EventOptions::default(), &list, &at(2024, 6, 3, 1, 0)).is_none());
    }

    #[test]
    fn only_fixed_events_restrict_sessions_and_uploads() {
        let options = EventOptions {
            enabled: true,
            disallow_uploads: true,
            minutes_uploads_disallowed: 30,
            suspend_minutes: 10,
            ..Default::default()
        };
        for mode in [EventMode::Slide, EventMode::Idle] {
            let window = event_window(&options, BoardEvent { mode, ..daily("03:00:00") }, at(2024, 6, 3, 3, 0));
            for now in [at(2024, 6, 3, 2, 55), at(2024, 6, 3, 4, 0)] {
                assert!(!window.is_suspended(&now));
                assert!(!window.uploads_blocked(&now));
                assert_eq!(window.minutes_until_suspend(&now), i64::from(i32::MAX));
            }
        }
    }
}
