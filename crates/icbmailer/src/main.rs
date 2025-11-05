use std::{fs, time::Duration};

use icy_net::{
    Connection,
    pattern_recognizer::PatternRecognizer,
    telnet::{TelnetConnection, TermCaps, TerminalEmulation},
    zconnect::{
        BlockCode, EndTransmission, ZConnectBlock, ZConnectState,
        commands::{Execute, ZConnectCommandBlock, mails},
        header::{Acer, TransferProtocol, ZConnectHeaderBlock},
    },
};
use tokio::time::Instant;

async fn try_connect(connection: &mut dyn Connection, pattern: Vec<&[u8]>, send: &[u8]) -> bool {
    let mut buf = [0; 1024];

    let mut login_pattern = pattern.iter().map(|p| PatternRecognizer::from(*p, true)).collect::<Vec<_>>();
    let mut instant = Instant::now();
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
            // print!("{}", *b as char);
            for p in &mut login_pattern {
                if p.push_ch(*b) {
                    println!("got trigger string SEND {}", String::from_utf8_lossy(send));
                    connection.send(send).await.unwrap();
                    return true;
                }
            }
        }

        if size > 0 && instant.elapsed() > Duration::from_secs(1) {
            connection.send(b"\r").await.unwrap();
            instant = Instant::now();
        }
    }
}

enum Begin {
    ZConnect,
    ZModemSend,
    ZModem,
}

async fn begin_zconnect(connection: &mut dyn Connection) -> Begin {
    let mut buf = [0; 1024];
    let mut zconnect_pattern = PatternRecognizer::from(b"begin", true);
    let mut zmodem_send_pattern = PatternRecognizer::from(b"B01", false); // ZRINIT for send starts with B01
    let mut zmodem_recv_pattern = PatternRecognizer::from(b"B00", false); // ZRQINIT starts with B00

    let mut instant = Instant::now();
    println!("Waiting for ZConnect or ZModem");
    loop {
        let size = connection.read(&mut buf).await.unwrap();

        for b in &buf[0..size] {
            print!("{}", *b as char);
            if zconnect_pattern.push_ch(*b) {
                println!("Begin ZConnect");
                return Begin::ZConnect;
            }
            if zmodem_send_pattern.push_ch(*b) {
                println!("Begin ZModem send");
                return Begin::ZModemSend;
            }
            if zmodem_recv_pattern.push_ch(*b) {
                println!("Begin ZModem recv");
                return Begin::ZModem;
            }
        }
        if size > 0 && instant.elapsed() > Duration::from_secs(1) {
            connection.send(b"\r").await.unwrap();
            instant = Instant::now();
        }
    }
}

struct ZConnect {
    cur_block: BlockCode,
}

pub async fn read_string(connection: &mut dyn Connection) -> String {
    let mut res = String::new();

    println!("Reading string...");
    let mut buf = [0; 1024];
    let mut last = 0;
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
            print!("{}", *b as char);
            res.push(*b as char);
            if *b == b'\r' && last == b'\r' {
                return res;
            }
            last = *b;
        }
    }
}

impl ZConnect {
    pub async fn send_block(&mut self, connection: &mut dyn Connection, block: &dyn ZConnectBlock) -> icy_net::Result<()> {
        println!("---- sending block:\n{}", block.display());
        connection.send(block.display().as_bytes()).await.unwrap();
        loop {
            let res = read_string(connection).await;
            println!("---- got block:");
            println!("{}", res);
            match ZConnectCommandBlock::parse(&res) {
                Ok(blk) => {
                    if blk.state() == ZConnectState::Ack(self.cur_block) {
                        let mut nak_block = ZConnectCommandBlock::default();
                        nak_block.set_state(ZConnectState::Tme(self.cur_block));
                        connection.send(nak_block.display().as_bytes()).await.unwrap();
                    }
                    break;
                }
                Err(err) => {
                    log::error!("Error parsing block: {}", err);
                    println!("Error parsing block: {}", err);
                    let mut nak_block = ZConnectCommandBlock::default();
                    nak_block.set_state(ZConnectState::Nak0);
                    connection.send(nak_block.display().as_bytes()).await.unwrap();
                }
            }
        }
        self.cur_block = self.cur_block.next();

        Ok(())
    }

    pub async fn recv_block(&mut self, connection: &mut dyn Connection) -> icy_net::Result<ZConnectCommandBlock> {
        loop {
            let res = read_string(connection).await;
            println!("---------------");
            println!("Received: {}", res);
            println!("---------------");

            match ZConnectCommandBlock::parse(&res) {
                Ok(blk) => {
                    if let ZConnectState::Block(block_code) = blk.state() {
                        let mut ack_block = ZConnectCommandBlock::default();
                        ack_block.set_state(ZConnectState::Ack(block_code));
                        connection.send(ack_block.display().as_bytes()).await.unwrap();
                        self.cur_block = block_code.next();
                        // read TME
                        read_string(connection).await;
                    }
                    return Ok(blk);
                }
                Err(err) => {
                    println!("Error parsing block: {}", err);
                    log::error!("Error parsing block: {}", err);
                    let mut nak_block = ZConnectCommandBlock::default();
                    nak_block.set_state(ZConnectState::Nak0);
                    connection.send(nak_block.display().as_bytes()).await.unwrap();
                    continue;
                }
            }
        }
    }
}
/*
async fn zconnect_login(connection: &mut dyn Connection) -> bool {
    try_connect(connection, vec![b"ogin", b"ame"], b"zconnect\r").await &&
    try_connect(connection, vec![b"word", b"wort"], b"0zconnec\r").await
}*/

