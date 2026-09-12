//! Session policy for accounting. The arithmetic and audit writer live in the
//! independent board accounting module; this module owns posting boundaries.
use async_recursion::async_recursion;
use chrono::{DateTime, Local, Utc};

use super::{IcyBoardState, Session, display_flags};
use crate::{
    Res,
    icy_board::{
        accounting::{TrackingEntry, append_tracking, format_credit, minutes_used, peak_at, time_charge},
        accounting_cfg::AccountingConfig,
        icb_config::{AccountingOptions, IcbConfig},
        icb_text::IceText,
        macro_parser::MacroCommand,
        pcb::user_inf::AccountUserInf,
        sec_levels::SecurityLevel,
    },
};

pub use crate::icy_board::accounting::AccountingMode;

/// An authenticated call's accounting snapshot. Constructing/loading a Session
/// does not activate it: the login owner must explicitly call accounting_start.
#[derive(Clone)]
pub struct AccountingSession {
    pub mode: AccountingMode,
    pub options: AccountingOptions,
    pub rates: AccountingConfig,
    /// An effective security ceiling, never a write to the user's security level.
    pub security_override: Option<u8>,
    pub(super) checking: bool,
    pub(super) begun: bool,
    start_balance_seeded: bool,
    finished: bool,
    finish_saved: bool,
    finish_save_required: bool,
    invocation_depth: usize,
    finish_requested: bool,
    pub(super) invocation_settlement_failed: bool,
    finish_at: Option<DateTime<Utc>>,
    global_started: Option<DateTime<Utc>>,
    conference_started: Option<DateTime<Utc>>,
    holidays: Vec<String>,
    last_balance: f64,
    warned: bool,
    exclude_local_tracking: bool,
    resolved_security: Option<u8>,
    baseline: Option<AccountUserInf>,
    baseline_known: bool,
}

impl Default for AccountingSession {
    fn default() -> Self {
        Self {
            mode: AccountingMode::Disabled,
            options: IcbConfig::default().accounting,
            rates: AccountingConfig::default(),
            security_override: None,
            checking: false,
            begun: false,
            start_balance_seeded: false,
            finished: false,
            finish_saved: false,
            finish_save_required: false,
            invocation_depth: 0,
            finish_requested: false,
            invocation_settlement_failed: false,
            finish_at: None,
            global_started: None,
            conference_started: None,
            holidays: Vec::new(),
            last_balance: 0.0,
            warned: false,
            exclude_local_tracking: false,
            resolved_security: None,
            baseline: None,
            baseline_known: false,
        }
    }
}

fn selected_mode(enabled: bool, level: Option<&SecurityLevel>, tracking_path: bool) -> AccountingMode {
    if !enabled {
        return AccountingMode::Disabled;
    }
    match level {
        Some(level) if level.accounting_tracking => {
            if tracking_path {
                AccountingMode::Tracking
            } else {
                AccountingMode::Disabled
            }
        }
        Some(level) if level.is_enabled => AccountingMode::Enforced,
        _ => AccountingMode::Disabled,
    }
}

fn finite(value: f64) -> Res<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Non-finite accounting balance or charge".into())
    }
}

impl Session {
    fn accounting_balance_at(&self, now: DateTime<Utc>) -> Res<f64> {
        if self.accounting.mode == AccountingMode::Disabled {
            return Ok(0.0);
        }
        let account = self
            .current_user
            .as_ref()
            .and_then(|user| user.account.as_ref())
            .ok_or("Active accounting has no account")?;
        let pending = self.accounting.global_started.map_or(0.0, |start| {
            time_charge(&self.accounting.rates, &self.accounting.options, &self.accounting.holidays, start, now, true)
        });
        finite(account.balance(self.accounting.options.concurrent_tracking, finite(pending)?))
    }

    /// Public numeric/PPE view. Corrupt arithmetic fails closed with a finite
    /// negative sentinel; the runtime check also reports the underlying error.
    pub fn calculate_balance(&self) -> f64 {
        self.accounting_balance_at(Utc::now()).unwrap_or(-f64::MAX)
    }
}

impl IcyBoardState {
    pub fn accounting_active(&self) -> bool {
        self.session.accounting.begun && !self.session.accounting.finished && self.session.accounting.mode != AccountingMode::Disabled
    }

    pub fn accounting_rates(&self) -> AccountingConfig {
        if self.accounting_active() {
            self.session.accounting.rates.clone()
        } else {
            AccountingConfig::default()
        }
    }

