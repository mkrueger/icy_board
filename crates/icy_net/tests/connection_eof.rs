use std::{io, time::Duration};

use async_trait::async_trait;
use futures_util::SinkExt;
use icy_net::{
    Connection, ConnectionType, NetError,
    channel::ChannelConnection,
    raw::RawConnection,
    rlogin::{RloginConfig, RloginConnection},
    telnet::{TelnetConnection, TerminalEmulation},
    websocket::accept_websocket,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_tungstenite::tungstenite::{Message, protocol::CloseFrame, protocol::frame::coding::CloseCode};

const DEADLINE: Duration = Duration::from_secs(2);
const QUIET: Duration = Duration::from_millis(20);

fn assert_closed<T: std::fmt::Debug>(result: icy_net::Result<T>) {
    let error = result.unwrap_err();
    assert!(matches!(error.downcast_ref::<NetError>(), Some(NetError::ConnectionClosed)), "{error:?}");
}

// A timeout cannot interrupt a tight loop whose reads are immediately ready.
// Fail on the second call so a regression in the default read_u8 stays bounded.
struct BoundedRead {
    calls: usize,
    fail: bool,
}

#[async_trait]
impl Connection for BoundedRead {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    async fn read(&mut self, _: &mut [u8]) -> icy_net::Result<usize> {
        self.calls += 1;
        assert_eq!(self.calls, 1, "read retried after EOF/error");
        if self.fail {
            Err(io::Error::new(io::ErrorKind::ConnectionReset, "test transport failure").into())
        } else {
            Ok(0)
        }
    }

    async fn try_read(&mut self, _: &mut [u8]) -> icy_net::Result<usize> {
        panic!("blocking helpers must not call try_read")
    }

    async fn send(&mut self, _: &[u8]) -> icy_net::Result<()> {
        unreachable!()
    }
}

#[tokio::test]
async fn default_read_u8_stops_at_first_eof() {
    let mut connection = BoundedRead { calls: 0, fail: false };
    assert_closed(connection.read_u8().await);
    assert_eq!(connection.calls, 1);
}

#[tokio::test]
async fn default_read_exact_and_read_u8_agree_on_eof() {
    let mut connection = BoundedRead { calls: 0, fail: false };
    assert_closed(connection.read_exact(&mut [0; 1]).await);
    assert_eq!(connection.calls, 1);
}

#[tokio::test]
async fn default_read_u8_propagates_transport_error() {
    let mut connection = BoundedRead { calls: 0, fail: true };
    let error = connection.read_u8().await.unwrap_err();
    assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(connection.calls, 1);
}

#[tokio::test]
async fn channel_empty_messages_wait_for_delayed_payload_then_eof() {
    let (mut connection, mut peer) = ChannelConnection::create_pair();
    peer.send(&[]).await.unwrap();
    peer.send(&[]).await.unwrap();
    let mut read = Box::pin(connection.read_u8());
    assert!(timeout(QUIET, &mut read).await.is_err(), "empty messages are not EOF");
    peer.send(b"AB").await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(timeout(DEADLINE, read).await.unwrap().unwrap(), b'A');
    assert_eq!(connection.read_u8().await.unwrap(), b'B');
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
}

#[tokio::test]
async fn channel_read_exact_skips_empty_messages() {
    let (mut connection, mut peer) = ChannelConnection::create_pair();
    for data in [&b""[..], &b"A"[..], &b""[..], &b"BC"[..]] {
        peer.send(data).await.unwrap();
    }
    peer.shutdown().await.unwrap();
    let mut buf = [0; 3];
    timeout(DEADLINE, connection.read_exact(&mut buf)).await.unwrap().unwrap();
    assert_eq!(&buf, b"ABC");
    assert_closed(connection.read_exact(&mut [0; 1]).await);
}

#[tokio::test]
async fn channel_empty_buffer_does_not_wait_or_consume_payload() {
    let (mut connection, mut peer) = ChannelConnection::create_pair();
    assert_eq!(timeout(DEADLINE, connection.read(&mut [])).await.unwrap().unwrap(), 0);
    peer.send(b"A").await.unwrap();
    assert_eq!(connection.read(&mut []).await.unwrap(), 0);
    assert_eq!(connection.read_u8().await.unwrap(), b'A');
}

#[tokio::test]
async fn channel_try_read_still_returns_zero_when_idle_or_empty() {
    let (mut connection, mut peer) = ChannelConnection::create_pair();
    let mut buf = [0; 1];
    assert_eq!(connection.try_read(&mut buf).await.unwrap(), 0);
    peer.send(&[]).await.unwrap();
    assert_eq!(connection.try_read(&mut buf).await.unwrap(), 0);
    peer.send(b"A").await.unwrap();
    assert_eq!(connection.try_read(&mut buf).await.unwrap(), 1);
    assert_eq!(buf, *b"A");
}

async fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let peer = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    peer.set_nodelay(true).unwrap();
    (stream, peer)
}