async fn janus_login(connection: &mut dyn Connection, system: &str, password: &str) -> bool {
    println!("logging in...");
    if !try_connect(connection, vec![b"Username:"], b"JANUS\r").await {
        return false;
    }
    println!("send system name...");
    if !try_connect(connection, vec![b"Systemname:"], system.as_bytes()).await {
        return false;
    }
    println!("send password...");
    if !try_connect(connection, vec![b"word", b"wort"], password.as_bytes()).await {
        return false;
    }
    println!("logged in");
    true
}

#[tokio::main]
async fn main() {
    let caps = TermCaps {
        window_size: (80, 25),
        terminal: TerminalEmulation::Ascii,
    };

    let mut connection = TelnetConnection::open("ip", caps, Duration::from_secs(2)).await.unwrap();
    if !janus_login(&mut connection, "name\r", "pw\r").await {
        return;
    }
    loop {
        match begin_zconnect(&mut connection).await {
            Begin::ZConnect => {
                let mut header = ZConnectHeaderBlock::default();
                header.add_acer(0, Acer::ZIP);
                header.add_acer(0, Acer::Arj);
                header.add_acer(0, Acer::ZOO);
                header.add_acer(0, Acer::LHArc);
                header.add_acer(0, Acer::LHA);
                header.add_iso2(0, "V.32");
                header.set_password("IcyBoardTest");
                header.set_port(0);
                header.add_protocol(0, TransferProtocol::ZModem);
                header.add_protocol(0, TransferProtocol::ZModem8k);
                header.set_system("Icy Shadow BBS");
                header.set_sysop("SYSOP");
                header.add_phone(0, "1234567890");

                let mut zcon = ZConnect { cur_block: BlockCode::Block1 };
                println!("send header...");

                zcon.send_block(&mut connection, &header).await.unwrap();
                let _header = zcon.recv_block(&mut connection).await.unwrap();

                println!("send get request...");
                let get_mail = ZConnectCommandBlock::default().get(mails::ALL);
                zcon.send_block(&mut connection, &get_mail).await.unwrap();

                let _header = zcon.recv_block(&mut connection).await.unwrap();

                let execute = ZConnectCommandBlock::default().execute(Execute::Yes);
                println!("send execute now...");
                zcon.send_block(&mut connection, &execute).await.unwrap();

                let _header = zcon.recv_block(&mut connection).await.unwrap();

                println!("send eot & begin...");
                zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
                zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
                zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
                zcon.send_block(&mut connection, &ZConnectCommandBlock::BEG5).await.unwrap();

                println!("wait for receive...");
                for _ in 0..3 {
                    let header = zcon.recv_block(&mut connection).await.unwrap();
                    println!("Received block: {}", header.display());
                    if header.state() != ZConnectState::Eot(EndTransmission::Prot5) {
                        let execute = ZConnectCommandBlock::default().logoff();
                        zcon.send_block(&mut connection, &execute).await.unwrap();
                        return;
                    }
                }
                initate_download(&mut zcon, &mut connection).await;
                return;
            }
            Begin::ZModemSend => {
                let mut zcon = ZConnect { cur_block: BlockCode::Block1 };
                initate_upload(&mut zcon, &mut connection).await;
                continue;
            }
            Begin::ZModem => {
                let mut zcon = ZConnect { cur_block: BlockCode::Block1 };
                initate_download(&mut zcon, &mut connection).await;
                return;
            }
        }
    }
}

async fn initate_download(zcon: &mut ZConnect, connection: &mut dyn Connection) {
    println!("initiate transfer...");

    let mut proto = icy_net::protocol::TransferProtocolType::ZModem.create();
    let mut ts = proto.initiate_recv(connection).await.unwrap();

    while !ts.is_finished {
        if let Err(err) = proto.update_transfer(connection, &mut ts).await {
            log::error!("ZModem error: {}", err);
        }
        println!("transferred: {}\r", ts.recieve_state.total_bytes_transfered);
    }
    println!("transfer finished");
    for (file, path) in &ts.recieve_state.finished_files {
        println!("Copy: {} to {}", path.display(), file);
        let _ = fs::copy(path, file);
    }
    let execute = ZConnectCommandBlock::default().logoff();
    zcon.send_block(connection, &execute).await.unwrap();
    let header = zcon.recv_block(connection).await.unwrap();
    println!("Received block: {}", header.display());
}

async fn initate_upload(_zcon: &mut ZConnect, connection: &mut dyn Connection) {
    println!("initiate transfer...");

    let mut proto = icy_net::protocol::TransferProtocolType::ZModem.create();
    let mut ts = proto.initiate_send(connection, &[]).await.unwrap();

    while !ts.is_finished {
        if let Err(err) = proto.update_transfer(connection, &mut ts).await {
            log::error!("ZModem error: {}", err);
        }
        println!("transferred: {}\r", ts.recieve_state.total_bytes_transfered);
    }
    println!("transfer finished");
}
