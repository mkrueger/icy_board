//! Normal-login-only recovery. Proof is not a logged-in session or a permanent password.
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox, transport::smtp::authentication::Credentials};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;

use super::{
    IcyBoard,
    icb_config::PasswordStorageMethod,
    user_base::{Password, PasswordVerdict, User},
};
use crate::Res;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PasswordRecoveryConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    /// False means mandatory STARTTLS, never opportunistic TLS.
    pub implicit_tls: bool,
    pub sender: String,
    pub smtp_username: String,
    pub smtp_password_env: String,
    pub ttl_minutes: u32,
    pub cooldown_minutes: u32,
    pub account_per_hour: u32,
    pub board_per_hour: u32,
    pub max_attempts: u32,
    pub timeout_seconds: u32,
}

impl Default for PasswordRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_host: String::new(),
            smtp_port: 587,
            implicit_tls: false,
            sender: String::new(),
            smtp_username: String::new(),
            smtp_password_env: String::new(),
            ttl_minutes: 30,
            cooldown_minutes: 10,
            account_per_hour: 3,
            board_per_hour: 50,
            max_attempts: 5,
            timeout_seconds: 15,
        }
    }
}

impl PasswordRecoveryConfig {
    pub fn validate(&self, storage: PasswordStorageMethod) -> Result<(), &'static str> {
        if !self.enabled {
            return Ok(());
        }
        if storage == PasswordStorageMethod::PlainText {
            return Err("Password recovery requires Argon2 or BCrypt password storage");
        }
        if self.smtp_host.is_empty()
            || self.smtp_host.len() > 253
            || self.smtp_host.contains(|c: char| c.is_whitespace() || c.is_control())
            || self.smtp_port == 0
            || mailbox(&self.sender).is_none()
        {
            return Err("Password recovery requires a SMTP host, port and single sender mailbox");
        }
        if self.smtp_username.len() > 254
            || self.smtp_username.contains(['\r', '\n'])
            || (!self.smtp_username.is_empty()
                && (self.smtp_password_env.is_empty() || !self.smtp_password_env.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')))
        {
            return Err("SMTP credentials must reference a password environment variable");
        }
        if !(1..=60).contains(&self.ttl_minutes)
            || !(1..=60).contains(&self.cooldown_minutes)
            || !(1..=10).contains(&self.account_per_hour)
            || !(1..=500).contains(&self.board_per_hour)
            || !(1..=10).contains(&self.max_attempts)
            || !(1..=30).contains(&self.timeout_seconds)
        {
            return Err("Password recovery limits are outside their safe ranges");
        }
        Ok(())
    }
}

fn mailbox(value: &str) -> Option<Mailbox> {
    if value.len() > 254 || value.contains(['\r', '\n']) {
        return None;
    }
    // Accept only the stored address, not a display name or mailbox list.
    value.parse::<lettre::Address>().ok().map(|address| Mailbox::new(None, address))
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryChallenge {
    // Persisted challenges start in the new process generation after a restart.
    #[serde(skip)]
    runtime_generation: u64,
    id: String,
    hash: Password,
    context: String,
    revision: u64,
    issued: DateTime<Utc>,
    expires: DateTime<Utc>,
    attempts: u32,
}

impl std::fmt::Debug for RecoveryChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryChallenge(<redacted>)")
    }
}

/// Compare records, not Password::PartialEq (which verifies plaintext against hashes).
pub fn security_fingerprint(user: &User) -> String {
    let record = security_record(user);
    format!("{:x}", Sha256::digest(record.as_bytes()))
}

fn security_record(user: &User) -> String {
    #[derive(Serialize)]
    struct Security<'a> {
        name: &'a str,
        alias: &'a str,
        email: &'a str,
        password: &'a super::user_base::PasswordInfo,
        level: u8,
        expired_level: u8,
        deleted: bool,
        disabled: bool,
    }
    toml::to_string(&Security {
        name: &user.name,
        alias: &user.alias,
        email: &user.email,
        password: &user.password,
        level: user.security_level,
        expired_level: user.exp_security_level,
        deleted: user.flags.delete_flag,
        disabled: user.flags.disabled_flag,
    })
    .expect("security record serialization")
}

pub fn normalize_security(user: &mut User) {
    let stamp = security_fingerprint(user);
    if user.security_stamp != stamp {
        user.credential_revision = user.credential_revision.saturating_add(1);
        user.recovery = None;
        user.security_stamp = stamp;
    }
}

