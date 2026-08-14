//! Bulk maintenance of the user base: selecting records by criteria and the
//! operations a sysop runs over that selection.
//!
//! The logic lives here rather than in a tool so the system manager, the sysop
//! command and any batch run all decide the same way who is affected.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};

use crate::Res;

use super::user_base::{ConferenceFlags, User, UserBase};
use super::write_atomic;

/// Which of the two security fields a criterion or an operation looks at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SecurityField {
    #[default]
    Normal,
    Expired,
}

/// Picks the users an operation runs over.
///
/// The security range narrows the field, the conditions below it widen it: a
/// user has to be inside the range and match at least one condition. With no
/// condition set every user in the range is selected, which is what the bulk
/// edits want; packing sets conditions and lets the keep rules veto them.
#[derive(Clone, Debug)]
pub struct UserSelection {
    pub min_security: u8,
    pub max_security: u8,
    /// Compare the range against the expired level instead of the current one.
    pub security_field: SecurityField,

    pub last_on_before: Option<DateTime<Utc>>,
    pub inactive_days: Option<u32>,
    pub never_logged_on: bool,
    pub delete_flagged: bool,
    pub disabled: bool,
    /// Level zero, which is what locking a caller out leaves behind.
    pub locked_out: bool,
    pub expired_before: Option<DateTime<Utc>>,

    /// Users at or above this level survive every condition above.
    pub keep_security_at_least: Option<u8>,
    /// Keep users locked out by hand, that is level 0 without the delete flag.
    pub keep_locked_out: bool,
    /// The first record is the sysop and is never touched unless this is off.
    pub protect_first_record: bool,
    /// Names that must survive, used for the callers online during a pack.
    pub protected_names: Vec<String>,
}

impl Default for UserSelection {
    fn default() -> Self {
        Self {
            min_security: 0,
            max_security: u8::MAX,
            security_field: SecurityField::Normal,
            last_on_before: None,
            inactive_days: None,
            never_logged_on: false,
            delete_flagged: false,
            disabled: false,
            locked_out: false,
            expired_before: None,
            keep_security_at_least: None,
            keep_locked_out: false,
            protect_first_record: true,
            protected_names: Vec::new(),
        }
    }
}

impl UserSelection {
    pub fn with_security_range(mut self, min: u8, max: u8) -> Self {
        self.min_security = min;
        self.max_security = max;
        self
    }

    fn has_conditions(&self) -> bool {
        self.last_on_before.is_some()
            || self.inactive_days.is_some()
            || self.never_logged_on
            || self.delete_flagged
            || self.disabled
            || self.locked_out
            || self.expired_before.is_some()
    }

    fn is_protected(&self, index: usize, user: &User) -> bool {
        if self.protect_first_record && index == 0 {
            return true;
        }
        if let Some(keep) = self.keep_security_at_least
            && user.security_level >= keep
        {
            return true;
        }
        if self.keep_locked_out && user.security_level == 0 && !user.flags.delete_flag {
            return true;
        }
        self.protected_names.iter().any(|name| user.is_valid_loginname(name))
    }

    /// True when `user` at position `index` is picked, `now` being the reference
    /// point for the day based conditions.
    pub fn matches(&self, index: usize, user: &User, now: DateTime<Utc>) -> bool {
        if self.is_protected(index, user) {
            return false;
        }

        let level = match self.security_field {
            SecurityField::Normal => user.security_level,
            SecurityField::Expired => user.exp_security_level,
        };
        if level < self.min_security || level > self.max_security {
            return false;
        }

        if !self.has_conditions() {
            return true;
        }

        if self.delete_flagged && user.flags.delete_flag {
            return true;
        }
        if self.disabled && user.flags.disabled_flag {
            return true;
        }
        if self.locked_out && user.security_level == 0 {
            return true;
        }
        if self.never_logged_on && user.stats.num_times_on == 0 {
            return true;
        }
        if let Some(before) = self.last_on_before
            && user.stats.last_on < before
        {
            return true;
        }
        if let Some(days) = self.inactive_days
            && user.stats.last_on < now - Duration::days(days as i64)
        {
            return true;
        }
        if let Some(before) = self.expired_before {
            // A user without an expiration date has no subscription to expire.
            if user.expiration_date != DateTime::<Utc>::default() && user.expiration_date < before {
                return true;
            }
        }
        false
    }

