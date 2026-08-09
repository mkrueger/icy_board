use std::path::PathBuf;

use jamjam::util::echmoail::EchomailAddress;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::IcyBoardSerializer;

pub mod bundle;
pub mod packet;

/// The port fidonet technology networks reserved for binkp.
pub const DEFAULT_BINKP_PORT: u16 = icy_net::binkp::DEFAULT_PORT;

/// One of the addresses this board answers to. A board that joined more than
/// one network has one of these per network.
#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FtnAka {
    #[serde_as(as = "DisplayFromStr")]
    pub address: EchomailAddress,

    /// The network the address belongs to, sent after an '@' in binkp.
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub domain: String,
}

impl FtnAka {
    pub fn to_5d(&self) -> String {
        if self.domain.is_empty() {
            return self.address.to_string();
        }
        format!("{}@{}", self.address, self.domain)
    }
}

/// A system this board exchanges mail with - the uplink of a node, the boss of
/// a point, or a downlink being fed from here.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FtnLink {
    #[serde_as(as = "DisplayFromStr")]
    pub address: EchomailAddress,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub domain: String,

    pub host: String,

    #[serde(default = "FtnLink::default_port")]
    pub port: u16,

    /// Binkp either sends this or answers a challenge with it, so it cannot be
    /// stored hashed the way user passwords are.
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,

    /// Zero means the link is polled only when the sysop asks for it.
    #[serde(default)]
    pub poll_minutes: u32,
}

impl FtnLink {
    fn default_port() -> u16 {
        DEFAULT_BINKP_PORT
    }

    pub fn to_5d(&self) -> String {
        if self.domain.is_empty() {
            return self.address.to_string();
        }
        format!("{}@{}", self.address, self.domain)
    }
}

impl Default for FtnLink {
    fn default() -> Self {
        Self {
            address: EchomailAddress::default(),
            domain: String::new(),
            host: String::new(),
            port: DEFAULT_BINKP_PORT,
            password: String::new(),
            poll_minutes: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FtnConfig {
    /// Where a session drops the bundles it received.
    pub inbound: PathBuf,

    /// Where bundles wait for the next session.
    pub outbound: PathBuf,

    /// Appended to every echomail message this board originates.
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,

    // Toml demands that the tables come last.
    #[serde(rename = "aka", default)]
    pub akas: Vec<FtnAka>,

    #[serde(rename = "link", default)]
    pub links: Vec<FtnLink>,
}

impl FtnConfig {
    /// The address a link is greeted with when it belongs to no known network.
    pub fn primary_aka(&self) -> Option<&FtnAka> {
        self.akas.first()
    }

    /// The address to present to a link, which is the one from its own network.
    pub fn aka_for(&self, link: &FtnLink) -> Option<&FtnAka> {
        self.akas
            .iter()
            .find(|aka| aka.domain.eq_ignore_ascii_case(&link.domain) && aka.address.zone == link.address.zone)
            .or_else(|| self.akas.iter().find(|aka| aka.address.zone == link.address.zone))
            .or_else(|| self.primary_aka())
    }

    pub fn is_configured(&self) -> bool {
        !self.akas.is_empty()
    }

    /// Mail waits in a directory of its own per link, because a flat outbound
    /// would offer every bundle to every system that calls.
    pub fn outbound_for(&self, link: &FtnLink) -> PathBuf {
        self.outbound.join(link.address.to_string().replace([':', '/'], "."))
    }
}

impl Default for FtnConfig {
    fn default() -> Self {
        Self {
            akas: Vec::new(),
            links: Vec::new(),
            inbound: PathBuf::from("ftn/inbound"),
            outbound: PathBuf::from("ftn/outbound"),
            origin: String::new(),
        }
    }
}

impl IcyBoardSerializer for FtnConfig {
    const FILE_TYPE: &'static str = "ftn";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aka(address: &str, domain: &str) -> FtnAka {
        FtnAka {
            address: EchomailAddress::parse(address).unwrap(),
            domain: domain.to_string(),
        }
    }

    fn link(address: &str, domain: &str) -> FtnLink {
        FtnLink {
            address: EchomailAddress::parse(address).unwrap(),
            domain: domain.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_every_link_has_an_outbound_directory_of_its_own() {
        let config = FtnConfig::default();

        assert_eq!(config.outbound_for(&link("21:1/100", "fsxnet")), PathBuf::from("ftn/outbound/21.1.100"));
        assert_ne!(
            config.outbound_for(&link("21:1/100", "fsxnet")),
            config.outbound_for(&link("21:1/101", "fsxnet"))
        );
    }

    #[test]
    fn test_a_link_is_greeted_with_the_address_from_its_own_network() {
        let mut config = FtnConfig::default();
        config.akas = vec![aka("21:1/100", "fsxnet"), aka("618:500/20", "micronet")];
        assert_eq!(config.aka_for(&link("618:500/1", "micronet")).unwrap().address.to_string(), "618:500/20");
    }

    #[test]
    fn test_a_link_of_an_unknown_network_falls_back_to_the_first_address() {
        let mut config = FtnConfig::default();
        config.akas = vec![aka("21:1/100", "fsxnet"), aka("618:500/20", "micronet")];
        assert_eq!(config.aka_for(&link("1:123/456", "fidonet")).unwrap().address.to_string(), "21:1/100");
    }

    #[test]
    fn test_an_address_without_a_domain_is_written_as_four_dimensions() {
        assert_eq!(aka("21:1/100.5", "").to_5d(), "21:1/100.5");
        assert_eq!(aka("21:1/100", "fsxnet").to_5d(), "21:1/100@fsxnet");
    }

    #[test]
    fn test_a_filled_config_survives_a_round_trip_through_toml() {
        let config = FtnConfig {
            origin: "Icy Board".to_string(),
            akas: vec![aka("21:1/100", "fsxnet")],
            links: vec![FtnLink {
                host: "hub.example.org".to_string(),
                password: "secret".to_string(),
                poll_minutes: 30,
                ..link("21:1/1", "fsxnet")
            }],
            ..Default::default()
        };
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<FtnConfig>(&text).unwrap(), config);
    }

    #[test]
    fn test_a_link_without_a_port_gets_the_binkp_one() {
        let config: FtnConfig = toml::from_str(
            r#"
            inbound = "ftn/inbound"
            outbound = "ftn/outbound"
            [[aka]]
            address = "21:1/100"
            domain = "fsxnet"
            [[link]]
            address = "21:1/1"
            host = "hub.example.org"
            "#,
        )
        .unwrap();
        assert_eq!(config.links[0].port, DEFAULT_BINKP_PORT);
        assert_eq!(config.akas[0].address.node, 100);
    }
}