/// Preserve authoritative security fields during unrelated saves from older nodes.
pub fn merge_security(local: &User, baseline: &User, live: &User, merged: &mut User) -> Res<()> {
    let changed = security_fingerprint(local) != security_fingerprint(baseline);
    let stale = baseline.credential_revision != live.credential_revision || security_fingerprint(baseline) != security_fingerprint(live);
    if changed && stale {
        return Err("Credentials changed on another node; relogin required".into());
    }
    if !changed {
        merged.name = live.name.clone();
        merged.alias = live.alias.clone();
        merged.email = live.email.clone();
        merged.password = live.password.clone();
        merged.security_level = live.security_level;
        merged.exp_security_level = live.exp_security_level;
        merged.flags.delete_flag = live.flags.delete_flag;
        merged.flags.disabled_flag = live.flags.disabled_flag;
    }
    merged.credential_revision = live.credential_revision;
    merged.security_stamp = live.security_stamp.clone();
    merged.recovery = live.recovery.clone();
    merged.recovery_issues = live.recovery_issues.clone();
    normalize_security(merged);
    Ok(())
}

fn eligible(board: &IcyBoard, index: usize) -> bool {
    let c = &board.config;
    c.password_recovery.enabled
        && c.password_recovery.validate(c.system_control.password_storage_method).is_ok()
        && index != 0
        && board.users.get(index).is_some_and(|u| {
            !u.flags.delete_flag
                && !u.flags.disabled_flag
                && u.security_level < c.sysop_command_level.sysop
                && u.exp_security_level < c.sysop_command_level.sysop
                && !u.password.password.is_empty()
                && matches!(u.password.password, Password::Argon2(_) | Password::BCrypt(_))
                && mailbox(&u.email).is_some()
        })
}

fn usable(user: &User, challenge: &RecoveryChallenge, now: DateTime<Utc>, max_attempts: u32) -> bool {
    matches!(challenge.hash, Password::Argon2(_))
        && challenge.issued <= now
        && now < challenge.expires
        && challenge.expires <= challenge.issued + chrono::Duration::minutes(60)
        && challenge.attempts < max_attempts
        && challenge.revision == user.credential_revision
        && challenge.context == security_fingerprint(user)
}

fn random_secret() -> Res<String> {
    // 32 symbols, so masking is unbiased: 12 characters carry 60 random bits.
    const ALPHABET: &[u8; 32] = b"ABCDEFGHJKMNPQRSTVWXYZ23456789!?";
    let mut bytes = [0u8; 12];
    OsRng.try_fill_bytes(&mut bytes).map_err(|_| "Recovery randomness unavailable")?;
    Ok(bytes.iter().map(|b| ALPHABET[(b & 31) as usize] as char).collect())
}

fn challenge_id() -> Res<String> {
    let mut bytes = [0u8; 16];
    OsRng.try_fill_bytes(&mut bytes).map_err(|_| "Recovery randomness unavailable")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[async_trait]
pub trait MailSender: Send + Sync {
    /// Errors must be categories only: never return a body, address or SMTP credential.
    async fn send(&self, config: &PasswordRecoveryConfig, to: &str, subject: &str, body: String) -> Result<(), ()>;
}

pub struct SmtpMailSender;

#[async_trait]
impl MailSender for SmtpMailSender {
    async fn send(&self, config: &PasswordRecoveryConfig, to: &str, subject: &str, body: String) -> Result<(), ()> {
        let message = Message::builder()
            .from(mailbox(&config.sender).ok_or(())?)
            .to(mailbox(to).ok_or(())?)
            .subject(subject)
            .body(body)
            .map_err(|_| ())?;
        let builder = if config.implicit_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
        }
        .map_err(|_| ())?;
        let mut builder = builder.port(config.smtp_port).timeout(Some(Duration::from_secs(config.timeout_seconds.into())));
        if !config.smtp_username.is_empty() {
            let password = std::env::var(&config.smtp_password_env).map_err(|_| ())?;
            builder = builder.credentials(Credentials::new(config.smtp_username.clone(), password));
        }
        builder.build().send(message).await.map(|_| ()).map_err(|_| ())
    }
}

pub struct RecoveryService {
    sender: Arc<dyn MailSender>,
    generation: AtomicU64,
    /// Reject overload rather than queue an unbounded collection of hash jobs.
    slots: Arc<Semaphore>,
    /// Slow mail delivery must not occupy the authentication workers.
    authentication_slots: Arc<Semaphore>,
    issues: Mutex<Vec<DateTime<Utc>>>,
}

impl Default for RecoveryService {
    fn default() -> Self {
        Self::new(Arc::new(SmtpMailSender))
    }
}

/// Opaque capability, bound to the challenge and security context, not the user index alone.
pub struct RecoveryProof {
    index: usize,
    challenge: RecoveryChallenge,
}

pub enum LoginPassword {
    Permanent,
    Temporary(RecoveryProof),
    Invalid,
}

impl RecoveryService {
    pub fn new(sender: Arc<dyn MailSender>) -> Self {
        Self {
            sender,
            generation: AtomicU64::new(0),
            slots: Arc::new(Semaphore::new(2)),
            authentication_slots: Arc::new(Semaphore::new(2)),
            issues: Mutex::new(Vec::new()),
        }
    }

