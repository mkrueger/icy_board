use std::time::Duration;

use super::KermitError;
use crate::Connection;

pub const MAX_LEN: usize = 94;
pub const MAX_LONG_LEN: usize = 4096;
pub const PARAMETERS: &[u8] = b"~% @-#&1~\" K+";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub sequence: u8,
    pub kind: u8,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct Quoting {
    pub control: u8,
    pub binary: Option<u8>,
    pub repeat: Option<u8>,
}

impl Default for Quoting {
    fn default() -> Self {
        Self {
            control: b'#',
            binary: None,
            repeat: None,
        }
    }
}

impl Quoting {
    pub fn encode(&self, data: &[u8], limit: usize) -> (Vec<u8>, usize) {
        let mut encoded = Vec::with_capacity(limit.min(data.len()));
        let mut used = 0;
        while used < data.len() {
            let mut quoted = [0; 3];
            let mut length = 0;
            let mut byte = data[used];
            if let Some(binary) = self.binary
                && byte & 0x80 != 0
            {
                quoted[length] = binary;
                length += 1;
                byte &= 0x7f;
            }
            let low = byte & 0x7f;
            if low < 32 || low == 127 {
                quoted[length] = self.control;
                length += 1;
                byte ^= 64;
            } else if low == self.control || Some(low) == self.binary || Some(low) == self.repeat {
                quoted[length] = self.control;
                length += 1;
            }
            quoted[length] = byte;
            length += 1;
            let mut count = 1;
            if let Some(repeat) = self.repeat {
                let run = data[used..].iter().take(94).take_while(|&&b| b == data[used]).count();
                if run * length > length + 2 && encoded.len() + length + 2 <= limit {
                    encoded.extend([repeat, run as u8 + 32]);
                    count = run;
                }
            }
            if encoded.len() + length > limit {
                break;
            }
            encoded.extend_from_slice(&quoted[..length]);
            used += count;
        }
        (encoded, used)
    }

    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>, KermitError> {
        let mut result = Vec::new();
        let mut data = data.iter().copied();
        while let Some(mut byte) = data.next() {
            let mut count = 1;
            if Some(byte) == self.repeat {
                count = data
                    .next()
                    .and_then(|b| b.checked_sub(32))
                    .filter(|&n| n > 0 && n <= 94)
                    .ok_or(KermitError::InvalidPacket)?;
                byte = data.next().ok_or(KermitError::InvalidPacket)?;
            }
            let mut high = 0;
            if Some(byte) == self.binary {
                high = 0x80;
                byte = data.next().ok_or(KermitError::InvalidPacket)?;
            }
            if byte == self.control {
                byte = data.next().ok_or(KermitError::InvalidPacket)?;
                if matches!(byte & 0x7f, 63..=95) {
                    byte ^= 64;
                }
            }
            result.extend(std::iter::repeat_n(byte | high, count as usize));
        }
        Ok(result)
    }
}

pub struct Parameters {
    pub max_len: usize,
    pub long_len: Option<usize>,
    pub timeout: Duration,
    pub padding: usize,
    pub pad: u8,
    pub eol: u8,
    pub incoming: Quoting,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            max_len: 80,
            long_len: None,
            timeout: Duration::from_secs(5),
            padding: 0,
            pad: 0,
            eol: 13,
            incoming: Quoting::default(),
        }
    }
}

fn prefix(byte: u8) -> bool {
    matches!(byte, 33..=62 | 96..=126)
}

impl Parameters {
    pub fn parse(data: &[u8], receiving: bool) -> Result<Self, KermitError> {
        let mut result = Self::default();
        let field = |i: usize| data.get(i).copied().filter(|b| *b != b' ');
        let number = |i| -> Result<Option<u8>, KermitError> {
            field(i)
                .map(|b| b.checked_sub(32).filter(|&n| n as usize <= MAX_LEN).ok_or(KermitError::InvalidParameters))
                .transpose()
        };
        if let Some(n) = number(0)? {
            if n < 12 {
                return Err(KermitError::InvalidParameters);
            }
            result.max_len = n as usize;
        }
        if let Some(n) = number(1)? {
            result.timeout = Duration::from_secs(n as u64);
        }
        result.padding = number(2)?.unwrap_or(0) as usize;
        result.pad = field(3).unwrap_or(b'@') ^ 64;
        result.eol = number(4)?.unwrap_or(13);
        if result.eol > 31 {
            return Err(KermitError::InvalidParameters);
        }
        result.incoming.control = field(5).unwrap_or(b'#');
        if !prefix(result.incoming.control) {
            return Err(KermitError::InvalidParameters);
        }
        result.incoming.binary = match field(6) {
            Some(b'Y' | b'&') => Some(b'&'),
            Some(b) if receiving && prefix(b) => Some(b),
            _ => None,
        };
        result.incoming.repeat = if field(8) == Some(b'~') { Some(b'~') } else { None };
        if result.incoming.binary == Some(result.incoming.control)
            || result.incoming.repeat == Some(result.incoming.control)
            || result.incoming.binary.is_some() && result.incoming.binary == result.incoming.repeat
            || result.incoming.binary == Some(b'#')
        {
            return Err(KermitError::InvalidParameters);
        }
        if let Some(&first_capability) = data.get(9) {
            let mut index = 9;
            loop {
                let capability = data.get(index).copied().ok_or(KermitError::InvalidParameters)?;
                if !(32..=95).contains(&capability) {
                    return Err(KermitError::InvalidParameters);
                }
                if (capability - 32) & 1 == 0 {
                    break;
                }
                index += 1;
            }
            if (first_capability - 32) & 2 != 0 {
                let length = match (data.get(index + 2), data.get(index + 3)) {
                    (Some(&high), Some(&low)) if (32..=126).contains(&high) && (32..=126).contains(&low) => (high - 32) as usize * 95 + (low - 32) as usize,
                    (Some(_), Some(_)) => return Err(KermitError::InvalidParameters),
                    _ => 500,
                };
                if length == 0 {
                    return Err(KermitError::InvalidParameters);
                }
                // The Send-Init ACK must still fit in a short packet.
                if !receiving || result.max_len >= PARAMETERS.len() + 3 {
                    result.long_len = Some(length.min(MAX_LONG_LEN));
                }
            }
        }
        Ok(result)
    }

