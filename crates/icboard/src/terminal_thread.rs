use base64::{Engine as _, engine::general_purpose};
use icy_board_engine::Res;
use icy_engine::{EditableScreen, Screen, Size, TextPane, TextScreen};
use icy_engine_gui::music::{
    SoundThread,
    audio_apc::{self, AudioFeatureQuery},
};
use icy_net::Connection;
use icy_parser_core::{AnsiParser, CommandParser};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
use tokio::sync::mpsc;

pub struct ConnectionThreadData {
    pub rx: mpsc::Receiver<SendData>,
    pub com: Box<dyn Connection>,
    pub _thread_is_running: bool,
    pub _is_connected: bool,
    media: LocalMedia,
}

const MAX_CACHED_MEDIA_SIZE: usize = 32 * 1024 * 1024;

struct LocalMedia {
    cache_directory: PathBuf,
    sound: SoundThread,
    pending: Vec<u8>,
}

impl LocalMedia {
    fn new() -> Self {
        let cache_directory = std::env::temp_dir().join(format!("icboard-tui-media-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&cache_directory);
        let mut sound = SoundThread::new();
        let _ = sound.configure(true, 0.8, None);
        Self {
            cache_directory,
            sound,
            pending: Vec::new(),
        }
    }

    fn store(&self, name: &str, encoded: &str) -> bool {
        let Some(path) = safe_cache_path(&self.cache_directory, name) else {
            return false;
        };
        if encoded.len() > MAX_CACHED_MEDIA_SIZE * 4 / 3 + 4 {
            return false;
        }
        let Ok(data) = general_purpose::STANDARD.decode(encoded) else {
            return false;
        };
        if data.len() > MAX_CACHED_MEDIA_SIZE {
            return false;
        }
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        std::fs::write(path, data).is_ok()
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let path = safe_cache_path(&self.cache_directory, name)?;
        let metadata = std::fs::metadata(&path).ok()?;
        if metadata.len() > MAX_CACHED_MEDIA_SIZE as u64 {
            return None;
        }
        std::fs::read(path).ok()
    }
}

impl Drop for LocalMedia {
    fn drop(&mut self) {
        self.sound.clear();
        let _ = std::fs::remove_dir_all(&self.cache_directory);
    }
}

fn safe_cache_path(directory: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.len() > 128 {
        return None;
    }
    let mut path = directory.to_path_buf();
    for component in name.split('/') {
        if component.is_empty() || component == "." || component == ".." || component.contains('\\') || component.contains('\0') {
            return None;
        }
        path.push(component);
    }
    Some(path)
}

const EMSI_IRQ: &[u8; 15] = b"**EMSI_IRQ8E08\r";
pub fn start_update_thread(com: Box<dyn Connection>, screen: Arc<Mutex<TextScreen>>) -> (thread::JoinHandle<()>, mpsc::Sender<SendData>) {
    let (tx, rx) = mpsc::channel(32);
    (
        std::thread::Builder::new()
            .name("Terminal update".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let mut buffer_parser = AnsiParser::default();
                    // buffer_parser.bs_is_ctrl_char = true;
                    let mut connection = ConnectionThreadData {
                        _is_connected: false,
                        com,
                        _thread_is_running: true,
                        rx,
                        media: LocalMedia::new(),
                    };
                    let mut data = [0; 1024 * 64];
                    loop {
                        tokio::select! {
                                                    read_data = connection.com.read(&mut data) => {
                                                        match read_data {
                                                            Err(err) => {
                                                                log::error!("run_update_thread::read_data: {err}");
                        //                                        update_thread.lock().is_connected = false;
                                                                break;
                                                            }
                                                            Ok(size) => {
                                                                if size > 0 {
                                                                    if data.starts_with(b"\x1B[0c") {
                                                                        connection.com.send(b" \x08").await.unwrap();
                                                                        continue;
                                                                    }

                                                                    if data.starts_with(b"\x1B[999;999H\x1B[6n") {
                                                                        connection.com.send(b" \x08").await.unwrap();
                                                                        continue;
                                                                    }


                                                                    if data.starts_with(b"\x1B[1;1H\x01\xF6\x1C\x1B[6n") {
                                                                        connection.com.send(b" \x08").await.unwrap();
                                                                        continue;
                                                                    }

                                                                    if data.starts_with(EMSI_IRQ) {
                                                                        connection.com.send(b" \x08").await.unwrap();
                                                                        continue;
                                                                    }

                                                                    process_terminal_data(
                                                                        &mut connection,
                                                                        &mut buffer_parser,
                                                                        &screen,
                                                                        &data[..size],
                                                                    )
                                                                    .await;
                                                                } else {
                                                                    std::thread::sleep(std::time::Duration::from_millis(20));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Some(data) = connection.rx.recv() => {
                                                        let _ = handle_receive(&mut connection, data).await;
                                                    }
                                                };
                    }
                });
            })
            .unwrap(),
        tx,
    )
}

async fn process_terminal_data(connection: &mut ConnectionThreadData, parser: &mut AnsiParser, screen: &Arc<Mutex<TextScreen>>, data: &[u8]) {
    if contains_sequence(data, b"\x1b[?1016$p") {
        let _ = connection.com.send(b"\x1b[?1016;1$y").await;
    }
    connection.media.pending.extend_from_slice(data);
    loop {
        let Some(start) = connection.media.pending.windows(2).position(|window| window == b"\x1b_") else {
            let keep = usize::from(connection.media.pending.last() == Some(&0x1B));
            let parse_len = connection.media.pending.len().saturating_sub(keep);
            parse_screen(parser, screen, &connection.media.pending[..parse_len]);
            connection.media.pending.drain(..parse_len);
            break;
        };
        parse_screen(parser, screen, &connection.media.pending[..start]);
        connection.media.pending.drain(..start);
        let Some(end) = connection.media.pending[2..]
            .windows(2)
            .position(|window| window == b"\x1b\\")
            .map(|end| end + 2)
        else {
            break;
        };
        let command = connection.media.pending[2..end].to_vec();
        let whole = connection.media.pending[..end + 2].to_vec();
        connection.media.pending.drain(..end + 2);
        if !handle_apc(connection, screen, &command).await {
            parse_screen(parser, screen, &whole);
        }
    }
}

fn contains_sequence(data: &[u8], needle: &[u8]) -> bool {
    data.windows(needle.len()).any(|window| window == needle)
}

fn parse_screen(parser: &mut AnsiParser, screen: &Arc<Mutex<TextScreen>>, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let mut screen = screen.lock().unwrap();
    parser.parse(data, &mut icy_engine::ScreenSink::new(&mut *screen));
}

async fn handle_apc(connection: &mut ConnectionThreadData, screen: &Arc<Mutex<TextScreen>>, data: &[u8]) -> bool {
    let Ok(payload) = std::str::from_utf8(data) else {
        return false;
    };
    if let Some(arguments) = payload.strip_prefix("SyncTERM:C;S;") {
        let Some((name, encoded)) = arguments.split_once(';') else {
            return true;
        };
        if !connection.media.store(name, encoded) {
            log::warn!("Local terminal rejected cached media {name:?}");
        }
        return true;
    }
    if let Some(arguments) = payload.strip_prefix("SyncTERM:C;DrawJXL;") {
        let (options, name) = arguments.rsplit_once(';').map_or(("", arguments), |(options, name)| (options, name));
        let Some(bytes) = connection.media.read(name) else {
            log::warn!("Local terminal cannot read cached JXL {name:?}");
            return true;
        };
        let mut screen = screen.lock().unwrap();
        let font = screen.font_dimensions();
        let screen_size = Size::new(screen.width(), screen.height());
        if let Some((position, sixel)) = icy_engine::decode_image_blob(&bytes, true, options, font, screen_size) {
            screen.add_sixel(position, sixel);
        }
        return true;
    }
    if payload == "SyncTERM:Q;JXL" {
        let _ = connection.com.send(b"\x1b[=1;1-n").await;
        return true;
    }
    if let Some(query) = audio_apc::parse_feature_query(payload) {
        let response = match query {
            AudioFeatureQuery::Sndfile => format!("\x1b[=7;{};1n", audio_apc::FEATURE_SNDFILE),
            AudioFeatureQuery::SndfileFormat { major, subtype } => {
                let available = u8::from(audio_apc::supports_format(major, subtype));
                format!("\x1b[=7;{};{major};{subtype};{available}n", audio_apc::FEATURE_SNDFILE_FORMAT)
            }
        };
        let _ = connection.com.send(response.as_bytes()).await;
        return true;
    }
    if let Some(command) = audio_apc::parse_audio_apc(payload) {
        if let Err(err) = connection.media.sound.audio_apc(command, Some(connection.media.cache_directory.clone())) {
            log::warn!("Local Audio APC command failed: {err}");
        }
        return true;
    }
    payload.starts_with("SyncTERM:A;")
}

async fn handle_receive(c: &mut ConnectionThreadData, data: SendData) -> Res<()> {
    match data {
        SendData::Data(buf) => {
            c.com.send(&buf).await?;
        }

        SendData::_Disconnect => {
            c.com.shutdown().await?;
        }
    }
    Ok(())
}

/// Data that is sent to the connection thread
#[derive(Debug)]
pub enum SendData {
    Data(Vec<u8>),
    _Disconnect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_media_paths_allow_namespaces_but_not_traversal() {
        let root = Path::new("/tmp/icboard-media-test");
        assert_eq!(safe_cache_path(root, "snd/abc.ogg"), Some(root.join("snd/abc.ogg")));
        assert!(safe_cache_path(root, "../secret").is_none());
        assert!(safe_cache_path(root, "/absolute").is_none());
        assert!(safe_cache_path(root, "a\\b").is_none());
    }

    #[test]
    fn recognizes_pixel_mouse_mode_query_inside_enable_sequence() {
        assert!(contains_sequence(b"\x1b[?1003h\x1b[?1006h\x1b[?1016h\x1b[?1016$p", b"\x1b[?1016$p"));
    }
}
