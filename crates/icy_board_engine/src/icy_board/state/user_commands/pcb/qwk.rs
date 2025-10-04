use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek, Write},
};

use bstr::BString;
use chrono::{DateTime, Local, Utc};
use jamjam::{
    jam::{JamMessage, JamMessageBase},
    qwk::{control::ControlDat, qwk_message::QWKMessage},
    util::basic_real::BasicReal,
};
use tokio::fs;
use zip::write::SimpleFileOptions;

use crate::{
    Res,
    icy_board::{
        commands::CommandType,
        icb_config::IcbColor,
        icb_text::IceText,
        message_area::MessageArea,
        state::{IcyBoardState, functions::display_flags},
        user_base::LastReadStatus,
    },
    vm::TerminalTarget,
};

use super::u_upload_file::create_protocol;

const MASK_CONFNUMBERS: &str = "0123456789-SDL?";

impl IcyBoardState {
    async fn set_last_read(&mut self, table: &HashMap<u16, (usize, usize)>) -> Res<()> {
        self.session.op_text = format!("(1-{})", table.keys().max().unwrap_or(&0));
        let input = self
            .input_field(
                IceText::SelectArea,
                9,
                &crate::icy_board::state::functions::MASK_NUM,
                CommandType::QWK.get_help(),
                None,
                display_flags::UPCASE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;

        let Ok(number) = input.parse::<u16>() else {
            return Ok(());
        };
        let Some((conf_num, area_num)) = table.get(&number) else {
            return Ok(());
        };
        let high_msg = self.board.lock().await.conferences[*conf_num].areas.as_ref().unwrap()[*area_num].get_high_msg() as usize;

        self.session.op_text = format!("(1-{})", high_msg);

        let input = self
            .input_field(
                IceText::SetLastMessageReadPointer,
                9,
                &crate::icy_board::state::functions::MASK_NUM,
                CommandType::QWK.get_help(),
                Some(self.session.page_len.to_string()),
                display_flags::UPCASE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        let Ok(number) = input.parse::<usize>() else {
            return Ok(());
        };
        let number = number.min(high_msg);
        if let Some(user) = &mut self.session.current_user {
            user.lastread_ptr_flags
                .entry((*conf_num, *area_num))
                .or_insert_with(|| LastReadStatus {
                    include_qwk: true,
                    highest_msg_read: 0,
                    last_read: 0,
                })
                .last_read = number;
        }
        Ok(())
    }

    pub async fn qwk_command(&mut self) -> Res<()> {
        loop {
            if self.session.tokens.is_empty() {
                let input = self
                    .input_field(
                        IceText::QWKCommands,
                        2,
                        &"DSU",
                        CommandType::QWK.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::STACKED | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                log::info!("QWK command input: '{}'", input);
                self.session.push_tokens(&input);
            };

            if let Some(token) = self.session.tokens.pop_front() {
                match token.as_str() {
                    "D" => {
                        self.create_qwk_packet().await?;
                        break;
                    }
                    "U" => {
                        self.upload_qwk_reply().await?;
                        break;
                    }
                    "S" => {
                        self.select_qwk_areas().await?;
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn get_number_to_msgid(&self) -> (HashMap<u16, (usize, usize)>, HashMap<(usize, usize), u16>) {
        let conferences = &self.board.lock().await.conferences;
        let mut number_to_msgid = HashMap::new();
        for (i, conf) in conferences.iter().enumerate() {
            if let Some(areas) = &conf.areas {
                for (j, area) in areas.iter().enumerate() {
                    if area.qwk_conference_number != 0 {
                        number_to_msgid.insert(area.qwk_conference_number, (i, j));
                    }
                }
            }
        }

        let mut number = 1;
        for (i, conf) in conferences.iter().enumerate() {
            if let Some(areas) = &conf.areas {
                for (j, area) in areas.iter().enumerate() {
                    if area.qwk_conference_number != 0 {
                        while number_to_msgid.contains_key(&number) {
                            number += 1;
                        }
                        number_to_msgid.insert(number, (i, j));
                        number += 1;
                    }
                }
            }
        }
        let mut msgid_to_number = HashMap::new();

        for (k, v) in number_to_msgid.iter() {
            msgid_to_number.insert(*v, *k);
        }

        (number_to_msgid, msgid_to_number)
    }

    async fn select_qwk_areas(&mut self) -> Res<()> {
        let divider = "-".repeat(79);
        let num_lines = if self.session.page_len < 4 || self.session.page_len > 50 {
            19
        } else {
            self.session.page_len as usize - 4
        };
        let mut done = false;

        let conferences = &self.board.lock().await.conferences.clone();
        let (number_to_msgid, msgid_to_number) = self.get_number_to_msgid().await;

        while !done {
            if self.session.tokens.is_empty() {
                self.print_area_header(&divider).await?;
                let mut line_number = 0;
                for (i, conf) in conferences.iter().enumerate() {
                    if let Some(areas) = &conf.areas {
                        for (j, area) in areas.iter().enumerate() {
                            if let Some(number) = msgid_to_number.get(&(i, j)) {
                                self.print_area_line(*number, i, j, area).await?;
                            }
                            line_number += 1;
                        }
                    }
                }
                for _ in line_number..num_lines {
                    self.new_line().await?;
                }
                self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
                self.println(TerminalTarget::Both, &divider).await?;

                let txt = if self.session.expert_mode() {
                    IceText::QWKListCommandsExpertmode
                } else {
                    IceText::QWKListCommands
                };
                let help = "hlpqwk";
                let text = self
                    .input_field(
                        txt,
                        58,
                        MASK_CONFNUMBERS,
                        help,
                        None,
                        display_flags::ERASELINE | display_flags::STACKED | display_flags::UPCASE,
                    )
                    .await?;
                self.session.push_tokens(&text);
            } else {
                done = true;
            }
            if self.session.tokens.is_empty() {
                break;
            }
            while let Some(token) = self.session.tokens.pop_front() {
                match token.as_str() {
                    "Q" => {
                        done = true;
                        break;
                    }
                    "S" => {
                        self.change_qkw_selection(&number_to_msgid, 0, number_to_msgid.len(), Some(true)).await?;
                    }
                    "D" => {
                        self.change_qkw_selection(&number_to_msgid, 0, number_to_msgid.len(), Some(false)).await?;
                    }
                    "L" => {
                        self.set_last_read(&number_to_msgid).await?;
                    }
                    _ => {
                        let mut str = token;
                        let value;
                        if str.ends_with('D') {
                            value = Some(false);
                            str.pop();
                        } else if str.ends_with('S') {
                            value = Some(true);
                            str.pop();
                        } else {
                            value = None;
                        }

                        if str.contains('-') {
                            let mut parts = str.split('-');
                            if let (Some(from), Some(to)) = (parts.next(), parts.next()) {
                                if let (Ok(from), Ok(to)) = (from.parse::<usize>(), to.parse::<usize>()) {
                                    self.change_qkw_selection(&number_to_msgid, from, to, value).await?;
                                }
                            }
                        } else {
                            if let Ok(num) = str.parse::<usize>() {
                                self.change_qkw_selection(&number_to_msgid, num, num, value).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn print_area_header(&mut self, divider: &str) -> Res<()> {
        self.clear_screen(TerminalTarget::Both).await?;
        self.display_text(IceText::MessageAreaListHeader1, display_flags::NEWLINE).await?;
        self.display_text(IceText::MessageAreaListHeader2, display_flags::NEWLINE).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
        self.println(TerminalTarget::Both, divider).await?;
        self.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }

    async fn print_area_line(&mut self, area_number: u16, conference: usize, num: usize, area: &MessageArea) -> Res<()> {
        self.reset_color(TerminalTarget::Both).await?;

        let str = format!("{:5}{}", area_number, ' ');
        self.print(TerminalTarget::Both, &str).await?;

        self.print(TerminalTarget::Both, &area.name).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_dark_gray()).await?;

        for i in area.name.len()..54 {
            self.print(TerminalTarget::Both, if i % 2 == 0 { " " } else { "." }).await?;
        }
        self.set_color(TerminalTarget::Both, IcbColor::dos_gray()).await?;

        if let Some(user) = &self.session.current_user {
            let (include, last_read) = if let Some(last_read) = user.lastread_ptr_flags.get(&(conference, num)) {
                (last_read.include_qwk, last_read.highest_msg_read)
            } else {
                (true, 0)
            };

            let high = area.get_high_msg();

            self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;

            self.print(TerminalTarget::Both, &format!(" {:<6}", last_read)).await?;
            self.print(TerminalTarget::Both, &format!(" {:<6}", high)).await?;
            if include {
                self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
                self.print(TerminalTarget::Both, &"X").await?;
            }
        }
        self.new_line().await?;

        Ok(())
    }

    async fn change_qkw_selection(&mut self, table: &HashMap<u16, (usize, usize)>, from: usize, to: usize, set_selection_to: Option<bool>) -> Res<()> {
        if let Some(user) = &mut self.session.current_user {
            for i in from..=to {
                if let Some((conference, num)) = table.get(&(i as u16)) {
                    user.lastread_ptr_flags
                        .entry((*conference, *num))
                        .or_insert_with(|| LastReadStatus {
                            include_qwk: true,
                            highest_msg_read: 0,
                            last_read: 0,
                        })
                        .include_qwk = set_selection_to.unwrap_or(
                        !user
                            .lastread_ptr_flags
                            .entry((*conference, *num))
                            .or_insert_with(|| LastReadStatus {
                                include_qwk: true,
                                highest_msg_read: 0,
                                last_read: 0,
                            })
                            .include_qwk,
                    );
                }
            }
        }
        Ok(())
    }

    async fn create_qwk_packet(&mut self) -> Res<()> {
        let output_path = temp_file::empty().path().to_path_buf();
        fs::create_dir_all(&output_path).await?;
        let mut qwk_package = output_path.join("mail.qwk");
        let (_number_to_msgid, msgid_to_number) = self.get_number_to_msgid().await;

        {
            let board = self.board.lock().await;
            let bbs_name: BString = if board.config.qwk_settings.bbs_name.is_empty() {
                board.config.board.name.clone().into()
            } else {
                board.config.qwk_settings.bbs_name.clone().into()
            };
            let mut control_dat = ControlDat {
                bbs_name: bbs_name.clone(),
                bbs_city_and_state: if board.config.qwk_settings.bbs_city_and_state.is_empty() {
                    board.config.board.location.clone().into()
                } else {
                    board.config.qwk_settings.bbs_city_and_state.clone().into()
                },
                bbs_phone_number: if board.config.qwk_settings.bbs_phone_number.is_empty() {
                    board.config.board.notice.clone().into()
                } else {
                    board.config.qwk_settings.bbs_phone_number.clone().into()
                },
                bbs_sysop_name: if board.config.qwk_settings.bbs_sysop_name.is_empty() {
                    board.config.board.operator.clone().into()
                } else {
                    board.config.qwk_settings.bbs_sysop_name.clone().into()
                },
                bbs_id: if board.config.qwk_settings.bbs_id.is_empty() {
                    bbs_name.clone().into()
                } else {
                    board.config.qwk_settings.bbs_id.clone().into()
                },
                serial_number: 0,
                creation_time: Local::now().format("%m/%d/%y,%H:%M").to_string().into(),
                qmail_user_name: self.session.user_name.clone().into(),
                qmail_menu_name: BString::from(""),
                zero_line: "0".into(),
                message_count: 0,
                conferences: Vec::new(),
                welcome_screen: BString::from(""),
                news_screen: BString::from(""),
                logoff_screen: BString::from(""),
            };
            let Some(user) = &mut self.session.current_user else {
                return Ok(());
            };

            if !control_dat.bbs_id.is_empty() {
                qwk_package = output_path.join(format!("{}.qwk", control_dat.bbs_id));
            }

            let conferences = board.conferences.clone();
            let mut msg_writer = format!("Produced by Icy Board {}", env!("CARGO_PKG_VERSION")).bytes().collect::<Vec<u8>>();
            msg_writer.resize(128, b' ');

            let mut cur_block = 2;
            let mut ndx_data = HashMap::new();
            for (i, conf) in conferences.iter().enumerate() {
                if let Some(areas) = &conf.areas {
                    for (j, area) in areas.iter().enumerate() {
                        let ptr = if let Some(ptr) = user.lastread_ptr_flags.get(&(i, j)) {
                            ptr.clone()
                        } else {
                            LastReadStatus::default()
                        };
                        if !ptr.include_qwk {
                            continue;
                        }
                        let Some(conference_number) = msgid_to_number.get(&(i, j)) else {
                            continue;
                        };
                        let conference_number = *conference_number;
                        control_dat.conferences.push(jamjam::qwk::control::Conference {
                            number: conference_number,
                            name: if area.qwk_name.is_empty() {
                                area.name.clone().into()
                            } else {
                                area.qwk_name.clone().into()
                            },
                        });

                        let is_extended = true;
                        let message_base_file = area.path.clone();

                        match JamMessageBase::open(&message_base_file) {
                            Ok(message_base) => {
                                let base = message_base.base_messagenumber();
                                let active = message_base.active_messages();

                                ndx_data.insert(conference_number, Vec::new());
                                for i in ptr.highest_msg_read as u32..(base + active) {
                                    if let Ok(header) = message_base.read_header(i) {
                                        if let Ok(text) = message_base.read_msg_text(&header) {
                                            let date_time = DateTime::from_timestamp(header.date_written as i64, 0).unwrap_or(Utc::now());
                                            let qwk_msg = QWKMessage {
                                                msg_number: header.message_number,
                                                from: header.get_from().unwrap().clone(),
                                                to: header.get_to().unwrap().clone(),
                                                subj: header.get_subject().unwrap().clone(),
                                                date_time: date_time.format("%m-%d-%y%H:%M").to_string().into(),
                                                text,
                                                status: b' ',
                                                password: BString::from(""),
                                                ref_msg_number: header.reply_to,
                                                logical_message_number: control_dat.message_count as u16,
                                                active_flag: if header.is_deleted() { 226 } else { 225 },
                                                conference_number,
                                                net_tag: b' ',
                                            };
                                            let blocks = qwk_msg.write(&mut msg_writer, is_extended)?;
                                            ndx_data.get_mut(&conference_number).unwrap().push(BasicReal::from(cur_block));
                                            control_dat.message_count += 1;
                                            cur_block += blocks as i32;
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                log::error!("Message index load error {}", err);
                                log::error!("Creating new message index at {}", message_base_file.display());
                            }
                        }
                    }
                }
            }

            let file = std::fs::File::create(&qwk_package).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("control.dat", SimpleFileOptions::default())?;
            zip.write_all(&control_dat.to_vec())?;

            zip.start_file("messages.dat", SimpleFileOptions::default())?;
            zip.write_all(&msg_writer)?;

            for (cnf, ndx) in ndx_data.iter() {
                zip.start_file(&format!("{:03}.ndx", cnf), SimpleFileOptions::default())?;
                for br in ndx {
                    zip.write_all(br.bytes())?;
                    zip.write(&[*cnf as u8])?;
                }
            }
            zip.finish()?;
        }
        self.add_flagged_file(&qwk_package, true, false).await?;
        self.download(false).await?;
        self.session.flagged_files.retain(|f| f != &qwk_package);
        fs::remove_dir_all(output_path).await?;
        Ok(())
    }

    async fn upload_qwk_reply(&mut self) -> Res<()> {
        let cur_protocol = if let Some(user) = &self.session.current_user {
            user.protocol.clone()
        } else {
            String::new()
        };

        let prot_str = self.ask_protocols(&cur_protocol).await?;
        if prot_str.is_empty() {
            return Ok(());
        }
        let Some(protocol) = self.get_protocol(prot_str).await else {
            return Ok(());
        };

        let mut prot = create_protocol(&protocol);
        let bbs_id = {
            let board = self.board.lock().await;
            if board.config.qwk_settings.bbs_id.is_empty() {
                board.config.board.name.clone()
            } else {
                board.config.qwk_settings.bbs_id.clone()
            }
        };
        match prot.initiate_recv(&mut *self.connection).await {
            Ok(mut state) => {
                while !state.is_finished {
                    if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                        log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                        self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                        break;
                    }
                }
                self.display_text(IceText::TransferSuccessful, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;

                for (_x, path) in state.recieve_state.finished_files {
                    self.display_text(IceText::ExtractingMessages, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;

                    let mut archive = zip::ZipArchive::new(std::fs::File::open(&path)?)?;
                    if let Ok(mut arch) = archive.by_name(&format!("{}.MSG", bbs_id)) {
                        let mut buf = Vec::new();
                        arch.read_to_end(&mut buf)?;
                        let mut cursor = Cursor::new(buf);
                        cursor.seek(std::io::SeekFrom::Start(128))?;

                        let conferences = &self.board.lock().await.conferences.clone();
                        let mut number_to_msgid = Vec::new();
                        for (i, conf) in conferences.iter().enumerate() {
                            if let Some(areas) = &conf.areas {
                                for (j, _area) in areas.iter().enumerate() {
                                    number_to_msgid.push((i, j));
                                }
                            }
                        }

                        while let Ok(msg) = QWKMessage::read(&mut cursor, true) {
                            if let Some((conf, area)) = number_to_msgid.get(msg.msg_number as usize) {
                                let jam_msg = JamMessage::default()
                                    .with_from(msg.from)
                                    .with_to(msg.to)
                                    .with_subject(msg.subj)
                                    .with_date_time(Utc::now())
                                    .with_text(msg.text);
                                self.send_message(*conf as i32, *area as i32, jam_msg, IceText::ReplySuccessful).await?;
                            } else {
                                self.display_text(IceText::ReplyFailed, display_flags::NEWLINE).await?;
                            }
                        }
                    } else {
                        self.display_text(IceText::ErrorExtracting, display_flags::NEWLINE).await?;
                    }

                    std::fs::remove_file(&path)?;
                }
            }
            Err(e) => {
                log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                self.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
            }
        }
        Ok(())
    }
}