    /// Re-resolve PWRD after security changes without calling security processing
    /// again. This deliberately cannot authenticate an as-yet unstarted session.
    pub async fn accounting_refresh(&mut self) -> Res<()> {
        if !self.session.accounting.begun || self.session.accounting.finished {
            return Ok(());
        }
        let (mode, options, exclude_local) = {
            let board = self.get_board().await;
            let options = &board.config.accounting;
            let mode = selected_mode(
                options.enabled,
                board.sec_levels.find_match(self.session.cur_security, &self.session.last_password),
                !options.tracking_file.as_os_str().is_empty(),
            );
            if mode == self.session.accounting.mode && self.session.accounting.resolved_security.is_some() {
                return Ok(());
            }
            // Resolve against the board, not cwd. Empty paths must stay empty.
            let mut snapshot = options.clone();
            for path in [
                &mut snapshot.cfg_file,
                &mut snapshot.tracking_file,
                &mut snapshot.peak_holiday_list_file,
                &mut snapshot.info_file,
                &mut snapshot.warning_file,
                &mut snapshot.logoff_file,
            ] {
                if !path.as_os_str().is_empty() {
                    *path = board.resolve_file(path);
                }
            }
            (mode, snapshot, board.config.switches.exclude_local_calls_stats)
        };
        let rates = if mode != AccountingMode::Disabled {
            let rates = options
                .accounting_config
                .clone()
                .ok_or("Accounting is enabled but its rate configuration is missing")?;
            rates.validate()?;
            if let Some(account) = self.session.current_user.as_ref().and_then(|user| user.account.as_ref()) {
                account.validate()?;
                finite(account.balance(options.concurrent_tracking, 0.0))?;
            }
            rates
        } else {
            AccountingConfig::default()
        };
        // Settle the OLD snapshot before changing mode/rates. Disabled intervals
        // have no clock and can never be billed by a later re-enable.
        let now = Utc::now();
        if self.accounting_active() {
            self.accounting_settle_global_at(now)?;
            self.accounting_settle_conference_at(now)?;
        }
        let holidays = if mode != AccountingMode::Disabled && !options.peak_holiday_list_file.as_os_str().is_empty() {
            match std::fs::read_to_string(&options.peak_holiday_list_file) {
                Ok(text) => text.lines().map(str::to_owned).collect(),
                Err(error) => {
                    log::warn!("Cannot read accounting holidays {}: {error}", options.peak_holiday_list_file.display());
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        self.session.accounting.mode = mode;
        self.session.accounting.options = options;
        self.session.accounting.rates = rates;
        self.session.accounting.holidays = holidays;
        self.session.accounting.exclude_local_tracking = exclude_local;
        self.session.accounting.resolved_security = Some(self.session.cur_security);
        let clock = (mode != AccountingMode::Disabled).then_some(now);
        self.session.accounting.global_started = clock;
        self.session.accounting.conference_started = clock;
        if mode != AccountingMode::Disabled {
            let account = self
                .session
                .current_user
                .as_mut()
                .ok_or("Accounting has no current user")?
                .account
                .get_or_insert_with(AccountUserInf::default);
            if !self.session.accounting.start_balance_seeded {
                account.start_this_session = finite(account.balance(self.session.accounting.options.concurrent_tracking, 0.0))?;
                self.session.accounting.start_balance_seeded = true;
            }
        }
        Ok(())
    }

    /// Begin exactly once, after the login owner authenticated the selected user.
    /// Missing accounts on existing users start at zero; registration owns grants
    /// of new_user_balance, so loading an old account cannot mint a new grant.
    #[async_recursion(?Send)]
    pub async fn accounting_start(&mut self) -> Res<()> {
        if self.session.accounting.begun {
            return Ok(());
        }
        if self.session.cur_user_id < 0 || self.session.current_user.is_none() {
            return Err("Accounting requires an authenticated current user".into());
        }
        if let Some(account) = self.session.current_user.as_ref().and_then(|user| user.account.as_ref()) {
            account.validate()?;
        }
        let before = self.session.accounting.clone();
        let before_user = self.session.current_user.clone();
        let before_security = self.session.cur_security;
        let before_limit = self.session.time_limit;
        if !self.session.accounting.baseline_known {
            self.accounting_mark_saved();
        }
        self.session.accounting.begun = true;
        let setup: Res<()> = async {
            self.accounting_refresh().await?;
            // Authentication/setup is not a billable interval. In particular an
            // empty-account level change must not generate TIME ONLINE records.
            self.session.accounting.global_started = None;
            self.session.accounting.conference_started = None;
            if self.accounting_active() {
                finite(self.session.current_conference.charge_time)?;
                finite(self.session.current_conference.charge_msg_read)?;
                finite(self.session.current_conference.charge_msg_write)?;
                let balance = self.session.accounting_balance_at(Utc::now())?;
                // Resolve an empty login before the logon debit; do not loop
                // between levels, and never promote a caller via SEC_DROP.
                if balance <= 0.0 && self.session.accounting.mode == AccountingMode::Enforced {
                    self.accounting_drop_security().await?;
                }
            }
            Ok(())
        }
        .await;
        // Record has no fallible work after its monetary commit. Thus every
        // error here is safe to roll back and retry; no audit/debit is duplicated.
        // LOGIN.C logs how the caller reached the board: "LOCAL" or the speed.
        let connection = if self.session.is_local {
            "LOCAL".to_string()
        } else {
            self.get_bps().max(0).to_string()
        };
        let posted = setup.and_then(|()| self.accounting_record(2, "LOGON", &connection, self.accounting_rates().charge_per_logon, 1));
        if let Err(error) = posted {
            self.session.accounting = before;
            self.session.current_user = before_user;
            self.session.cur_security = before_security;
            self.session.time_limit = before_limit;
            return Err(error);
        }
        let clock = self.accounting_active().then_some(Utc::now());
        self.session.accounting.global_started = clock;
        self.session.accounting.conference_started = clock;
        // A call consuming the last credit may still use costless activities.
        self.session.accounting.last_balance = self.session.calculate_balance();
        Ok(())
    }

    /// Posting is synchronous and has one monetary commit point. Audit failure
    /// is logged after that point, never returned as an unposted/retryable debit.
    pub(crate) fn accounting_record(&mut self, field: usize, activity: &str, sub: &str, unit: f64, qty: i64) -> Res<f64> {
        if !self.accounting_active() {
            return Ok(0.0);
        }
        let amount = finite(finite(unit)? * qty as f64)?;
        let user = self.session.current_user.as_mut().ok_or("Accounting has no current user")?;
        let account = user.account.as_mut().ok_or("Accounting has no current account")?;
        // Validate the resulting balance too: finite fields can still overflow
        // their sum, and concurrent max must not conceal an overflowing category.
        let mut posted = account.clone();
        posted.apply_charge(field, amount)?;
        finite(posted.balance(self.session.accounting.options.concurrent_tracking, 0.0))?;
        *account = posted;
        if (amount == 0.0 && self.session.accounting.mode == AccountingMode::Enforced)
            || self.session.accounting.options.tracking_file.as_os_str().is_empty()
            || (self.session.is_local && self.session.accounting.exclude_local_tracking)
        {
            return Ok(amount);
        }
        let entry = TrackingEntry {
            at: Local::now(),
            user: user.get_name().clone(),
            node: (self.node + 1).min(u16::MAX as usize) as u16,
            conference: self.session.current_conference_number,
            activity: activity.into(),
            sub_activity: sub.into(),
            unit_cost: unit,
            quantity: qty,
            value: amount,
        };
        if let Err(error) = append_tracking(&self.session.accounting.options.tracking_file, &entry) {
            log::error!("Accounting charge POSTED but tracking failed ({} / {} / {amount}): {error}", activity, sub);
        }
        Ok(amount)
    }

    fn accounting_settle_global_at(&mut self, now: DateTime<Utc>) -> Res<()> {
        if !self.accounting_active() {
            return Ok(());
        }
        if let Some(start) = self.session.accounting.global_started {
            if now <= start {
                return Ok(());
            }
            self.session.accounting.rates.validate()?;
            // Use the domain calendar classifier with unit rates to obtain
            // quantities for the two audit rows, including DST and holidays.
            let counter = AccountingConfig {
                charge_per_time: 1.0,
                ..Default::default()
            };
            let normal = finite(time_charge(
                &counter,
                &self.session.accounting.options,
                &self.session.accounting.holidays,
                start,
                now,
                false,
            ))? as i64;
            let peak = (now.timestamp().div_euclid(60) - start.timestamp().div_euclid(60) - normal).max(0);
            let rates = self.accounting_rates();
            // Preflight both posts before either commit. There is no await or
            // fallible tracking propagation between them, so a retry cannot
            // duplicate the first half of a normal/peak settlement.
            let mut candidate = self
                .session
                .current_user
                .as_ref()
                .and_then(|user| user.account.clone())
                .ok_or("Accounting has no account")?;
            if peak != 0 {
                candidate.apply_charge(3, finite(rates.charge_per_peak_time * peak as f64)?)?;
                finite(candidate.balance(self.session.accounting.options.concurrent_tracking, 0.0))?;
            }
            candidate.apply_charge(3, finite(rates.charge_per_time * normal as f64)?)?;
            finite(candidate.balance(self.session.accounting.options.concurrent_tracking, 0.0))?;
            if peak != 0 {
                self.accounting_record(3, "TIME ONLINE", "PEAK", rates.charge_per_peak_time, peak)?;
            }
            self.accounting_record(3, "TIME ONLINE", "", rates.charge_per_time, normal)?;
            self.session.accounting.global_started = Some(now);
        }
        Ok(())
    }

    fn accounting_settle_conference_at(&mut self, now: DateTime<Utc>) -> Res<()> {
        if self.accounting_active() {
            if let Some(start) = self.session.accounting.conference_started {
                let quantity = minutes_used(now - start);
                if quantity != 0 {
                    self.accounting_record(3, "CONF TIME", "", self.session.current_conference.charge_time, quantity)?;
                }
            }
            self.session.accounting.conference_started = Some(now);
        } else {
            self.session.accounting.conference_started = None;
        }
        Ok(())
    }

    pub async fn accounting_settle_conference(&mut self) -> Res<()> {
        self.accounting_settle_conference_at(Utc::now())
    }

    pub(crate) fn accounting_begin_invocation(&mut self) {
        self.session.accounting.invocation_depth += 1;
    }

    pub(crate) fn accounting_invocation_active(&self) -> bool {
        self.session.accounting.invocation_depth != 0
    }

    /// Every started command/door must unwind here, after posting its usage,
    /// including on handler/settlement errors. Only the outermost one finishes.
    #[async_recursion(?Send)]
    pub(crate) async fn accounting_end_invocation(&mut self) -> Res<()> {
        debug_assert!(self.accounting_invocation_active());
        self.session.accounting.invocation_depth -= 1;
        if self.accounting_invocation_active() || !self.session.accounting.finish_requested {
            return Ok(());
        }
        if self.session.logoff_pending.is_some() {
            self.accounting_complete_logoff().await
        } else {
            self.accounting_finish().await
        }
    }

    /// No display/input here. A request inside a command/door leaves accounting
    /// active until all enclosing usage has settled. Failed persistence may be
    /// retried, but neither clock nor audit entries are posted twice.
    pub async fn accounting_finish(&mut self) -> Res<()> {
        self.session.accounting.finish_save_required |= self.pending_user_save.is_some();
        self.reconcile_pending_user_save(true).await?;
        self.session.accounting.finish_requested = true;
        if self.accounting_invocation_active() {
            return Ok(());
        }
        if !self.session.accounting.begun || self.session.accounting.finish_saved {
            if self.session.accounting.finish_save_required {
                self.persist_final_user().await?;
                self.session.accounting.finish_save_required = false;
            }
            return Ok(());
        }
        if !self.session.accounting.finished {
            let now = *self.session.accounting.finish_at.get_or_insert_with(Utc::now);
            self.accounting_settle_global_at(now)?;
            self.session.accounting.global_started = None;
            self.accounting_settle_conference_at(now)?;
            self.session.accounting.conference_started = None;
            self.session.accounting.finished = true;
        }
        self.persist_final_user().await?;
        self.session.accounting.finish_saved = true;
        self.session.accounting.finish_save_required = false;
        Ok(())
    }

    async fn accounting_drop_security(&mut self) -> Res<bool> {
        if self.session.accounting.options.ignore_empty_sec_level {
            return Ok(false);
        }
        let target = self
            .session
            .current_user
            .as_ref()
            .and_then(|user| user.account.as_ref())
            .ok_or("Accounting has no account")?
            .drop_sec_level;
        if target >= self.session.cur_security {
            return Ok(false);
        }
        let now = Utc::now();
        self.accounting_settle_global_at(now)?;
        self.accounting_settle_conference_at(now)?;
        self.session.accounting.security_override = Some(target);
        self.session.cur_security = target;
        // No call to apply_conference_security: it would loop back through the
        // accounting resolver. Re-read limits, then re-resolve the mode directly.
        let old_limit = self.session.time_limit;
        self.apply_pwrd_limits().await;
        self.limit_time_for_event().await;
        if self.session.time_adjusted_for_event && old_limit != 0 {
            self.session.time_limit = if self.session.time_limit == 0 {
                old_limit
            } else {
                self.session.time_limit.min(old_limit)
            };
        }
        self.accounting_refresh().await?;
        Ok(true)
    }

    /// All recursive display/input entry points see checking=true. Always clear
    /// it after the awaited work, including every returned error.
    #[async_recursion(?Send)]
    pub async fn accounting_check_balance(&mut self) -> Res<()> {
        if self.session.accounting.checking || !self.session.accounting.begun || self.session.accounting.finished || self.session.accounting.finish_requested {
            return Ok(());
        }
        self.session.accounting.checking = true;
        let result = self.accounting_check_balance_inner().await;
        self.session.accounting.checking = false;
        if result.is_err() {
            self.session.request_logoff = true;
        }
        result
    }

    async fn accounting_check_balance_inner(&mut self) -> Res<()> {
        self.accounting_refresh().await?;
        if self.session.accounting.mode != AccountingMode::Enforced {
            return Ok(());
        }
        let balance = self.session.accounting_balance_at(Utc::now())?;
        let changed = balance != self.session.accounting.last_balance;
        self.session.accounting.last_balance = balance;
        if changed && balance <= 0.0 && self.accounting_drop_security().await? {
            self.display_text(
                IceText::CreditExceeded,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT | display_flags::BELL,
            )
            .await?;
            self.display_text(IceText::SecurityChanged, display_flags::NEWLINE | display_flags::LOGIT)
                .await?;
        }
        if self.session.accounting.mode != AccountingMode::Enforced {
            return Ok(());
        }
        let balance = self.session.accounting_balance_at(Utc::now())?;
        self.session.accounting.last_balance = balance;
        let rate = if peak_at(&self.session.accounting.options, &self.session.accounting.holidays, Local::now()) {
            self.session.accounting.rates.charge_per_peak_time
        } else {
            self.session.accounting.rates.charge_per_time
        };
        let per_minute = finite(rate + self.session.current_conference.charge_time)?;
        if per_minute > 0.0 {
            // A fraction of a minute remains usable. Grace belongs to the
            // balance preview, not a second allowance added to this time cap.
            let remaining = (balance.max(0.0) / per_minute).ceil().min(i32::MAX as f64) as i64;
            let online = (Utc::now() - self.session.login_date).num_minutes().max(0);
            let cap = online.saturating_add(remaining).min(i32::MAX as i64) as i32;
            // Zero is the engine's UNLIMITED sentinel, not an expired limit.
            let cap = if cap == 0 { -1 } else { cap };
            if self.session.time_limit == 0 || cap < self.session.time_limit {
                self.session.time_limit = cap;
            }
        }
        if balance <= self.session.accounting.rates.warn_level {
            if !self.session.accounting.warned {
                self.session.accounting.warned = true;
                let path = self.session.accounting.options.warning_file.clone();
                if !path.as_os_str().is_empty() {
                    self.display_file(&path).await?;
                }
            }
        } else {
            self.session.accounting.warned = false;
        }
        Ok(())
    }

    pub(crate) fn accounting_credit_insufficient(&self, charge: f64, reserved: f64) -> Res<bool> {
        finite(charge)?;
        finite(reserved)?;
        if charge <= 0.0 || self.session.accounting.mode != AccountingMode::Enforced || !self.accounting_active() {
            return Ok(false);
        }
        Ok(charge > finite(self.session.accounting_balance_at(Utc::now())? - reserved)?)
    }

    #[async_recursion(?Send)]
    pub async fn accounting_insufficient(&mut self, charge: f64, reserved: f64) -> Res<bool> {
        let insufficient = self.accounting_credit_insufficient(charge, reserved)?;
        if insufficient && !self.session.accounting.checking {
            self.session.accounting.checking = true;
            let previous = std::mem::replace(&mut self.session.op_text, format_credit(charge, self.session.accounting.options.use_money));
            let result = self
                .display_text(
                    IceText::InsufficientCredits,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT | display_flags::BELL,
                )
                .await;
            self.session.op_text = previous;
            self.session.accounting.checking = false;
            result?;
        }
        Ok(insufficient)
    }

    pub async fn accounting_charge_message(&mut self, private: bool, echo: bool, to: &str) -> Res<()> {
        let rates = self.accounting_rates();
        let (field, activity, rate) = if private {
            (8, "MSG WRITE PRIV", rates.charge_per_msg_write_private)
        } else if echo {
            (7, "MSG WRITE ECHO", rates.charge_per_msg_write_echoed)
        } else {
            (6, "MSG WRITE", rates.charge_per_msg_written)
        };
        self.accounting_record(field, activity, to, rate + self.session.current_conference.charge_msg_write, 1)?;
        self.accounting_check_balance().await
    }

    pub(super) fn accounting_macro(&mut self, command: &MacroCommand) -> String {
        let active = self.session.accounting.mode != AccountingMode::Disabled;
        if (matches!(command, MacroCommand::CredLeft) && self.session.accounting.mode != AccountingMode::Enforced)
            || (matches!(command, MacroCommand::CredStart) && !active)
        {
            return self.unlimited_text();
        }
        if !active {
            return "0".into();
        }
        let Some(account) = self.session.current_user.as_ref().and_then(|user| user.account.as_ref()) else {
            return "invalid".into();
        };
        let balance = self.session.calculate_balance();
        let value = match command {
            MacroCommand::CredLeft => balance,
            MacroCommand::CredNow => account.start_this_session - balance,
            MacroCommand::CredStart => account.starting_balance,
            MacroCommand::CredUsed => account.starting_balance - balance,
            _ => return String::new(),
        };
        format_credit(value, self.session.accounting.options.use_money)
    }

    pub(super) fn accounting_update_baseline(&self) -> Option<Option<AccountUserInf>> {
        self.session.accounting.baseline_known.then(|| self.session.accounting.baseline.clone())
    }

    #[cfg(test)]
    pub(super) fn accounting_set_update_baseline(&self, baseline: &mut super::User) {
        if let Some(account) = self.accounting_update_baseline() {
            baseline.account = account;
        }
    }

    pub(super) fn accounting_mark_saved(&mut self) {
        self.session.accounting.baseline = self.session.current_user.as_ref().and_then(|user| user.account.clone());
        self.session.accounting.baseline_known = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::user_store::merge_account;
    use crate::icy_board::{IcyBoard, bbs::BBS, user_base::User};
    use chrono::Duration;
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::{path::PathBuf, sync::Arc};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    async fn fixture() -> (TempDir, IcyBoardState, ChannelConnection) {
        let root = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::new();
        board.root_path = root.path().to_path_buf();
        board.config.paths.user_file = PathBuf::from("users.toml");
        board.config.paths.caller_log = PathBuf::new();
        board.config.accounting.enabled = true;
        board.config.accounting.ignore_empty_sec_level = false;
        board.config.accounting.concurrent_tracking = false;
        board.config.accounting.info_file = PathBuf::new();
        board.config.accounting.warning_file = PathBuf::new();
        board.config.accounting.logoff_file = PathBuf::new();
        board.config.accounting.tracking_file = PathBuf::new();
        board.config.accounting.peak_holiday_list_file = PathBuf::new();
        board.config.accounting.accounting_config = Some(AccountingConfig::default());
        board.sec_levels.push(SecurityLevel {
            security: 10,
            is_enabled: true,
            time_per_day: 60,
            ..Default::default()
        });
        board.users.new_user(User {
            name: "ACCOUNT TEST".into(),
            security_level: 10,
            account: Some(AccountUserInf {
                starting_balance: 100.0,
                ..Default::default()
            }),
            ..Default::default()
        });
        let user = board.users[0].clone();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user);
        state.session.cur_user_id = 0;
        state.session.cur_security = 10;
        state.session.user_name = "ACCOUNT TEST".into();
        state.session.page_len = 0;
        state.session.disp_options.count_lines = false;
        (root, state, peer)
    }

    fn account(state: &IcyBoardState) -> &AccountUserInf {
        state.session.current_user.as_ref().unwrap().account.as_ref().unwrap()
    }

    fn account_mut(state: &mut IcyBoardState) -> &mut AccountUserInf {
        state.session.current_user.as_mut().unwrap().account.as_mut().unwrap()
    }

    #[test]
    fn mode_requires_matching_level_and_tracking_path_with_t_precedence() {
        let mut level = SecurityLevel {
            is_enabled: true,
            ..Default::default()
        };
        assert_eq!(selected_mode(false, Some(&level), true), AccountingMode::Disabled);
        assert_eq!(selected_mode(true, None, true), AccountingMode::Disabled);
        assert_eq!(selected_mode(true, Some(&SecurityLevel::default()), true), AccountingMode::Disabled);
        assert_eq!(selected_mode(true, Some(&level), false), AccountingMode::Enforced);
        level.accounting_tracking = true;
        assert_eq!(selected_mode(true, Some(&level), false), AccountingMode::Disabled);
        assert_eq!(selected_mode(true, Some(&level), true), AccountingMode::Tracking);
    }

    #[tokio::test]
    async fn login_must_be_explicit_and_missing_rates_are_not_free() {
        let (_root, mut state, _peer) = fixture().await;
        state.accounting_refresh().await.unwrap();
        assert!(!state.accounting_active());
        assert_eq!(state.accounting_record(2, "LOGON", "", 10.0, 1).unwrap(), 0.0);
        state.session.cur_user_id = -1;
        assert!(state.accounting_start().await.is_err());
        state.session.cur_user_id = 0;
        state.get_board().await.config.accounting.accounting_config = None;
        assert!(state.accounting_start().await.is_err());
        assert!(!state.session.accounting.begun);
        state.get_board().await.config.accounting.accounting_config = Some(AccountingConfig::default());
        state.accounting_start().await.unwrap();
        assert!(state.accounting_active());
    }

    #[tokio::test]
    async fn failed_start_can_retry_without_retaining_a_seed_or_logon_debit() {
        let (_root, mut state, _peer) = fixture().await;
        account_mut(&mut state).starting_balance = f64::MAX;
        account_mut(&mut state).debit_call = f64::MAX;
        state.get_board().await.config.accounting.ignore_empty_sec_level = true;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_logon = f64::MAX;
        let original = account(&state).clone();
        assert!(state.accounting_start().await.is_err());
        assert!(!state.session.accounting.begun);
        assert_eq!(*account(&state), original);
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_logon = 0.0;
        state.accounting_start().await.unwrap();
        assert_eq!(account(&state).debit_call, f64::MAX);
    }

    #[tokio::test]
    async fn calendar_preview_has_one_grace_minute_and_uses_selected_balance_mode() {
        let (_root, mut state, _peer) = fixture().await;
        {
            let mut board = state.get_board().await;
            let rates = board.config.accounting.accounting_config.as_mut().unwrap();
            rates.charge_per_time = 2.0;
            rates.charge_per_peak_time = 2.0;
        }
        state.accounting_start().await.unwrap();
        let start = DateTime::parse_from_rfc3339("2026-09-07T12:00:59Z").unwrap().with_timezone(&Utc);
        state.session.accounting.global_started = Some(start);
        let account = account_mut(&mut state);
        account.debit_call = 20.0;
        account.debit_msg_read = 30.0;
        account.debit_time = 10.0;
        assert_eq!(state.session.accounting_balance_at(start + Duration::seconds(1)).unwrap(), 40.0);
        assert_eq!(state.session.accounting_balance_at(start + Duration::seconds(61)).unwrap(), 38.0);
        state.session.accounting.options.concurrent_tracking = true;
        assert_eq!(state.session.accounting_balance_at(start + Duration::seconds(61)).unwrap(), 70.0);
    }

    #[tokio::test]
    async fn activating_after_disabled_login_seeds_a_missing_account_without_billing_the_gap() {
        let (_root, mut state, _peer) = fixture().await;
        state.get_board().await.config.accounting.enabled = false;
        state.session.current_user.as_mut().unwrap().account = None;
        state.accounting_start().await.unwrap();
        assert!(state.session.current_user.as_ref().unwrap().account.is_none());
        state.session.login_date = Utc::now() - Duration::hours(1);
        state.get_board().await.config.accounting.enabled = true;
        let now = Utc::now();
        state.accounting_refresh().await.unwrap();
        assert_eq!(account(&state).starting_balance, 0.0);
        assert_eq!(account(&state).start_this_session, 0.0);
        assert_eq!(account(&state).debit_time, 0.0);
        assert!(state.session.accounting.global_started.unwrap() >= now);
    }

    #[tokio::test]
    async fn start_finish_and_final_time_post_exactly_once() {
        let (root, mut state, _peer) = fixture().await;
        {
            let mut board = state.get_board().await;
            board.config.accounting.tracking_file = PathBuf::from("tracking.txt");
            let rates = board.config.accounting.accounting_config.as_mut().unwrap();
            rates.charge_per_logon = 5.0;
            rates.charge_per_time = 2.0;
            rates.charge_per_peak_time = 2.0;
        }
        state.accounting_start().await.unwrap();
        state.accounting_start().await.unwrap();
        assert_eq!(account(&state).start_this_session, 100.0);
        assert_eq!(account(&state).debit_call, 5.0);
        let end = Utc::now();
        state.session.accounting.global_started = Some(end - Duration::minutes(2));
        state.session.accounting.conference_started = Some(end - Duration::seconds(90));
        state.session.accounting.finish_at = Some(end);
        state.session.current_conference.charge_time = 3.0;
        assert_eq!(state.session.accounting_balance_at(end).unwrap(), 93.0, "preview waives one global minute");
        state.accounting_finish().await.unwrap();
        let finished = account(&state).clone();
        assert_eq!(finished.debit_time, 10.0, "final global 2*2 plus rounded conference 2*3");
        state.accounting_finish().await.unwrap();
        state.accounting_start().await.unwrap();
        assert_eq!(*account(&state), finished);
        assert!(!state.accounting_active());
        assert_eq!(state.session.calculate_balance(), 85.0);
        assert_eq!(std::fs::read_to_string(root.path().join("tracking.txt")).unwrap().lines().count(), 3);
    }

    #[tokio::test]
    async fn ordinary_profile_saves_keep_clocks_active_and_count_only_new_minutes() {
        let (_root, mut state, _peer) = fixture().await;
        state.accounting_start().await.unwrap();
        state.session.login_date = Utc::now() - Duration::minutes(10) - Duration::seconds(15);
        let user = state.session.current_user.as_mut().unwrap();
        user.stats.last_on = state.session.login_date;
        user.stats.minutes_today = 7;
        let clocks = (state.session.accounting.global_started, state.session.accounting.conference_started);

        // Both W and LANG use this ordinary save path, not session finalization.
        for _ in 0..2 {
            state.save_current_user().await.unwrap();
            assert!(state.accounting_active());
            assert!(!state.session.accounting.finish_requested);
            assert_eq!((state.session.accounting.global_started, state.session.accounting.conference_started), clocks);
            assert_eq!(state.session.current_user.as_ref().unwrap().stats.minutes_today, 17);
        }
        state.accounting_record(4, "READ", "AFTER SAVE", 5.0, 1).unwrap();
        state.session.login_date -= Duration::minutes(2);
        // Keep this elapsed-time simulation independent of the calendar reset
        // policy, including when the test runs just after midnight.
        state.session.current_user.as_mut().unwrap().stats.last_on = state.session.login_date;
        state.save_current_user().await.unwrap();
        assert!(state.accounting_active());
        assert_eq!(state.get_board().await.users[0].stats.minutes_today, 19);
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_msg_read, 5.0);

        // Online time after the profile saves must still reach final billing.
        let end = Utc::now();
        state.session.accounting.rates.charge_per_time = 2.0;
        state.session.accounting.rates.charge_per_peak_time = 2.0;
        state.session.accounting.global_started = Some(end - Duration::minutes(2));
        state.session.accounting.finish_at = Some(end);
        state.accounting_finish().await.unwrap();
        state.accounting_finish().await.unwrap();
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_time, 4.0);
        assert_eq!(state.get_board().await.users[0].stats.minutes_today, 19);
    }

    #[tokio::test]
    async fn failed_profile_save_retry_neither_finishes_nor_recounts_minutes() {
        let (root, mut state, _peer) = fixture().await;
        state.accounting_start().await.unwrap();
        state.session.login_date = Utc::now() - Duration::minutes(10) - Duration::seconds(15);
        let user = state.session.current_user.as_mut().unwrap();
        user.stats.last_on = state.session.login_date;
        user.stats.minutes_today = 0;
        state.get_board().await.config.paths.user_file = root.path().to_path_buf();
        assert!(state.save_current_user().await.is_err());
        assert!(state.accounting_active());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.minutes_today, 10);
        state.accounting_record(4, "READ", "AFTER FAILED SAVE", 3.0, 1).unwrap();
        state.get_board().await.config.paths.user_file = PathBuf::from("users.toml");
        state.save_current_user().await.unwrap();
        state.save_current_user().await.unwrap();
        assert!(state.accounting_active());
        assert_eq!(state.get_board().await.users[0].stats.minutes_today, 10);
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_msg_read, 3.0);
    }

    #[tokio::test]
    async fn lang_command_save_keeps_accounting_active_for_later_charges() {
        use crate::icy_board::language::Language;
        let (root, mut state, _peer) = fixture().await;
        {
            let mut board = state.get_board().await;
            board.config.paths.command_display_path = root.path().join("missing-commands");
            board.languages.clear();
            board.languages.push(Language {
                description: "English".into(),
                locale: "en_US".into(),
                extension: "eng".into(),
                yes_char: 'Y',
                no_char: 'N',
            });
        }
        state.accounting_start().await.unwrap();
        state.session.tokens.push_back("1".into());
        tokio::time::timeout(std::time::Duration::from_secs(3), state.set_language_cmd())
            .await
            .unwrap()
            .unwrap();
        assert!(state.accounting_active());
        assert_eq!(state.get_board().await.users[0].language, "eng");
        state.accounting_record(4, "READ", "AFTER LANG", 5.0, 1).unwrap();
        state.save_current_user().await.unwrap();
        assert!(state.accounting_active());
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_msg_read, 5.0);
    }

    #[tokio::test]
    async fn failed_final_save_rolls_back_shared_user_and_retry_does_not_bill_again() {
        let (root, mut state, _peer) = fixture().await;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_time = 2.0;
        state
            .get_board()
            .await
            .config
            .accounting
            .accounting_config
            .as_mut()
            .unwrap()
            .charge_per_peak_time = 2.0;
        state.accounting_start().await.unwrap();
        let end = Utc::now();
        state.session.accounting.global_started = Some(end - Duration::minutes(2));
        state.session.accounting.finish_at = Some(end);
        state.get_board().await.config.paths.user_file = root.path().to_path_buf();
        assert!(state.accounting_finish().await.is_err());
        assert_eq!(account(&state).debit_time, 4.0);
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_time, 0.0);
        assert!(!state.session.accounting.finish_saved);
        state.get_board().await.config.paths.user_file = PathBuf::from("users.toml");
        state.accounting_finish().await.unwrap();
        assert_eq!(account(&state).debit_time, 4.0);
        assert_eq!(state.get_board().await.users[0].account.as_ref().unwrap().debit_time, 4.0);
    }

    #[tokio::test]
    async fn disabled_and_tracking_sessions_do_not_enforce() {
        let (root, mut state, _peer) = fixture().await;
        state.get_board().await.sec_levels[0].accounting_tracking = true;
        state.accounting_start().await.unwrap();
        assert_eq!(state.session.accounting.mode, AccountingMode::Disabled);
        assert_eq!(state.accounting_record(9, "DOWNLOAD", "", 200.0, 1).unwrap(), 0.0);
        state.get_board().await.config.accounting.tracking_file = root.path().join("tracking.txt");
        state.accounting_refresh().await.unwrap();
        assert_eq!(state.session.accounting.mode, AccountingMode::Tracking);
        assert_eq!(state.accounting_record(9, "DOWNLOAD", "", 200.0, 1).unwrap(), 200.0);
        state.accounting_record(9, "FREE", "", 0.0, 1).unwrap();
        assert!(!state.accounting_insufficient(500.0, 0.0).await.unwrap());
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.cur_security, 10);
        assert_eq!(state.session.calculate_balance(), -100.0);
        assert_eq!(std::fs::read_to_string(root.path().join("tracking.txt")).unwrap().lines().count(), 2);
    }

    #[tokio::test]
    async fn empty_login_drops_before_call_without_modifying_permanent_security() {
        let (_root, mut state, _peer) = fixture().await;
        account_mut(&mut state).starting_balance = 0.0;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_logon = 5.0;
        state.accounting_start().await.unwrap();
        assert_eq!(state.session.cur_security, 0);
        assert_eq!(state.session.current_user.as_ref().unwrap().security_level, 10);
        assert_eq!(account(&state).debit_call, 0.0);
        assert_eq!(state.session.accounting.mode, AccountingMode::Disabled);
        state.session.current_conference.add_conference_security = 50;
        state.apply_conference_security().await;
        assert_eq!(state.session.cur_security, 0, "conference must not undo the session override");
    }

    #[tokio::test]
    async fn missing_existing_account_does_not_receive_new_user_grant_or_promotion() {
        let (_root, mut state, _peer) = fixture().await;
        state.session.current_user.as_mut().unwrap().account = None;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().new_user_balance = 500.0;
        state.accounting_start().await.unwrap();
        assert_eq!(account(&state).starting_balance, 0.0);
        assert_eq!(state.session.cur_security, 0);

        let (_root, mut state, _peer) = fixture().await;
        account_mut(&mut state).starting_balance = 0.0;
        account_mut(&mut state).drop_sec_level = 255;
        state.accounting_start().await.unwrap();
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.cur_security, 10);
        assert_eq!(state.session.accounting.security_override, None);
    }

    #[tokio::test]
    async fn zero_after_logon_keeps_free_activities_but_a_new_debit_drops() {
        let (_root, mut state, _peer) = fixture().await;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_logon = 100.0;
        state.accounting_start().await.unwrap();
        state.accounting_check_balance().await.unwrap();
        state.accounting_record(4, "READ", "", 0.0, 1).unwrap();
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.cur_security, 10);
        state.accounting_record(4, "READ", "", 1.0, 1).unwrap();
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.cur_security, 0);
        assert_eq!(state.session.accounting.mode, AccountingMode::Disabled);
        assert!(!state.session.accounting.checking);
    }

    #[tokio::test]
    async fn exhaustion_settles_global_time_before_disabling_and_reenable_has_new_clock() {
        let (_root, mut state, _peer) = fixture().await;
        {
            let mut board = state.get_board().await;
            let rates = board.config.accounting.accounting_config.as_mut().unwrap();
            rates.charge_per_time = 2.0;
            rates.charge_per_peak_time = 2.0;
        }
        state.accounting_start().await.unwrap();
        state.session.accounting.global_started = Some(Utc::now() - Duration::minutes(2));
        state.accounting_record(4, "READ", "", 100.0, 1).unwrap();
        state.accounting_check_balance().await.unwrap();
        assert_eq!(account(&state).debit_time, 4.0);
        assert_eq!(state.session.accounting.mode, AccountingMode::Disabled);
        assert!(state.session.accounting.global_started.is_none());
        // A disabled interval has no clock; re-enable at the current instant.
        let before = Utc::now();
        state.session.cur_security = 10;
        state.accounting_refresh().await.unwrap();
        assert!(state.session.accounting.global_started.unwrap() >= before);
        assert_eq!(account(&state).debit_time, 4.0);
    }

    #[tokio::test]
    async fn audit_failure_returns_posted_value_and_invalid_amount_does_not_mutate() {
        let (root, mut state, _peer) = fixture().await;
        state.get_board().await.config.accounting.tracking_file = root.path().join("missing-parent/tracking.txt");
        state.accounting_start().await.unwrap();
        assert_eq!(state.accounting_record(4, "READ", "", 5.0, 1).unwrap(), 5.0);
        assert_eq!(account(&state).debit_msg_read, 5.0);
        assert!(state.accounting_record(4, "READ", "", f64::MAX, 2).is_err());
        assert_eq!(account(&state).debit_msg_read, 5.0);
    }

    #[tokio::test]
    async fn balance_errors_fail_closed_and_clear_recursion_guard() {
        let (_root, mut state, _peer) = fixture().await;
        state.accounting_start().await.unwrap();
        account_mut(&mut state).debit_time = f64::NAN;
        assert!(state.session.calculate_balance().is_finite());
        assert!(state.session.calculate_balance() < 0.0);
        assert!(state.accounting_check_balance().await.is_err());
        assert!(!state.session.accounting.checking);
        assert!(state.session.request_logoff);
    }

    #[tokio::test]
    async fn insufficient_display_restores_guard_and_operator_text_even_on_disconnect() {
        let (_root, mut state, peer) = fixture().await;
        state.accounting_start().await.unwrap();
        assert!(!state.accounting_insufficient(50.0, 50.0).await.unwrap());
        state.session.op_text = "saved".into();
        drop(peer);
        let _ = state.accounting_insufficient(50.0, 51.0).await;
        assert!(!state.session.accounting.checking);
        assert_eq!(state.session.op_text, "saved");
        assert_eq!(state.session.calculate_balance(), 100.0, "reservations are not charges");
    }

    #[tokio::test]
    async fn message_categories_include_conference_surcharge_and_private_takes_precedence() {
        let (_root, mut state, _peer) = fixture().await;
        {
            let mut board = state.get_board().await;
            let rates = board.config.accounting.accounting_config.as_mut().unwrap();
            rates.charge_per_msg_written = 1.0;
            rates.charge_per_msg_write_echoed = 2.0;
            rates.charge_per_msg_write_private = 3.0;
        }
        state.accounting_start().await.unwrap();
        state.session.current_conference.charge_msg_write = 4.0;
        state.accounting_charge_message(false, false, "ALL").await.unwrap();
        state.accounting_charge_message(false, true, "ECHO").await.unwrap();
        state.accounting_charge_message(true, true, "RECIPIENT").await.unwrap();
        assert_eq!(account(&state).debit_msg_write, 5.0);
        assert_eq!(account(&state).debit_msg_write_echoed, 6.0);
        assert_eq!(account(&state).debit_msg_write_private, 7.0);
    }

    #[tokio::test]
    async fn macros_use_mode_and_distinguish_session_spending_from_lifetime_spending() {
        let (_root, mut state, _peer) = fixture().await;
        account_mut(&mut state).debit_special = 20.0;
        state.accounting_start().await.unwrap();
        state.accounting_record(4, "READ", "", 5.0, 1).unwrap();
        assert_eq!(state.accounting_macro(&MacroCommand::CredLeft), "75");
        assert_eq!(state.accounting_macro(&MacroCommand::CredNow), "5");
        assert_eq!(state.accounting_macro(&MacroCommand::CredStart), "100");
        assert_eq!(state.accounting_macro(&MacroCommand::CredUsed), "25");
        state.session.accounting.options.use_money = true;
        assert_eq!(state.accounting_macro(&MacroCommand::CredNow), "$5.00");
        state.session.accounting.mode = AccountingMode::Tracking;
        let unlimited = state.unlimited_text();
        assert_eq!(state.accounting_macro(&MacroCommand::CredLeft), unlimited);
        state.session.accounting.mode = AccountingMode::Disabled;
        assert_eq!(state.accounting_macro(&MacroCommand::CredStart), unlimited);
        assert_eq!(state.accounting_macro(&MacroCommand::CredNow), "0");
        assert_eq!(state.accounting_macro(&MacroCommand::CredUsed), "0");
    }

    #[tokio::test]
    async fn budget_caps_unlimited_time_but_never_extends_an_event_limit() {
        let (_root, mut state, _peer) = fixture().await;
        state.get_board().await.config.accounting.accounting_config.as_mut().unwrap().charge_per_time = 10.0;
        state
            .get_board()
            .await
            .config
            .accounting
            .accounting_config
            .as_mut()
            .unwrap()
            .charge_per_peak_time = 10.0;
        state.accounting_start().await.unwrap();
        state.session.current_conference.charge_time = 10.0;
        state.session.time_limit = 0;
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.time_limit, 5, "balance divided by global plus conference rate");
        state.session.time_adjusted_for_event = true;
        state.session.time_limit = 2;
        state.accounting_check_balance().await.unwrap();
        assert_eq!(state.session.time_limit, 2);
    }

    #[tokio::test]
    async fn input_checks_accounting_before_returning_stuffed_keys() {
        let (_root, mut state, _peer) = fixture().await;
        state.accounting_start().await.unwrap();
        state.accounting_record(4, "READ", "", 101.0, 1).unwrap();
        state.char_buffer.push_back(super::super::KeyChar::new(super::super::KeySource::User, 'X'));
        let key = state.get_char(crate::vm::TerminalTarget::Both).await.unwrap();
        assert_eq!(key.unwrap().ch, 'X');
        assert_eq!(state.session.cur_security, 0);
    }

    #[tokio::test]
    async fn two_nodes_merge_debits_and_credits_without_replaying_persisted_deltas() {
        let (_root, mut first, _peer) = fixture().await;
        let (_root2, mut second, _peer2) = fixture().await;
        second.board = first.board.clone();
        first.accounting_start().await.unwrap();
        second.accounting_start().await.unwrap();
        first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
        second.accounting_record(4, "READ", "", 5.0, 1).unwrap();
        second.accounting_record(16, "CREDIT", "", 4.0, 1).unwrap();
        first.persist_current_user().await.unwrap();
        second.persist_current_user().await.unwrap();
        first.persist_current_user().await.unwrap();
        second.persist_current_user().await.unwrap();
        assert_eq!(account(&first).start_this_session, 100.0);
        assert_eq!(account(&second).start_this_session, 100.0);
        assert_eq!(first.accounting_macro(&MacroCommand::CredNow), "2");
        assert_eq!(second.accounting_macro(&MacroCommand::CredNow), "1");
        let board = first.get_board().await;
        let merged = board.users[0].account.as_ref().unwrap();
        assert_eq!(merged.debit_msg_read, 7.0);
        assert_eq!(merged.credit_special, 4.0);
        assert_eq!(merged.balance(false, 0.0), 97.0);
    }

    #[test]
    fn merge_covers_balance_fields_and_only_overrides_sec_drop_if_locally_changed() {
        let baseline = AccountUserInf {
            starting_balance: 100.0,
            start_this_session: 80.0,
            drop_sec_level: 5,
            ..Default::default()
        };
        let mut local = baseline.clone();
        local.starting_balance += 10.0;
        local.start_this_session += 3.0;
        let mut latest = baseline.clone();
        latest.starting_balance += 20.0;
        latest.start_this_session += 2.0;
        latest.drop_sec_level = 7;
        let merged = merge_account(Some(&local), Some(&latest), Some(&baseline)).unwrap().unwrap();
        assert_eq!(merged.starting_balance, 130.0);
        assert_eq!(merged.start_this_session, 85.0);
        assert_eq!(merged.drop_sec_level, 7);
        local.drop_sec_level = 9;
        assert_eq!(merge_account(Some(&local), Some(&latest), Some(&baseline)).unwrap().unwrap().drop_sec_level, 9);
        assert_eq!(merge_account(None, Some(&latest), None).unwrap(), Some(latest));
    }
}
