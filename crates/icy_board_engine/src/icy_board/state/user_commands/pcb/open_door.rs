use std::{process::Stdio, time::Duration};

use crate::icy_board::commands::CommandType;
use crate::icy_board::state::menu_runner::ActivityUsage;
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType},
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ASCII, display_flags},
    },
};
use icy_engine::TextPane;
use icy_net::{
    Connection,
    telnet::{TelnetConnection, TermCaps, TerminalEmulation},
};
use regex::Regex;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static DOS_MACHINE_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// A connected loopback pair; the door inherits its own end and never sees the
/// caller's connection.
#[cfg(unix)]
struct DoorSocket {
    board: tokio::net::TcpStream,
    door: std::os::fd::OwnedFd,
}

#[cfg(unix)]
impl DoorSocket {
    fn open() -> Res<Self> {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let board = std::net::TcpStream::connect(listener.local_addr()?)?;
        let expected = board.local_addr()?;
        let door = loop {
            // Another local process can reach the listener first, so accept only our own endpoint.
            let (candidate, peer) = listener.accept()?;
            if peer == expected {
                break candidate;
            }
        };
        drop(listener);
        board.set_nodelay(true)?;
        door.set_nodelay(true)?;
        board.set_nonblocking(true)?;
        Ok(Self {
            board: tokio::net::TcpStream::from_std(board)?,
            door: rustix::io::fcntl_dupfd_cloexec(door, 3)?,
        })
    }

    fn handle(&self) -> i64 {
        std::os::fd::AsRawFd::as_raw_fd(&self.door).into()
    }
}

#[cfg(not(unix))]
struct DoorSocket;

#[cfg(not(unix))]
impl DoorSocket {
    fn open() -> Res<Self> {
        Err("doors cannot be handed a socket connection on this platform yet".into())
    }

    fn handle(&self) -> i64 {
        0
    }
}

/// An explicitly priced command opening a priced door is two configured
/// activities, never an implicit duplicate of the door's own surcharge.
struct DoorUsage<'a> {
    door: ActivityUsage,
    command: Option<&'a mut ActivityUsage>,
    restore_terminal: bool,
}

impl DoorUsage<'_> {
    fn start(&mut self, state: &mut IcyBoardState) -> Res<()> {
        if let Some(command) = self.command.as_deref_mut() {
            command.start(state)?;
        }
        self.door.start(state)
    }
}

fn dos_runtime_remaining(login_date: chrono::DateTime<chrono::Utc>, time_limit: i32, max_runtime_seconds: u32) -> Option<Duration> {
    let maximum = Duration::from_secs(
        if max_runtime_seconds == 0 {
            crate::icy_board::doors::DEFAULT_DOS_MAX_RUNTIME_SECONDS
        } else {
            max_runtime_seconds
        }
        .into(),
    );
    let session_remaining = if time_limit == 0 {
        None
    } else {
        let deadline = login_date + chrono::Duration::minutes(time_limit.into());
        Some((deadline - chrono::Utc::now()).to_std().unwrap_or(Duration::ZERO))
    };
    Some(session_remaining.map_or(maximum, |remaining| remaining.min(maximum)))
}

fn dos_output_for_terminal(bytes: &[u8], utf8: bool) -> Vec<u8> {
    if !utf8 {
        return bytes.to_vec();
    }
    let mut output = Vec::with_capacity(bytes.len());
    let mut utf8_buffer = [0; 4];
    for byte in bytes {
        let encoded = codepages::tables::CP437_TO_UNICODE[*byte as usize].encode_utf8(&mut utf8_buffer);
        output.extend_from_slice(encoded.as_bytes());
    }
    output
}

fn append_cp437(output: &mut Vec<u8>, text: &str) {
    output.extend(text.chars().map(|ch| {
        if ch.is_ascii() {
            ch as u8
        } else {
            codepages::tables::UNICODE_TO_CP437.get(&ch).copied().unwrap_or(b'.')
        }
    }));
}

#[derive(Default)]
pub(crate) struct DosInputEncoder {
    pending_utf8: Vec<u8>,
}

