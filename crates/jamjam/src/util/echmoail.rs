use std::fmt;

#[derive(Default)]
pub struct EchomailAddress {
    pub zone: u16,
    pub net: u16,
    pub node: u16,
    pub point: u16,
}

impl EchomailAddress {
    pub fn new(zone: u16, net: u16, node: u16, point: u16) -> Self {
        EchomailAddress { zone, net, node, point }
    }

    pub fn parse(input: &str) -> Option<Self> {
        let mut state = EchoParser::Zone;
        let mut result = EchomailAddress::default();
        let mut got_number = false;
        for c in input.chars() {
            match state {
                EchoParser::Zone => {
                    if c == ':' {
                        state = EchoParser::Net;
                        if !got_number {
                            return None;
                        }
                        got_number = false;
                        continue;
                    }
                    if c.is_ascii_digit() {
                        if let Some(next) = result.zone.checked_mul(10).and_then(|z| z.checked_add((c as u8 - b'0') as u16)) {
                            result.zone = next;
                        } else {
                            return None;
                        }
                        got_number = true;
                    } else {
                        return None;
                    }
                }
                EchoParser::Net => {
                    if c == '/' {
                        state = EchoParser::Node;
                        if !got_number {
                            return None;
                        }
                        got_number = false;
                        continue;
                    }
                    if c.is_ascii_digit() {
                        if let Some(next) = result.net.checked_mul(10).and_then(|z| z.checked_add((c as u8 - b'0') as u16)) {
                            result.net = next;
                        } else {
                            return None;
                        }
                        got_number = true;
                    } else {
                        return None;
                    }
                }
                EchoParser::Node => {
                    if c == '.' {
                        state = EchoParser::Point;
                        if !got_number {
                            return None;
                        }
                        got_number = false;
                        continue;
                    }

                    if c.is_ascii_digit() {
                        if let Some(next) = result.node.checked_mul(10).and_then(|z| z.checked_add((c as u8 - b'0') as u16)) {
                            result.node = next;
                        } else {
                            return None;
                        }
                        got_number = true;
                    } else {
                        return None;
                    }
                }
                EchoParser::Point => {
                    if c == '.' {
                        return None;
                    }
                    if c.is_ascii_digit() {
                        if let Some(next) = result.point.checked_mul(10).and_then(|z| z.checked_add((c as u8 - b'0') as u16)) {
                            result.point = next;
                        } else {
                            return None;
                        }
                        got_number = true;
                    } else {
                        return None;
                    }
                }
            }
        }

        if got_number && (state == EchoParser::Point || state == EchoParser::Node) {
            Some(result)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
enum EchoParser {
    Zone,
    Net,
    Node,
    Point,
}

impl fmt::Display for EchomailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.point == 0 {
            return write!(f, "{}:{}/{}", self.zone, self.net, self.node);
        }
        write!(f, "{}:{}/{}.{}", self.zone, self.net, self.node, self.point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_display() {
        let addr = EchomailAddress::new(1, 2, 3, 4);
        assert_eq!(addr.to_string(), "1:2/3.4");

        let addr = EchomailAddress::new(1, 2, 3, 0);
        assert_eq!(addr.to_string(), "1:2/3");
    }

    #[test]
    fn test_parse() {
        let addr = EchomailAddress::parse("1:2/3.4").unwrap();
        assert_eq!(addr.zone, 1);
        assert_eq!(addr.net, 2);
        assert_eq!(addr.node, 3);
        assert_eq!(addr.point, 4);

        let addr = EchomailAddress::parse("1:2/3").unwrap();
        assert_eq!(addr.zone, 1);
        assert_eq!(addr.net, 2);
        assert_eq!(addr.node, 3);
        assert_eq!(addr.point, 0);

        let addr = EchomailAddress::parse("1:2/3.4.5");
        assert!(addr.is_none());

        let addr = EchomailAddress::parse("1:2");
        assert!(addr.is_none());

        let addr = EchomailAddress::parse("1:2/3.");
        assert!(addr.is_none());
    }
}
