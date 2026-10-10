use std::{process::Stdio, time::Duration};

use icy_net::{
    connection::raw::RawConnection,
    protocol::{Protocol, Zmodem},
};
use tokio::{io::AsyncReadExt, net::TcpListener, process::Command, time::timeout};

#[tokio::test]
#[ignore = "requires an independent lrzsz sender; set ICY_LSZ to its executable"]
async fn receive_from_lrzsz() {
    let sender = std::env::var("ICY_LSZ").expect("set ICY_LSZ to the lrzsz sz/lsz executable");
    let source = tempfile::tempdir().unwrap();
    let file = source.path().join("interop.bin");
    let expected: Vec<u8> = (0..32768).map(|n| (n % 256) as u8).collect();
    std::fs::write(&file, &expected).unwrap();
    for crc16 in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut command = Command::new(&sender);
        command.args(["--binary", "-vv", "--tcp-client"]);
        command.arg(listener.local_addr().unwrap().to_string());
        if crc16 {
            command.arg("--16-bit-crc");
        }
        let mut child = command
            .arg(&file)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (stream, _) = timeout(Duration::from_secs(5), listener.accept()).await.unwrap().unwrap();
        let mut connection = RawConnection::accept(stream).await.unwrap();
        let mut receiver = Zmodem::new(1024);
        let mut transfer = receiver.initiate_recv(&mut connection).await.unwrap();
        let result = timeout(Duration::from_secs(15), async {
            while !transfer.is_finished {
                receiver.update_transfer(&mut connection, &mut transfer).await.unwrap();
            }
        })
        .await;
        if result.is_err() {
            child.kill().await.unwrap();
            let mut diagnostics = String::new();
            child.stderr.take().unwrap().read_to_string(&mut diagnostics).await.unwrap();
            panic!(
                "independent sender stalled (CRC16={crc16}): {:?}\n{diagnostics}",
                transfer.recieve_state.output_log
            );
        }
        assert_eq!(transfer.recieve_state.errors, 3, "{:?}", transfer.recieve_state.output_log);
        assert_eq!(transfer.recieve_state.finished_files.len(), 1);
        let (name, received) = &transfer.recieve_state.finished_files[0];
        assert_eq!(name, "interop.bin");
        assert_eq!(std::fs::read(received).unwrap(), expected);
        std::fs::remove_file(received).unwrap();
        let status = timeout(Duration::from_secs(5), child.wait()).await.unwrap().unwrap();
        assert!(status.success(), "independent sender reported {status}");
    }
}
