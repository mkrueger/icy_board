use std::time::Duration;

use icy_net::{
    pattern_recognizer::PatternRecognizer,
    telnet::{TelnetConnection, TermCaps, TerminalEmulation},
    zconnect::{
        commands::{mails, Execute, ZConnectCommandBlock},
        header::{Acer, TransferProtocol, ZConnectHeaderBlock},
        BlockCode, EndTransmission, ZConnectBlock, ZConnectState,
    },
    Connection,
};

async fn login(connection: &mut dyn Connection) -> bool {
    let mut buf = [0; 1024];

    let mut login_pattern = [PatternRecognizer::from(b"ogin", true), PatternRecognizer::from(b"ame", true)];
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
            print!("{}", *b as char);
            for p in &mut login_pattern {
                if p.push_ch(*b) {
                    connection.send(b"zconnect\r").await.unwrap();
                    return true;
                }
            }
        }

        if size > 0 {
            connection.send(b"\r").await.unwrap();
        }
    }
}

async fn password(connection: &mut dyn Connection) -> bool {
    let mut buf = [0; 1024];

    let mut password_pattern = [PatternRecognizer::from(b"word", true), PatternRecognizer::from(b"wort", true)];
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
            print!("{}", *b as char);
            for p in &mut password_pattern {
                if p.push_ch(*b) {
                    connection.send(b"0zconnec\r").await.unwrap();
                    return true;
                }
            }
        }
    }
}

async fn begin_zconnect(connection: &mut dyn Connection) -> bool {
    let mut buf = [0; 1024];
    let mut password_pattern = PatternRecognizer::from(b"begin", true);
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
            print!("{}", *b as char);
            if password_pattern.push_ch(*b) {
                return true;
            }
        }
    }
}

struct ZConnect {
    cur_block: BlockCode,
}

pub async fn read_string(connection: &mut dyn Connection) -> String {
    let mut res = String::new();

    let mut buf = [0; 1024];
    let mut last = 0;
    loop {
        let size = connection.read(&mut buf).await.unwrap();
        for b in &buf[0..size] {
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
        connection.send(block.display().as_bytes()).await.unwrap();
        loop {
            let res = read_string(connection).await;
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

#[tokio::main]
async fn main() {
    let caps = TermCaps {
        window_size: (80, 25),
        terminal: TerminalEmulation::Ascii,
    };

    let mut connection = TelnetConnection::open(&"ADDRESS", caps, Duration::from_secs(2)).await.unwrap();
    if !login(&mut connection).await {
        return;
    }
    if !password(&mut connection).await {
        return;
    }
    if !begin_zconnect(&mut connection).await {
        return;
    }
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

    zcon.send_block(&mut connection, &header).await.unwrap();
    let header = zcon.recv_block(&mut connection).await.unwrap();
    println!("Received header: {}", header.display());

    let get_mail = ZConnectCommandBlock::default().get(mails::ALL);
    zcon.send_block(&mut connection, &get_mail).await.unwrap();

    let header = zcon.recv_block(&mut connection).await.unwrap();
    println!("Received block: {}", header.display());

    let execute = ZConnectCommandBlock::default().execute(Execute::Yes);
    zcon.send_block(&mut connection, &execute).await.unwrap();

    let header = zcon.recv_block(&mut connection).await.unwrap();
    println!("Received block: {}", header.display());

    zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
    zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
    zcon.send_block(&mut connection, &ZConnectCommandBlock::EOT4).await.unwrap();
    zcon.send_block(&mut connection, &ZConnectCommandBlock::BEG5).await.unwrap();

    for i in 0..3 {
        let header = zcon.recv_block(&mut connection).await.unwrap();
        println!("Received block: {}", header.display());
        if header.state() != ZConnectState::Eot(EndTransmission::Prot5) {
            let execute = ZConnectCommandBlock::default().logoff();
            zcon.send_block(&mut connection, &execute).await.unwrap();
            return;
        }
    }
    let mut proto = icy_net::protocol::TransferProtocolType::ZModem.create();
    let mut ts = proto.initiate_recv(&mut connection).await.unwrap();

    while !ts.is_finished {
        proto.update_transfer(&mut connection, &mut ts).await.unwrap();
    }
    let execute = ZConnectCommandBlock::default().logoff();
    zcon.send_block(&mut connection, &execute).await.unwrap();
    let header = zcon.recv_block(&mut connection).await.unwrap();
    println!("Received block: {}", header.display());
}