    pub fn revoke_runtime_challenges(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn is_revoked(&self, challenge: &RecoveryChallenge) -> bool {
        challenge.runtime_generation != self.generation.load(Ordering::SeqCst)
    }

    pub async fn issue(&self, board: &Arc<tokio::sync::Mutex<IcyBoard>>, index: usize, now: DateTime<Utc>) -> Res<bool> {
        let Ok(slot) = self.slots.clone().try_acquire_owned() else {
            return Ok(false);
        };
        let generation = self.generation.load(Ordering::SeqCst);
        let (config, user, board_name) = {
            let mut b = board.lock().await;
            if !eligible(&b, index) {
                return Ok(false);
            }
            let config = b.config.password_recovery.clone();
            let previous = b.users[index].clone();
            let user = &mut b.users[index];
            normalize_security(user);
            if user
                .recovery_issues
                .iter()
                .any(|t| *t > now || *t + chrono::Duration::minutes(config.cooldown_minutes.into()) > now)
            {
                return Ok(false);
            }
            user.recovery_issues.retain(|t| *t + chrono::Duration::hours(1) > now);
            if user.recovery_issues.len() >= config.account_per_hour as usize {
                return Ok(false);
            }
            {
                let mut issues = self.issues.lock().unwrap();
                issues.retain(|t| *t + chrono::Duration::hours(1) > now);
                // Persisted account histories also enforce the board limit after restart.
                let persisted = b
                    .users
                    .iter()
                    .flat_map(|u| &u.recovery_issues)
                    .filter(|t| **t + chrono::Duration::hours(1) > now)
                    .count();
                if issues.iter().any(|t| *t > now) || issues.len().max(persisted) >= config.board_per_hour as usize {
                    return Ok(false);
                }
                issues.push(now);
            }
            b.users[index].recovery_issues.push(now);
            if let Err(e) = b.save_userbase() {
                b.users[index] = previous;
                return Err(e);
            }
            (config, b.users[index].clone(), b.config.board.name.clone())
        };
        let secret = random_secret()?;
        let plain = secret.clone();
        let (hash, slot) = tokio::task::spawn_blocking(move || (Password::new_argon2(plain), slot)).await?;
        let challenge = RecoveryChallenge {
            runtime_generation: generation,
            id: challenge_id()?,
            hash,
            context: security_fingerprint(&user),
            revision: user.credential_revision,
            issued: now,
            expires: now + chrono::Duration::minutes(config.ttl_minutes.into()),
            attempts: 0,
        };
        {
            let mut b = board.lock().await;
            if self.generation.load(Ordering::SeqCst) != generation
                || !eligible(&b, index)
                || b.config.password_recovery != config
                || security_fingerprint(&b.users[index]) != challenge.context
                || b.users[index].credential_revision != challenge.revision
            {
                return Ok(false);
            }
            let previous = b.users[index].clone();
            b.users[index].recovery = Some(challenge.clone());
            if let Err(e) = b.save_userbase() {
                b.users[index] = previous;
                return Err(e);
            }
        }
        let german = user.language.to_ascii_lowercase().starts_with("de");
        let body = if german {
            format!(
                "{board_name}\n\nHallo {},\nIhr temporaeres Passwort: {secret}\nGueltig fuer {} Minuten. Melden Sie sich normal am BBS an und waehlen Sie ein neues Passwort. Danach erneut anmelden.\nFalls nicht angefordert, ignorieren Sie diese E-Mail. Ihr bisheriges Passwort bleibt gueltig.\n",
                user.name, config.ttl_minutes
            )
        } else {
            format!(
                "{board_name}\n\nHello {},\nYour temporary password: {secret}\nValid for {} minutes. Log in normally to the BBS and choose a new password, then log in again.\nIf you did not request this email, ignore it. Your existing password remains valid.\n",
                user.name, config.ttl_minutes
            )
        };
        // Awaited by the caller: no detached SMTP jobs, retry spool or startup sends.
        let result = tokio::time::timeout(
            Duration::from_secs(config.timeout_seconds.into()),
            self.sender.send(
                &config,
                &user.email,
                if german { "BBS temporaeres Passwort" } else { "BBS temporary password" },
                body,
            ),
        )
        .await;
        drop(slot);
        if matches!(result, Ok(Err(()))) {
            let mut b = board.lock().await;
            if b.users.get(index).and_then(|u| u.recovery.as_ref()).is_some_and(|c| c.id == challenge.id) {
                let previous = b.users[index].clone();
                b.users[index].recovery = None;
                if let Err(e) = b.save_userbase() {
                    b.users[index] = previous;
                    return Err(e);
                }
            }
        }
        Ok(matches!(result, Ok(Ok(()))))
    }

    pub async fn verify(&self, board: &Arc<tokio::sync::Mutex<IcyBoard>>, index: usize, candidate: String, now: DateTime<Utc>) -> Res<LoginPassword> {
        let started = std::time::Instant::now();
        let slot = self.authentication_slots.clone().acquire_owned().await?;
        let (user, challenge) = {
            let mut b = board.lock().await;
            let Some(user) = b.users.get(index) else {
                return Ok(LoginPassword::Invalid);
            };
            if user.flags.disabled_flag || user.flags.delete_flag {
                return Ok(LoginPassword::Invalid);
            }
            let user = user.clone();
            let challenge = if eligible(&b, index) {
                user.recovery.clone().filter(|c| {
                    c.runtime_generation == self.generation.load(Ordering::SeqCst) && usable(&user, c, now, b.config.password_recovery.max_attempts)
                })
            } else {
                None
            };
            // Reserve an attempt before hashing so parallel nodes share the same finite budget.
            if challenge.is_some() {
                b.users[index].recovery.as_mut().unwrap().attempts += 1;
                if let Err(e) = b.save_userbase() {
                    b.users[index] = user;
                    return Err(e);
                }
            }
            (user, challenge)
        };
        let snapshot = user.clone();
        let proof_challenge = challenge.clone();
        let (normal, temporary, _slot) = tokio::task::spawn_blocking(move || {
            (
                snapshot.password.password.is_valid(&candidate),
                proof_challenge.is_some_and(|c| c.hash.is_valid(&candidate)),
                slot,
            )
        })
        .await?;
        let mut b = board.lock().await;
        let Some(live) = b.users.get(index) else {
            return Ok(LoginPassword::Invalid);
        };
        if security_fingerprint(live) != security_fingerprint(&user) || live.credential_revision != user.credential_revision {
            return Ok(LoginPassword::Invalid);
        }
        if normal {
            if live.recovery.is_some() {
                let previous = live.clone();
                b.users[index].recovery = None;
                if let Err(e) = b.save_userbase() {
                    b.users[index] = previous;
                    return Err(e);
                }
            }
            return Ok(LoginPassword::Permanent);
        }
        if temporary && eligible(&b, index) {
            let c = challenge.unwrap();
            let finished = now + chrono::Duration::from_std(started.elapsed()).unwrap_or_default();
            if usable(live, &c, finished, b.config.password_recovery.max_attempts)
                && live
                    .recovery
                    .as_ref()
                    .is_some_and(|current| current.id == c.id && current.runtime_generation == self.generation.load(Ordering::SeqCst))
            {
                return Ok(LoginPassword::Temporary(RecoveryProof { index, challenge: c }));
            }
        }
        Ok(LoginPassword::Invalid)
    }

    pub async fn complete(&self, board: &Arc<tokio::sync::Mutex<IcyBoard>>, proof: &RecoveryProof, candidate: String, now: DateTime<Utc>) -> Res<bool> {
        let started = std::time::Instant::now();
        let slot = self.authentication_slots.clone().acquire_owned().await?;
        let (user, config) = {
            let b = board.lock().await;
            if !proof_valid(&b, proof, now) {
                return Ok(false);
            }
            (b.users[proof.index].clone(), b.config.clone())
        };
        if candidate.is_empty() || candidate.len() > 12 {
            return Ok(false);
        }
        let context = security_fingerprint(&user);
        let challenge = proof.challenge.clone();
        let (password, _slot) = tokio::task::spawn_blocking(move || {
            if user.password.check_new_password(&user.name, &candidate, config.limits.min_pwd_length) != PasswordVerdict::Ok
                || challenge.hash.is_valid(&candidate)
            {
                return (None, slot);
            }
            let hash =
                if matches!(user.password.password, Password::Argon2(_)) || config.system_control.password_storage_method == PasswordStorageMethod::Argon2 {
                    Password::new_argon2(candidate)
                } else {
                    Password::new_bcrypt(candidate)
                };
            (Some(hash), slot)
        })
        .await?;
        let Some(password) = password else {
            return Ok(false);
        };
        let mut b = board.lock().await;
        let now = now + chrono::Duration::from_std(started.elapsed()).unwrap_or_default();
        if !proof_valid(&b, proof, now)
            || security_fingerprint(&b.users[proof.index]) != context
            || b.config.limits.min_pwd_length != config.limits.min_pwd_length
            || b.config.system_control.password_storage_method != config.system_control.password_storage_method
        {
            return Ok(false);
        }
        let previous = b.users[proof.index].clone();
        let expire_days = b.config.limits.password_expire_days;
        b.users[proof.index].password.accept_new_password(password, now, expire_days);
        b.users[proof.index].recovery = None;
        // One authoritative atomic file replacement publishes both credential and consumption.
        if let Err(e) = b.save_userbase() {
            b.users[proof.index] = previous;
            return Err(e);
        }
        Ok(true)
    }
}

fn proof_valid(board: &IcyBoard, proof: &RecoveryProof, now: DateTime<Utc>) -> bool {
    eligible(board, proof.index)
        && board.users[proof.index].recovery.as_ref().is_some_and(|c| {
            c.runtime_generation == board.password_recovery_service.generation.load(Ordering::SeqCst)
                && c.id == proof.challenge.id
                && usable(&board.users[proof.index], &proof.challenge, now, board.config.password_recovery.max_attempts)
        })
}

impl super::state::IcyBoardState {
    pub async fn credentials_still_current(&self) -> bool {
        let Some((revision, stamp)) = &self.session.authenticated_security else {
            return true;
        };
        let board = self.get_board().await;
        board.users.get(self.session.cur_user_id as usize).is_some_and(|live| {
            !live.flags.disabled_flag && !live.flags.delete_flag && live.credential_revision == *revision && security_fingerprint(live) == *stamp
        })
    }