    pub fn select(&self, base: &UserBase, now: DateTime<Utc>) -> Vec<usize> {
        base.iter()
            .enumerate()
            .filter(|(index, user)| self.matches(*index, user, now))
            .map(|(index, _)| index)
            .collect()
    }
}

/// What an operation did, for the confirmation screen and the log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaintenanceReport {
    pub matched: usize,
    pub changed: usize,
    /// Names of the users that were changed or removed.
    pub names: Vec<String>,
}

impl MaintenanceReport {
    fn record(&mut self, user: &User) {
        self.changed += 1;
        self.names.push(user.get_name().clone());
    }
}

/// How to move an expiration date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpirationChange {
    SetDate(DateTime<Utc>),
    AddDays(i64),
}

/// Removes the selected users. Their conference flags and read pointers live in
/// the record itself and go with it.
pub fn pack(base: &mut UserBase, selection: &UserSelection, now: DateTime<Utc>) -> MaintenanceReport {
    let doomed = selection.select(base, now);
    let mut report = MaintenanceReport {
        matched: doomed.len(),
        ..Default::default()
    };
    for index in &doomed {
        report.record(&base[*index]);
    }

    let mut index = 0;
    base.retain(|_| {
        let keep = !doomed.contains(&index);
        index += 1;
        keep
    });
    report
}

pub fn adjust_security(base: &mut UserBase, selection: &UserSelection, new_level: u8, target: SecurityField, now: DateTime<Utc>) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let field = match target {
            SecurityField::Normal => &mut user.security_level,
            SecurityField::Expired => &mut user.exp_security_level,
        };
        if *field == new_level {
            return false;
        }
        *field = new_level;
        true
    })
}

/// Makes the expired level the current one, for subscriptions that will not be renewed.
pub fn copy_expired_security(base: &mut UserBase, selection: &UserSelection, now: DateTime<Utc>) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        if user.security_level == user.exp_security_level {
            return false;
        }
        user.security_level = user.exp_security_level;
        true
    })
}

pub fn adjust_expiration(base: &mut UserBase, selection: &UserSelection, change: ExpirationChange, now: DateTime<Utc>) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let new_date = match change {
            ExpirationChange::SetDate(date) => date,
            ExpirationChange::AddDays(days) => {
                // Nothing to extend for a user who never had a subscription.
                if user.expiration_date == DateTime::<Utc>::default() {
                    return false;
                }
                user.expiration_date + Duration::days(days)
            }
        };
        if user.expiration_date == new_date {
            return false;
        }
        user.expiration_date = new_date;
        true
    })
}

/// Sets `flags` for every conference in `conferences`.
pub fn conference_register(
    base: &mut UserBase,
    selection: &UserSelection,
    conferences: &[usize],
    flags: ConferenceFlags,
    reset_lastread: bool,
    now: DateTime<Utc>,
) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let mut changed = false;
        for conf in conferences {
            let current = user.conference_flags.get(conf).copied().unwrap_or(ConferenceFlags::None);
            let updated = current | flags;
            if updated != current {
                user.conference_flags.insert(*conf, updated);
                changed = true;
            }
            if reset_lastread {
                changed |= clear_lastread(user, *conf);
            }
        }
        changed
    })
}

/// Clears `flags` for every conference in `conferences`.
pub fn conference_unregister(
    base: &mut UserBase,
    selection: &UserSelection,
    conferences: &[usize],
    flags: ConferenceFlags,
    reset_lastread: bool,
    now: DateTime<Utc>,
) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let mut changed = false;
        for conf in conferences {
            if let Some(current) = user.conference_flags.get(conf).copied() {
                let updated = current & !flags;
                if updated != current {
                    changed = true;
                    if updated == ConferenceFlags::None {
                        user.conference_flags.remove(conf);
                    } else {
                        user.conference_flags.insert(*conf, updated);
                    }
                }
            }
            if reset_lastread {
                changed |= clear_lastread(user, *conf);
            }
        }
        changed
    })
}

