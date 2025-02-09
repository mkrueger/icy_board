use std::{os::unix::thread, time::Duration};

use regex::Regex;
use tokio::time::sleep;

use crate::Connection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalProgram {
    IcyTerm,
    SyncTerm,
    Unknown,
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalCaps {
    pub program: TerminalProgram,
    pub term_size: (u16, u16),
    pub is_utf8: bool,
    pub rip_version: Option<String>,
}

impl TerminalCaps {
    pub const LOCAL: TerminalCaps = TerminalCaps {
        program: TerminalProgram::Unknown,
        term_size: (80, 25),
        is_utf8: false,
        rip_version: None,
    };

    pub async fn detect(com: &mut dyn Connection) -> crate::Result<Self> {
        com.send(b"\x1B[0c").await?;

        let mut buf = [0; 1024];
        let mut program = TerminalProgram::Unknown;
        let instant = std::time::Instant::now();
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            program = if result.contains("73;99;121;84;101;114;109") {
                TerminalProgram::IcyTerm
            } else if result.contains("67;84;101;114") {
                TerminalProgram::SyncTerm
            } else {
                TerminalProgram::Name(result)
            };
            break;
        }

        com.send(b"\x1B[999;999H\x1B[6n").await?;
        let instant = std::time::Instant::now();
        let mut term_size = (80, 25);
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            if result.ends_with("R") {
                term_size = parse_cursor_pos(result);
            }
            break;
        }
        com.send(b"\x1B[!\x07\x07\x07").await?;
        let mut rip_version = None;
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            rip_version = parse_rip_version(&result);
            break;
        }

        com.send(b"\x1B[1;1H\x01\xF6\x1C\x1B[6n").await?;
        let instant = std::time::Instant::now();
        let mut is_utf8 = false;
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            if result.ends_with("R") {
                is_utf8 = parse_cursor_pos(result).0 == 1;
            }
            break;
        }
        Ok(Self {
            program,
            term_size,
            is_utf8,
            rip_version,
        })
    }
}

lazy_static::lazy_static! {
    static ref RIP_REGEX:Regex = Regex::new("RIPSCRIP(\\d+)").unwrap();
}
fn parse_rip_version(data: &str) -> Option<String> {
    if let Some(caps) = RIP_REGEX.captures(data) {
        if let Some(r) = caps.get(1) {
            return Some(r.as_str().to_string());
        }
    }
    None
}

fn parse_cursor_pos(result: String) -> (u16, u16) {
    let mut y = 0;
    let mut x = 0;
    let mut parse_x = false;
    for b in result.chars() {
        if let Some(digit) = b.to_digit(10) {
            if parse_x {
                x = x * 10 + digit as u16;
            } else {
                y = y * 10 + digit as u16;
            }
        }
        if b == ';' {
            parse_x = true;
        }
    }
    (x, y)
}

#[cfg(test)]
mod test {
    use crate::termcap_detect::parse_rip_version;

    #[test]
    fn test_parse_rip() {
        assert_eq!(parse_rip_version("NEEALEG"), None);
        assert_eq!(parse_rip_version("RIPSCRIP015410\0"), Some("015410".to_string()));
    }
}
