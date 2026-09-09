use crate::Res;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone)]
pub enum Password {
    PlainText(String),
    BCrypt(String),
    Argon2(String),
    /// A secret that may be compared but never shown, like a door password a PPE reads.
    Protected(String),
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Password::PlainText(_) => write!(f, "PlainText(******)"),
            Password::BCrypt(_) => write!(f, "BCrypt(******)"),
            Password::Argon2(_) => write!(f, "Argon2(******)"),
            Password::Protected(_) => write!(f, "Protected(******)"),
        }
    }
}

impl PartialEq for Password {
    fn eq(&self, other: &Self) -> bool {
        match (self.unhashed(), other.unhashed()) {
            (Some(left), Some(right)) => left == right,
            (Some(plain), None) => other.verify(plain),
            (None, Some(plain)) => self.verify(plain),
            // A hash carries its own salt, so only an identical bcrypt record matches.
            (None, None) => match (self, other) {
                (Self::BCrypt(l), Self::BCrypt(r)) => l == r,
                _ => false,
            },
        }
    }
}

impl Default for Password {
    fn default() -> Self {
        Password::PlainText(String::new())
    }
}

impl Password {
    pub fn new_plaintext(str: impl Into<String>) -> Res<Self> {
        Ok(Password::PlainText(str.into().to_lowercase()))
    }

    pub fn new_protected(str: impl Into<String>) -> Password {
        Password::Protected(str.into().to_lowercase())
    }

    #[must_use]
    pub fn protected(&self) -> Password {
        match self {
            Password::PlainText(s) => Password::Protected(s.clone()),
            other => other.clone(),
        }
    }

    fn unhashed(&self) -> Option<&str> {
        match self {
            Password::PlainText(s) | Password::Protected(s) => Some(s),
            Password::Argon2(_) | Password::BCrypt(_) => None,
        }
    }

    fn verify(&self, plain: &str) -> bool {
        match self {
            Password::Argon2(hash) => {
                if let Ok(parsed_hash) = PasswordHash::new(hash) {
                    Argon2::default().verify_password(plain.as_bytes(), &parsed_hash).is_ok()
                } else {
                    false
                }
            }
            Password::BCrypt(hash) => bcrypt::verify(plain, hash).unwrap_or(false),
            Password::PlainText(s) | Password::Protected(s) => s == plain,
        }
    }

    pub fn new_argon2(str: impl Into<String>) -> Password {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2.hash_password(str.into().to_lowercase().as_bytes(), &salt).unwrap().to_string();
        Password::Argon2(password_hash)
    }

    pub fn new_bcrypt(str: impl Into<String>) -> Password {
        let hashed = bcrypt::hash(str.into().to_lowercase(), bcrypt::DEFAULT_COST).unwrap();
        Password::BCrypt(hashed)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Password::PlainText(s) | Password::Argon2(s) | Password::BCrypt(s) | Password::Protected(s) => s.is_empty(),
        }
    }

    pub fn is_valid(&self, pwd: &str) -> bool {
        self == &Password::PlainText(pwd.to_lowercase().clone())
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Password::PlainText(s) => write!(f, "{s}"),
            Password::Argon2(_) | Password::BCrypt(_) | Password::Protected(_) => write!(f, "******"),
        }
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|p| {
            if let Some(rest) = p.strip_prefix("bcrypt:") {
                return Password::BCrypt(rest.to_string());
            }
            if p.starts_with("$argon2") {
                Password::Argon2(p)
            } else if p.len() >= 2 && p.starts_with('"') && p.ends_with('"') {
                Password::PlainText(p[1..p.len() - 1].to_string())
            } else {
                Password::PlainText(p)
            }
        })
    }
}

impl Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Password::PlainText(key) | Password::Protected(key) => format!("\"{key}\"").serialize(serializer),
            Password::Argon2(key) => key.serialize(serializer),
            Password::BCrypt(key) => format!("bcrypt:{key}").serialize(serializer),
        }
    }
}

impl FromStr for Password {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Password::PlainText(s.to_string()))
    }
}