impl DosInputEncoder {
    pub(crate) fn encode(&mut self, bytes: &[u8], utf8: bool) -> Vec<u8> {
        if !utf8 {
            return bytes.to_vec();
        }
        self.pending_utf8.extend_from_slice(bytes);
        let mut output = Vec::with_capacity(self.pending_utf8.len());
        loop {
            match std::str::from_utf8(&self.pending_utf8) {
                Ok(text) => {
                    append_cp437(&mut output, text);
                    self.pending_utf8.clear();
                    break;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    let error_len = error.error_len();
                    let valid = std::str::from_utf8(&self.pending_utf8[..valid_up_to]).expect("valid UTF-8 prefix");
                    append_cp437(&mut output, valid);
                    self.pending_utf8.drain(..valid_up_to);
                    let Some(error_len) = error_len else { break };
                    output.push(b'.');
                    self.pending_utf8.drain(..error_len);
                }
            }
        }
        output
    }
}

impl IcyBoardState {
    /// `PCBoard`'s command dispatcher falls through to the door list when a caller
    /// with OPEN access types a word that is not a command; the door name is
    /// matched as a prefix, the way `searchdoorlist` does (DOORS.C). Returns true
    /// when a door was found and run.
    pub async fn try_open_matching_door(&mut self, name: &str) -> Res<bool> {
        let open_access = self.session.user_command_level.cmd_open_door.clone();
        if !open_access.session_can_access(&self.session) {
            return Ok(false);
        }
        self.run_named_door(name).await
    }

    /// Runs the door whose name `name` begins, matched as a prefix the way
    /// `searchdoorlist` does. Returns false when no door answers to the name.
    pub async fn run_named_door(&mut self, name: &str) -> Res<bool> {
        self.run_named_door_with_usage(name, None).await
    }