#[tokio::test]
async fn raw_tcp_delivers_payload_then_default_read_u8_fails_on_eof() {
    let (stream, mut peer) = tcp_pair().await;
    let mut connection = RawConnection::accept(stream).await.unwrap();
    peer.write_all(b"A").await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(timeout(DEADLINE, connection.read_u8()).await.unwrap().unwrap(), b'A');
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
    assert_closed(connection.read_exact(&mut [0; 1]).await);
}

#[tokio::test]
async fn telnet_fragmented_negotiation_waits_for_payload_then_eof() {
    let (stream, mut peer) = tcp_pair().await;
    let mut connection = TelnetConnection::accept(stream).unwrap();
    assert_eq!(timeout(DEADLINE, connection.read(&mut [])).await.unwrap().unwrap(), 0);
    let mut read = Box::pin(connection.read_u8());
    // IAC WILL BINARY, then IAC NOP: every wire byte arrives separately,
    // and read_u8 also forces the parser to use a one-byte input buffer.
    for byte in [255, 251, 0, 255, 241] {
        peer.write_all(&[byte]).await.unwrap();
        assert!(timeout(QUIET, &mut read).await.is_err(), "negotiation is not EOF or payload");
    }
    let mut reply = [0; 3];
    timeout(DEADLINE, peer.read_exact(&mut reply)).await.unwrap().unwrap();
    assert_eq!(reply, [255, 253, 0]); // IAC DO BINARY
    peer.write_all(b"A\xff\xffB").await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(timeout(DEADLINE, read).await.unwrap().unwrap(), b'A');
    assert_eq!(connection.read_u8().await.unwrap(), 255);
    assert_eq!(connection.read_u8().await.unwrap(), b'B');
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
}

#[tokio::test]
async fn telnet_eof_in_partial_negotiation_is_not_an_infinite_wait() {
    let (stream, mut peer) = tcp_pair().await;
    let mut connection = TelnetConnection::accept(stream).unwrap();
    peer.write_all(&[255, 251]).await.unwrap();
    peer.shutdown().await.unwrap();
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
}

#[tokio::test]
async fn rlogin_empty_buffer_does_not_close_live_connection() {
    let (stream, mut peer) = tcp_pair().await;
    let cfg = RloginConfig {
        user_name: String::new(),
        password: String::new(),
        terminal_emulation: TerminalEmulation::Ansi,
        swapped: false,
        escape_sequence: None,
    };
    let mut connection = RloginConnection::accept(stream, cfg).await.unwrap();
    assert_eq!(timeout(DEADLINE, connection.read(&mut [])).await.unwrap().unwrap(), 0);
    peer.write_all(b"A").await.unwrap();
    peer.shutdown().await.unwrap();
    assert_eq!(timeout(DEADLINE, connection.read_u8()).await.unwrap().unwrap(), b'A');
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
}

#[tokio::test]
async fn websocket_skips_empty_and_control_messages_and_stops_at_close() {
    let (stream, peer) = tcp_pair().await;
    let (server, client) = timeout(DEADLINE, async {
        tokio::join!(accept_websocket(stream), tokio_tungstenite::client_async("ws://localhost/", peer))
    })
    .await
    .unwrap();
    let mut connection = server.unwrap();
    let (mut peer, _) = client.unwrap();
    assert_eq!(timeout(DEADLINE, connection.read(&mut [])).await.unwrap().unwrap(), 0);
    let mut read = Box::pin(connection.read_u8());
    for message in [
        Message::Binary(Vec::new().into()),
        Message::Text("".into()),
        Message::Ping(Vec::new().into()),
        Message::Ping(b"not payload".to_vec().into()),
        Message::Pong(b"not payload".to_vec().into()),
    ] {
        peer.send(message).await.unwrap();
        assert!(timeout(QUIET, &mut read).await.is_err(), "empty/control messages are not EOF or payload");
    }
    peer.send(Message::Binary(b"AB".to_vec().into())).await.unwrap();
    peer.send(Message::Text("C".into())).await.unwrap();
    peer.send(Message::Close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: "not payload".into(),
    })))
    .await
    .unwrap();
    assert_eq!(timeout(DEADLINE, read).await.unwrap().unwrap(), b'A');
    assert_eq!(connection.read_u8().await.unwrap(), b'B');
    assert_eq!(connection.read_u8().await.unwrap(), b'C');
    assert_closed(timeout(DEADLINE, connection.read_u8()).await.unwrap());
}

#[tokio::test]
async fn websocket_transport_failure_is_propagated() {
    let (stream, peer) = tcp_pair().await;
    let (server, client) = timeout(DEADLINE, async {
        tokio::join!(accept_websocket(stream), tokio_tungstenite::client_async("ws://localhost/", peer))
    })
    .await
    .unwrap();
    let mut connection = server.unwrap();
    drop(client.unwrap()); // No WebSocket close handshake.
    let error = timeout(DEADLINE, connection.read_u8()).await.unwrap().unwrap_err();
    assert!(error.downcast_ref::<tokio_tungstenite::tungstenite::Error>().is_some(), "{error:?}");
}