    /// Close the pre-authentication snapshot race without interpreting a recovery hash as a password.
    pub async fn authorize_normal_login(&mut self) -> Res<bool> {
        let Some(baseline) = self.session.security_baseline.as_ref() else {
            return Ok(false);
        };
        let mut board = self.get_board().await;
        let index = self.session.cur_user_id as usize;
        let Some(live) = board.users.get(index) else {
            return Ok(false);
        };
        if live.flags.disabled_flag
            || live.flags.delete_flag
            || security_fingerprint(live) != security_fingerprint(baseline)
            || live.credential_revision != baseline.credential_revision
        {
            return Ok(false);
        }
        if live.recovery.is_some() {
            let previous = live.clone();
            board.users[index].recovery = None;
            if let Err(e) = board.save_userbase() {
                board.users[index] = previous;
                return Err(e);
            }
        }
        let security = (board.users[index].credential_revision, security_fingerprint(&board.users[index]));
        drop(board);
        self.session.authenticated_security = Some(security);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::IcyBoardSerializer;
    use super::*;

    #[derive(Default)]
    struct Capture {
        bodies: Mutex<Vec<String>>,
        fail: bool,
    }
    #[async_trait]
    impl MailSender for Capture {
        async fn send(&self, _: &PasswordRecoveryConfig, to: &str, _: &str, body: String) -> Result<(), ()> {
            assert_eq!(to, "caller@example.invalid");
            self.bodies.lock().unwrap().push(body);
            if self.fail { Err(()) } else { Ok(()) }
        }
    }

    fn fixture_with(sender: Arc<dyn MailSender>) -> (tempfile::TempDir, Arc<tokio::sync::Mutex<IcyBoard>>, Arc<RecoveryService>) {
        let dir = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::new();
        board.root_path = dir.path().to_path_buf();
        board.file_name = dir.path().join("board.toml");
        board.config.paths.user_file = dir.path().join("users.toml");
        board.config.paths.conferences = dir.path().join("conferences.toml");
        board.config.paths.ftn_file.clear();
        board.config.paths.qwknet_file.clear();
        board.config.paths.zconnect_file.clear();
        board.config.system_control.password_storage_method = PasswordStorageMethod::Argon2;
        board.config.password_recovery = PasswordRecoveryConfig {
            enabled: true,
            smtp_host: "smtp.example.invalid".into(),
            sender: "bbs@example.invalid".into(),
            ..Default::default()
        };
        board.users.new_user(User::default());
        let mut user = User {
            name: "Test Caller".into(),
            email: "caller@example.invalid".into(),
            security_level: 10,
            ..Default::default()
        };
        user.password.password = Password::new_argon2("old-secret");
        board.users.new_user(user);
        let service = Arc::new(RecoveryService::new(sender));
        board.password_recovery_service = service.clone();
        board.save_userbase().unwrap();
        (dir, Arc::new(tokio::sync::Mutex::new(board)), service)
    }

    fn fixture() -> (tempfile::TempDir, Arc<tokio::sync::Mutex<IcyBoard>>, Arc<RecoveryService>, Arc<Capture>) {
        let capture = Arc::new(Capture::default());
        let (dir, board, service) = fixture_with(capture.clone());
        (dir, board, service, capture)
    }

    fn secret(capture: &Capture) -> String {
        capture
            .bodies
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("Your temporary password: "))
            .unwrap()
            .into()
    }

