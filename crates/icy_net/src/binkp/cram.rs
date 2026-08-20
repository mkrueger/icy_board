//! The challenge response binkp offers instead of sending the password in clear.

const BLOCK_SIZE: usize = 64;

/// HMAC-MD5 as defined by RFC 2104, which is what binkp keys with the session
/// password and feeds the remote's challenge.
pub fn cram_md5(password: &[u8], challenge: &[u8]) -> [u8; 16] {
    let mut key = [0u8; BLOCK_SIZE];
    if password.len() > BLOCK_SIZE {
        key[..16].copy_from_slice(&md5::compute(password).0);
    } else {
        key[..password.len()].copy_from_slice(password);
    }

    let mut inner = md5::Context::new();
    inner.consume(key.iter().map(|b| b ^ 0x36).collect::<Vec<u8>>());
    inner.consume(challenge);
    let inner = inner.finalize();

    let mut outer = md5::Context::new();
    outer.consume(key.iter().map(|b| b ^ 0x5c).collect::<Vec<u8>>());
    outer.consume(inner.0);
    outer.finalize().0
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_first_hmac_md5_vector_of_rfc_2202() {
        assert_eq!(to_hex(&cram_md5(&[0x0b; 16], b"Hi There")), "9294727a3638bb1c13f48ef8158bfc9d");
    }

    #[test]
    fn test_the_second_hmac_md5_vector_of_rfc_2202() {
        assert_eq!(to_hex(&cram_md5(b"Jefe", b"what do ya want for nothing?")), "750c783e6ab0b503eaa86e310a5db738");
    }

    #[test]
    fn test_a_key_longer_than_the_block_is_hashed_first() {
        assert_eq!(
            to_hex(&cram_md5(&[0xaa; 80], b"Test Using Larger Than Block-Size Key - Hash Key First")),
            "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd"
        );
    }

    #[test]
    fn test_hex_survives_a_round_trip() {
        assert_eq!(from_hex("00ff10").unwrap(), vec![0x00, 0xff, 0x10]);
        assert_eq!(from_hex("0f0"), None);
        assert_eq!(from_hex("zz"), None);
    }
}
