use crate::crc;

pub mod commands;
pub mod header;

// const Z_CONNECT_USER: &str = "zconnect";
// const Z_CONNECT_PWD: &str = "0zconnec";

// const BEGIN : &str = "BEGIN";

#[derive(Clone, Copy, PartialEq)]
pub enum BlockCode {
    Block1,
    Block2,
    Block3,
    Block4,
}
impl BlockCode {
    pub fn next(&self) -> BlockCode {
        match self {
            BlockCode::Block1 => BlockCode::Block2,
            BlockCode::Block2 => BlockCode::Block3,
            BlockCode::Block3 => BlockCode::Block4,
            BlockCode::Block4 => BlockCode::Block1,
        }
    }
}

impl std::fmt::Display for BlockCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockCode::Block1 => write!(f, "1"),
            BlockCode::Block2 => write!(f, "2"),
            BlockCode::Block3 => write!(f, "3"),
            BlockCode::Block4 => write!(f, "4"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProtocolTransition {
    Prot5,
    Prot6,
}

impl std::fmt::Display for ProtocolTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolTransition::Prot5 => write!(f, "5"),
            ProtocolTransition::Prot6 => write!(f, "6"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum EndTransmission {
    End1,
    End2,
    End3,
    End4,
    Prot5,
    Prot6,
}

impl std::fmt::Display for EndTransmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndTransmission::End1 => write!(f, "1"),
            EndTransmission::End2 => write!(f, "2"),
            EndTransmission::End3 => write!(f, "3"),
            EndTransmission::End4 => write!(f, "4"),
            EndTransmission::Prot5 => write!(f, "5"),
            EndTransmission::Prot6 => write!(f, "6"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ZConnectState {
    Block(BlockCode),
    Ack(BlockCode),
    Tme(BlockCode),
    Eot(EndTransmission),
    Begin(ProtocolTransition),
    Nak0,
}

impl Default for ZConnectState {
    fn default() -> Self {
        ZConnectState::Block(BlockCode::Block1)
    }
}

impl std::fmt::Display for ZConnectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZConnectState::Block(b) => write!(f, "BLK{}", *b),
            ZConnectState::Ack(b) => write!(f, "ACK{}", *b),
            ZConnectState::Tme(b) => write!(f, "TME{}", *b),
            ZConnectState::Eot(b) => write!(f, "EOT{}", *b),
            ZConnectState::Begin(b) => write!(f, "BEG{}", *b),
            ZConnectState::Nak0 => write!(f, "NAK0"),
        }
    }
}

pub trait ZConnectBlock {
    fn state(&self) -> ZConnectState;
    fn set_state(&mut self, state: ZConnectState);

    fn generate_lines(&self) -> Vec<String>;

    fn block(&mut self, block: BlockCode) {
        self.set_state(ZConnectState::Block(block));
    }

    fn ack(&mut self, block: BlockCode) {
        self.set_state(ZConnectState::Ack(block));
    }

    fn tme(&mut self, block: BlockCode) {
        self.set_state(ZConnectState::Tme(block));
    }

    fn eot(&mut self, block: EndTransmission) {
        self.set_state(ZConnectState::Eot(block));
    }

    fn begin(&mut self, block: ProtocolTransition) {
        self.set_state(ZConnectState::Begin(block));
    }

    fn display(&self) -> String {
        let mut lines = self.generate_lines();
        lines.push(format!("Status:{}", self.state()));
        let mut crc = u16::MAX;
        for cmd_str in &lines {
            for b in cmd_str.as_bytes() {
                crc = crate::crc::buggy_update(crc, *b);
            }
        }
        lines.push(format!("CRC:{:04X}\r", crc));

        let mut res = lines.join("\r");
        res.push('\r');
        res
    }

    fn parse_cmd(&mut self, command: &str, parameter: String) -> crate::Result<()>;

    fn parse_block(&mut self, input: &str) -> crate::Result<()> {
        let mut command = String::new();
        let mut parameter = String::new();

        let mut is_start = true;
        let mut crc = u16::MAX;
        let mut last_crc = crc;
        let mut expected_crc = u16::MAX;
        for c in input.chars() {
            if c == '\r' {
                is_start = true;
                if !command.is_empty() {
                    match command.as_str() {
                        "STATUS" => self.set_state(parse_state(&parameter)?),
                        "CRC" => {
                            expected_crc = u16::from_str_radix(&parameter, 16)?;
                            crc = last_crc;
                        }
                        _ => {
                            self.parse_cmd(&command, parameter)?;
                        }
                    }
                    command.clear();
                    parameter = String::new();
                    last_crc = crc;
                }
                continue;
            }
            if c < ' ' || c >= '\x7F' {
                continue;
            }
            crc = crc::buggy_update(crc, c as u8);
            if is_start {
                if c == ':' {
                    is_start = false;
                } else {
                    command.push(c.to_ascii_uppercase());
                }
            } else {
                parameter.push(c);
            }
        }

        if crc != expected_crc {
            return Err("CRC mismatch".into());
        }
        Ok(())
    }
}

fn parse_state(parameter: &str) -> crate::Result<ZConnectState> {
    // My excuse: this was generated by an AI :)
    match parameter {
        "BLK1" => Ok(ZConnectState::Block(BlockCode::Block1)),
        "BLK2" => Ok(ZConnectState::Block(BlockCode::Block2)),
        "BLK3" => Ok(ZConnectState::Block(BlockCode::Block3)),
        "BLK4" => Ok(ZConnectState::Block(BlockCode::Block4)),
        "ACK1" => Ok(ZConnectState::Ack(BlockCode::Block1)),
        "ACK2" => Ok(ZConnectState::Ack(BlockCode::Block2)),
        "ACK3" => Ok(ZConnectState::Ack(BlockCode::Block3)),
        "ACK4" => Ok(ZConnectState::Ack(BlockCode::Block4)),
        "TME1" => Ok(ZConnectState::Tme(BlockCode::Block1)),
        "TME2" => Ok(ZConnectState::Tme(BlockCode::Block2)),
        "TME3" => Ok(ZConnectState::Tme(BlockCode::Block3)),
        "TME4" => Ok(ZConnectState::Tme(BlockCode::Block4)),
        "EOT1" => Ok(ZConnectState::Eot(EndTransmission::End1)),
        "EOT2" => Ok(ZConnectState::Eot(EndTransmission::End2)),
        "EOT3" => Ok(ZConnectState::Eot(EndTransmission::End3)),
        "EOT4" => Ok(ZConnectState::Eot(EndTransmission::End4)),
        "EOT5" => Ok(ZConnectState::Eot(EndTransmission::Prot5)),
        "EOT6" => Ok(ZConnectState::Eot(EndTransmission::Prot6)),
        "BEG5" => Ok(ZConnectState::Begin(ProtocolTransition::Prot5)),
        "BEG6" => Ok(ZConnectState::Begin(ProtocolTransition::Prot6)),
        "NAK0" => Ok(ZConnectState::Nak0),
        _ => Err("Invalid state".into()),
    }
}
