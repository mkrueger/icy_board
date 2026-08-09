use std::{path::PathBuf, time::Duration};

use crate::{Connection, raw::RawConnection};

use super::{BatchResult, BinkpIdentity, OutboundFile, RemoteInfo, originate_session, transfer_batch};

/// The port fidonet technology networks reserved for binkp.
pub const DEFAULT_PORT: u16 = 24554;

/// Everything a session needs to know before it can be opened, because none of
/// it can be asked for once the connection is up.
#[derive(Clone, Debug)]
pub struct PollRequest {
    pub host: String,
    pub port: u16,
    pub identity: BinkpIdentity,

    /// The address that has to answer, empty to accept whoever picks up.
    pub called: String,
    pub password: String,

    pub outbound: Vec<PathBuf>,
    pub inbound: PathBuf,

    pub connect_timeout: Duration,
    pub session_timeout: Duration,
}

impl Default for PollRequest {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: DEFAULT_PORT,
            identity: BinkpIdentity::default(),
            called: String::new(),
            password: String::new(),
            outbound: Vec::new(),
            inbound: PathBuf::new(),
            connect_timeout: Duration::from_secs(30),
            session_timeout: Duration::from_secs(300),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PollResult {
    pub remote: RemoteInfo,
    pub batch: BatchResult,
}

/// Calls a system and exchanges one batch of mail with it.
pub async fn poll(request: &PollRequest) -> crate::Result<PollResult> {
    let mut connection = RawConnection::open(&(request.host.as_str(), request.port), request.connect_timeout).await?;
    poll_over(&mut connection, request).await
}

/// The part of a poll that does not care how the connection was obtained.
pub async fn poll_over(connection: &mut dyn Connection, request: &PollRequest) -> crate::Result<PollResult> {
    let remote = originate_session(connection, &request.identity, &request.called, &request.password).await?;

    let mut files = Vec::with_capacity(request.outbound.len());
    for path in &request.outbound {
        files.push(OutboundFile::open(path).await?);
    }
    let batch = transfer_batch(connection, files, &request.inbound, request.session_timeout).await?;

    // The mail is already across, so a refused goodbye is not worth a failed poll.
    let _ = connection.shutdown().await;
    Ok(PollResult { remote, batch })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        binkp::{BinkpCommand, Frame},
        channel::ChannelConnection,
    };

    #[tokio::test]
    async fn test_a_poll_carries_on_into_the_batch_once_the_password_was_taken() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet").send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Ok, "secure").send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            while !matches!(Frame::read(&mut peer).await.unwrap(), Frame::Command(BinkpCommand::Eob, _)) {}
        });

        let request = PollRequest {
            identity: BinkpIdentity {
                addresses: vec!["21:1/100@fsxnet".to_string()],
                ..Default::default()
            },
            called: "21:1/1@fsxnet".to_string(),
            password: "secret".to_string(),
            inbound,
            session_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        let result = poll_over(&mut ours, &request).await.unwrap();

        assert_eq!(result.remote.addresses, vec!["21:1/1@fsxnet".to_string()]);
        assert!(result.remote.secure);
        assert_eq!(result.batch, BatchResult::default());
    }

    #[tokio::test]
    async fn test_a_poll_finds_the_system_it_was_pointed_at() {
        let directory = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut answering = RawConnection::accept(stream).await.unwrap();
            Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet").send(&mut answering).await.unwrap();
            Frame::command(BinkpCommand::Ok, "").send(&mut answering).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut answering).await.unwrap();
            while !matches!(Frame::read(&mut answering).await.unwrap(), Frame::Command(BinkpCommand::Eob, _)) {}
        });

        let request = PollRequest {
            host: "127.0.0.1".to_string(),
            port,
            identity: BinkpIdentity {
                addresses: vec!["21:1/100@fsxnet".to_string()],
                ..Default::default()
            },
            called: "21:1/1@fsxnet".to_string(),
            inbound: directory.path().join("in"),
            session_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        let result = poll(&request).await.unwrap();

        assert_eq!(result.remote.addresses, vec!["21:1/1@fsxnet".to_string()]);
        assert_eq!(result.batch, BatchResult::default());
    }
}
