use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::path_is_empty;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Telnet {
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub display_file: PathBuf,
}

impl Default for Telnet {
    fn default() -> Self {
        Self {
            is_enabled: true,
            port: 23,
            address: String::new(),
            display_file: PathBuf::new(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SSH {
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub display_file: PathBuf,
}

impl Default for SSH {
    fn default() -> Self {
        Self {
            is_enabled: false,
            port: 22,
            address: String::new(),
            display_file: PathBuf::new(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Websocket {
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub display_file: PathBuf,
}

impl Default for Websocket {
    fn default() -> Self {
        Self {
            is_enabled: false,
            port: 8810,
            address: String::new(),
            display_file: PathBuf::new(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SecureWebsocket {
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub display_file: PathBuf,

    pub cert_pem: PathBuf,
    pub key_pem: PathBuf,
}

impl Default for SecureWebsocket {
    fn default() -> Self {
        Self {
            is_enabled: false,
            port: 8811,
            address: String::new(),
            display_file: PathBuf::new(),
            cert_pem: PathBuf::new(),
            key_pem: PathBuf::new(),
        }
    }
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoginServer {
    pub telnet: Telnet,
    pub ssh: SSH,
    pub websocket: Websocket,
    pub secure_websocket: SecureWebsocket,
}