    pub(crate) async fn run_named_door_with_usage(&mut self, name: &str, usage: Option<&mut ActivityUsage>) -> Res<bool> {
        let Some(doors) = self.session.current_conference.doors.clone() else {
            return Ok(false);
        };
        let needle = name.to_uppercase();
        for (i, door) in doors.doors.iter().enumerate() {
            if door.name.to_uppercase().starts_with(&needle) {
                self.set_activity(NodeStatus::RunningDoor).await;
                self.run_door_with_usage(&doors, door, i, usage).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn open_door(&mut self) -> Res<()> {
        self.open_door_with_usage(None).await
    }

    pub(crate) async fn open_door_with_usage(&mut self, usage: Option<&mut ActivityUsage>) -> Res<()> {
        self.set_activity(NodeStatus::RunningDoor).await;
        let doors = self.session.current_conference.doors.clone().unwrap_or_default();
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
                &MASK_ASCII,
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
            if number > 0
                && let Some(b) = doors.get(number - 1)
            {
                self.run_door_with_usage(&doors, b, number, usage).await?;
                //                    self.display_current_menu = true;
                return Ok(());
            }
        } else {
            for (i, d) in doors.doors.iter().enumerate() {
                if d.name.to_uppercase().starts_with(&text.to_uppercase()) {
                    self.run_door_with_usage(&doors, d, i, usage).await?;
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
        self.run_door_with_usage(door_list, door, door_number, None).await
    }

    async fn run_door_with_usage(&mut self, door_list: &DoorList, door: &Door, door_number: usize, command: Option<&mut ActivityUsage>) -> Res<()> {
        if !door.securiy_level.session_can_access(&self.session) {
            self.display_text(
                IceText::DOORNotAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )
            .await?;
            return Ok(());
        }

        // The sysop walks past both of these gates.
        if !self.session.is_sysop && !door.password.is_empty() {
            let answer = self
                .input_field(
                    IceText::PasswordForDOOR,
                    12,
                    &MASK_ASCII,
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::UPCASE | display_flags::ECHODOTS,
                )
                .await?;
            if answer.is_empty() || !answer.eq_ignore_ascii_case(&door.password) {
                self.display_text(IceText::BadPasswordForDOOR, display_flags::NEWLINE | display_flags::LOGIT)
                    .await?;
                return Ok(());
            }
        }

        // Running a door drops the flag list, so give the user a way out.
        if !self.session.flagged_files.is_empty() {
            self.display_text(IceText::FilesAreFlagged, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                .await?;
            self.session.op_text.clone_from(&door.name);
            let answer = self
                .input_field(
                    IceText::ContinueDOOR,
                    1,
                    "",
                    "",
                    Some(self.session.no_char.to_uppercase().to_string()),
                    display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;
            if answer != self.session.yes_char.to_uppercase().to_string() {
                return Ok(());
            }
        }

        if self.session.request_logoff {
            return Ok(());
        }
        let usage = ActivityUsage::new("DOOR USAGE", "DOOR USAGE MIN", &door.name, door.charge_per_use, door.charge_per_minute);
        let reserved = command.as_ref().map_or(0.0, |usage| usage.pending_cost());
        if !usage.allowed_with_reserved(self, reserved).await? {
            return Ok(());
        }
        let mut usage = DoorUsage {
            door: usage,
            command,
            restore_terminal: false,
        };
        // Selection, password/security, confirmation and admission have all
        // succeeded. Each backend marks its actual launch, not setup attempts.
        let result = match door.door_type {
            DoorType::BBSlink => {
                if let Some(DoorServerAccount::BBSLink(bbslink)) = door_list.accounts.first() {
                    self.run_bbslink_door_started(bbslink, door, &mut usage).await
                } else {
                    Err("BBSLink door has no server account".into())
                }
            }
            DoorType::Local => self.run_local_door(door, door_number, &mut usage).await,
            DoorType::Dos => self.run_dos_door(door, door_number, &mut usage).await,
        };
        let result = if usage.restore_terminal
            && !self.session.request_logoff
            && self.session.disp_options.grapics_mode != crate::icy_board::state::GraphicsMode::Ctty
        {
            let reset = b"\x18\x1b[?6l\x1b[r\x1b[?69l\x1b[?7h\x1b[?25h\x1b[0m";
            let restored = self.write_terminal_bytes(crate::vm::TerminalTarget::Both, reset, reset).await;
            result.and(restored)
        } else {
            result
        };
        usage.door.finish(self, result).await
    }

    async fn run_local_door(&mut self, door: &crate::icy_board::doors::Door, door_number: usize, usage: &mut DoorUsage<'_>) -> Res<()> {
        let file_name = self.resolve_path(&door.path);
        if file_name.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("ppe")) {
            // Load before posting: a missing/corrupt PPE never started. Reuse
            // the loaded executable rather than racing a second file read.
            let executable = match crate::executable::Executable::read_file(&file_name, false) {
                Ok(executable) => executable,
                Err(error) => {
                    self.session.op_text = error.to_string();
                    let result = self
                        .display_text(IceText::ErrorLoadingPPE, display_flags::LFBEFORE | display_flags::LFAFTER)
                        .await;
                    self.session.tokens.clear();
                    return result;
                }
            };
            if self.ppe_nesting >= 16 {
                self.session.tokens.clear();
                return Ok(());
            }
            let file_name = file_name.canonicalize()?;
            usage.start(self)?;
            let result = self.run_executable_with_color_restore(&file_name, None, executable, true).await;
            self.session.tokens.clear();
            result?;
            return Ok(());
        }
        let working_directory = if door.working_directory.is_empty() {
            file_name.parent().unwrap().to_path_buf()
        } else {
            self.resolve_path(&door.working_directory)
        };
        crate::icy_board::doors::launch::check_arguments(door)?;
        let Some(_parallel) = crate::icy_board::doors::launch::acquire(&door.name, door.max_parallel) else {
            self.display_text(
                IceText::DOORNotAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )
            .await?;
            return Ok(());
        };
        // Every launch owns its drop file, so parallel sessions cannot read each other's.
        let drop_directory = tempfile::Builder::new().prefix("icbdoor").tempdir()?;
        let socket = if door.provide_socket_connection { Some(DoorSocket::open()?) } else { None };
        door.create_drop_file_with_socket(self, drop_directory.path(), door_number, socket.as_ref().map(DoorSocket::handle))
            .await?;
        let placeholders = crate::icy_board::doors::launch::Placeholders {
            drop_file: door.drop_file.file_name(self.node).map(|name| drop_directory.path().join(name)),
            node: self.node,
            user_id: self.session.cur_user_id,
            user_name: self.session.user_name.clone(),
            time_left_seconds: i64::from(self.session.minutes_left()) * 60,
            term_width: self.session.term_caps.term_size.0,
            term_height: self.session.term_caps.term_size.1,
            socket_handle: socket.as_ref().map(DoorSocket::handle),
        };
        let arguments = door
            .args
            .iter()
            .map(|argument| crate::icy_board::doors::launch::expand(argument, &placeholders))
            .collect::<Res<Vec<String>>>()?;
        // Shell execution is considered started when the shell spawns; a later
        // shell error/nonzero exit is billable, just like a game's runtime error.
        #[cfg(unix)]
        let (mut cmd, mut stdout, mut stdin): (
            tokio::process::Child,
            Box<dyn tokio::io::AsyncRead + Send + Unpin>,
            Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
        ) = if let Some(socket) = socket {
            let DoorSocket { board, door: inherited } = socket;
            let mut command = if door.use_shell_execute {
                let mut command = tokio::process::Command::new("sh");
                command.arg("-c").arg("exec \"$0\" \"$@\"").arg(&file_name);
                command
            } else {
                tokio::process::Command::new(&file_name)
            };
            command
                .args(&arguments)
                .current_dir(&working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .process_group(0)
                .kill_on_drop(true);
            unsafe {
                command.pre_exec(move || {
                    rustix::io::fcntl_setfd(&inherited, rustix::io::FdFlags::empty())?;
                    Ok(())
                });
            }
            let child = command.spawn()?;
            drop(command);
            let (reader, writer) = board.into_split();
            (child, Box::new(reader), Box::new(writer))
        } else {
            let (pty, pts) = pty_process::open()?;
            let mut attributes = rustix::termios::tcgetattr(&pts)?;
            attributes.make_raw();
            rustix::termios::tcsetattr(&pts, rustix::termios::OptionalActions::Now, &attributes)?;
            let (width, height) = self.session.term_caps.term_size;
            pty.resize(pty_process::Size::new(height.max(1), width.max(1)))?;
            let mut command = if door.use_shell_execute {
                pty_process::Command::new("sh").arg("-c").arg("exec \"$0\" \"$@\"").arg(&file_name)
            } else {
                pty_process::Command::new(&file_name)
            };
            for argument in &arguments {
                command = command.arg(argument);
            }
            let child = command.current_dir(&working_directory).stderr(Stdio::inherit()).kill_on_drop(true).spawn(pts)?;
            let (stdout, stdin) = pty.into_split();
            (child, Box::new(stdout), Box::new(stdin))
        };
        #[cfg(not(unix))]
        let mut cmd = {
            let mut command = if door.use_shell_execute {
                let mut command = tokio::process::Command::new("sh");
                command.arg("-c").arg("exec \"$0\" \"$@\"").arg(&file_name);
                command
            } else {
                tokio::process::Command::new(&file_name)
            };
            command
                .args(&arguments)
                .current_dir(&working_directory)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true);
            command.spawn()?
        };
        usage.restore_terminal = true;
        usage.start(self)?;

        let mut write_buf = vec![0; 32 * 1024];
        let mut read_buf = vec![0; 128 * 1024];
        #[cfg(not(unix))]
        let mut stdin = cmd.stdin.take().unwrap();
        #[cfg(not(unix))]
        let mut stdout = cmd.stdout.take().unwrap();
        let mut exited = false;
        let mut drain_deadline = tokio::time::Instant::now();

        loop {
            tokio::select! {
                write_data = stdout.read(&mut read_buf)=> {
                    match write_data {
                        Ok(size) => {
                            if size > 0 {
                                log::debug!("door sent {size} bytes");
                                if self.connection.send(&read_buf[0..size]).await.is_err() {
                                    break;
                                }
                                self.track_door_output(&read_buf[0..size]);
                                let mut remove_sysop_connection = false;
                                let node_state = &mut self.node_state.lock().await;
                                if let Some(sysop_connection) = &mut node_state[self.node].as_mut().unwrap().sysop_connection
                                    && let Err(_) = sysop_connection.send(&read_buf[0..size]).await {
                                        remove_sysop_connection = true;
                                    }
                                if remove_sysop_connection {
                                    node_state[self.node].as_mut().unwrap().sysop_connection = None;
                                }
                                drain_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                            } else {
                                break;
                            }
                        }
                        #[cfg(unix)]
                        Err(error) if !door.provide_socket_connection && error.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error()) => break,
                        Err(e) => {
                            log::error!("Error reading from door: {e}");
                            break;
                        }
                    }
                }
                read_data = self.connection.read(&mut write_buf) => {
                    match read_data {
                        Ok(size) => {
                            if size == 0 {
                                self.session.request_logoff = true;
                                break;
                            }
                            if stdin.write_all(&write_buf[0..size]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                status = cmd.wait(), if !exited => {
                    status?;
                    exited = true;
                    drain_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                }
                _ = tokio::time::sleep_until(drain_deadline), if exited => break,
            };
        }
        drop(stdin);
        drop(stdout);
        if cmd.try_wait()?.is_none() {
            cmd.kill().await?;
        }
        log::info!("door exited.");

        Ok(())
    }

    async fn run_dos_door(&mut self, door: &crate::icy_board::doors::Door, door_number: usize, usage: &mut DoorUsage<'_>) -> Res<()> {
        let _machine_guard = DOS_MACHINE_LOCK.lock().await;
        let source_path = self.resolve_path(&door.path);
        let assets = self.resolve_path(&"assets/dos");
        let base_image_path = assets.join("freedos.img");
        let image_path = assets.join("doors").join(crate::icy_board::doors::dos::image_file_name(&door.name));
        let bios_path = assets.join("seabios.bin");
        let vga_bios_path = assets.join("vgabios.bin");
        if !source_path.is_dir() {
            return Err(format!("DOS door path is not a directory: {}", source_path.display()).into());
        }
        if door.dos_command.trim().is_empty() {
            return Err(format!("DOS command is empty for door '{}'", door.name).into());
        }
        crate::icy_board::doors::dos::validate_simple_command(&source_path, &door.dos_command)?;
        if !crate::icy_board::doors::dos::dos_assets_ready(&assets) {
            log::info!("Preparing missing DOS assets in {}", assets.display());
            self.display_text(IceText::PreparingDosAssets, display_flags::NEWLINE).await?;
            let prepare_assets = assets.clone();
            let prepared = tokio::task::spawn_blocking(move || crate::icy_board::doors::dos::prepare_dos_assets(&prepare_assets)).await?;
            if let Err(error) = prepared {
                self.display_text(IceText::DosPreparationFailed, display_flags::NEWLINE).await?;
                return Err(format!(
                    "Could not prepare native DOS assets: {error}. Retry or run 'icbsetup dos-image {}' before opening the door",
                    self.root_path.display()
                )
                .into());
            }
        }
        if crate::icy_board::doors::dos::create_door_image(&base_image_path, &image_path, &source_path)? {
            log::info!("Created DOS door image {} from {}", image_path.display(), source_path.display());
        }

        let drop_directory = tempfile::tempdir()?;
        door.create_drop_file(self, drop_directory.path(), door_number).await?;
        let mut files = Vec::new();
        for entry in std::fs::read_dir(drop_directory.path())? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                files.push((entry.file_name().to_string_lossy().to_string(), std::fs::read(entry.path())?));
            }
        }
        let drop_file = files.first().map(|(name, _)| name.as_str()).unwrap_or("");
        let run_batch = crate::icy_board::doors::dos::expand_run_batch(door, self.node, drop_file)?;
        crate::icy_board::doors::dos::inject_session_files(&image_path, &files, &run_batch)?;

        let runtime_remaining = dos_runtime_remaining(self.session.login_date, self.session.time_limit, door.dos_max_runtime_seconds)
            .expect("DOS doors always have a hard runtime limit");
        let mut session = crate::icy_board::doors::dos::start_session(&image_path, &bios_path, &vga_bios_path, door.dos_memory_mb, runtime_remaining)?;
        usage.restore_terminal = true;
        // Native DOS has started once its emulator worker was successfully
        // launched. Guest boot/game failures after this point are billable.
        usage.start(self)?;
        let mut input = vec![0; 32 * 1024];
        let mut input_encoder = DosInputEncoder::default();
        let startup_timeout = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(startup_timeout);
        let session_timeout = async move {
            tokio::time::sleep(runtime_remaining).await;
        };
        tokio::pin!(session_timeout);
        let mut received_output = false;
        let mut worker_finished = false;
        loop {
            tokio::select! {
                output = session.output.recv() => {
                    let Some(output) = output else { break };
                    received_output = true;
                    let caller_output = dos_output_for_terminal(&output, self.session.term_caps.is_utf8);
                    self.connection.send(&caller_output).await?;
                    self.track_door_output(&output);
                    let node_state = &mut self.node_state.lock().await;
                    if let Some(sysop_connection) = &mut node_state[self.node].as_mut().unwrap().sysop_connection {
                        let _ = sysop_connection.send(&output).await;
                    }
                }
                read = self.connection.read(&mut input) => {
                    let size = read?;
                    if size == 0 {
                        self.session.request_logoff = true;
                        break;
                    }
                    let dos_input = input_encoder.encode(&input[..size], self.session.term_caps.is_utf8);
                    if !dos_input.is_empty() && session.input.send(dos_input).is_err() {
                        break;
                    }
                }
                result = &mut session.finished => {
                    worker_finished = true;
                    result.map_err(|_| "native DOS emulator thread ended unexpectedly; see icboard.log for the panic backtrace")??;
                    break;
                }
                _ = &mut startup_timeout, if !received_output => {
                    let stopped = session.stop().await;
                    if !stopped {
                        log::error!(
                            "native DOS emulator for '{}' did not stop within 5 seconds after startup timeout",
                            door.name
                        );
                    }
                    return Err(format!(
                        "native DOS door '{}' produced no COM1 output within 30 seconds; emulator stopped: {}; image: {}",
                        door.name,
                        stopped,
                        image_path.display()
                    )
                    .into());
                }
                _ = &mut session_timeout => {
                    if !session.stop().await {
                        log::error!(
                            "native DOS emulator for '{}' did not stop within 5 seconds after its runtime limit",
                            door.name
                        );
                    }
                    let _ = self.connection.send(b"\r\nDOS door runtime limit reached; returning to Icy Board.\r\n").await;
                    self.check_time_left().await;
                    worker_finished = true;
                    break;
                }
            }
        }
        if !worker_finished && !session.stop().await {
            log::error!(
                "native DOS emulator for '{}' did not stop within 5 seconds after the caller disconnected",
                door.name
            );
        }
        Ok(())
    }

    pub(crate) async fn send_editor_output(&mut self, bytes: &[u8]) -> Res<()> {
        let output = dos_output_for_terminal(bytes, self.session.term_caps.is_utf8);
        self.connection.send(&output).await?;
        {
            let mut nodes = self.node_state.lock().await;
            if let Some(node) = nodes.get_mut(self.node).and_then(Option::as_mut)
                && let Some(connection) = &mut node.sysop_connection
            {
                if connection.send(bytes).await.is_err() {
                    node.sysop_connection = None;
                }
            }
        }
        self.track_door_output(bytes);
        Ok(())
    }

    pub(crate) async fn run_dos_editor(
        &mut self,
        config: &crate::icy_board::icb_config::ExternalEditorConfig,
        directory: &std::path::Path,
        remaining: Duration,
    ) -> Res<bool> {
        use crate::icy_board::doors::dos;
        let started = std::time::Instant::now();
        let _machine_guard = tokio::time::timeout(remaining, DOS_MACHINE_LOCK.lock())
            .await
            .map_err(|_| "DOS editor is busy")?;
        let source = self.resolve_path(&config.path).canonicalize()?;
        if !source.is_dir() {
            return Err("DOS editor path must be its installation directory".into());
        }
        if config.arguments.trim().is_empty() || config.arguments.contains(['\r', '\n']) {
            return Err("DOS editor arguments must contain one command line".into());
        }
        let assets = self.resolve_path(&"assets/dos");
        let image = directory.join("editor.img");
        dos::create_door_image(&assets.join("freedos.img"), &image, &source)?;
        let command = config
            .arguments
            .replace("{work}", "C:\\DOOR")
            .replace("{node}", &self.node.to_string())
            .replace("{baud}", "57600")
            .replace("{time}", &remaining.as_secs().div_ceil(60).to_string());
        let mut files = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            if entry.path() != image && entry.file_type()?.is_file() {
                files.push((entry.file_name().to_string_lossy().to_string(), std::fs::read(entry.path())?));
            }
        }
        files.push(("RESULT.ED".into(), Vec::new()));
        files.push(("ICBEDIT.RC".into(), Vec::new()));
        dos::inject_session_files(&image, &files, &dos::editor_run_batch(&command))?;
        let remaining = remaining.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err("DOS editor timed out preparing its image".into());
        }
        let mut session = dos::start_session(
            &image,
            &assets.join("seabios.bin"),
            &assets.join("vgabios.bin"),
            config.dos_memory_mb.max(1),
            remaining,
        )?;
        let mut input = vec![0; 4096];
        let mut encoder = DosInputEncoder::default();
        let mut output_open = true;
        let mut finished = false;
        let timeout = tokio::time::sleep(remaining);
        tokio::pin!(timeout);
        let result: Res<bool> = async {
            loop {
                tokio::select! {
                    output = session.output.recv(), if output_open => {
                        if let Some(output) = output {
                            self.send_editor_output(&output).await?;
                        } else { output_open = false; }
                    },
                    read = self.connection.read(&mut input) => {
                        let count = read?;
                        if count == 0 { self.session.request_logoff = true; return Ok(false); }
                        let bytes = encoder.encode(&input[..count], self.session.term_caps.is_utf8);
                        if !bytes.is_empty() { session.input.send(bytes).map_err(|_| "DOS editor input closed")?; }
                    },
                    result = &mut session.finished => {
                        finished = true;
                        result.map_err(|_| "DOS editor worker failed")??;
                        return Ok(true);
                    },
                    _ = &mut timeout => { self.check_time_left().await; return Err("DOS editor timed out".into()); }
                }
            }
        }
        .await;
        if !finished && !session.stop().await {
            return Err("DOS editor worker did not stop".into());
        }
        if !result? {
            return Ok(false);
        }
        let status = dos::read_editor_file(&image, "ICBEDIT.RC", 32)?;
        match String::from_utf8(status)?.trim() {
            "0" => {
                let limit = self.get_board().await.config.message.max_msg_lines.max(1) as usize * 80 * 4 + 3;
                for (name, limit) in [("MSGTMP", limit), ("RESULT.ED", 4096)] {
                    match dos::read_editor_file(&image, name, limit) {
                        Ok(bytes) => std::fs::write(directory.join(name), bytes)?,
                        Err(error)
                            if name == "RESULT.ED"
                                && error
                                    .downcast_ref::<std::io::Error>()
                                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => {}
                        Err(error) => return Err(error),
                    }
                }
                Ok(true)
            }
            "1" => Ok(false),
            "2" => {
                self.session.request_logoff = true;
                Ok(false)
            }
            status => Err(format!("DOS editor failed or did not return a status: {status}").into()),
        }
    }

    pub async fn run_bbslink_door(&mut self, bbslink: &BBSLink, door: &Door) -> Res<()> {
        // Keep direct callers on the same gates and billing path as OPEN.
        let list = DoorList {
            accounts: vec![DoorServerAccount::BBSLink(bbslink.clone())],
            doors: Vec::new(),
        };
        self.run_door(&list, door, door.number).await
    }

    async fn run_bbslink_door_started(&mut self, bbslink: &BBSLink, door: &Door, usage: &mut DoorUsage<'_>) -> Res<()> {
        log::info!("Running door: {}, requesting token", door.path);
        let x_key: String = (0..12)
            .map(|_| {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                CHARSET[fastrand::usize(..CHARSET.len())] as char
            })
            .collect();
        let token = reqwest::get(format!("https://games.bbslink.net/token.php?{x_key}")).await?.text().await?;
        log::info!("got token {token}, sending credentials");
        /* Not sure why this doesn't work:
        let mut map = http::header::HeaderMap::new();
        map.insert("X-User", self.session.cur_user.into());
        map.insert("X-System", bbslink.system_code.parse()?);
        map.insert("X-Auth", format!("{:x}", md5::compute(bbslink.auth_code.clone() + token.as_str())).parse()?);
        map.insert("X-Code", format!("{:x}", md5::compute(bbslink.sheme_code.clone() + token.as_str())).parse()?);
        map.insert("X-Rows", self.user_screen.buffer.height().into());
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
            "https://games.bbslink.net/auth.php?key={}&user={}&system={}&auth={:x}&scheme={:x}&rows={}&door={}&token={}&type={}&version={}",
            x_key,
            self.session.cur_user_id,
            bbslink.system_code,
            md5::compute(bbslink.auth_code.clone() + token.as_str()),
            md5::compute(bbslink.sheme_code.clone() + token.as_str()),
            self.display_screen().buffer.height(),
            door.path,
            token,
            "icy_board",
            *crate::VERSION
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
                    usage.start(self)?;
                    let () = execute_door(&mut connection, self).await?;
                    return Ok(());
                }
                log::info!("got server response '{str}'");
                for e in parse_bbslink_error(&str) {
                    log::error!("Unauthorised: {e}");
                }
                self.display_text(
                    IceText::DOORNotAvailable,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
            }
            Err(e) => {
                log::error!("Error opening door : {e}");
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
                            state.track_door_output(&read_buf[0..size]);
                            let node_state = &mut state.node_state.lock().await;
                            if let Some(sysop_connection) = &mut node_state[state.node].as_mut().unwrap().sysop_connection {
                                let _ = sysop_connection.send(&read_buf[0..size]).await;
                            }

                          } else {
                            return Ok(());
                          }
                     }
                     Err(e) => {
                        log::error!("Error reading from connection: {e}");
                        return Err(e);
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
                        } else {
                            state.session.request_logoff = true;
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading from connection: {e}");
                        return Err(e);
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
        if let Some(m) = cp.get(1)
            && let Ok(e) = m.as_str().parse::<usize>()
        {
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
    use std::time::Duration;

    use crate::icy_board::state::user_commands::pcb::open_door::{BBSLinkError, DosInputEncoder, dos_output_for_terminal, dos_runtime_remaining};

    use super::parse_bbslink_error;
    #[test]
    fn test_parse_bbslink_error() {
        let output = parse_bbslink_error("Unauthorised (Error 7)*xxUnauthorised (Error 2)");
        assert_eq!(output, vec![BBSLinkError::Error7, BBSLinkError::Error2]);
    }

    #[test]
    fn dos_output_is_cp437_for_legacy_terminals_and_utf8_for_unicode_terminals() {
        let cp437 = [0x1B, b'[', b'1', b'm', 0x81, 0x82, 0xC4];
        assert_eq!(dos_output_for_terminal(&cp437, false), cp437);
        assert_eq!(dos_output_for_terminal(&cp437, true), "\x1B[1müé─".as_bytes());
    }

    #[test]
    fn utf8_dos_input_is_encoded_as_cp437_across_read_boundaries() {
        let mut encoder = DosInputEncoder::default();
        assert_eq!(encoder.encode(&[0x1B, b'[', b'A', 0xC3], true), b"\x1B[A");
        assert_eq!(encoder.encode(&[0xBC, 0xE2, 0x94], true), vec![0x81]);
        assert_eq!(encoder.encode(&[0x80], true), vec![0xC4]);
        assert_eq!(encoder.encode("€".as_bytes(), true), b".");
        let controls: Vec<u8> = (0..32).collect();
        assert_eq!(encoder.encode(&controls, true), controls);
    }

    #[test]
    fn dos_doors_observe_runtime_limits() {
        let now = chrono::Utc::now();
        assert_eq!(
            dos_runtime_remaining(now, 0, 0),
            Some(Duration::from_secs(crate::icy_board::doors::DEFAULT_DOS_MAX_RUNTIME_SECONDS.into()))
        );
        assert_eq!(dos_runtime_remaining(now - chrono::Duration::minutes(2), 1, 0), Some(Duration::ZERO));

        let remaining = dos_runtime_remaining(now, 1, 0).unwrap();
        assert!(remaining > Duration::from_secs(59));
        assert!(remaining <= Duration::from_secs(60));

        assert_eq!(dos_runtime_remaining(now, 0, 180), Some(Duration::from_secs(180)));
        assert_eq!(dos_runtime_remaining(now, 1, 30), Some(Duration::from_secs(30)));
    }
}
