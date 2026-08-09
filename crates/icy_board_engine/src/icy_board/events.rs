use chrono::{DateTime, Datelike, Duration, Local, TimeZone};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardEvent {
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
}

fn enabled_by_default() -> bool {
    true
}

impl Default for BoardEvent {
    fn default() -> Self {
        Self {
            description: String::new(),
            enabled: true,
            time: IcbTime::default(),
            days: IcbDoW::all(),
            mode: EventMode::default(),
            command: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EventList {
    #[serde(rename = "event", default)]
    events: Vec<BoardEvent>,
}

impl IcyBoardSerializer for EventList {
    const FILE_TYPE: &'static str = "events";
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
    /// The event that fires next, and when. Ties are broken by the order in the file.
    pub fn next_occurrence(&self, now: &DateTime<Local>) -> Option<(&BoardEvent, DateTime<Local>)> {
        let mut best: Option<(&BoardEvent, DateTime<Local>)> = None;
        for event in &self.events {
            if !event.enabled || event.days.is_empty() {
                continue;
            }
            for offset in 0..8 {
                let date = now.date_naive() + Duration::days(offset);
                if !event.days.contains(date.weekday()) {
                    continue;
                }
                let Some(naive) = date.and_hms_opt(event.time.get_hour() as u32, event.time.get_minute() as u32, event.time.get_second() as u32) else {
                    continue;
                };
                // A clock change may swallow the wall time entirely - then the event is skipped.
                let Some(at) = now.timezone().from_local_datetime(&naive).earliest() else {
                    continue;
                };
                if at <= *now {
                    continue;
                }
                if best.as_ref().is_none_or(|(_, best_at)| at < *best_at) {
                    best = Some((event, at));
                }
                break;
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
        *now >= self.suspend_at
    }

    pub fn uploads_blocked(&self, now: &DateTime<Local>) -> bool {
        self.uploads_stop_at.is_some_and(|stop| *now >= stop)
    }

    /// Minutes a session started now may still last.
    pub fn minutes_until_suspend(&self, now: &DateTime<Local>) -> i64 {
        (self.suspend_at - *now).num_minutes().max(0)
    }
}

/// `None` when events are switched off or nothing is scheduled.
pub fn next_window(options: &EventOptions, events: &EventList, now: &DateTime<Local>) -> Option<EventWindow> {
    if !options.enabled {
        return None;
    }
    let (event, run_at) = events.next_occurrence(now)?;
    let uploads_stop_at = if options.disallow_uploads {
        Some(run_at - Duration::minutes(options.minutes_uploads_disallowed as i64))
    } else {
        None
    };
    Some(EventWindow {
        event: event.clone(),
        run_at,
        suspend_at: run_at - Duration::minutes(options.suspend_minutes as i64),
        uploads_stop_at,
    })
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
}