    pub fn data_limit(&self) -> usize {
        (self.max_len - 3).max(self.long_len.map_or(0, |length| length - 1))
    }

    pub fn outgoing(&self) -> Quoting {
        Quoting {
            control: b'#',
            ..self.incoming
        }
    }

    pub fn response(&self) -> Vec<u8> {
        let mut response = PARAMETERS.to_vec();
        response[6] = self.incoming.binary.unwrap_or(b'N');
        response[8] = self.incoming.repeat.unwrap_or(b' ');
        if self.long_len.is_none() {
            response.truncate(9);
        }
        response
    }

    pub fn frame(&self, packet: &Packet) -> Result<Vec<u8>, KermitError> {
        let length = packet.data.len() + 3;
        if packet.data.len() > self.data_limit() || packet.sequence >= 64 {
            return Err(KermitError::InvalidPacket);
        }
        let mut frame = vec![self.pad; self.padding];
        frame.push(1);
        let start = frame.len();
        if length <= self.max_len {
            frame.extend([length as u8 + 32, packet.sequence + 32, packet.kind]);
        } else {
            let extended_length = packet.data.len() + 1;
            frame.extend([
                32,
                packet.sequence + 32,
                packet.kind,
                (extended_length / 95) as u8 + 32,
                (extended_length % 95) as u8 + 32,
            ]);
            frame.push(checksum(&frame[start..]));
        }
        frame.extend_from_slice(&packet.data);
        frame.push(checksum(&frame[start..]));
        frame.push(self.eol);
        Ok(frame)
    }
}

fn checksum(bytes: &[u8]) -> u8 {
    let sum: u32 = bytes.iter().map(|&b| b as u32).sum();
    (((sum + ((sum & 0xc0) >> 6)) & 0x3f) as u8) + 32
}

#[derive(Default)]
pub struct Reader {
    bytes: Vec<u8>,
    pub long_packets: bool,
}