    async fn proof(service: &RecoveryService, board: &Arc<tokio::sync::Mutex<IcyBoard>>, secret: String, now: DateTime<Utc>) -> RecoveryProof {
        match service.verify(board, 1, secret, now).await.unwrap() {
            LoginPassword::Temporary(proof) => proof,
            _ => panic!("expected restricted proof"),
        }
    }

    #[test]
    fn configuration_is_off_for_old_files_and_rejects_unsafe_settings() {
        let default: PasswordRecoveryConfig = toml::from_str("").unwrap();
        assert!(!default.enabled);
        assert_eq!(default.ttl_minutes, 30);
        let mut config = default;
        config.enabled = true;
        assert!(config.validate(PasswordStorageMethod::Argon2).is_err());
        config.smtp_host = "smtp.example.invalid".into();
        config.sender = "bbs@example.invalid".into();
        assert!(config.validate(PasswordStorageMethod::Argon2).is_ok());
        assert!(config.validate(PasswordStorageMethod::PlainText).is_err());
        config.sender = "bbs@example.invalid\r\nBcc: other@example.invalid".into();
        assert!(config.validate(PasswordStorageMethod::Argon2).is_err());
        let old_config = toml::to_string(&super::super::icb_config::IcbConfig::new()).unwrap();
        let mut table: toml::Table = toml::from_str(&old_config).unwrap();
        table.remove("password_recovery");
        let decoded: super::super::icb_config::IcbConfig = table.try_into().unwrap();
        assert!(!decoded.password_recovery.enabled);
    }

