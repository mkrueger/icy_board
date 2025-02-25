use std::{process::Stdio, time::Duration};

use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType},
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ALNUM, display_flags},
    },
};
use icy_engine::TextPane;
use icy_net::{
    Connection,
    telnet::{TelnetConnection, TermCaps, TerminalEmulation},
};
use rand::distr::{Alphanumeric, SampleString};
use regex::Regex;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl IcyBoardState {
    pub async fn open_door(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::RunningDoor).await;
        let doors = self.session.current_conference.doors.as_ref().unwrap().clone();
        if doors.is_empty() {
            self.display_text(
                IceText::NoDOORSAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }

        let display_current_menu = self.session.tokens.is_empty();
        if display_current_menu {
            let file = self.session.current_conference.doors_menu.clone();
            self.session.disp_options.no_change();
            self.display_menu(&file).await?;
        }
        let text = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                if self.session.expert_mode() {
                    IceText::DOORNumberCommandExpertmode
                } else {
                    IceText::DOORNumber
                },
                20,
                &MASK_ALNUM,
                CommandType::OpenDoor.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
            )
            .await?
        };

        if text.is_empty() {
            return Ok(());
        }

        if let Ok(number) = text.parse::<usize>() {
            if number > 0 {
                if let Some(b) = doors.get(number - 1) {
                    self.run_door(&doors, b, number).await?;
                    //                    self.display_current_menu = true;
                    return Ok(());
                }
            }
        } else {
            for (i, d) in doors.doors.iter().enumerate() {
                if d.name.to_uppercase().starts_with(&text.to_uppercase()) {
                    self.run_door(&doors, d, i).await?;
                    //                    self.display_current_menu = true;
                    return Ok(());
                }
            }
        }

        self.display_text(IceText::InvalidDOOR, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER)
            .await?;
        Ok(())
    }

    pub async fn run_door(&mut self, door_list: &DoorList, door: &Door, door_number: usize) -> Res<()> {
        if !door.securiy_level.session_can_access(&self.session) {
            self.display_text(
                IceText::DOORNotAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )
            .await?;
            return Ok(());
        }

        match door.door_type {
            DoorType::BBSlink => {
                let DoorServerAccount::BBSLink(bbslink) = &door_list.accounts[0];
                self.run_bbslink_door(bbslink, door).await?;
            }
            DoorType::Local => {
                self.run_local_door(door, door_number).await?;
            }
        }
        Ok(())
    }

    async fn run_local_door(&mut self, door: &crate::icy_board::doors::Door, door_number: usize) -> Res<()> {
        let file_name = self.resolve_path(&door.path);
        if door.path.ends_with("ppe") {
            self.run_ppe(&file_name, None).await?;
            return Ok(());
        }
        let working_directory = file_name.parent().unwrap();
        door.create_drop_file(&self, &working_directory, door_number).await?;
        let mut cmd = if door.use_shell_execute {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(format!("{}", file_name.display()))
                .current_dir(working_directory)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?
        } else {
            tokio::process::Command::new(&file_name)
                .current_dir(working_directory)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?
        };

        let mut write_buf = vec![0; 32 * 1024];
        let mut read_buf = vec![0; 128 * 1024];
        let mut stidn = cmd.stdin.take().unwrap();
        let mut stdout = cmd.stdout.take().unwrap();

        loop {
            tokio::select! {
                write_data = stdout.read(&mut read_buf)=> {
                    match write_data {
                        Ok(size) => {
                            if size > 0 {
                                log::info!("{}", String::from_utf8_lossy(&read_buf[0..size]));
                                if let Err(_) = self.connection.send(&read_buf[0..size]).await {
                                    break;
                                }
                                let mut remove_sysop_connection = false;
                                let node_state = &mut self.node_state.lock().await;
                                if let Some(sysop_connection) = &mut node_state[self.node].as_mut().unwrap().sysop_connection {
                                    if let Err(_) = sysop_connection.send(&read_buf[0..size]).await {
                                        remove_sysop_connection = true;
                                    }
                                }
                                if remove_sysop_connection {
                                    node_state[self.node].as_mut().unwrap().sysop_connection = None;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Error reading from door: {}", e);
                            break;
                        }
                    }
                }
                read_data = self.connection.read(&mut write_buf) => {
                    match read_data {
                        Ok(size) => {
                            if let Err(_) = stidn.write_all(&write_buf[0..size]).await {
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            };

            if cmd.try_wait()?.is_some() {
                break;
            } else {
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        log::info!("door exited.");

        Ok(())
    }

    pub async fn run_bbslink_door(&mut self, bbslink: &BBSLink, door: &Door) -> Res<()> {
        log::info!("Running door: {}, requesting token", door.path);
        let x_key = Alphanumeric.sample_string(&mut rand::rng(), 12);
        let token = reqwest::get(format!("https://games.bbslink.net/token.php?{x_key}")).await?.text().await?;
        log::info!("got token {}, sending credentials", token);
        /* Not sure why this doesn't work:
        let mut map = http::header::HeaderMap::new();
        map.insert("X-User", self.session.cur_user.into());
        map.insert("X-System", bbslink.system_code.parse()?);
        map.insert("X-Auth", format!("{:x}", md5::compute(bbslink.auth_code.clone() + token.as_str())).parse()?);
        map.insert("X-Code", format!("{:x}", md5::compute(bbslink.sheme_code.clone() + token.as_str())).parse()?);
        map.insert("X-Rows", self.user_screen.buffer.get_height().into());
        map.insert("X-Key", x_key.parse()?);
        map.insert("X-Door", door.path.parse()?);
        map.insert("X-Token", token.parse()?);
        map.insert("X-Type", header::HeaderValue::from_static("icy_board"));
        map.insert("X-Version", crate::VERSION.to_string().parse()?);



        let response = reqwest::Client::builder()
          //  .user_agent(format!("icy_board/{}", VERSION.to_string()))
            .default_headers(map)
            .build()?
            .get(format!("https://games.bbslink.net/auth.php?key={x_key}")).send().await?.text().await;
        */

        let url = format!(
            "https://games.bbslink.net/auth.php?key={}&user={}&system={}&auth={}&scheme={}&rows={}&door={}&token={}&type={}&version={}",
            x_key,
            self.session.cur_user_id,
            bbslink.system_code,
            format!("{:x}", md5::compute(bbslink.auth_code.clone() + token.as_str())),
            format!("{:x}", md5::compute(bbslink.sheme_code.clone() + token.as_str())),
            self.display_screen().buffer.get_height(),
            door.path,
            token,
            "icy_board",
            crate::VERSION.to_string()
        );
        let response = reqwest::get(url).await?.text().await;

        match response {
            Ok(str) => {
                if str == "complete" {
                    let mut connection = TelnetConnection::open(
                        "games.bbslink.net:23",
                        TermCaps {
                            window_size: (80, 24),
                            terminal: TerminalEmulation::Ansi,
                        },
                        Duration::from_millis(500),
                    )
                    .await?;
                    log::info!("Connected to door server");
                    let _ = execute_door(&mut connection, self).await?;
                    return Ok(());
                }
                log::info!("got server response '{}'", str);
                for e in parse_bbslink_error(&str) {
                    log::error!("Unauthorised: {}", e);
                }
                self.display_text(
                    IceText::DOORNotAvailable,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
            }
            Err(e) => {
                log::error!("Error opening door : {}", e);
            }
        }

        Ok(())
    }
}

async fn execute_door(door_connection: &mut dyn Connection, state: &mut crate::icy_board::state::IcyBoardState) -> Res<()> {
    let mut read_buf = vec![0; 64 * 1024];
    let mut write_buf = vec![0; 8 * 1024];
    loop {
        tokio::select! {
            read = door_connection.read(&mut read_buf) => {
               match read {
                     Ok(size) => {
                          if size > 0 {
                            state.connection.send(&read_buf[0..size]).await?;
                            let node_state = &mut state.node_state.lock().await;
                            if let Some(sysop_connection) = &mut node_state[state.node].as_mut().unwrap().sysop_connection {
                                let _ = sysop_connection.send(&read_buf[0..size]).await;
                            }

                          } else {
                            return Ok(());
                          }
                     }
                     Err(e) => {
                        log::error!("Error reading from connection: {}", e);
                        return Err(e.into());
                    }
               }
            }
            write = state.connection.read(&mut write_buf) => {
                match write {
                    Ok(size) => {
                        if size > 0 {
                            door_connection.send(&write_buf[0..size]).await?;

                            let node_state = &mut state.node_state.lock().await;
                            if let Some(sysop_connection) = &mut node_state[state.node].as_mut().unwrap().sysop_connection {
                                let size = sysop_connection.read(&mut read_buf).await?;
                                if size > 0 {
                                    door_connection.send(&read_buf[0..size]).await?;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading from connection: {}", e);
                        return Err(e.into());
                    }
                }
            }
        }
    }
}

pub fn parse_bbslink_error(error: &str) -> Vec<BBSLinkError> {
    let re = Regex::new("\\(Error\\s(\\d+)\\)").unwrap();
    let mut errors = Vec::new();
    for cp in re.captures_iter(error) {
        if let Some(m) = cp.get(1) {
            if let Ok(e) = m.as_str().parse::<usize>() {
                match e {
                    0 => errors.push(BBSLinkError::Error0),
                    1 => errors.push(BBSLinkError::Error1),
                    2 => errors.push(BBSLinkError::Error2),
                    3 => errors.push(BBSLinkError::Error3),
                    4 => errors.push(BBSLinkError::Error4),
                    5 => errors.push(BBSLinkError::Error5),
                    6 => errors.push(BBSLinkError::Error6),
                    7 => errors.push(BBSLinkError::Error7),
                    _ => errors.push(BBSLinkError::UnknownError(e)),
                }
            }
        }
    }
    errors
}

#[derive(Error, Debug, PartialEq)]
pub enum BBSLinkError {
    #[error("No X-Key passed in URL")]
    Error0,
    #[error("X-Key header does not match X-Key passed in URL")]
    Error1,
    #[error("Incorrect Scheme Code")]
    Error2,
    #[error("Incorrect Authorisation Code (System Code valid)")]
    Error3,
    #[error("Unknown System Code")]
    Error4,
    #[error("Unknown door code")]
    Error5,
    #[error("Expired Token")]
    Error6,
    #[error("No authentication data found in headers or URL")]
    Error7,
    #[error("Unknown error code: {0}")]
    UnknownError(usize),
}

#[cfg(test)]
mod test {
    use crate::icy_board::state::user_commands::pcb::open_door::BBSLinkError;

    use super::parse_bbslink_error;
    #[test]
    fn test_parse_bbslink_error() {
        let output = parse_bbslink_error("Unauthorised (Error 7)*xxUnauthorised (Error 2)");
        assert_eq!(output, vec![BBSLinkError::Error7, BBSLinkError::Error2]);
    }
}
