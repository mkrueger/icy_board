use std::{fs::File, io::Read, path::Path, process::Stdio, time::Duration};

use crate::{
    Res,
    icy_board::{
        doors::Door,
        icb_config::{ExternalEditorConfig, ExternalEditorMode},
        read_data_with_encoding_detection,
        state::IcyBoardState,
    },
    vm::{DiskIO, run},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{EditResult, EditState};

pub(crate) struct EditorOutput {
    pub text: String,
    pub subject: Option<String>,
    pub editor: Option<String>,
}

impl IcyBoardState {
    #[async_recursion::async_recursion(?Send)]
    pub(crate) async fn run_external_editor(&mut self, config: &ExternalEditorConfig, editor: &mut EditState, area: &str, private: bool) -> Res<EditResult> {
        if config.path.trim().is_empty() {
            return Err("external editor path is empty".into());
        }
        if self.session.request_logoff {
            return Ok(EditResult::Abort);
        }
        let directory = tempfile::tempdir()?;
        let text = if editor.msg.is_empty() && !editor.quote_text.is_empty() {
            let mut quotes = EditState {
                quote_text: editor.quote_text.clone(),
                max_lines: editor.max_lines,
                max_line_length: editor.max_line_length,
                ..Default::default()
            };
            let count = quotes.quote_text.iter().filter(|line| !line.starts_with('\x01')).count();
            if count > 0 && !quotes.insert_quote(1, count) {
                return Err("quoted text exceeds the message line limit".into());
            }
            quotes.msg.join("\n")
        } else {
            editor.msg.join("\n")
        };
        write_input(
            directory.path(),
            &editor.from,
            &editor.to,
            &editor.subj,
            area,
            private,
            &text,
            config.mode == ExternalEditorMode::Ppe,
        )?;
        if config.mode != ExternalEditorMode::Ppe {
            Door {
                drop_file: config.drop_file.clone(),
                ..Default::default()
            }
            .create_drop_file(self, directory.path(), 0)
            .await?;
            if config.drop_file == crate::icy_board::doors::DropFile::ExitInfoBBS {
                Door {
                    drop_file: crate::icy_board::doors::DropFile::DorInfo,
                    ..Default::default()
                }
                .create_drop_file(self, directory.path(), 0)
                .await?;
            }
            if matches!(
                config.drop_file,
                crate::icy_board::doors::DropFile::DorInfo | crate::icy_board::doors::DropFile::ExitInfoBBS
            ) && self.node != 0
            {
                std::fs::copy(
                    directory.path().join(format!("DORINFO{}.DEF", self.node + 1)),
                    directory.path().join("DORINFO1.DEF"),
                )?;
            }
        }
        let maximum = Duration::from_secs(if config.timeout_seconds == 0 { 3600 } else { config.timeout_seconds }.into());
        let remaining = if self.session.time_limit == 0 {
            maximum
        } else {
            let deadline = self.session.login_date + chrono::Duration::minutes(self.session.time_limit.into());
            maximum.min((deadline - chrono::Utc::now()).to_std().unwrap_or(Duration::ZERO))
        };
        if remaining.is_zero() {
            self.check_time_left().await;
            return Ok(EditResult::Abort);
        }
        let saved = match config.mode {
            ExternalEditorMode::Internal => return Err("internal editor passed to external runner".into()),
            ExternalEditorMode::Ppe => {
                let executable_path = self.resolve_path(&config.path).canonicalize()?;
                let executable = crate::executable::Executable::read_file(&executable_path, false)?;
                if self.ppe_nesting >= 16 {
                    return Err("PPE nesting limit reached".into());
                }
                let arguments = shell_words::split(&config.arguments)?;
                let parent = executable_path.parent().unwrap().to_str().ok_or("invalid PPE path")?;
                let mut tokens = std::mem::take(&mut self.session.tokens);
                self.session.tokens.push_back(directory.path().to_string_lossy().to_string());
                self.session.tokens.extend(arguments);
                let previous_user_color = crate::icy_board::icb_config::IcbColor::Dos(Self::dos_attribute(self.user_screen.buffer.caret.attribute));
                let previous_sysop_color = crate::icy_board::icb_config::IcbColor::Dos(Self::dos_attribute(self.sysop_screen.buffer.caret.attribute));
                let mut io = DiskIO::new(parent, None);
                let nesting = self.ppe_nesting;
                self.ppe_nesting += 1;
                let result = tokio::time::timeout(remaining, run(&executable_path, &executable, &mut io, self)).await;
                self.ppe_nesting = nesting;
                std::mem::swap(&mut self.session.tokens, &mut tokens);
                if self.ppe_nesting == 0 {
                    self.cleanup_ppl_media().await;
                }
                self.restore_ppe_color(crate::vm::TerminalTarget::User, previous_user_color).await?;
                self.restore_ppe_color(crate::vm::TerminalTarget::Sysop, previous_sysop_color).await?;
                match result {
                    Ok(result) => result?,
                    Err(_) => {
                        self.check_time_left().await;
                        return Err("external editor timed out".into());
                    }
                }
            }
            ExternalEditorMode::Dos => self.run_dos_editor(config, directory.path(), remaining).await?,
            ExternalEditorMode::Program | ExternalEditorMode::Script => self.run_native_editor(config, directory.path(), remaining).await?,
        };
        if !saved || self.session.request_logoff {
            return Ok(EditResult::Abort);
        }
        let Some(output) = read_output(directory.path(), editor.max_lines, false)? else {
            return Ok(EditResult::Abort);
        };
        editor.msg = output.text.lines().map(str::to_owned).collect();
        if let Some(subject) = output.subject {
            editor.subj = subject;
        }
        editor.editor_details = output.editor;
        Ok(EditResult::SendMessage)
    }

    async fn run_native_editor(&mut self, config: &ExternalEditorConfig, directory: &Path, remaining: Duration) -> Res<bool> {
        let path = self.resolve_path(&config.path).canonicalize()?;
        let arguments = shell_words::split(&config.arguments)?
            .into_iter()
            .map(|argument| {
                argument
                    .replace("{work}", &directory.to_string_lossy())
                    .replace("{node}", &self.node.to_string())
                    .replace("{baud}", "57600")
                    .replace("{time}", &remaining.as_secs().div_ceil(60).to_string())
            })
            .collect::<Vec<_>>();
        let mut command = if config.mode == ExternalEditorMode::Script {
            let mut command = tokio::process::Command::new("sh");
            command.arg(&path);
            command
        } else {
            tokio::process::Command::new(&path)
        };
        let mut child = command
            .args(arguments)
            .current_dir(directory)
            .env("ICB_EDITOR_DIR", directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let mut input = child.stdin.take().ok_or("editor stdin unavailable")?;
        let mut output = child.stdout.take().ok_or("editor stdout unavailable")?;
        let mut input_buffer = vec![0; 4096];
        let mut output_buffer = vec![0; 32768];
        let mut encoder = crate::icy_board::state::user_commands::pcb::open_door::DosInputEncoder::default();
        let mut output_open = true;
        let mut exit_status = None;
        let timeout = tokio::time::sleep(remaining);
        tokio::pin!(timeout);
        let result = async {
            loop {
                if let Some(status) = exit_status
                    && !output_open
                {
                    return match status {
                        Some(0) => Ok(true),
                        Some(1) => Ok(false),
                        Some(2) => {
                            self.session.request_logoff = true;
                            Ok(false)
                        }
                        code => Err(format!("external editor failed with status {code:?}").into()),
                    };
                }
                tokio::select! {
                    status = child.wait(), if exit_status.is_none() => exit_status = Some(status?.code()),
                    read = output.read(&mut output_buffer), if output_open => {
                        let count = read?;
                        if count == 0 { output_open = false; }
                        else {
                            self.send_editor_output(&output_buffer[..count]).await?;
                        }
                    },
                    read = self.connection.read(&mut input_buffer), if exit_status.is_none() => {
                        let count = read?;
                        if count == 0 { self.session.request_logoff = true; return Ok(false); }
                        let bytes = encoder.encode(&input_buffer[..count], self.session.term_caps.is_utf8);
                        input.write_all(&bytes).await?;
                    },
                    _ = &mut timeout => { self.check_time_left().await; return Err("external editor timed out".into()); }
                }
            }
        }
        .await;
        if child.try_wait()?.is_none() {
            child.kill().await?;
        }
        result
    }
}

fn header_line(value: &str) -> String {
    value.chars().filter(|character| !character.is_control()).collect()
}

fn encode(text: &str, utf8: bool) -> Vec<u8> {
    if utf8 {
        let mut bytes = vec![0xef, 0xbb, 0xbf];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    } else {
        text.chars()
            .map(|character| codepages::tables::UNICODE_TO_CP437.get(&character).copied().unwrap_or(b'?'))
            .collect()
    }
}

fn write_input(path: &Path, from: &str, to: &str, subject: &str, area: &str, private: bool, text: &str, utf8: bool) -> Res<()> {
    let info = format!(
        "{}\r\n{}\r\n{}\r\n1\r\n{}\r\n{}\r\n",
        header_line(from),
        header_line(to),
        header_line(subject),
        header_line(area),
        if private { "YES" } else { "NO" }
    );
    std::fs::write(path.join("MSGINF"), encode(&info, utf8))?;
    let text = text.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
    std::fs::write(path.join("MSGTMP"), encode(&text, utf8))?;
    match std::fs::remove_file(path.join("RESULT.ED")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_limited(path: &Path, limit: usize) -> Res<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("external editor output exceeds the configured limit".into());
    }
    Ok(bytes)
}

fn decode_text(mut bytes: Vec<u8>) -> Res<String> {
    if !bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        for byte in &mut bytes {
            if *byte == 141 {
                *byte = b'\n';
            }
        }
    }
    Ok(read_data_with_encoding_detection(&bytes)?.replace("\r\n", "\n").replace('\r', "\n"))
}

fn read_output(path: &Path, max_lines: usize, subject_read_only: bool) -> Res<Option<EditorOutput>> {
    let text = decode_text(read_limited(&path.join("MSGTMP"), max_lines.saturating_mul(80 * 4).saturating_add(3))?)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    if text.lines().count() > max_lines {
        return Err("external editor returned too many message lines".into());
    }
    let mut output = EditorOutput {
        text,
        subject: None,
        editor: None,
    };
    match read_limited(&path.join("RESULT.ED"), 4096) {
        Ok(bytes) => {
            let result = read_data_with_encoding_detection(&bytes)?;
            let mut lines = result.lines();
            lines.next();
            if let Some(subject) = lines.next().map(str::trim).filter(|subject| !subject.is_empty()) {
                if !subject_read_only {
                    output.subject = Some(header_line(subject));
                }
            }
            output.editor = lines.next().map(str::trim).filter(|editor| !editor.is_empty()).map(header_line);
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => {}
        Err(error) => return Err(error),
    }
    Ok(Some(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_state(root: &Path) -> (IcyBoardState, icy_net::channel::ChannelConnection) {
        use crate::icy_board::{IcyBoard, bbs::BBS, user_base::User};
        use icy_net::{ConnectionType, channel::ChannelConnection};
        use std::sync::Arc;
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = root.into();
        board.users.new_user(User {
            name: "Editor Tester".into(),
            ..Default::default()
        });
        let user = board.users[0].clone();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user);
        state.session.user_name = "Editor Tester".into();
        state.session.time_limit = 30;
        (state, peer)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn external_editor_script_save_abort_and_error_preserve_draft() {
        for status in [0, 1, 2, 3] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = test_state(root.path()).await;
            let script = root.path().join("editor.sh");
            std::fs::write(&script, format!("test -f MSGINF || exit 4\nprintf 'edited body\\r\\n' > MSGTMP\nprintf '0\\r\\nChanged subject\\r\\nTest editor\\r\\n' > RESULT.ED\nprintf 'final output'\nexit {status}\n")).unwrap();
            let config = ExternalEditorConfig {
                mode: ExternalEditorMode::Script,
                path: script.to_string_lossy().into(),
                drop_file: crate::icy_board::doors::DropFile::None,
                ..Default::default()
            };
            let mut editor = EditState {
                msg: vec!["original".into()],
                subj: "Original subject".into(),
                max_lines: 100,
                max_line_length: 79,
                ..Default::default()
            };
            let result = state.run_external_editor(&config, &mut editor, "General", false).await;
            use icy_net::Connection;
            let mut buffer = [0; 64];
            let count = peer.try_read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..count], b"final output");
            match status {
                0 => {
                    assert_eq!(result.unwrap(), EditResult::SendMessage);
                    assert_eq!(editor.msg, vec!["edited body"]);
                    assert_eq!(editor.subj, "Changed subject");
                    assert_eq!(editor.editor_details.as_deref(), Some("Test editor"));
                }
                1 | 2 => assert_eq!(result.unwrap(), EditResult::Abort),
                _ => assert!(result.is_err()),
            }
            if status != 0 {
                assert_eq!(editor.msg, vec!["original"]);
                assert_eq!(editor.subj, "Original subject");
            }
            assert_eq!(state.session.request_logoff, status == 2);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn external_editor_invalid_output_and_timeout_leave_draft_unchanged() {
        for script in ["printf 'one\\ntwo\\n' > MSGTMP", "exec cat", "exit 3"] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, _peer) = test_state(root.path()).await;
            let path = root.path().join("editor.sh");
            std::fs::write(&path, script).unwrap();
            {
                let mut board = state.get_board().await;
                board.config.message.max_msg_lines = 1;
                board.config.message.external_editor = ExternalEditorConfig {
                    mode: ExternalEditorMode::Script,
                    path: path.to_string_lossy().into(),
                    drop_file: crate::icy_board::doors::DropFile::None,
                    timeout_seconds: 1,
                    ..Default::default()
                };
            }
            state.session.tokens.push_back("caller argument".into());
            let config = state.get_board().await.config.message.external_editor.clone();
            let mut editor = EditState { msg: vec!["original".into()], subj: "Original".into(), max_lines: 1, max_line_length: 79, ..Default::default() };
            assert!(state.run_external_editor(&config, &mut editor, "General", false).await.is_err());
            assert_eq!(editor.msg, ["original"]);
            assert_eq!(editor.subj, "Original");
            assert_eq!(state.session.tokens.front().map(String::as_str), Some("caller argument"));
        }
    }

    #[tokio::test]
    async fn external_editor_ppe_files_and_stop_restore_callers_tokens() {
        for stop in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, _peer) = test_state(root.path()).await;
            let source = format!(
                "STRING directory\nGETTOKEN directory\nFCREATE 1, directory + \"/MSGTMP\", O_WR, S_DN\nFPUTLN 1, \"PPE reply\"\nFCLOSE 1\nFCREATE 1, directory + \"/RESULT.ED\", O_WR, S_DN\nFPUTLN 1, \"0\"\nFPUTLN 1, \"PPE subject\"\nFPUTLN 1, \"Test PPE\"\nFCLOSE 1\n{}\n",
                if stop { "STOP" } else { "EXIT" }
            );
            let executable = crate::vm::tests::compile(&source);
            let path = root.path().join("editor.ppe");
            std::fs::write(&path, executable.to_buffer().unwrap()).unwrap();
            let config = ExternalEditorConfig {
                mode: ExternalEditorMode::Ppe,
                path: path.to_string_lossy().into(),
                ..Default::default()
            };
            state.session.tokens.push_back("caller argument".into());
            state.ppe_nesting = 1;
            let mut editor = EditState { msg: vec!["original body".into()], subj: "Original".into(), max_lines: 100, max_line_length: 79, ..Default::default() };
            let result = state.run_external_editor(&config, &mut editor, "General", false).await.unwrap();
            assert_eq!(result, if stop { EditResult::Abort } else { EditResult::SendMessage });
            assert_eq!(state.session.tokens.front().map(String::as_str), Some("caller argument"));
            assert_eq!(state.ppe_nesting, 1);
            assert_eq!(editor.msg.join("\n"), if stop { "original body" } else { "PPE reply" });
            assert_eq!(editor.subj, if stop { "Original" } else { "PPE subject" });
        }
    }

    #[tokio::test]
    async fn external_editor_dropfiles_finish_and_dorinfo_has_thirteen_lines() {
        let root = tempfile::tempdir().unwrap();
        let (state, _peer) = test_state(root.path()).await;
        for drop_file in [crate::icy_board::doors::DropFile::DorInfo, crate::icy_board::doors::DropFile::ExitInfoBBS] {
            tokio::time::timeout(
                Duration::from_secs(1),
                Door {
                    drop_file,
                    ..Default::default()
                }
                .create_drop_file(&state, root.path(), 0),
            )
            .await
            .unwrap()
            .unwrap();
        }
        let info = std::fs::read_to_string(root.path().join("DORINFO1.DEF")).unwrap();
        assert_eq!(info.lines().count(), 13);
        assert_eq!(info.lines().nth(6), Some("Editor"));
        assert_eq!(info.lines().nth(7), Some("Tester"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn external_editor_message_context_keeps_private_thread_and_attachment_metadata() {
        use bstr::BString;
        use jamjam::jam::{
            JamMessage, attributes,
            msg_header::{MessageSubfield, SubfieldType},
        };
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = test_state(root.path()).await;
        state.session.fse_mode = crate::icy_board::user_base::FSEMode::Yes;
        let script = root.path().join("editor.sh");
        std::fs::write(
            &script,
            "printf 'new body' > MSGTMP\nprintf '0\\r\\nNew subject\\r\\nTest editor\\r\\n' > RESULT.ED\n",
        )
        .unwrap();
        state.get_board().await.config.message.external_editor = ExternalEditorConfig {
            mode: ExternalEditorMode::Script,
            path: script.to_string_lossy().into(),
            drop_file: crate::icy_board::doors::DropFile::None,
            ..Default::default()
        };
        let original = JamMessage::default()
            .with_from(BString::from("From"))
            .with_to(BString::from("To"))
            .with_subject(BString::from("Subject"))
            .with_attributes(attributes::MSG_PRIVATE)
            .with_sub_field(MessageSubfield::new(SubfieldType::ReplyID, BString::from("thread")))
            .with_sub_field(MessageSubfield::new(SubfieldType::EnclFile, BString::from("file.zip")));
        let mut message = JamMessage::from_stored(original.header().clone(), BString::from("original body"));
        assert_eq!(state.edit_message_context(&mut message, Vec::new()).await.unwrap(), EditResult::SendMessage);
        assert_eq!(message.text().to_string(), "new body");
        assert_eq!(message.header().subject().unwrap().to_string(), "New subject");
        assert_eq!(message.header().attributes, original.header().attributes);
        for field in &original.header().sub_fields {
            if field.field_type() != SubfieldType::Subject {
                assert!(
                    message
                        .header()
                        .sub_fields
                        .iter()
                        .any(|updated| updated.field_type() == field.field_type() && updated.content() == field.content())
                );
            }
        }
        assert!(
            message
                .header()
                .sub_fields
                .iter()
                .any(|field| field.field_type() == SubfieldType::PID && field.content() == &BString::from("Test editor"))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "requires ICB_ICEEDIT_SOURCE and ICB_DOS_ASSETS"]
    async fn external_editor_real_iceedit() {
        use crate::icy_board::{IcyBoard, bbs::BBS, doors::DropFile, state::GraphicsMode, user_base::User};
        use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
        use std::sync::Arc;
        let root = tempfile::tempdir().unwrap();
        let source = std::path::PathBuf::from(std::env::var_os("ICB_ICEEDIT_SOURCE").expect("ICB_ICEEDIT_SOURCE"));
        let installation = root.path().join("iceedit");
        std::fs::create_dir(&installation).unwrap();
        for entry in std::fs::read_dir(&source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(entry.path(), installation.join(entry.file_name())).unwrap();
            }
        }
        if let Some(driver) = std::env::var_os("ICB_FOSSIL_DRIVER") {
            std::fs::copy(driver, installation.join("X00.EXE")).unwrap();
            std::fs::write(
                installation.join("ICBSTART.BAT"),
                b"@ECHO OFF\r\nX00.EXE E\r\nICEEDIT.EXE /D:C:\\DOOR /N:1 /T:30 /K:15\r\n",
            )
            .unwrap();
        }
        let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
        std::fs::create_dir_all(root.path().join("assets")).unwrap();
        std::os::unix::fs::symlink(assets, root.path().join("assets/dos")).unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = root.path().to_path_buf();
        board.users.new_user(User {
            name: "Editor Tester".into(),
            ..Default::default()
        });
        let user = board.users[0].clone();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user);
        state.session.user_name = "Editor Tester".into();
        state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
        state.session.term_caps.is_utf8 = true;
        state.session.time_limit = 30;
        state.session.page_len = 25;
        let config = ExternalEditorConfig {
            mode: ExternalEditorMode::Dos,
            path: installation.to_string_lossy().to_string(),
            arguments: std::env::var("ICB_ICEEDIT_COMMAND").unwrap_or_else(|_| "ICEEDIT.EXE /D:C:\\DOOR /N:1 /T:30 /K:15".into()),
            drop_file: if std::env::var_os("ICB_ICEEDIT_EXITINFO").is_some() {
                DropFile::ExitInfoBBS
            } else {
                DropFile::DorInfo
            },
            timeout_seconds: 40,
            ..Default::default()
        };
        let mut editor = EditState {
            from: "Editor Tester".into(),
            to: "Recipient".into(),
            subj: "ICE editor test".into(),
            max_lines: 100,
            max_line_length: 79,
            ..Default::default()
        };
        let abort = std::env::var_os("ICB_ICEEDIT_ABORT").is_some();
        let mut transcript = Vec::new();
        let mut buffer = vec![0; 32768];
        let trigger = std::env::var("ICB_ICEEDIT_TRIGGER").unwrap_or_else(|_| "\x1b[5;1H".into());
        let keys = if abort {
            String::new()
        } else {
            std::env::var("ICB_ICEEDIT_KEYS").unwrap_or_else(|_| "ICB editor roundtrip\r\x1a".into())
        };
        let mut sent = false;
        let mut pending_keys = std::collections::VecDeque::new();
        let mut key_clock = tokio::time::interval(Duration::from_millis(20));
        let result = {
            let future = state.run_external_editor(&config, &mut editor, "General", false);
            tokio::pin!(future);
            loop {
                tokio::select! {
                    result = &mut future => break result,
                    _ = key_clock.tick(), if !pending_keys.is_empty() => {
                        peer.send(&[pending_keys.pop_front().unwrap()]).await.unwrap();
                    },
                    read = peer.read(&mut buffer) => {
                        let count = read.unwrap();
                        if count == 0 { panic!("editor connection closed"); }
                        transcript.extend_from_slice(&buffer[..count]);
                        if !sent && String::from_utf8_lossy(&transcript).contains(&trigger) {
                            use icy_engine::TextPane;
                            let mut screen = crate::icy_board::state::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
                            screen.write_bytes(&transcript);
                            assert_eq!((screen.buffer.width(), screen.buffer.height()), (80, 25));
                            let rendered = (0..25).map(|row| (0..80).map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch).collect::<String>()).collect::<Vec<_>>().join("\n");
                            for text in ["Editor Tester", "Recipient", "ICE editor test", "General"] { assert!(rendered.contains(text), "missing {text}: {rendered}"); }
                            if let Some(path) = std::env::var_os("ICB_ICEEDIT_SCREEN") { std::fs::write(path, &rendered).unwrap(); }
                            eprintln!("ICE Edit input triggered: {} bytes", keys.len());
                            pending_keys.extend(keys.bytes());
                            sent = true;
                        }
                    }
                }
            }
        };
        if let Some(path) = std::env::var_os("ICB_ICEEDIT_TRANSCRIPT") {
            std::fs::write(path, &transcript).unwrap();
        }
        eprintln!("ICE Edit result: {result:?}; screen checked: {sent}");
        assert!(sent, "editor screen did not become ready");
        assert_eq!(result.unwrap(), if abort { EditResult::Abort } else { EditResult::SendMessage });
        if abort {
            assert!(editor.msg.is_empty());
        } else {
            assert!(editor.msg.join("\n").contains("ICB editor roundtrip"), "unexpected text: {:?}", editor.msg);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "requires ICB_DOS_ASSETS"]
    async fn external_editor_dos_status_and_optional_result_roundtrip() {
        let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
        for status in [0, 1, 2, 3] {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join("assets")).unwrap();
            std::os::unix::fs::symlink(&assets, root.path().join("assets/dos")).unwrap();
            let installation = root.path().join("editor");
            std::fs::create_dir(&installation).unwrap();
            std::fs::write(installation.join("STATUS.COM"), [0xb8, status, 0x4c, 0xcd, 0x21]).unwrap();
            std::fs::write(
                installation.join("EDITOR.BAT"),
                b"@ECHO OFF\r\nECHO edited body>MSGTMP\r\nDEL RESULT.ED\r\nSTATUS.COM\r\n",
            )
            .unwrap();
            let directory = tempfile::tempdir().unwrap();
            write_input(directory.path(), "From", "To", "Subject", "General", false, "original", false).unwrap();
            let (mut state, _peer) = test_state(root.path()).await;
            let config = ExternalEditorConfig {
                mode: ExternalEditorMode::Dos,
                path: installation.to_string_lossy().into(),
                arguments: "EDITOR.BAT".into(),
                ..Default::default()
            };
            let result = state.run_dos_editor(&config, directory.path(), Duration::from_secs(30)).await;
            match status {
                0 => assert!(result.unwrap()),
                1 | 2 => assert!(!result.unwrap()),
                _ => assert!(result.is_err()),
            }
            assert_eq!(state.session.request_logoff, status == 2);
            let output = read_output(directory.path(), 100, false).unwrap().unwrap();
            assert_eq!(output.text.trim(), if status == 0 { "edited body" } else { "original" });
            assert_eq!(output.subject, None);
        }
    }

    #[test]
    fn external_editor_ra_input_has_six_lines_and_no_stale_result() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("RESULT.ED"), b"stale").unwrap();
        write_input(
            directory.path(),
            "From\nInjected",
            "To",
            "Subject",
            "General",
            true,
            " > line1\n > line2\n",
            false,
        )
        .unwrap();
        assert_eq!(
            std::fs::read(directory.path().join("MSGINF")).unwrap(),
            b"FromInjected\r\nTo\r\nSubject\r\n1\r\nGeneral\r\nYES\r\n"
        );
        assert_eq!(std::fs::read(directory.path().join("MSGTMP")).unwrap(), b" > line1\r\n > line2\r\n");
        assert!(!directory.path().join("RESULT.ED").exists());
    }

    #[test]
    fn external_editor_imports_cp437_soft_returns_and_optional_result() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("MSGTMP"), b"Gr\x81sse\x8dnext\r\n").unwrap();
        let result = read_output(directory.path(), 10, false).unwrap().unwrap();
        assert_eq!(result.text, "Gr\u{fc}sse\nnext\n");
        assert_eq!(result.subject, None);
        std::fs::write(directory.path().join("RESULT.ED"), b"1\r\nNew subject\r\nExample 1.0\r\n").unwrap();
        let result = read_output(directory.path(), 10, false).unwrap().unwrap();
        assert_eq!(result.subject.as_deref(), Some("New subject"));
        assert_eq!(result.editor.as_deref(), Some("Example 1.0"));
        assert_eq!(read_output(directory.path(), 10, true).unwrap().unwrap().subject, None);
    }

    #[test]
    fn external_editor_accepts_ppe_utf8_but_rejects_empty_or_oversized_text() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("MSGTMP"), encode("\u{14d}\n", true)).unwrap();
        assert_eq!(read_output(directory.path(), 1, false).unwrap().unwrap().text, "\u{14d}\n");
        std::fs::write(directory.path().join("MSGTMP"), b" \r\n").unwrap();
        assert!(read_output(directory.path(), 1, false).unwrap().is_none());
        std::fs::write(directory.path().join("MSGTMP"), b"one\ntwo\n").unwrap();
        assert!(read_output(directory.path(), 1, false).is_err());
        std::fs::write(directory.path().join("MSGTMP"), vec![b'a'; 324]).unwrap();
        assert!(read_output(directory.path(), 1, false).is_err());
    }
}
