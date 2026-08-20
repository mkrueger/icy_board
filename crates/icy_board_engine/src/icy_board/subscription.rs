use chrono::{DateTime, NaiveDate, Utc};

use super::user_base::ConferenceFlags;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionStatus {
    Disabled,
    NeverExpires,
    Current { days_left: i64 },
    Warning { days_left: i64 },
    Expired { days_overdue: i64 },
}

impl SubscriptionStatus {
    pub fn is_expired(self) -> bool {
        matches!(self, Self::Expired { .. })
    }
}

/// `PCBoard` compares calendar dates: the expiration date itself is still valid.
pub fn status(enabled: bool, expiration: DateTime<Utc>, warning_days: u32, today: NaiveDate) -> SubscriptionStatus {
    if !enabled {
        return SubscriptionStatus::Disabled;
    }
    if expiration == DateTime::<Utc>::default() {
        return SubscriptionStatus::NeverExpires;
    }

    let days_left = (expiration.date_naive() - today).num_days();
    if days_left < 0 {
        SubscriptionStatus::Expired { days_overdue: -days_left }
    } else if days_left < warning_days as i64 {
        SubscriptionStatus::Warning { days_left }
    } else {
        SubscriptionStatus::Current { days_left }
    }
}

pub fn days_until_expiration(enabled: bool, expiration: DateTime<Utc>, today: NaiveDate) -> Option<i64> {
    match status(enabled, expiration, 0, today) {
        SubscriptionStatus::Disabled | SubscriptionStatus::NeverExpires => None,
        SubscriptionStatus::Current { days_left } | SubscriptionStatus::Warning { days_left } => Some(days_left),
        SubscriptionStatus::Expired { days_overdue } => Some(-days_overdue),
    }
}

pub fn new_user_expiration(enabled: bool, subscription_length: u32, login_date: DateTime<Utc>) -> DateTime<Utc> {
    if enabled && subscription_length > 0 {
        login_date + chrono::Duration::days(subscription_length as i64)
    } else {
        DateTime::<Utc>::default()
    }
}

pub fn conference_access(expired: bool, flags: ConferenceFlags) -> bool {
    if expired {
        flags.contains(ConferenceFlags::Registered | ConferenceFlags::Expired)
    } else {
        flags.contains(ConferenceFlags::Registered)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn at(days: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 17, 12, 0, 0).unwrap() + Duration::days(days)
    }

    fn today() -> NaiveDate {
        at(0).date_naive()
    }

    #[test]
    fn disabled_and_missing_subscriptions_never_expire() {
        assert_eq!(status(false, at(-1), 30, today()), SubscriptionStatus::Disabled);
        assert_eq!(status(true, DateTime::<Utc>::default(), 30, today()), SubscriptionStatus::NeverExpires);
    }

    #[test]
    fn the_expiration_date_itself_is_still_current() {
        assert_eq!(status(true, at(0), 0, today()), SubscriptionStatus::Current { days_left: 0 });
    }

    #[test]
    fn the_warning_window_starts_strictly_inside_the_configured_days() {
        assert_eq!(status(true, at(30), 30, today()), SubscriptionStatus::Current { days_left: 30 });
        assert_eq!(status(true, at(29), 30, today()), SubscriptionStatus::Warning { days_left: 29 });
    }

    #[test]
    fn a_past_date_is_expired() {
        assert_eq!(status(true, at(-3), 30, today()), SubscriptionStatus::Expired { days_overdue: 3 });
    }

    #[test]
    fn expiration_days_are_signed() {
        assert_eq!(days_until_expiration(true, at(12), today()), Some(12));
        assert_eq!(days_until_expiration(true, at(-2), today()), Some(-2));
        assert_eq!(days_until_expiration(true, DateTime::<Utc>::default(), today()), None);
    }

    #[test]
    fn new_users_get_the_configured_subscription_length() {
        assert_eq!(new_user_expiration(true, 365, at(0)), at(365));
        assert_eq!(new_user_expiration(true, 0, at(0)), DateTime::<Utc>::default());
        assert_eq!(new_user_expiration(false, 365, at(0)), DateTime::<Utc>::default());
    }

    #[test]
    fn conference_access_switches_to_the_expired_registration() {
        assert!(conference_access(false, ConferenceFlags::Registered));
        assert!(!conference_access(true, ConferenceFlags::Registered));
        assert!(conference_access(true, ConferenceFlags::Registered | ConferenceFlags::Expired));
        assert!(!conference_access(true, ConferenceFlags::Expired));
    }
}