    #[test]
    fn generated_secret_is_uniform_mask_compatible_and_case_insensitive_alphabet() {
        for _ in 0..128 {
            let value = random_secret().unwrap();
            assert_eq!(value.len(), 12);
            assert!(value.chars().all(|c| super::super::state::functions::MASK_PASSWORD.contains(c)));
            assert!(!value.contains(['I', 'L', 'O', 'U', '0', '1']));
        }
    }

    #[tokio::test]
    async fn hash_only_reset_is_atomic_replay_safe_and_survives_reload() {
        let (dir, board, service, capture) = fixture();
        let now = Utc::now();
        assert!(service.issue(&board, 1, now).await.unwrap());
        let temporary = secret(&capture);
        let file = std::fs::read_to_string(dir.path().join("users.toml")).unwrap();
        assert!(!file.contains(&temporary));
        assert!(!file.contains(&temporary.to_lowercase()));
        assert!(file.contains("$argon2"));
        {
            let mut b = board.lock().await;
            assert!(b.users[1].password.password.is_valid("old-secret"));
            assert!(!b.users[1].password.password.is_valid(&temporary));
            assert!(!format!("{:?}", b.users[1].recovery).contains(&temporary));
            b.users.export_pcboard(&dir.path().join("USERS"), &dir.path().join("USERS.INF")).unwrap();
            assert!(!String::from_utf8_lossy(&std::fs::read(dir.path().join("USERS")).unwrap()).contains(&temporary));
            b.users = super::super::user_base::UserBase::load(&dir.path().join("users.toml")).unwrap();
        }
        let proof = proof(&service, &board, temporary.clone(), now).await;
        assert!(!service.complete(&board, &proof, temporary, now).await.unwrap());
        assert!(!service.complete(&board, &proof, "old-secret".into(), now).await.unwrap());
        assert!(service.complete(&board, &proof, "brand-new".into(), now).await.unwrap());
        assert!(!service.complete(&board, &proof, "other-new".into(), now).await.unwrap());
        let users = super::super::user_base::UserBase::load(&dir.path().join("users.toml")).unwrap();
        assert!(users[1].recovery.is_none());
        assert!(users[1].password.password.is_valid("brand-new"));
        assert!(!users[1].password.password.is_valid("old-secret"));
        assert!(users[1].password.prev_pwd.iter().all(|p| matches!(p, Password::Argon2(_))));
    }

    #[tokio::test]
    async fn attempt_budget_and_ttl_are_authoritative() {
        let (_dir, board, service, capture) = fixture();
        let now = Utc::now();
        service.issue(&board, 1, now).await.unwrap();
        let secret = secret(&capture);
        assert!(matches!(
            service.verify(&board, 1, secret.clone(), now - chrono::Duration::seconds(1)).await.unwrap(),
            LoginPassword::Invalid
        ));
        assert!(matches!(
            service.verify(&board, 1, secret.clone(), now + chrono::Duration::minutes(30)).await.unwrap(),
            LoginPassword::Invalid
        ));
        for _ in 0..5 {
            assert!(matches!(service.verify(&board, 1, "wrong".into(), now).await.unwrap(), LoginPassword::Invalid));
        }
        assert!(matches!(service.verify(&board, 1, secret, now).await.unwrap(), LoginPassword::Invalid));
        assert_eq!(board.lock().await.users[1].recovery.as_ref().unwrap().attempts, 5);
        assert!(matches!(
            service.verify(&board, 1, "old-secret".into(), now).await.unwrap(),
            LoginPassword::Permanent
        ));
        assert!(board.lock().await.users[1].recovery.is_none());
    }

