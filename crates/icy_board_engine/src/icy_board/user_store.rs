//! Typed, optimistic user updates under the board's existing exclusive lock.

use std::collections::HashMap;

use chrono::NaiveDate;
use thiserror::Error;

use crate::Res;

use super::{
    IcyBoard, IcyBoardSerializer, password_recovery,
    user_base::{ConferenceFlags, LastReadStatus, User, UserBase, UserStats},
    user_inf::AccountUserInf,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserUpdateMode {
    Edit,
    /// Edited daily counters belong to `day`; callers reset them at rollover.
    Session {
        day: NaiveDate,
    },
    /// On session close, retain live conflicting fields without losing activity deltas.
    FinalSession {
        day: NaiveDate,
    },
}

impl UserUpdateMode {
    fn resolve<T: Clone>(self, result: Res<T>, latest: &T) -> Res<T> {
        match result {
            Err(error)
                if matches!(self, Self::FinalSession { .. }) && matches!(error.downcast_ref::<UserUpdateError>(), Some(UserUpdateError::Conflict { .. })) =>
            {
                log::warn!("Keeping stored field at session close: {error}");
                Ok(latest.clone())
            }
            result => result,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserUpdateError {
    #[error("Concurrent user update conflicts with field {field}")]
    Conflict { field: String },
    #[error("User update identity no longer exists")]
    MissingIdentity,
    #[error("User update identity is ambiguous")]
    AmbiguousIdentity,
    #[error("User update identity has been replaced")]
    IdentityChanged,
}

fn conflict(field: &str) -> Box<dyn std::error::Error + Send + Sync> {
    UserUpdateError::Conflict { field: field.into() }.into()
}

fn three_way<T: PartialEq + Clone>(baseline: &T, edited: &T, latest: &T, field: &str) -> Res<T> {
    if edited == baseline {
        Ok(latest.clone())
    } else if latest == baseline || latest == edited {
        Ok(edited.clone())
    } else {
        Err(conflict(field))
    }
}

// Only instantiated with non-secret fields; passwords require record fingerprints.
macro_rules! fields {
    ($merged:ident, $baseline:ident, $edited:ident, $latest:ident, $prefix:literal; $($field:ident),+ $(,)?) => {
        fields!($merged, $baseline, $edited, $latest, UserUpdateMode::Edit, $prefix; $($field),+);
    };
    ($merged:ident, $baseline:ident, $edited:ident, $latest:ident, $mode:expr, $prefix:literal; $($field:ident),+ $(,)?) => {
        $($merged.$field = $mode.resolve(three_way(&$baseline.$field, &$edited.$field, &$latest.$field, concat!($prefix, stringify!($field))), &$latest.$field)?;)+
    };
}

/// Merge one snapshot, without mutating any input. Security remains a conservative
/// credential group; recovery state and revisions are always taken from `latest`.
pub fn merge_user(baseline: &User, edited: &User, latest: &User, mode: UserUpdateMode) -> Res<User> {
    let mut merged = latest.clone();
    fields!(merged, baseline, edited, latest, mode, "";
        path, verify_answer, city_or_state, city, state, street1, street2, zip, country,
        gender, web, contacts, date_format, language, bus_data_phone, home_voice_phone,
        birth_date, user_comment, sysop_comment, custom_comment1, custom_comment2,
        custom_comment3, custom_comment4, custom_comment5, expiration_date,
        protocol, page_len, last_conference, elapsed_time_on, qwk_config, bank,
        chat_status, tpa_records,
    );
    {
        let baseline = &baseline.flags;
        let edited = &edited.flags;
        let latest = &latest.flags;
        let mut flags = latest.clone();
        fields!(flags, baseline, edited, latest, mode, "flags.";
            expert_mode, is_dirty, msg_clear, has_mail, fse_mode, scroll_msg_body,
            use_short_filedescr, long_msg_header, wide_editor, use_graphics, use_alias,
        );
        merged.flags = flags;
    }

    // These fields, including the two administrative flags, belong to the security group.
    merged.name.clone_from(&edited.name);
    merged.alias.clone_from(&edited.alias);
    merged.email.clone_from(&edited.email);
    merged.password = edited.password.clone();
    merged.security_level = edited.security_level;
    merged.exp_security_level = edited.exp_security_level;
    merged.flags.delete_flag = edited.flags.delete_flag;
    merged.flags.disabled_flag = edited.flags.disabled_flag;
    if password_recovery::merge_security(edited, baseline, latest, &mut merged).is_err() {
        mode.resolve(Err(conflict("credentials")), &())?;
        password_recovery::merge_security(baseline, baseline, latest, &mut merged)?;
    }

    merged.date_last_dir_read = advance(
        baseline.date_last_dir_read,
        edited.date_last_dir_read,
        latest.date_last_dir_read,
        mode,
        "date_last_dir_read",
    )?;
    merged.stats = merge_stats(&baseline.stats, &edited.stats, &latest.stats, mode)?;
    merged.conference_flags = merge_conferences(&baseline.conference_flags, &edited.conference_flags, &latest.conference_flags, mode)?;
    merged.lastread_ptr_flags = merge_lastreads(&baseline.lastread_ptr_flags, &edited.lastread_ptr_flags, &latest.lastread_ptr_flags, mode)?;
    merged.account = match mode {
        UserUpdateMode::Edit => edit_account(baseline.account.as_ref(), edited.account.as_ref(), latest.account.as_ref())?,
        UserUpdateMode::Session { .. } | UserUpdateMode::FinalSession { .. } => {
            merge_account(edited.account.as_ref(), latest.account.as_ref(), baseline.account.as_ref())?
        }
    };
    Ok(merged)
}

impl IcyBoard {
    /// Resolve the exact original primary name, not the alias/login-name lookup.
    pub fn update_user(&mut self, baseline: &User, edited: &User, mode: UserUpdateMode) -> Res<User> {
        let concurrent_tracking = self.config.accounting.concurrent_tracking;
        let index = self.edit_users(|users| {
            let mut matches = users.iter().enumerate().filter(|(_, user)| user.name == baseline.name);
            let (index, latest) = matches.next().ok_or(UserUpdateError::MissingIdentity)?;
            if matches.next().is_some() {
                return Err(UserUpdateError::AmbiguousIdentity.into());
            }
            if latest.stats.first_date_on != baseline.stats.first_date_on {
                return Err(UserUpdateError::IdentityChanged.into());
            }
            let merged = merge_user(baseline, edited, latest, mode)?;
            if let Some(account) = &merged.account {
                finite(account.balance(concurrent_tracking, 0.0))?;
            }
            if merged.name != baseline.name && users.iter().enumerate().any(|(other, user)| other != index && user.name == merged.name) {
                return Err(UserUpdateError::AmbiguousIdentity.into());
            }
            users[index] = merged;
            Ok(index)
        })?;
        // Return the normalized, persisted record so callers can refresh their baseline.
        Ok(self.users[index].clone())
    }

    /// Publish only after the complete staged base has been saved successfully.
    pub fn edit_users<R>(&mut self, edit: impl FnOnce(&mut UserBase) -> Res<R>) -> Res<R> {
        let mut staging = self.users.clone();
        let result = edit(&mut staging)?;
        for user in staging.iter_mut() {
            password_recovery::normalize_security(user);
            if !self.config.password_recovery.enabled
                || user
                    .recovery
                    .as_ref()
                    .is_some_and(|challenge| self.password_recovery_service.is_revoked(challenge))
            {
                user.recovery = None;
            }
        }
        staging.save(&self.resolve_file(&self.config.paths.user_file))?;
        self.users = staging;
        self.user_revision += 1;
        Ok(result)
    }
}

fn advance<T: Ord + Copy>(baseline: T, edited: T, latest: T, mode: UserUpdateMode, field: &str) -> Res<T> {
    if !matches!(mode, UserUpdateMode::Edit) && edited >= baseline && latest >= baseline {
        Ok(edited.max(latest))
    } else {
        mode.resolve(three_way(&baseline, &edited, &latest, field), &latest)
    }
}

fn counter(baseline: u64, edited: u64, latest: u64, mode: UserUpdateMode, field: &str) -> Res<u64> {
    if !matches!(mode, UserUpdateMode::Edit) && edited >= baseline {
        Ok(latest.saturating_add(edited - baseline))
    } else {
        // Explicit reductions are overrides, not negative activity on another node.
        mode.resolve(three_way(&baseline, &edited, &latest, field), &latest)
    }
}

fn merge_stats(baseline: &UserStats, edited: &UserStats, latest: &UserStats, mode: UserUpdateMode) -> Res<UserStats> {
    let mut merged = latest.clone();
    merged.first_date_on = mode.resolve(
        three_way(&baseline.first_date_on, &edited.first_date_on, &latest.first_date_on, "stats.first_date_on"),
        &latest.first_date_on,
    )?;
    merged.last_on = advance(baseline.last_on, edited.last_on, latest.last_on, mode, "stats.last_on")?;
    macro_rules! counters {
        ($($field:ident),+ $(,)?) => {
            $(merged.$field = counter(baseline.$field, edited.$field, latest.$field, mode, concat!("stats.", stringify!($field)))?;)+
        };
    }
    counters!(
        num_times_on,
        messages_read,
        messages_left,
        num_sec_viol,
        num_not_reg,
        num_reach_dnld_lim,
        num_file_not_found,
        num_password_failures,
        num_verify_errors,
        num_sysop_pages,
        num_group_chats,
        num_comments,
        num_uploads,
        num_downloads,
        total_dnld_bytes,
        total_upld_bytes,
        total_doors_executed,
    );
    match mode {
        UserUpdateMode::Edit => {
            fields!(merged, baseline, edited, latest, "stats.";
                today_num_downloads, today_num_uploads, today_dnld_bytes, today_upld_bytes, minutes_today,
            );
        }
        UserUpdateMode::Session { day } | UserUpdateMode::FinalSession { day } => {
            // An old session cannot put yesterday's usage into a newer daily bucket.
            if latest.last_on.date_naive() > day || baseline.last_on.date_naive() > day {
                return Ok(merged);
            }
            let baseline_same_day = baseline.last_on.date_naive() == day;
            let latest_same_day = latest.last_on.date_naive() == day;
            macro_rules! daily {
                ($($field:ident),+ $(,)?) => {
                    $(merged.$field = counter(
                        if baseline_same_day { baseline.$field } else { 0 },
                        edited.$field,
                        if latest_same_day { latest.$field } else { 0 },
                        mode, concat!("stats.", stringify!($field)),
                    )?;)+
                };
            }
            daily!(today_num_downloads, today_num_uploads, today_upld_bytes);
            merged.minutes_today = counter(
                if baseline_same_day { u64::from(baseline.minutes_today) } else { 0 },
                u64::from(edited.minutes_today),
                if latest_same_day { u64::from(latest.minutes_today) } else { 0 },
                mode,
                "stats.minutes_today",
            )?
            .min(u64::from(u16::MAX)) as u16;
            // Upload credit makes this counter signed; use a wider intermediate.
            let base = if baseline_same_day { baseline.today_dnld_bytes } else { 0 };
            let live = if latest_same_day { latest.today_dnld_bytes } else { 0 };
            merged.today_dnld_bytes =
                (i128::from(live) + i128::from(edited.today_dnld_bytes) - i128::from(base)).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
            merged.last_on = merged.last_on.max(day.and_hms_opt(0, 0, 0).expect("midnight").and_utc());
        }
    }
    Ok(merged)
}

fn merge_conferences(
    baseline: &HashMap<usize, ConferenceFlags>,
    edited: &HashMap<usize, ConferenceFlags>,
    latest: &HashMap<usize, ConferenceFlags>,
    mode: UserUpdateMode,
) -> Res<HashMap<usize, ConferenceFlags>> {
    let mut merged = latest.clone();
    let mut keys: Vec<_> = baseline.keys().chain(edited.keys()).copied().collect();
    keys.sort_unstable();
    keys.dedup();
    for key in keys {
        let base = baseline.get(&key).copied();
        let edit = edited.get(&key).copied();
        let live = latest.get(&key).copied();
        let field = "conference_flags";
        let value = match (base, edit, live) {
            (base, Some(edit), Some(live)) => {
                let base = base.unwrap_or(ConferenceFlags::None);
                let changed = base.bits() ^ edit.bits();
                Some(ConferenceFlags::from_bits_truncate((live.bits() & !changed) | (edit.bits() & changed)))
            }
            _ => mode.resolve(three_way(&base, &edit, &live, field), &live)?,
        };
        if let Some(value) = value {
            merged.insert(key, value);
        } else {
            merged.remove(&key);
        }
    }
    Ok(merged)
}

fn merge_lastreads(
    baseline: &HashMap<(usize, usize), LastReadStatus>,
    edited: &HashMap<(usize, usize), LastReadStatus>,
    latest: &HashMap<(usize, usize), LastReadStatus>,
    mode: UserUpdateMode,
) -> Res<HashMap<(usize, usize), LastReadStatus>> {
    let mut merged = latest.clone();
    let mut keys: Vec<_> = baseline.keys().chain(edited.keys()).copied().collect();
    keys.sort_unstable();
    keys.dedup();
    for key in keys {
        let base = baseline.get(&key).copied();
        let edit = edited.get(&key).copied();
        let live = latest.get(&key).copied();
        let value = match (base, edit, live) {
            (base, Some(edit), Some(live)) => {
                let base = base.unwrap_or_default();
                Some(LastReadStatus {
                    last_read: advance(base.last_read, edit.last_read, live.last_read, mode, "lastread_ptr_flags.last_read")?,
                    highest_msg_read: advance(
                        base.highest_msg_read,
                        edit.highest_msg_read,
                        live.highest_msg_read,
                        mode,
                        "lastread_ptr_flags.highest_msg_read",
                    )?,
                    include_qwk: mode.resolve(
                        three_way(&base.include_qwk, &edit.include_qwk, &live.include_qwk, "lastread_ptr_flags.include_qwk"),
                        &live.include_qwk,
                    )?,
                })
            }
            _ => mode.resolve(three_way(&base, &edit, &live, "lastread_ptr_flags"), &live)?,
        };
        if let Some(value) = value {
            merged.insert(key, value);
        } else {
            merged.remove(&key);
        }
    }
    Ok(merged)
}

fn edit_account(baseline: Option<&AccountUserInf>, edited: Option<&AccountUserInf>, latest: Option<&AccountUserInf>) -> Res<Option<AccountUserInf>> {
    for account in [baseline, edited, latest].into_iter().flatten() {
        account.validate()?;
    }
    let (Some(edited), Some(latest)) = (edited, latest) else {
        return Ok(three_way(&baseline, &edited, &latest, "account")?.cloned());
    };
    let zero = AccountUserInf::default();
    let baseline = baseline.unwrap_or(&zero);
    let mut merged = latest.clone();
    fields!(merged, baseline, edited, latest, "account.";
        starting_balance, start_this_session, debit_call, debit_time, debit_msg_read,
        debit_msg_read_capture, debit_msg_write, debit_msg_write_echoed,
        debit_msg_write_private, debit_download_file, debit_download_bytes,
        debit_group_chat, debit_tpu, debit_special, credit_upload_file,
        credit_upload_bytes, credit_special, drop_sec_level,
    );
    Ok(Some(merged))
}

fn finite(value: f64) -> Res<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Non-finite accounting balance or charge".into())
    }
}

/// Existing session-accounting delta semantics, independent of session state.
pub(crate) fn merge_account(
    current: Option<&AccountUserInf>,
    latest: Option<&AccountUserInf>,
    baseline: Option<&AccountUserInf>,
) -> Res<Option<AccountUserInf>> {
    // A session that never created an account must not delete another node's.
    let Some(current) = current else {
        return Ok(latest.cloned());
    };
    current.validate()?;
    let zero = AccountUserInf::default();
    let baseline = baseline.unwrap_or(&zero);
    baseline.validate()?;
    let mut merged = latest.cloned().unwrap_or_default();
    merged.validate()?;
    macro_rules! delta {
        ($($field:ident),+ $(,)?) => {
            $(merged.$field = finite(merged.$field + finite(current.$field - baseline.$field)?)?;)+
        };
    }
    delta!(
        starting_balance,
        start_this_session,
        debit_call,
        debit_time,
        debit_msg_read,
        debit_msg_read_capture,
        debit_msg_write,
        debit_msg_write_echoed,
        debit_msg_write_private,
        debit_download_file,
        debit_download_bytes,
        debit_group_chat,
        debit_tpu,
        debit_special,
        credit_upload_file,
        credit_upload_bytes,
        credit_special,
    );
    if current.drop_sec_level != baseline.drop_sec_level {
        merged.drop_sec_level = current.drop_sec_level;
    }
    Ok(Some(merged))
}

#[cfg(test)]
#[path = "user_store_tests.rs"]
mod tests;
