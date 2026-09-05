use std::path::PathBuf;

use jamjam::util::echomail::EchomailAddress;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::IcyBoardSerializer;
use crate::Res;

pub mod bundle;
pub mod packet;
pub mod queue;
pub mod toss;

/// The port fidonet technology networks reserved for binkp.
pub const DEFAULT_BINKP_PORT: u16 = icy_net::binkp::DEFAULT_PORT;

/// A failure in the file system or in a parser says what went wrong but not
/// which mail it was busy with, and that is the half the sysop needs.
pub(crate) trait Context<T> {
    fn context(self, what: impl FnOnce() -> String) -> Res<T>;
}

impl<T, E: Into<Box<dyn std::error::Error + Send + Sync>>> Context<T> for Result<T, E> {
    fn context(self, what: impl FnOnce() -> String) -> Res<T> {
        self.map_err(|err| format!("{}: {}", what(), err.into()).into())
    }
}

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

    /// The echo tags this link carries. Mail written here is offered to a link
    /// only for the areas it asked for, and to every link that asked.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<String>,
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

    pub fn carries(&self, tag: &str) -> bool {
        self.areas.iter().any(|area| area.eq_ignore_ascii_case(tag))
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
            areas: Vec::new(),
        }
    }
}

/// The decisions the tosser and the mailer would otherwise make on their own.
/// `PCBoard` kept the same set in the fido block of `PCBOARD.DAT`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FtnOptions {
    /// Read what is waiting in the inbound.
    pub process_in: bool,

    /// Pack what was written here for the links that carry it.
    pub process_out: bool,

    /// Toss packets that were addressed to another system as well. A hub does
    /// this, a leaf node has no business reading someone else's mail.
    pub process_orphan: bool,

    /// Call the links. A board that is only ever called leaves this off.
    pub dial_out: bool,

    /// Toss the inbound as soon as a session has ended.
    pub import_after_xfer: bool,

    /// Drop a message whose id has been seen in the area before.
    pub check_dupe_msg_id: bool,

    /// Drop a message whose path already names this board. It has come back
    /// the long way around, and the id check misses it when the id is gone.
    pub check_dupe_path: bool,

    /// How many message ids per area the duplicate check looks back over, zero
    /// for all of them. A busy area makes that list long.
    pub msgs_to_track: u32,

    /// Netmail from a node that is not configured as a link goes to a base of
    /// its own. `PCBoard` represented trusted FTN nodes as `~FIDO~` users; links
    /// are their Icy Board equivalent.
    pub secure: bool,

    /// Netmail addressed to "Sysop" is handed to the name the sysop reads
    /// under, which is what the rest of the board delivers to.
    pub sysop_change: bool,

    /// A tag no area carries becomes an area of its own instead of being
    /// counted and dropped.
    pub auto_add: bool,

    /// The conference an area added that way is attached to.
    pub auto_add_conference: usize,

    /// A tag no area carries is passed on to the links that asked for it. A
    /// hub feeds an area it does not read itself this way.
    pub pass_thru: bool,

    /// The zone and net a two dimensional packet header is completed with.
    pub default_zone: u16,
    pub default_net: u16,

    /// Say what the mailer is doing rather than only what went wrong.
    pub verbose_log: bool,
}

impl Default for FtnOptions {
    fn default() -> Self {
        Self {
            process_in: true,
            process_out: true,
            process_orphan: false,
            dial_out: true,
            import_after_xfer: true,
            check_dupe_msg_id: true,
            check_dupe_path: false,
            msgs_to_track: 0,
            secure: false,
            sysop_change: true,
            auto_add: false,
            auto_add_conference: 0,
            pass_thru: false,
            default_zone: 0,
            default_net: 0,
            verbose_log: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FtnConfig {
    /// Where a session drops the bundles it received.
    pub inbound: PathBuf,

    /// Where bundles wait for the next session.
    pub outbound: PathBuf,

    /// The message base arriving netmail is written to. Netmail is addressed to
    /// a person, but nothing here knows yet which user that is, so it lands in
    /// one place for the sysop to look at.
    #[serde(default = "FtnConfig::default_netmail")]
    pub netmail: PathBuf,

    /// Where netmail from an unconfigured node goes when `options.secure` is
    /// set.
    #[serde(default = "FtnConfig::default_bad_netmail")]
    pub bad_netmail: PathBuf,

    /// Where the base of an area added by `options.auto_add` is created.
    #[serde(default = "FtnConfig::default_new_areas")]
    pub new_areas: PathBuf,

    /// Appended to every echomail message this board originates.
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,

    // Toml demands that the tables come last.
    #[serde(default)]
    pub options: FtnOptions,

    #[serde(rename = "aka", default)]
    pub akas: Vec<FtnAka>,

    #[serde(rename = "link", default)]
    pub links: Vec<FtnLink>,
}

impl FtnConfig {
    fn default_netmail() -> PathBuf {
        PathBuf::from("ftn/netmail")
    }

    fn default_bad_netmail() -> PathBuf {
        PathBuf::from("ftn/badmail")
    }

    fn default_new_areas() -> PathBuf {
        PathBuf::from("ftn/areas")
    }

    /// One of the addresses this board answers to, which is what tells mail
    /// meant for it from mail that only passes through.
    pub fn answers_to(&self, address: &EchomailAddress) -> bool {
        self.akas.iter().any(|aka| aka.address == *address)
    }

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
            netmail: Self::default_netmail(),
            bad_netmail: Self::default_bad_netmail(),
            new_areas: Self::default_new_areas(),
            origin: String::new(),
            options: FtnOptions::default(),
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
        let config = FtnConfig {
            akas: vec![aka("21:1/100", "fsxnet"), aka("618:500/20", "micronet")],
            ..Default::default()
        };
        assert_eq!(config.aka_for(&link("618:500/1", "micronet")).unwrap().address.to_string(), "618:500/20");
    }

    #[test]
    fn test_a_link_of_an_unknown_network_falls_back_to_the_first_address() {
        let config = FtnConfig {
            akas: vec![aka("21:1/100", "fsxnet"), aka("618:500/20", "micronet")],
            ..Default::default()
        };
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

    #[test]
    fn test_a_link_only_carries_the_areas_it_asked_for() {
        let config: FtnConfig = toml::from_str(
            r#"
            inbound = "ftn/inbound"
            outbound = "ftn/outbound"
            [[link]]
            address = "21:1/1"
            host = "hub.example.org"
            areas = ["FSX_GEN", "FSX_BBS"]
            "#,
        )
        .unwrap();
        assert!(config.links[0].carries("fsx_gen"));
        assert!(!config.links[0].carries("FSX_MYS"));
    }
}