/// Carries `flags` from one conference over to another and drops them in the old one.
pub fn conference_move(
    base: &mut UserBase,
    selection: &UserSelection,
    from: usize,
    to: usize,
    flags: ConferenceFlags,
    move_lastread: bool,
    move_last_conference: bool,
    now: DateTime<Utc>,
) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let Some(current) = user.conference_flags.get(&from).copied() else {
            return false;
        };
        let moved = current & flags;
        if moved == ConferenceFlags::None {
            return false;
        }

        let remaining = current & !flags;
        if remaining == ConferenceFlags::None {
            user.conference_flags.remove(&from);
        } else {
            user.conference_flags.insert(from, remaining);
        }
        let target = user.conference_flags.get(&to).copied().unwrap_or(ConferenceFlags::None);
        user.conference_flags.insert(to, target | moved);

        if move_lastread {
            let carried: Vec<_> = user
                .lastread_ptr_flags
                .iter()
                .filter(|((conf, _), _)| *conf == from)
                .map(|((_, area), status)| (*area, *status))
                .collect();
            for (area, status) in carried {
                user.lastread_ptr_flags.remove(&(from, area));
                user.lastread_ptr_flags.insert((to, area), status);
            }
        }
        if move_last_conference && user.last_conference as usize == from {
            user.last_conference = to as u16;
        }
        true
    })
}

/// Rewrites the phone fields as digits in `999 999-9999` shape so they sort.
pub fn standardize_phones(base: &mut UserBase, selection: &UserSelection, now: DateTime<Utc>) -> MaintenanceReport {
    apply(base, selection, now, |user| {
        let bus = format_phone(&user.bus_data_phone);
        let home = format_phone(&user.home_voice_phone);
        if bus == user.bus_data_phone && home == user.home_voice_phone {
            return false;
        }
        user.bus_data_phone = bus;
        user.home_voice_phone = home;
        true
    })
}

/// The orders the user file can be put in. The single fields come first, then
/// the ones that fall back on the name, as the original offered them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Password,
    BusinessPhone,
    HomePhone,
    RegistrationExpiration,
    Comment1,
    Comment2,
    City,

    SecurityThenName,
    TimesOnThenName,
    FilesDownloadedThenName,
    FilesUploadedThenName,
    FileRatioThenName,
    BytesDownloadedThenName,
    BytesUploadedThenName,
    BytesRatioThenName,
}

impl SortKey {
    /// Everything below the single fields sorts on a number and then the name.
    pub fn is_multi_field(&self) -> bool {
        !matches!(
            self,
            SortKey::Name
                | SortKey::Password
                | SortKey::BusinessPhone
                | SortKey::HomePhone
                | SortKey::RegistrationExpiration
                | SortKey::Comment1
                | SortKey::Comment2
                | SortKey::City
        )
    }
}

/// A ratio of zero downloads is worth its uploads, so a leech and a donor do not
/// land next to each other.
fn ratio(uploaded: u64, downloaded: u64) -> f64 {
    uploaded as f64 / downloaded.max(1) as f64
}

fn text_key(user: &User, key: SortKey) -> String {
    match key {
        SortKey::Name => user.get_name().to_lowercase(),
        SortKey::Password => user.password.password.to_string().to_lowercase(),
        SortKey::BusinessPhone => user.bus_data_phone.to_lowercase(),
        SortKey::HomePhone => user.home_voice_phone.to_lowercase(),
        SortKey::RegistrationExpiration => user.expiration_date.to_rfc3339(),
        SortKey::Comment1 => user.user_comment.to_lowercase(),
        SortKey::Comment2 => user.sysop_comment.to_lowercase(),
        SortKey::City => user.city_or_state.to_lowercase(),
        _ => String::new(),
    }
}

fn number_key(user: &User, key: SortKey) -> f64 {
    match key {
        SortKey::SecurityThenName => user.security_level as f64,
        SortKey::TimesOnThenName => user.stats.num_times_on as f64,
        SortKey::FilesDownloadedThenName => user.stats.num_downloads as f64,
        SortKey::FilesUploadedThenName => user.stats.num_uploads as f64,
        SortKey::FileRatioThenName => ratio(user.stats.num_uploads, user.stats.num_downloads),
        SortKey::BytesDownloadedThenName => user.stats.total_dnld_bytes as f64,
        SortKey::BytesUploadedThenName => user.stats.total_upld_bytes as f64,
        SortKey::BytesRatioThenName => ratio(user.stats.total_upld_bytes, user.stats.total_dnld_bytes),
        _ => 0.0,
    }
}

