use std::fmt::Display;

use crate::{Connection, NetError};

/// The SIZE field of a frame header is 15 bits wide (FTS-1026, 4).
pub const MAX_DATA_SIZE: usize = 0x7fff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinkpCommand {
    Nul = 0,
    Adr = 1,
    Pwd = 2,
    File = 3,
    Ok = 4,
    Eob = 5,
    Got = 6,
    Err = 7,
    Bsy = 8,
    Get = 9,
    Skip = 10,
}

impl BinkpCommand {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(BinkpCommand::Nul),
            1 => Some(BinkpCommand::Adr),
            2 => Some(BinkpCommand::Pwd),
            3 => Some(BinkpCommand::File),
            4 => Some(BinkpCommand::Ok),
            5 => Some(BinkpCommand::Eob),
            6 => Some(BinkpCommand::Got),
            7 => Some(BinkpCommand::Err),
            8 => Some(BinkpCommand::Bsy),
            9 => Some(BinkpCommand::Get),
            10 => Some(BinkpCommand::Skip),
            _ => None,
        }
    }
}

impl Display for BinkpCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinkpCommand::Nul => write!(f, "M_NUL"),
            BinkpCommand::Adr => write!(f, "M_ADR"),
            BinkpCommand::Pwd => write!(f, "M_PWD"),
            BinkpCommand::File => write!(f, "M_FILE"),
            BinkpCommand::Ok => write!(f, "M_OK"),
            BinkpCommand::Eob => write!(f, "M_EOB"),
            BinkpCommand::Got => write!(f, "M_GOT"),
            BinkpCommand::Err => write!(f, "M_ERR"),
            BinkpCommand::Bsy => write!(f, "M_BSY"),
            BinkpCommand::Get => write!(f, "M_GET"),
            BinkpCommand::Skip => write!(f, "M_SKIP"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    Command(BinkpCommand, String),
    Data(Vec<u8>),
}

impl Frame {
    pub fn command(command: BinkpCommand, argument: impl Into<String>) -> Self {
        Frame::Command(command, argument.into())
    }

    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        let (flag, data) = match self {
            Frame::Command(command, argument) => {
                let mut data = Vec::with_capacity(1 + argument.len());
                data.push(*command as u8);
                data.extend_from_slice(argument.as_bytes());
                (0x8000, data)
            }
            Frame::Data(data) => (0, data.clone()),
        };
        if data.len() > MAX_DATA_SIZE {
            return Err(NetError::BinkpFrameTooLarge(data.len()).into());
        }
        let header = flag | data.len() as u16;
        let mut bytes = Vec::with_capacity(2 + data.len());
        bytes.extend_from_slice(&header.to_be_bytes());
        bytes.extend_from_slice(&data);
        Ok(bytes)
    }

    /// Reads one frame. Empty frames are legal but carry nothing, so they are skipped.
    pub async fn read(connection: &mut dyn Connection) -> crate::Result<Frame> {
        loop {
            let mut header = [0u8; 2];
            connection.read_exact(&mut header).await?;
            let header = u16::from_be_bytes(header);
            let is_command = header & 0x8000 != 0;
            let size = (header & 0x7fff) as usize;
            if size == 0 {
                continue;
            }
            let mut data = vec![0u8; size];
            connection.read_exact(&mut data).await?;

            if !is_command {
                return Ok(Frame::Data(data));
            }
            let Some(command) = BinkpCommand::from_u8(data[0]) else {
                return Err(NetError::UnknownBinkpCommand(data[0]).into());
            };
            let argument = &data[1..];
            // The argument may or may not be terminated; a trailing null is not part of it.
            let argument = argument.strip_suffix(&[0]).unwrap_or(argument);
            return Ok(Frame::Command(command, String::from_utf8_lossy(argument).to_string()));
        }
    }

    pub async fn send(&self, connection: &mut dyn Connection) -> crate::Result<()> {
        connection.send(&self.to_bytes()?).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ChannelConnection;

    #[test]
    fn test_empty_command_is_two_octets_of_header_and_the_id() {
        assert_eq!(Frame::command(BinkpCommand::Ok, "").to_bytes().unwrap(), vec![0x80, 0x01, 4]);
    }

    #[test]
    fn test_command_argument_follows_the_id() {
        assert_eq!(
            Frame::command(BinkpCommand::Nul, "TEST").to_bytes().unwrap(),
            vec![0x80, 0x05, 0, b'T', b'E', b'S', b'T']
        );
    }

    #[test]
    fn test_data_frame_leaves_the_command_bit_clear() {
        assert_eq!(Frame::Data(vec![1, 2, 3]).to_bytes().unwrap(), vec![0x00, 0x03, 1, 2, 3]);
    }

    #[test]
    fn test_a_frame_larger_than_the_size_field_is_refused() {
        assert!(Frame::Data(vec![0; MAX_DATA_SIZE + 1]).to_bytes().is_err());
    }

    async fn read_back(bytes: &[u8]) -> crate::Result<Frame> {
        let (mut peer, mut connection) = ChannelConnection::create_pair();
        peer.send(bytes).await?;
        Frame::read(&mut connection).await
    }

    #[tokio::test]
    async fn test_reading_a_command_yields_its_argument() {
        let frame = read_back(&Frame::command(BinkpCommand::Adr, "21:1/100@fsxnet").to_bytes().unwrap())
            .await
            .unwrap();
        assert_eq!(frame, Frame::Command(BinkpCommand::Adr, "21:1/100@fsxnet".to_string()));
    }

    #[tokio::test]
    async fn test_a_terminating_null_is_not_part_of_the_argument() {
        let frame = read_back(&[0x80, 0x06, 0, b'T', b'E', b'S', b'T', 0]).await.unwrap();
        assert_eq!(frame, Frame::Command(BinkpCommand::Nul, "TEST".to_string()));
    }

    #[tokio::test]
    async fn test_reading_a_data_frame_yields_its_bytes() {
        let frame = read_back(&Frame::Data(vec![1, 2, 3]).to_bytes().unwrap()).await.unwrap();
        assert_eq!(frame, Frame::Data(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_an_empty_frame_is_dropped_and_the_next_one_is_returned() {
        let mut bytes = vec![0x80, 0x00];
        bytes.extend(Frame::command(BinkpCommand::Eob, "").to_bytes().unwrap());
        assert_eq!(read_back(&bytes).await.unwrap(), Frame::Command(BinkpCommand::Eob, String::new()));
    }

    #[tokio::test]
    async fn test_an_unknown_command_id_is_an_error() {
        assert!(read_back(&[0x80, 0x01, 99]).await.is_err());
    }
}
