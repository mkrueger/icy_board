//! Leaf ZCONNECT public-text gateway. Private mail and opaque content are retained,
//! never interpreted as public mail. Transport is implemented separately in `poll`.

use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::IcyBoardSerializer;
use crate::Res;

mod packet;
pub mod poll;
pub use packet::{ScanReport, TossReport, acknowledge_outbound, pending_packet, scan, toss};
pub use poll::poll;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ZconnectConfig {
    pub enabled: bool,
    pub local_system: String,
    pub local_user: String,
    pub inbound: PathBuf,
    pub outbound: PathBuf,
    #[serde(rename = "link")]
    pub links: Vec<ZconnectLink>,
}

impl Default for ZconnectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            local_system: String::new(),
            local_user: "sysop".into(),
            inbound: "zconnect/inbound".into(),
            outbound: "zconnect/outbound".into(),
            links: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ZconnectLink {
    pub id: String,
    pub host: String,
    /// Optional expected peer SYS display name, not cryptographic authentication.
    pub remote_system: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub login: String,
    pub timeout_secs: u32,
    #[serde(rename = "area")]
    pub areas: Vec<ZconnectArea>,
}

impl Default for ZconnectLink {
    fn default() -> Self {
        Self {
            id: String::new(),
            host: String::new(),
            remote_system: String::new(),
            port: 23,
            username: String::new(),
            password: String::new(),
            login: "zconnect".into(),
            timeout_secs: 60,
            areas: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ZconnectArea {
    pub remote_board: String,
    pub local_area: PathBuf,
    #[serde(default)]
    pub read_only: bool,
}

impl IcyBoardSerializer for ZconnectConfig {
    const FILE_TYPE: &'static str = "zconnect";
}

impl ZconnectConfig {
    pub fn link(&self, id: &str) -> Option<&ZconnectLink> {
        self.links.iter().find(|link| link.id.eq_ignore_ascii_case(id))
    }

    pub fn validate(&self) -> Res<()> {
        // An untouched, disabled configuration is loadable; operations require an identity.
        if (!self.local_system.is_empty() || self.enabled || !self.links.is_empty()) && !fqdn(&self.local_system) {
            return Err("ZCONNECT local_system must be a fully qualified DNS name".into());
        }
        if !localpart(&self.local_user) {
            return Err("ZCONNECT local_user must be a safe address localpart".into());
        }
        safe_path(&self.inbound)?;
        safe_path(&self.outbound)?;
        if self.inbound == self.outbound {
            return Err("ZCONNECT inbound and outbound must differ".into());
        }
        let mut ids = HashSet::new();
        for link in &self.links {
            if !spool_id(&link.id) || !ids.insert(link.id.to_ascii_lowercase()) {
                return Err(format!("Invalid or duplicate ZCONNECT link id {:?}", link.id).into());
            }
            // Empty hosts allow manual/offline packet exchange.
            if !link.host.is_empty() && !(fqdn(&link.host) || link.host.parse::<std::net::IpAddr>().is_ok() || dns_label(&link.host)) {
                return Err(format!("Invalid ZCONNECT host for {}", link.id).into());
            }
            if link.remote_system.len() > 255 || !link.remote_system.bytes().all(|b| (32..=126).contains(&b)) {
                return Err("ZCONNECT remote_system must be at most 255 printable ASCII bytes without line breaks".into());
            }
            if link.port == 0 || !(1..=3600).contains(&link.timeout_secs) || !matches!(link.login.as_str(), "zconnect" | "janus" | "direct") {
                return Err(format!("Invalid ZCONNECT runtime settings for {}", link.id).into());
            }
            for value in [&link.username, &link.password] {
                if value.len() > 1024 || !value.bytes().all(|b| (32..=126).contains(&b)) {
                    return Err("ZCONNECT credentials must be printable ASCII without line breaks".into());
                }
            }
            let mut boards = HashSet::new();
            let mut paths = HashSet::new();
            for area in &link.areas {
                if !board_name(&area.remote_board) || !boards.insert(area.remote_board.to_ascii_uppercase()) {
                    return Err(format!("Invalid or duplicate ZCONNECT board {:?}", area.remote_board).into());
                }
                safe_path(&area.local_area)?;
                // JAM changes the supplied extension; reject aliases such as area.foo/area.bar.
                if !paths.insert(area.local_area.with_extension("jhr").to_string_lossy().to_lowercase()) {
                    return Err(format!("Duplicate ZCONNECT local area in {}", link.id).into());
                }
            }
        }
        Ok(())
    }
}

fn safe_path(path: &Path) -> Res<()> {
    if path.as_os_str().is_empty()
        || path.to_str().is_none_or(|s| s.chars().any(char::is_control) || s.contains('\\'))
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir | Component::Prefix(_)))
        || !path.components().any(|c| matches!(c, Component::Normal(_)))
    {
        return Err(format!("Unsafe ZCONNECT path {}", path.display()).into());
    }
    Ok(())
}

fn spool_id(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.as_bytes()[0].is_ascii_alphanumeric() && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn dns_label(s: &str) -> bool {
    (1..=63).contains(&s.len())
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.as_bytes()[s.len() - 1].is_ascii_alphanumeric()
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn fqdn(s: &str) -> bool {
    s.len() <= 253 && s.contains('.') && s.split('.').all(dns_label)
}

fn localpart(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| (33..=124).contains(&b) && !b"@<>/\\()[]{}'`\",!%".contains(&b))
}

fn board_name(s: &str) -> bool {
    s.len() <= 1024
        && s.starts_with('/')
        && !s.ends_with('/')
        && !s.contains("//")
        && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b"/_!+-".contains(&b))
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn remote_system_defaults_and_round_trips_independently_of_host() {
        let mut link: ZconnectLink = toml::from_str("id = 'peer'\nhost = '192.0.2.1'").unwrap();
        assert!(link.remote_system.is_empty());
        assert!(ZconnectLink::default().remote_system.is_empty());
        link.remote_system = "The Remote BBS (Public)".into();
        let encoded = toml::to_string(&link).unwrap();
        assert_eq!(toml::from_str::<ZconnectLink>(&encoded).unwrap(), link);
    }

    #[test]
    fn remote_system_and_timeout_validation_boundaries() {
        let mut config = ZconnectConfig {
            local_system: "local.example".into(),
            links: vec![ZconnectLink {
                id: "peer".into(),
                host: "192.0.2.1".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        for name in [String::new(), "The Remote BBS (Public)".into(), "X".repeat(255)] {
            config.links[0].remote_system = name;
            config.validate().unwrap();
        }
        for name in [
            "bad\rSYS:injected".into(),
            "bad\nname".into(),
            "bad\tname".into(),
            "bad\0name".into(),
            "bad\x7fname".into(),
            "Büro".into(),
            "X".repeat(256),
        ] {
            config.links[0].remote_system = name;
            assert!(config.validate().is_err());
        }
        config.links[0].remote_system.clear();
        for seconds in [1, 30, 60, 3600] {
            config.links[0].timeout_secs = seconds;
            config.validate().unwrap();
        }
        for seconds in [0, 3601, u32::MAX] {
            config.links[0].timeout_secs = seconds;
            assert!(config.validate().is_err());
        }
    }
}