impl Reader {
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub async fn read(&mut self, com: &mut dyn Connection, finishing: bool) -> crate::Result<Option<Packet>> {
        loop {
            tokio::task::consume_budget().await;
            if self.bytes.is_empty() {
                let byte = com.read_u8().await?;
                if byte == 1 {
                    self.bytes.push(byte);
                    continue;
                }
                if finishing && byte != b'\r' && byte != b'\n' {
                    com.unread(&[byte])?;
                    return Ok(None);
                }
                continue;
            }
            if self.bytes.len() == 1 {
                self.bytes.push(com.read_u8().await?);
            }
            let extended = self.bytes[1] == 32 && self.long_packets;
            if !extended && !(35..=126).contains(&self.bytes[1]) {
                self.bytes.clear();
                return Err(KermitError::InvalidPacket.into());
            }
            let target = if extended {
                if self.bytes.len() < 7 {
                    7
                } else {
                    if !(32..=126).contains(&self.bytes[4]) || !(32..=126).contains(&self.bytes[5]) || checksum(&self.bytes[1..6]) != self.bytes[6] {
                        self.bytes.clear();
                        return Err(KermitError::InvalidPacket.into());
                    }
                    let length = (self.bytes[4] - 32) as usize * 95 + (self.bytes[5] - 32) as usize;
                    if !(1..=MAX_LONG_LEN).contains(&length) {
                        self.bytes.clear();
                        return Err(KermitError::InvalidPacket.into());
                    }
                    length + 7
                }
            } else {
                (self.bytes[1] - 32) as usize + 2
            };
            if self.bytes.len() == target {
                let bytes = std::mem::take(&mut self.bytes);
                if !(32..=95).contains(&bytes[2]) || checksum(&bytes[1..bytes.len() - 1]) != bytes[bytes.len() - 1] {
                    return Err(KermitError::InvalidPacket.into());
                }
                return Ok(Some(Packet {
                    sequence: bytes[2] - 32,
                    kind: bytes[3],
                    data: bytes[if extended { 7 } else { 4 }..bytes.len() - 1].to_vec(),
                }));
            }
            let mut buffer = [0; MAX_LONG_LEN];
            let count = com.read(&mut buffer[..target - self.bytes.len()]).await?;
            if count == 0 {
                return Err(crate::NetError::ConnectionClosed.into());
            }
            self.bytes.extend_from_slice(&buffer[..count]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_round_trips_every_byte_with_and_without_eighth_bit_prefixing() {
        let bytes: Vec<u8> = (0..=255).collect();
        for binary in [None, Some(b'&')] {
            for repeat in [None, Some(b'~')] {
                let quoting = Quoting { control: b'#', binary, repeat };
                let mut rest = bytes.as_slice();
                let mut decoded = Vec::new();
                while !rest.is_empty() {
                    let (encoded, count) = quoting.encode(rest, 3);
                    assert!(count > 0);
                    assert!(encoded.len() <= 3);
                    decoded.extend(quoting.decode(&encoded).unwrap());
                    rest = &rest[count..];
                }
                assert_eq!(decoded, bytes);
            }
        }
    }

    #[test]
    fn malformed_prefixes_and_parameters_are_rejected() {
        let quoting = Quoting {
            control: b'#',
            binary: Some(b'&'),
            repeat: Some(b'~'),
        };
        for bytes in [b"#".as_slice(), b"&", b"~", b"~ A", b"~\x7fA", b"~#&", b"~#&#"] {
            assert!(quoting.decode(bytes).is_err(), "{bytes:?}");
        }
        for params in [b"!".as_slice(), b"\x7f", b"~% @-#~1~", b"~% @-##1~", b"~% @~-&1~"] {
            assert!(Parameters::parse(params, true).is_err(), "{params:?}");
        }
        let custom = Parameters::parse(b"~% @-!`1~", true).unwrap();
        assert_eq!(custom.incoming.control, b'!');
        assert_eq!(custom.incoming.binary, Some(b'`'));
        assert_eq!(custom.response(), b"~% @-#`1~");
    }

    #[test]
    fn long_packet_parameters_handle_legacy_defaults_and_capability_continuations() {
        for (data, length) in [
            (b"~% @-#&1~".as_slice(), None),
            (PARAMETERS, Some(MAX_LONG_LEN)),
            (b"~% @-#&1~\"".as_slice(), Some(500)),
            (b"~% @-#&1~#  K+".as_slice(), Some(MAX_LONG_LEN)),
            (b"~% @-#&1~\" ~~".as_slice(), Some(MAX_LONG_LEN)),
            (b",% @-#&1~\" K+".as_slice(), None),
        ] {
            let peer = Parameters::parse(data, true).unwrap();
            assert_eq!(peer.long_len, length, "{data:?}");
            assert_eq!(peer.response().len(), if length.is_some() { 13 } else { 9 });
        }
        for data in [b"~% @-#&1~#".as_slice(), b"~% @-#&1~\" \x7f ", b"~% @-#&1~\"   "] {
            assert!(Parameters::parse(data, true).is_err(), "{data:?}");
        }
    }

    #[test]
    fn repeat_encoding_uses_bounded_runs_and_never_splits_a_prefix() {
        let quoting = Quoting {
            control: b'#',
            binary: Some(b'&'),
            repeat: Some(b'~'),
        };
        assert_eq!(quoting.encode(&[0; 120], 91), (b"~~#@~:#@".to_vec(), 120));
        assert_eq!(quoting.encode(&[255; 120], 4), (b"&#?".to_vec(), 1));
        assert_eq!(quoting.encode(&[255; 120], 5), (b"~~&#?".to_vec(), 94));
        assert_eq!(quoting.encode(&[b'A'; 3], 3), (b"AAA".to_vec(), 3));
        let bytes: Vec<u8> = (0..=255).flat_map(|byte| std::iter::repeat_n(byte, 120)).collect();
        for binary in [None, Some(b'&')] {
            for limit in [3, 5, 17, MAX_LONG_LEN - 1] {
                let quoting = Quoting { binary, ..quoting };
                let mut rest = bytes.as_slice();
                let mut decoded = Vec::new();
                while !rest.is_empty() {
                    let (encoded, count) = quoting.encode(rest, limit);
                    assert!(count > 0);
                    assert!(encoded.len() <= limit);
                    decoded.extend(quoting.decode(&encoded).unwrap());
                    rest = &rest[count..];
                }
                assert_eq!(decoded, bytes);
            }
        }
    }
}
