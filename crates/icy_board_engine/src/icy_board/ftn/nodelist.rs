//! Reads a fidonet nodelist. The file is a plain list of every system in the
//! network, and an entry carries no address of its own: it follows from the
//! `Zone`, `Region`, `Host` and `Hub` lines above it.

use std::path::Path;

use jamjam::util::echomail::EchomailAddress;

use super::{Context, DEFAULT_BINKP_PORT};
use crate::Res;

/// What a line says a system is. The keyword also decides which part of the
/// address the line sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeKind {
    Zone,
    Region,
    Host,
    Hub,
    #[default]
    Node,

    /// Listed but unpublished, held, or not answering. Mail still routes to a
    /// held node; a downed one is not worth calling.
    Private,
    Hold,
    Down,
    Point,
}

impl NodeKind {
    fn from_keyword(keyword: &str) -> Option<Self> {
        Some(match keyword.to_ascii_uppercase().as_str() {
            "" => Self::Node,
            "ZONE" => Self::Zone,
            "REGION" => Self::Region,
            "HOST" => Self::Host,
            "HUB" => Self::Hub,
            "PVT" => Self::Private,
            "HOLD" => Self::Hold,
            "DOWN" => Self::Down,
            "POINT" => Self::Point,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodelistEntry {
    pub address: EchomailAddress,
    pub kind: NodeKind,
    pub name: String,
    pub location: String,
    pub sysop: String,
    pub phone: String,
    pub speed: u32,
    pub flags: Vec<String>,
}

impl NodelistEntry {
    /// The value a flag carries after its colon, empty when the flag stands on
    /// its own and `None` when it is not there at all.
    pub fn flag(&self, wanted: &str) -> Option<&str> {
        self.flags.iter().find_map(|flag| {
            let (name, value) = match flag.split_once(':') {
                Some((name, value)) => (name, value),
                None => (flag.as_str(), ""),
            };
            name.eq_ignore_ascii_case(wanted).then_some(value)
        })
    }

    pub fn has_flag(&self, wanted: &str) -> bool {
        self.flag(wanted).is_some()
    }

    /// Whether the system takes mail at any hour, which is what makes calling
    /// it out of a routing decision worthwhile.
    pub fn takes_crash_mail(&self) -> bool {
        self.has_flag("CM")
    }

    /// Where to reach the system over binkp. `IBN` may name a host, a port, or
    /// both, and falls back to the address `INA` gives for all IP flags.
    pub fn binkp_address(&self) -> Option<(String, u16)> {
        if matches!(self.kind, NodeKind::Hold | NodeKind::Down) {
            return None;
        }
        let ibn = self.flag("IBN")?;
        let internet = || self.flag("INA").filter(|host| !host.is_empty()).map(str::to_string);

        if ibn.is_empty() {
            return Some((internet()?, DEFAULT_BINKP_PORT));
        }
        if let Ok(port) = ibn.parse::<u16>() {
            return Some((internet()?, port));
        }
        if let Some(rest) = ibn.strip_prefix('[')
            && let Some((host, port)) = rest.split_once("]:")
        {
            return Some((host.to_string(), port.parse().unwrap_or(DEFAULT_BINKP_PORT)));
        }
        // An unbracketed value with more than one colon is an IPv6 address,
        // not a host followed by a port.
        if ibn.matches(':').count() > 1 {
            return Some((ibn.to_string(), DEFAULT_BINKP_PORT));
        }
        match ibn.split_once(':') {
            Some((host, port)) => Some((host.to_string(), port.parse().unwrap_or(DEFAULT_BINKP_PORT))),
            None => Some((ibn.to_string(), DEFAULT_BINKP_PORT)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Nodelist {
    entries: Vec<NodelistEntry>,
}

/// A nodelist writes a space as an underscore, because a comma separates the
/// fields and a space would be hard to tell from padding.
fn readable(field: &str) -> String {
    field.replace('_', " ")
}

impl Nodelist {
    pub fn load(path: &Path) -> Res<Self> {
        let text = std::fs::read(path).context(|| format!("Cannot read the nodelist {}", path.display()))?;
        // A nodelist is plain ASCII, but a sysop name now and then is not.
        Ok(Self::parse(&String::from_utf8_lossy(&text)))
    }

    pub fn parse(text: &str) -> Self {
        let mut entries = Vec::new();
        let (mut zone, mut net, mut node) = (0u16, 0u16, 0u16);

        for line in text.lines() {
            // A nodelist made under DOS ends with a stray end-of-file mark.
            let line = line.trim_end_matches(['\r', '\u{1a}']);
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let mut fields = line.split(',');
            let Some(keyword) = fields.next() else {
                continue;
            };
            let Some(kind) = NodeKind::from_keyword(keyword.trim()) else {
                continue;
            };
            let Some(number) = fields.next().and_then(|number| number.trim().parse::<u16>().ok()) else {
                continue;
            };

            let has_context = match kind {
                NodeKind::Zone => true,
                NodeKind::Region | NodeKind::Host => zone != 0,
                NodeKind::Point => zone != 0 && net != 0 && node != 0,
                _ => zone != 0 && net != 0,
            };
            if !has_context {
                continue;
            }

            match kind {
                NodeKind::Zone => {
                    zone = number;
                    net = number;
                    node = 0;
                }
                NodeKind::Region | NodeKind::Host => {
                    net = number;
                    node = 0;
                }
                NodeKind::Point => {}
                _ => node = number,
            }
            let address = match kind {
                NodeKind::Point => EchomailAddress::new(zone, net, node, number),
                _ => EchomailAddress::new(zone, net, node, 0),
            };

            let mut next = || fields.next().unwrap_or_default().trim().to_string();
            entries.push(NodelistEntry {
                address,
                kind,
                name: readable(&next()),
                location: readable(&next()),
                sysop: readable(&next()),
                phone: next(),
                speed: next().parse().unwrap_or(0),
                flags: fields.map(|flag| flag.trim().to_string()).filter(|flag| !flag.is_empty()).collect(),
            });
        }
        Self { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodelistEntry> {
        self.entries.iter()
    }

    /// The system at an address. A four dimensional address only matches a
    /// point, so a node is asked for with its point set to zero.
    pub fn find(&self, address: &EchomailAddress) -> Option<&NodelistEntry> {
        self.entries.iter().find(|entry| entry.address == *address)
    }

    /// Every system whose name, sysop or location holds the text, which is how
    /// a nodelist is looked through when the address is what is being sought.
    pub fn search(&self, text: &str) -> Vec<&NodelistEntry> {
        let wanted = text.to_ascii_uppercase();
        self.entries
            .iter()
            .filter(|entry| {
                entry.name.to_ascii_uppercase().contains(&wanted)
                    || entry.sysop.to_ascii_uppercase().contains(&wanted)
                    || entry.location.to_ascii_uppercase().contains(&wanted)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = concat!(
        ";A This is a comment\r\n",
        "Zone,21,fsxNet_ZC,Somewhere,Zone_Coordinator,-Unpublished-,300,CM\r\n",
        "Host,1,fsxNet_Hub,Auckland_NZ,Avon_Riley,-Unpublished-,300,CM,IBN:agency.bbs.geek.nz\r\n",
        ",100,Icy_Board,Bonn_DE,Mike_Krueger,-Unpublished-,9600,CM,INA:icy.example,IBN\r\n",
        "Point,3,A_Point,Bonn_DE,Someone_Else,-Unpublished-,9600\r\n",
        ",101,Other_BBS,Berlin_DE,Someone,-Unpublished-,9600,IBN:other.example:12345\r\n",
        "Down,102,Gone_BBS,Nowhere,Nobody,-Unpublished-,300\r\n",
    );

    #[test]
    fn test_an_address_follows_from_the_lines_above_an_entry() {
        let list = Nodelist::parse(LIST);

        assert_eq!(list.find(&EchomailAddress::new(21, 21, 0, 0)).unwrap().kind, NodeKind::Zone);
        assert_eq!(list.find(&EchomailAddress::new(21, 1, 0, 0)).unwrap().name, "fsxNet Hub");
        assert_eq!(list.find(&EchomailAddress::new(21, 1, 100, 0)).unwrap().name, "Icy Board");
        assert_eq!(list.find(&EchomailAddress::new(21, 1, 101, 0)).unwrap().name, "Other BBS");
    }

    /// A point belongs to the node above it, not to the net.
    #[test]
    fn test_a_point_hangs_below_the_node_it_follows() {
        let list = Nodelist::parse(LIST);

        let point = list.find(&EchomailAddress::new(21, 1, 100, 3)).unwrap();
        assert_eq!(point.kind, NodeKind::Point);
        assert_eq!(point.sysop, "Someone Else");
    }

    #[test]
    fn test_an_underscore_stands_for_a_space() {
        let list = Nodelist::parse(LIST);
        let entry = list.find(&EchomailAddress::new(21, 1, 100, 0)).unwrap();

        assert_eq!(entry.sysop, "Mike Krueger");
        assert_eq!(entry.location, "Bonn DE");
    }

    #[test]
    fn test_a_comment_and_an_empty_line_are_not_entries() {
        assert_eq!(Nodelist::parse(LIST).len(), 6);
        assert!(Nodelist::parse(";only a comment\r\n\r\n").is_empty());
    }

    #[test]
    fn test_binkp_is_reached_at_the_host_the_flag_names() {
        let list = Nodelist::parse(LIST);

        let hub = list.find(&EchomailAddress::new(21, 1, 0, 0)).unwrap();
        assert_eq!(hub.binkp_address(), Some(("agency.bbs.geek.nz".to_string(), DEFAULT_BINKP_PORT)));
    }

    /// A bare `IBN` means binkp on the default port at the address `INA` gives.
    #[test]
    fn test_a_bare_binkp_flag_falls_back_to_the_internet_address() {
        let list = Nodelist::parse(LIST);

        let entry = list.find(&EchomailAddress::new(21, 1, 100, 0)).unwrap();
        assert_eq!(entry.binkp_address(), Some(("icy.example".to_string(), DEFAULT_BINKP_PORT)));
    }

    #[test]
    fn test_a_binkp_flag_can_name_a_port_of_its_own() {
        let list = Nodelist::parse(LIST);

        let entry = list.find(&EchomailAddress::new(21, 1, 101, 0)).unwrap();
        assert_eq!(entry.binkp_address(), Some(("other.example".to_string(), 12345)));
    }

    #[test]
    fn test_a_binkp_flag_can_name_an_ipv6_address() {
        let list = Nodelist::parse(concat!(
            "Zone,21,Z,L,S,P,300\r\n",
            "Host,1,H,L,S,P,300\r\n",
            ",1,Plain,L,S,P,300,IBN:2001:db8::1\r\n",
            ",2,Bracketed,L,S,P,300,IBN:[2001:db8::2]:24555\r\n",
        ));

        assert_eq!(
            list.find(&EchomailAddress::new(21, 1, 1, 0)).unwrap().binkp_address(),
            Some(("2001:db8::1".to_string(), DEFAULT_BINKP_PORT))
        );
        assert_eq!(
            list.find(&EchomailAddress::new(21, 1, 2, 0)).unwrap().binkp_address(),
            Some(("2001:db8::2".to_string(), 24555))
        );
    }

    #[test]
    fn test_a_system_without_a_binkp_flag_cannot_be_called_over_ip() {
        let list = Nodelist::parse(LIST);

        assert_eq!(list.find(&EchomailAddress::new(21, 1, 102, 0)).unwrap().binkp_address(), None);
    }

    #[test]
    fn test_a_held_or_down_system_is_not_called_even_when_it_names_binkp() {
        let list = Nodelist::parse(concat!(
            "Zone,21,Z,L,S,P,300\r\n",
            "Host,1,H,L,S,P,300\r\n",
            "Hold,1,Held,L,S,P,300,IBN:held.example\r\n",
            "Down,2,Down,L,S,P,300,IBN:down.example\r\n",
        ));

        assert_eq!(list.find(&EchomailAddress::new(21, 1, 1, 0)).unwrap().binkp_address(), None);
        assert_eq!(list.find(&EchomailAddress::new(21, 1, 2, 0)).unwrap().binkp_address(), None);
    }

    #[test]
    fn test_crash_mail_is_told_apart_from_the_rest() {
        let list = Nodelist::parse(LIST);

        assert!(list.find(&EchomailAddress::new(21, 1, 100, 0)).unwrap().takes_crash_mail());
        assert!(!list.find(&EchomailAddress::new(21, 1, 101, 0)).unwrap().takes_crash_mail());
    }

    #[test]
    fn test_a_system_is_looked_for_by_name_sysop_or_place() {
        let list = Nodelist::parse(LIST);

        assert_eq!(list.search("icy board").len(), 1);
        assert_eq!(list.search("krueger").len(), 1);
        assert_eq!(list.search("bonn").len(), 2);
        assert!(list.search("nothing here").is_empty());
    }

    #[test]
    fn test_a_line_that_makes_no_sense_is_stepped_over() {
        let list = Nodelist::parse("Nonsense,1,A,B,C,D,300\r\n,notanumber,A,B,C,D,300\r\nZone,21,Z,L,S,P,300\r\n");

        assert_eq!(list.len(), 1);
        assert_eq!(list.find(&EchomailAddress::new(21, 21, 0, 0)).unwrap().kind, NodeKind::Zone);
    }

    #[test]
    fn test_a_line_without_the_address_context_it_needs_is_stepped_over() {
        let list = Nodelist::parse(concat!(
            "Region,10,R,L,S,P,300\r\n",
            ",100,N,L,S,P,300\r\n",
            "Point,1,P,L,S,P,300\r\n",
            "Zone,21,Z,L,S,P,300\r\n",
        ));

        assert_eq!(list.len(), 1);
        assert_eq!(list.iter().next().unwrap().kind, NodeKind::Zone);
    }
}
