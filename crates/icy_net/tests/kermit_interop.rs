use std::{path::Path, process::Stdio, time::Duration};

use async_trait::async_trait;
use icy_net::{
    Connection, ConnectionType,
    connection::raw::RawConnection,
    protocol::{Kermit, Protocol},
};
use tokio::{io::AsyncReadExt, net::TcpListener, process::Command, time::timeout};

struct Wire {
    connection: RawConnection,
    parity: bool,
    marker: bool,
    received_long_packets: usize,
    sent_long_packets: usize,
}

#[async_trait]
impl Connection for Wire {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        let count = self.connection.read(buf).await?;
        // Emulate the serial driver's parity removal on a seven-bit link.
        if self.parity {
            for byte in &mut buf[..count] {
                *byte &= 127;
            }
        }
        for &byte in &buf[..count] {
            if self.marker && byte == 32 {
                self.received_long_packets += 1;
            }
            self.marker = byte == 1;
        }
        Ok(count)
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        match timeout(Duration::from_millis(1), self.read(buf)).await {
            Ok(result) => result,
            Err(_) => Ok(0),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        if buf.starts_with(&[1, 32]) {
            self.sent_long_packets += 1;
        }
        if self.parity {
            assert!(buf.iter().all(|b| *b < 128), "binary data was not quoted for the seven-bit link");
            let encoded: Vec<u8> = buf.iter().map(|b| b | ((b.count_ones() as u8 & 1) << 7)).collect();
            self.connection.send(&encoded).await
        } else {
            self.connection.send(buf).await
        }
    }

    fn unread(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.connection.unread(buf)
    }
}

async fn transfer(executable: &str, receiving: bool, parity: bool, source: &Path, destination: &Path) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let binary = source.join("binary.bin");
    let empty = source.join("empty.bin");
    let mut command = Command::new(executable);
    command.args(["-Y", "-B", "-Q", "-q", "-i", "-P", "-e", "8192", "-j", "127.0.0.1"]);
    command.arg(listener.local_addr().unwrap().port().to_string());
    if parity {
        command.args(["-p", "e"]);
    }
    if receiving {
        command.arg("-s").arg(&binary).arg(&empty);
    } else {
        command.arg("-r");
    }
    let mut child = command
        .current_dir(destination)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("could not start ICY_KERMIT");
    let (stream, _) = timeout(Duration::from_secs(5), listener.accept()).await.unwrap().unwrap();
    let mut connection = Wire {
        connection: RawConnection::accept(stream).await.unwrap(),
        parity,
        marker: false,
        received_long_packets: 0,
        sent_long_packets: 0,
    };
    let mut protocol = Kermit::new();
    let mut state = if receiving {
        protocol.initiate_recv(&mut connection).await.unwrap()
    } else {
        protocol.initiate_send(&mut connection, &[binary.clone(), empty]).await.unwrap()
    };
    let result = timeout(Duration::from_secs(30), async {
        while !state.is_finished {
            protocol.update_transfer(&mut connection, &mut state).await?;
        }
        icy_net::Result::Ok(())
    })
    .await;
    if !matches!(result, Ok(Ok(()))) {
        child.kill().await.unwrap();
        let mut diagnostics = String::new();
        child.stderr.take().unwrap().read_to_string(&mut diagnostics).await.unwrap();
        panic!("C-Kermit transfer failed (receiving={receiving}, parity={parity}): {result:?}\n{diagnostics}\n{state:?}");
    }
    let status = timeout(Duration::from_secs(5), child.wait()).await.unwrap().unwrap();
    let mut diagnostics = String::new();
    child.stderr.take().unwrap().read_to_string(&mut diagnostics).await.unwrap();
    assert!(status.success(), "C-Kermit failed: {status}\n{diagnostics}");
    if receiving {
        assert!(connection.received_long_packets > 0, "C-Kermit must actually send extended packets");
        assert_eq!(state.recieve_state.check_size, "Checksum/4096");
        assert_eq!(state.recieve_state.finished_files.len(), 2);
        for (name, path) in &state.recieve_state.finished_files {
            assert_eq!(std::fs::read(path).unwrap(), std::fs::read(source.join(name)).unwrap());
            std::fs::remove_file(path).unwrap();
        }
    } else {
        assert!(connection.sent_long_packets > 0, "Icy Net must actually send extended packets");
        assert_eq!(state.send_state.check_size, "Checksum/4096");
        assert_eq!(state.send_state.finished_files.len(), 2);
        assert_eq!(std::fs::read(destination.join("binary.bin")).unwrap(), std::fs::read(binary).unwrap());
        assert_eq!(std::fs::metadata(destination.join("empty.bin")).unwrap().len(), 0);
    }
}

#[tokio::test]
#[ignore = "requires C-Kermit; set ICY_KERMIT to its executable"]
async fn c_kermit_binary_batches_in_both_directions() {
    let executable = std::env::var("ICY_KERMIT").expect("set ICY_KERMIT to an independent C-Kermit executable");
    let source = tempfile::tempdir().unwrap();
    let expected: Vec<u8> = (0..262144).map(|n| n as u8).chain(std::iter::repeat_n(0xff, 4096)).collect();
    std::fs::write(source.path().join("binary.bin"), expected).unwrap();
    std::fs::write(source.path().join("empty.bin"), []).unwrap();
    for receiving in [true, false] {
        for parity in [false, true] {
            let destination = tempfile::tempdir().unwrap();
            transfer(&executable, receiving, parity, source.path(), destination.path()).await;
        }
    }
}