/// Puts the records in order. The first record stays where it is - it is the
/// sysop, and the board and its drop files count on finding it there.
pub fn sort(base: &mut UserBase, key: SortKey, reverse: bool) -> MaintenanceReport {
    let keep_first = !base.is_empty();
    let before: Vec<String> = base.iter().map(|user| user.get_name().clone()).collect();

    let start = usize::from(keep_first);
    let mut rest: Vec<User> = base.drain(start..).collect();
    rest.sort_by(|a, b| {
        let ordering = if key.is_multi_field() {
            number_key(a, key)
                .partial_cmp(&number_key(b, key))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.get_name().to_lowercase().cmp(&b.get_name().to_lowercase()))
        } else {
            text_key(a, key).cmp(&text_key(b, key))
        };
        if reverse { ordering.reverse() } else { ordering }
    });
    base.extend(rest);

    let moved: Vec<String> = base
        .iter()
        .enumerate()
        .filter(|(index, user)| before.get(*index) != Some(user.get_name()))
        .map(|(_, user)| user.get_name().clone())
        .collect();

    MaintenanceReport {
        matched: base.len(),
        changed: moved.len(),
        names: moved,
    }
}

fn apply<F>(base: &mut UserBase, selection: &UserSelection, now: DateTime<Utc>, mut op: F) -> MaintenanceReport
where
    F: FnMut(&mut User) -> bool,
{
    let selected = selection.select(base, now);
    let mut report = MaintenanceReport {
        matched: selected.len(),
        ..Default::default()
    };
    for index in selected {
        if op(&mut base[index]) {
            report.record(&base[index]);
        }
    }
    report
}

fn clear_lastread(user: &mut User, conference: usize) -> bool {
    let areas: Vec<usize> = user
        .lastread_ptr_flags
        .keys()
        .filter(|(conf, _)| *conf == conference)
        .map(|(_, area)| *area)
        .collect();
    let mut changed = false;
    for area in areas {
        changed |= user.lastread_ptr_flags.remove(&(conference, area)).is_some();
    }
    changed
}

fn format_phone(phone: &str) -> String {
    let digits: Vec<char> = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return String::new();
    }

    // Fill the mask from the right so a short number keeps its last digits.
    let mask = ['9', '9', '9', ' ', '9', '9', '9', '-', '9', '9', '9', '9'];
    let mut out = vec![' '; mask.len()];
    let mut digit = digits.len();
    for pos in (0..mask.len()).rev() {
        if digit == 0 {
            break;
        }
        if mask[pos] == '9' {
            digit -= 1;
            out[pos] = digits[digit];
        } else {
            out[pos] = mask[pos];
        }
    }
    out.iter().collect::<String>().trim_start().to_string()
}

/// A user record and where it sits, for list views that need to search or sort
/// without disturbing the order on disk.
pub fn find(base: &UserBase, needle: &str) -> Vec<usize> {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    base.iter()
        .enumerate()
        .filter(|(_, user)| user.get_name().to_lowercase().contains(&needle) || user.alias.to_lowercase().contains(&needle))
        .map(|(index, _)| index)
        .collect()
}

/// Where the copy of the user file taken before a destructive run is kept.
pub fn backup_path(users_file: &Path) -> PathBuf {
    let mut name = users_file.file_name().unwrap_or_default().to_string_lossy().to_string();
    name.push_str(".bak");
    users_file.with_file_name(name)
}

/// Copies the user file aside, replacing an older copy. One level, like the original.
pub fn create_backup(users_file: &Path) -> Res<PathBuf> {
    let target = backup_path(users_file);
    let contents = std::fs::read(users_file)?;
    write_atomic(&target, &contents)?;
    Ok(target)
}

pub fn has_backup(users_file: &Path) -> bool {
    backup_path(users_file).is_file()
}

/// Reads the copy back over the user file and returns the restored base.
pub fn restore_backup(users_file: &Path) -> Res<()> {
    let source = backup_path(users_file);
    let contents = std::fs::read(&source)?;
    write_atomic(users_file, &contents)?;
    Ok(())
}

/// When the backup was taken, for the screen that offers the undo.
pub fn backup_time(users_file: &Path) -> Option<DateTime<Utc>> {
    let meta = std::fs::metadata(backup_path(users_file)).ok()?;
    meta.modified().ok().map(DateTime::<Utc>::from)
}

