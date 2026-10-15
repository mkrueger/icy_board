use std::time::Duration;

use async_trait::async_trait;
use icy_net::{Connection, ConnectionType, connection::channel::ChannelConnection, protocol::TransferProtocolType};
use tokio::time::{Instant, sleep, timeout};

struct DelayedConnection {
    inner: ChannelConnection,
    delay: Duration,
    writes: usize,
    reads: usize,
    bytes: usize,
    classic: bool,
    long_packets: usize,
}

#[async_trait]
impl Connection for DelayedConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        self.reads += 1;
        self.inner.read(buf).await
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        self.inner.try_read(buf).await
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.writes += 1;
        self.bytes += buf.len();
        sleep(self.delay).await;
        if self.classic && buf.len() >= 17 && buf[..4] == [1, 48, 32, b'S'] {
            // Emulate a legacy peer by removing capability fields from Send-Init.
            let mut init = buf[..13].to_vec();
            init[1] = 44;
            let sum: usize = init[1..].iter().map(|b| *b as usize).sum();
            init.push(32 + ((sum + (sum / 64 % 4)) % 64) as u8);
            init.push(13);
            return self.inner.send(&init).await;
        }
        if buf.len() >= 7 && buf[..2] == [1, 32] {
            self.long_packets += 1;
        }
        self.inner.send(buf).await
    }

    fn unread(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.inner.unread(buf)
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        self.inner.shutdown().await
    }
}

#[tokio::test(start_paused = true)]
async fn long_packets_remove_the_classic_latency_bottleneck() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("probe.bin");
    for (name, data) in [
        ("ASCII", (0..65536).map(|n| b'0' + (n % 64) as u8).collect::<Vec<_>>()),
        ("binary", (0..65536).map(|n| n as u8).collect::<Vec<_>>()),
    ] {
        std::fs::write(&path, &data).unwrap();
        for rtt_ms in [20, 100] {
            let mut classic_elapsed = Duration::ZERO;
            for (kind, classic) in [
                (TransferProtocolType::Kermit, true),
                (TransferProtocolType::Kermit, false),
                (TransferProtocolType::YModem, false),
            ] {
                let (a, b) = ChannelConnection::create_pair();
                let wrap = |inner| DelayedConnection {
                    inner,
                    delay: Duration::from_millis(rtt_ms / 2),
                    writes: 0,
                    reads: 0,
                    bytes: 0,
                    classic,
                    long_packets: 0,
                };
                let (mut a, mut b) = (wrap(a), wrap(b));
                let (mut sender, mut receiver) = (kind.create(), kind.create());
                let mut recv = receiver.initiate_recv(&mut b).await.unwrap();
                let start = Instant::now();
                let mut sent = sender.initiate_send(&mut a, std::slice::from_ref(&path)).await.unwrap();
                let mut elapsed = Duration::ZERO;
                timeout(Duration::from_secs(300), async {
                    tokio::try_join!(
                        async {
                            while !sent.is_finished {
                                sender.update_transfer(&mut a, &mut sent).await?;
                            }
                            elapsed = start.elapsed();
                            a.shutdown().await
                        },
                        async {
                            while !recv.is_finished {
                                receiver.update_transfer(&mut b, &mut recv).await?;
                            }
                            icy_net::Result::Ok(())
                        }
                    )
                    .unwrap();
                })
                .await
                .unwrap();
                assert_eq!(recv.recieve_state.finished_files.len(), 1);
                let received = &recv.recieve_state.finished_files[0].1;
                assert_eq!(std::fs::read(received).unwrap(), data);
                std::fs::remove_file(received).unwrap();
                assert_eq!(sent.send_state.warnings, 0);
                assert_eq!(recv.recieve_state.warnings, 0);
                if kind == TransferProtocolType::Kermit {
                    assert!(b.reads <= a.writes * 6, "packet reads must be batched, not one read per byte");
                    if classic {
                        assert_eq!(a.long_packets, 0);
                        classic_elapsed = elapsed;
                    } else {
                        assert!(a.long_packets > 0);
                        assert!(
                            elapsed * 25 < classic_elapsed,
                            "long packets must improve latency-limited throughput by more than 25x"
                        );
                        assert!(data.len() as f64 / elapsed.as_secs_f64() > if rtt_ms == 20 { 90_000.0 } else { 18_000.0 });
                    }
                }
                println!(
                    "{kind:?} classic={classic} {name} RTT={rtt_ms}ms: {:.0} B/s in {:.3}s, {} sends, {} wire bytes, {} receiver reads, warnings={}/{}",
                    data.len() as f64 / elapsed.as_secs_f64(),
                    elapsed.as_secs_f64(),
                    a.writes,
                    a.bytes,
                    b.reads,
                    sent.send_state.warnings,
                    recv.recieve_state.warnings
                );
            }
        }
    }
}