    #[tokio::test]
    async fn issuance_limits_persist_and_overload_does_not_send() {
        let (_dir, board, service, capture) = fixture();
        let now = Utc::now();
        assert!(service.issue(&board, 1, now).await.unwrap());
        assert!(!service.issue(&board, 1, now).await.unwrap());
        assert!(!service.issue(&board, 1, now - chrono::Duration::minutes(1)).await.unwrap());
        for minutes in [10, 20] {
            assert!(service.issue(&board, 1, now + chrono::Duration::minutes(minutes)).await.unwrap());
        }
        let restarted = RecoveryService::new(capture.clone());
        assert!(!restarted.issue(&board, 1, now + chrono::Duration::minutes(30)).await.unwrap());
        let one = service.slots.clone().acquire_owned().await.unwrap();
        let two = service.slots.clone().acquire_owned().await.unwrap();
        assert!(!service.issue(&board, 1, now + chrono::Duration::hours(2)).await.unwrap());
        drop((one, two));
        assert_eq!(capture.bodies.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn saturated_mail_delivery_does_not_block_permanent_login() {
        let (_dir, board, service, _capture) = fixture();
        let one = service.slots.clone().acquire_owned().await.unwrap();
        let two = service.slots.clone().acquire_owned().await.unwrap();
        assert!(matches!(
            service.verify(&board, 1, "old-secret".into(), Utc::now()).await.unwrap(),
            LoginPassword::Permanent
        ));
        drop((one, two));
    }

    #[tokio::test]
    async fn german_mail_and_last_allowed_attempt_complete_with_bcrypt_policy() {
        let (_dir, board, service, capture) = fixture();
        {
            let mut b = board.lock().await;
            b.config.system_control.password_storage_method = PasswordStorageMethod::BCrypt;
            b.users[1].password.password = Password::new_bcrypt("old-secret");
            b.users[1].language = "de".into();
            b.save_userbase().unwrap();
        }
        let now = Utc::now();
        assert!(service.issue(&board, 1, now).await.unwrap());
        let temporary = {
            let bodies = capture.bodies.lock().unwrap();
            let body = bodies.last().unwrap();
            assert!(body.contains("Ihr bisheriges Passwort bleibt gueltig."));
            assert!(!body.contains("Your temporary password"));
            body.lines()
                .find_map(|line| line.strip_prefix("Ihr temporaeres Passwort: "))
                .unwrap()
                .to_string()
        };
        for _ in 0..4 {
            assert!(matches!(service.verify(&board, 1, "wrong".into(), now).await.unwrap(), LoginPassword::Invalid));
        }
        let proof = proof(&service, &board, temporary.to_lowercase(), now).await;
        assert_eq!(board.lock().await.users[1].recovery.as_ref().unwrap().attempts, 5);
        assert!(service.complete(&board, &proof, "brand-new".into(), now).await.unwrap());
        let b = board.lock().await;
        assert!(matches!(b.users[1].password.password, Password::BCrypt(_)));
        assert!(b.users[1].password.password.is_valid("BRAND-NEW"));
        assert!(b.users[1].recovery.is_none());
    }

    #[tokio::test]
    async fn board_limit_survives_service_restart() {
        let (_dir, board, service, capture) = fixture();
        board.lock().await.config.password_recovery.board_per_hour = 1;
        let now = Utc::now();
        assert!(service.issue(&board, 1, now).await.unwrap());
        let restarted = RecoveryService::new(capture);
        assert!(!restarted.issue(&board, 1, now + chrono::Duration::minutes(10)).await.unwrap());
    }

    #[tokio::test]
    async fn ineligible_users_never_send_and_cannot_complete() {
        let (_dir, board, service, capture) = fixture();
        let now = Utc::now();
        assert!(!service.issue(&board, 0, now).await.unwrap());
        assert!(!service.issue(&board, 999, now).await.unwrap());
        let original = board.lock().await.users[1].clone();
        for mutate in [
            (|u: &mut User| u.flags.disabled_flag = true) as fn(&mut User),
            |u| u.flags.delete_flag = true,
            |u| u.security_level = 100,
            |u| u.email.clear(),
            |u| u.email = "two@example.invalid,another@example.invalid".into(),
            |u| u.password.password = Password::PlainText("old-secret".into()),
        ] {
            {
                let mut b = board.lock().await;
                b.users[1] = original.clone();
                mutate(&mut b.users[1]);
            }
            assert!(!service.issue(&board, 1, now).await.unwrap());
        }
        assert!(capture.bodies.lock().unwrap().is_empty());
        board.lock().await.users[1] = original;
        service.issue(&board, 1, now).await.unwrap();
        let proof = proof(&service, &board, secret(&capture), now).await;
        board.lock().await.users[1].security_level = 100;
        assert!(!service.complete(&board, &proof, "brand-new".into(), now).await.unwrap());
    }

    #[tokio::test]
    async fn disk_failure_before_send_or_commit_preserves_old_password() {
        let (dir, board, service, capture) = fixture();
        let now = Utc::now();
        let good = board.lock().await.config.paths.user_file.clone();
        board.lock().await.config.paths.user_file = dir.path().to_path_buf();
        assert!(service.issue(&board, 1, now).await.is_err());
        assert!(capture.bodies.lock().unwrap().is_empty());
        board.lock().await.config.paths.user_file = good;
        assert!(service.issue(&board, 1, now).await.unwrap());
        let proof = proof(&service, &board, secret(&capture), now).await;
        let before = board.lock().await.users[1].clone();
        board.lock().await.config.paths.user_file = dir.path().to_path_buf();
        assert!(service.complete(&board, &proof, "brand-new".into(), now).await.is_err());
        let b = board.lock().await;
        assert_eq!(security_fingerprint(&before), security_fingerprint(&b.users[1]));
        assert!(b.users[1].recovery.is_some());
        assert!(b.users[1].password.password.is_valid("old-secret"));
    }

    #[tokio::test]
    async fn smtp_rejection_revokes_only_temporary_credential() {
        let capture = Arc::new(Capture {
            fail: true,
            ..Default::default()
        });
        let (_dir, board, service) = fixture_with(capture);
        assert!(!service.issue(&board, 1, Utc::now()).await.unwrap());
        let b = board.lock().await;
        assert!(b.users[1].recovery.is_none());
        assert!(b.users[1].password.password.is_valid("old-secret"));
        assert_eq!(b.users[1].recovery_issues.len(), 1);
    }

    struct NeverSend;
    #[async_trait]
    impl MailSender for NeverSend {
        async fn send(&self, _: &PasswordRecoveryConfig, _: &str, _: &str, _: String) -> Result<(), ()> {
            std::future::pending().await
        }
    }

    #[tokio::test]
    async fn smtp_deadline_leaves_only_expiring_hash_and_releases_slot() {
        let (_dir, board, service) = fixture_with(Arc::new(NeverSend));
        board.lock().await.config.password_recovery.timeout_seconds = 1;
        assert!(!service.issue(&board, 1, Utc::now()).await.unwrap());
        assert_eq!(service.slots.available_permits(), 2);
        assert!(board.lock().await.users[1].password.password.is_valid("old-secret"));
    }

    #[tokio::test]
    async fn two_proofs_race_only_one_commit_wins() {
        let (_dir, board, service, capture) = fixture();
        let now = Utc::now();
        service.issue(&board, 1, now).await.unwrap();
        let secret = secret(&capture);
        let first = proof(&service, &board, secret.clone(), now).await;
        let second = proof(&service, &board, secret, now).await;
        let (one, two) = tokio::join!(
            service.complete(&board, &first, "first-new".into(), now),
            service.complete(&board, &second, "second-new".into(), now)
        );
        assert_ne!(one.unwrap(), two.unwrap());
        assert!(board.lock().await.users[1].recovery.is_none());
    }

    #[tokio::test]
    async fn stale_profile_cannot_restore_credentials_and_stale_password_change_fails() {
        let (_dir, board, service, capture) = fixture();
        let now = Utc::now();
        let baseline = board.lock().await.users[1].clone();
        service.issue(&board, 1, now).await.unwrap();
        let proof = proof(&service, &board, secret(&capture), now).await;
        assert!(service.complete(&board, &proof, "brand-new".into(), now).await.unwrap());
        let live = board.lock().await.users[1].clone();
        let mut local = baseline.clone();
        local.city = "new city".into();
        let mut merged = local.clone();
        merge_security(&local, &baseline, &live, &mut merged).unwrap();
        assert_eq!(merged.city, "new city");
        assert_eq!(security_fingerprint(&merged), security_fingerprint(&live));
        assert!(merged.recovery.is_none());
        local.password.password = Password::new_argon2("stale-change");
        assert!(merge_security(&local, &baseline, &live, &mut merged).is_err());
    }

    #[tokio::test]
    async fn security_edits_and_disable_reenable_revoke_challenges() {
        let (dir, board, service, capture) = fixture();
        let now = Utc::now();
        service.issue(&board, 1, now).await.unwrap();
        let secret = secret(&capture);
        let challenge_user = board.lock().await.users[1].clone();
        for mutate in [
            (|u: &mut User| u.email = "changed@example.invalid".into()) as fn(&mut User),
            |u| u.name = "Renamed".into(),
            |u| u.flags.disabled_flag = true,
            |u| u.password.password = Password::PlainText("PPE-change".into()),
        ] {
            let mut b = board.lock().await;
            b.users[1] = challenge_user.clone();
            mutate(&mut b.users[1]);
            b.save_userbase().unwrap();
            assert!(b.users[1].recovery.is_none());
            assert!(b.users[1].credential_revision > challenge_user.credential_revision);
        }
        {
            let mut b = board.lock().await;
            b.users[1] = challenge_user;
            b.config.password_recovery.enabled = false;
            b.save().unwrap();
            b.config.password_recovery.enabled = true;
        }
        assert!(matches!(service.verify(&board, 1, secret, now).await.unwrap(), LoginPassword::Invalid));
        let users = super::super::user_base::UserBase::load(&dir.path().join("users.toml")).unwrap();
        assert!(users[1].recovery.is_none());
    }
}