/// Counts how many users hold each conference flag, for the conference screens.
pub fn conference_usage(base: &UserBase) -> HashMap<usize, usize> {
    let mut counts = HashMap::new();
    for user in base.iter() {
        for (conf, flags) in &user.conference_flags {
            if flags.contains(ConferenceFlags::Registered) {
                *counts.entry(*conf).or_insert(0) += 1;
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::IcyBoardSerializer;
    use crate::icy_board::user_base::LastReadStatus;

    fn user(name: &str, security: u8) -> User {
        User {
            name: name.to_string(),
            security_level: security,
            ..Default::default()
        }
    }

    fn base(users: Vec<User>) -> UserBase {
        let mut base = UserBase::default();
        for u in users {
            base.new_user(u);
        }
        base
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn days_ago(days: i64) -> DateTime<Utc> {
        now() - Duration::days(days)
    }

    fn names(base: &UserBase) -> Vec<String> {
        base.iter().map(|u| u.get_name().clone()).collect()
    }

    #[test]
    fn empty_selection_takes_everyone_but_the_sysop() {
        let base = base(vec![user("Sysop", 110), user("Alice", 20), user("Bob", 30)]);
        assert_eq!(vec![1, 2], UserSelection::default().select(&base, now()));
    }

    #[test]
    fn security_range_narrows_the_field() {
        let base = base(vec![user("Sysop", 110), user("Alice", 20), user("Bob", 30), user("Carol", 40)]);
        let selection = UserSelection::default().with_security_range(30, 40);
        assert_eq!(vec![2, 3], selection.select(&base, now()));
    }

    #[test]
    fn expired_level_can_drive_the_range() {
        let mut bob = user("Bob", 30);
        bob.exp_security_level = 10;
        let base = base(vec![user("Sysop", 110), bob]);
        let selection = UserSelection {
            security_field: SecurityField::Expired,
            ..UserSelection::default()
        }
        .with_security_range(0, 15);
        assert_eq!(vec![1], selection.select(&base, now()));
    }

    #[test]
    fn conditions_are_alternatives() {
        let mut deleted = user("Deleted", 20);
        deleted.flags.delete_flag = true;
        let mut idle = user("Idle", 20);
        idle.stats.last_on = days_ago(400);
        idle.stats.num_times_on = 5;
        let mut active = user("Active", 20);
        active.stats.last_on = days_ago(2);
        active.stats.num_times_on = 5;

        let base = base(vec![user("Sysop", 110), deleted, idle, active]);
        let selection = UserSelection {
            delete_flagged: true,
            inactive_days: Some(180),
            ..UserSelection::default()
        };
        assert_eq!(vec![1, 2], selection.select(&base, now()));
    }

    #[test]
    fn never_logged_on_ignores_users_with_calls() {
        let mut fresh = user("Fresh", 10);
        fresh.stats.num_times_on = 0;
        let mut seasoned = user("Seasoned", 10);
        seasoned.stats.num_times_on = 3;
        let base = base(vec![user("Sysop", 110), fresh, seasoned]);
        let selection = UserSelection {
            never_logged_on: true,
            ..UserSelection::default()
        };
        assert_eq!(vec![1], selection.select(&base, now()));
    }

    #[test]
    fn locked_out_users_go_unless_the_keep_rule_says_otherwise() {
        let base = base(vec![user("Sysop", 110), user("LockedOut", 0), user("Normal", 20)]);

        let selection = UserSelection {
            locked_out: true,
            ..UserSelection::default()
        };
        assert_eq!(vec![1], selection.select(&base, now()));

        let selection = UserSelection {
            keep_locked_out: true,
            ..selection
        };
        assert!(selection.select(&base, now()).is_empty());
    }

    #[test]
    fn subscription_condition_skips_users_without_a_date() {
        let mut expired = user("Expired", 20);
        expired.expiration_date = days_ago(30);
        let base = base(vec![user("Sysop", 110), expired, user("NoSubscription", 20)]);
        let selection = UserSelection {
            expired_before: Some(now()),
            ..UserSelection::default()
        };
        assert_eq!(vec![1], selection.select(&base, now()));
    }

    #[test]
    fn keep_rules_beat_the_conditions() {
        let mut deleted_vip = user("Vip", 90);
        deleted_vip.flags.delete_flag = true;
        let mut locked_out = user("LockedOut", 0);
        let mut deleted = user("Gone", 20);
        deleted.flags.delete_flag = true;
        locked_out.stats.last_on = days_ago(900);

        let base = base(vec![user("Sysop", 110), deleted_vip, locked_out, deleted]);
        let selection = UserSelection {
            delete_flagged: true,
            inactive_days: Some(180),
            keep_security_at_least: Some(80),
            keep_locked_out: true,
            ..UserSelection::default()
        };
        assert_eq!(vec![3], selection.select(&base, now()));
    }

    #[test]
    fn protected_names_survive_a_pack() {
        let mut idle = user("Idle Caller", 20);
        idle.stats.last_on = days_ago(900);
        let mut online = user("Online Caller", 20);
        online.stats.last_on = days_ago(900);

        let mut base = base(vec![user("Sysop", 110), idle, online]);
        let selection = UserSelection {
            inactive_days: Some(180),
            protected_names: vec!["Online Caller".to_string()],
            ..UserSelection::default()
        };
        let report = pack(&mut base, &selection, now());
        assert_eq!(1, report.changed);
        assert_eq!(vec!["Idle Caller".to_string()], report.names);
        assert_eq!(vec!["Sysop".to_string(), "Online Caller".to_string()], names(&base));
    }

    #[test]
    fn pack_removes_every_selected_record() {
        let mut base = base(vec![user("Sysop", 110), user("A", 10), user("B", 10), user("C", 10)]);
        base[2].flags.delete_flag = true;
        let selection = UserSelection {
            delete_flagged: true,
            ..UserSelection::default()
        };
        let report = pack(&mut base, &selection, now());
        assert_eq!(1, report.changed);
        assert_eq!(vec!["Sysop".to_string(), "A".to_string(), "C".to_string()], names(&base));
    }

    #[test]
    fn adjust_security_reports_only_real_changes() {
        let mut base = base(vec![user("Sysop", 110), user("A", 20), user("B", 30)]);
        let selection = UserSelection::default().with_security_range(20, 30);
        let report = adjust_security(&mut base, &selection, 30, SecurityField::Normal, now());
        assert_eq!(2, report.matched);
        assert_eq!(1, report.changed);
        assert_eq!(30, base[1].security_level);
    }

    #[test]
    fn copy_expired_security_overwrites_the_current_level() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60)]);
        base[1].exp_security_level = 10;
        copy_expired_security(&mut base, &UserSelection::default(), now());
        assert_eq!(10, base[1].security_level);
    }

    #[test]
    fn expiration_can_be_set_and_extended() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60), user("B", 60)]);
        base[1].expiration_date = days_ago(10);

        let target = now();
        let report = adjust_expiration(&mut base, &UserSelection::default(), ExpirationChange::SetDate(target), now());
        assert_eq!(2, report.changed);
        assert_eq!(target, base[1].expiration_date);

        adjust_expiration(&mut base, &UserSelection::default(), ExpirationChange::AddDays(7), now());
        assert_eq!(target + Duration::days(7), base[1].expiration_date);
    }

    #[test]
    fn adding_days_leaves_users_without_a_subscription_alone() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60)]);
        let report = adjust_expiration(&mut base, &UserSelection::default(), ExpirationChange::AddDays(7), now());
        assert_eq!(0, report.changed);
        assert_eq!(DateTime::<Utc>::default(), base[1].expiration_date);
    }

    #[test]
    fn conference_registration_sets_and_clears_flags() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60)]);
        let selection = UserSelection::default();

        conference_register(&mut base, &selection, &[3, 4], ConferenceFlags::Registered, false, now());
        assert!(base[1].conference_flags[&3].contains(ConferenceFlags::Registered));
        assert!(base[1].conference_flags[&4].contains(ConferenceFlags::Registered));

        conference_unregister(&mut base, &selection, &[3], ConferenceFlags::Registered, false, now());
        assert!(!base[1].conference_flags.contains_key(&3));
        assert!(base[1].conference_flags.contains_key(&4));
    }

    #[test]
    fn unregistering_can_drop_the_read_pointers() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60)]);
        base[1].lastread_ptr_flags.insert((3, 0), LastReadStatus::default());
        let selection = UserSelection::default();

        conference_register(&mut base, &selection, &[3], ConferenceFlags::Registered, false, now());
        conference_unregister(&mut base, &selection, &[3], ConferenceFlags::Registered, true, now());
        assert!(base[1].lastread_ptr_flags.is_empty());
    }

    #[test]
    fn moving_a_conference_carries_flags_and_pointers() {
        let mut base = base(vec![user("Sysop", 110), user("A", 60)]);
        base[1].conference_flags.insert(3, ConferenceFlags::Registered | ConferenceFlags::Selected);
        base[1].lastread_ptr_flags.insert(
            (3, 0),
            LastReadStatus {
                last_read: 42,
                highest_msg_read: 42,
                include_qwk: true,
            },
        );
        base[1].last_conference = 3;

        let report = conference_move(&mut base, &UserSelection::default(), 3, 7, ConferenceFlags::Registered, true, true, now());

        assert_eq!(1, report.changed);
        assert_eq!(ConferenceFlags::Selected, base[1].conference_flags[&3]);
        assert!(base[1].conference_flags[&7].contains(ConferenceFlags::Registered));
        assert_eq!(42, base[1].lastread_ptr_flags[&(7, 0)].last_read);
        assert!(!base[1].lastread_ptr_flags.contains_key(&(3, 0)));
        assert_eq!(7, base[1].last_conference);
    }

    #[test]
    fn sorting_by_name_leaves_the_sysop_in_front() {
        let mut base = base(vec![user("Sysop", 110), user("Zulu", 20), user("alpha", 20), user("Mike", 20)]);
        let report = sort(&mut base, SortKey::Name, false);

        assert_eq!(
            vec!["Sysop".to_string(), "alpha".to_string(), "Mike".to_string(), "Zulu".to_string()],
            names(&base)
        );
        assert_eq!(3, report.changed);
    }

    #[test]
    fn sorting_can_run_the_other_way() {
        let mut base = base(vec![user("Sysop", 110), user("alpha", 20), user("Zulu", 20)]);
        sort(&mut base, SortKey::Name, true);
        assert_eq!(vec!["Sysop".to_string(), "Zulu".to_string(), "alpha".to_string()], names(&base));
    }

    #[test]
    fn a_multi_field_sort_falls_back_on_the_name() {
        let mut base = base(vec![user("Sysop", 110), user("Bravo", 20), user("Alpha", 20), user("Charlie", 10)]);
        sort(&mut base, SortKey::SecurityThenName, false);
        assert_eq!(
            vec!["Sysop".to_string(), "Charlie".to_string(), "Alpha".to_string(), "Bravo".to_string()],
            names(&base)
        );
    }

    #[test]
    fn the_upload_ratio_counts_a_user_without_downloads() {
        let mut base = base(vec![user("Sysop", 110), user("Leech", 20), user("Donor", 20)]);
        base[1].stats.num_uploads = 0;
        base[1].stats.num_downloads = 40;
        base[2].stats.num_uploads = 30;
        base[2].stats.num_downloads = 0;

        sort(&mut base, SortKey::FileRatioThenName, false);
        assert_eq!(vec!["Sysop".to_string(), "Leech".to_string(), "Donor".to_string()], names(&base));
    }

    #[test]
    fn phone_numbers_get_a_common_shape() {
        assert_eq!("801 555-1234", format_phone("(801) 555-1234"));
        assert_eq!("801 555-1234", format_phone("8015551234"));
        assert_eq!("32-3835", format_phone("323-835"));
        assert_eq!("", format_phone("unlisted"));
    }

    #[test]
    fn find_matches_name_and_alias() {
        let mut base = base(vec![user("Sysop", 110), user("Alice Smith", 20)]);
        base[1].alias = "Nightowl".to_string();
        assert_eq!(vec![1], find(&base, "smith"));
        assert_eq!(vec![1], find(&base, "NIGHT"));
        assert!(find(&base, "").is_empty());
    }

    #[test]
    fn backup_round_trips_the_user_file() {
        let dir = tempfile::tempdir().unwrap();
        let users_file = dir.path().join("users.toml");

        let original = base(vec![user("Sysop", 110), user("A", 20)]);
        original.save(&users_file).unwrap();
        let before = std::fs::read(&users_file).unwrap();

        create_backup(&users_file).unwrap();
        assert!(has_backup(&users_file));

        let mut packed = original.clone();
        pack(
            &mut packed,
            &UserSelection {
                delete_flagged: false,
                ..UserSelection::default()
            },
            now(),
        );
        packed.save(&users_file).unwrap();
        assert_ne!(before, std::fs::read(&users_file).unwrap());

        restore_backup(&users_file).unwrap();
        assert_eq!(before, std::fs::read(&users_file).unwrap());
    }
}
