use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use subtle::ConstantTimeEq;

pub const SESSION_COOKIE: &str = "icbadmin_session";
const SESSION_TTL: Duration = Duration::from_secs(8 * 60 * 60);

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("no secure random source available");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

struct Session {
    csrf: String,
    expires: Instant,
}

pub struct AuthState {
    token: String,
    sessions: Mutex<HashMap<String, Session>>,
}

/// How a request proved that it is allowed to talk to the admin service.
pub enum Principal {
    /// `Authorization: Bearer <token>` - no CSRF risk, browsers cannot set it cross-site.
    Token,
    /// Session cookie - mutations additionally require the CSRF token of that session.
    Session { csrf: String },
}

impl AuthState {
    pub fn new(token: String) -> Self {
        Self {
            token,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn verify_token(&self, presented: &str) -> bool {
        self.token.as_bytes().ct_eq(presented.as_bytes()).into()
    }

    pub fn create_session(&self) -> String {
        let id = random_hex(32);
        let csrf = random_hex(32);
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|_, s| s.expires > Instant::now());
            sessions.insert(
                id.clone(),
                Session {
                    csrf,
                    expires: Instant::now() + SESSION_TTL,
                },
            );
        }
        id
    }

    pub fn destroy_session(&self, id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(id);
        }
    }

    pub fn session_csrf(&self, id: &str) -> Option<String> {
        let sessions = self.sessions.lock().ok()?;
        let session = sessions.get(id)?;
        if session.expires <= Instant::now() {
            return None;
        }
        Some(session.csrf.clone())
    }

    pub fn verify_csrf(csrf: &str, presented: &str) -> bool {
        csrf.as_bytes().ct_eq(presented.as_bytes()).into()
    }
}

pub fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key.trim() == name { Some(value.trim()) } else { None }
    })
}
