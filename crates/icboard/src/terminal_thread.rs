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
    pixel_buffers: [Option<Vec<u8>>; 2],
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
            pixel_buffers: std::array::from_fn(|_| None),
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

    /// The cache entries matching `pattern`, each with its digest, so a board can
    /// upload only what is missing.
    fn list(&self, pattern: &str) -> String {
        let mut entries = Vec::new();
        collect_cache_entries(&self.cache_directory, "", &mut entries);
        entries.sort();
        entries
            .iter()
            .filter(|name| cache_glob_matches(pattern, name))
            .filter_map(|name| {
                let data = self.read(name)?;
                Some(format!("{name}\t{:x}\n", md5::compute(&data)))
            })
            .collect()
    }
}

fn image_buffer(options: &str) -> Option<usize> {
    options
        .split(';')
        .find_map(|option| option.strip_prefix("B="))
        .and_then(|buffer| buffer.parse::<usize>().ok())
        .filter(|buffer| *buffer < 2)
}

fn collect_cache_entries(directory: &Path, prefix: &str, entries: &mut Vec<String>) {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in listing.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let relative = format!("{prefix}{name}");
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_cache_entries(&entry.path(), &format!("{relative}/"), entries);
        } else {
            entries.push(relative);
        }
    }
}

/// The listing glob, which in practice is a plain prefix such as `gfx/*`.
fn cache_glob_matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        Some((head, tail)) => name.len() >= head.len() + tail.len() && name.starts_with(head) && name.ends_with(tail),
        None => pattern == name,
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
                                                                    break;
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
    if contains_sequence(data, b"\x1b[<0c") {
        let _ = connection.com.send(b"\x1b[<1;4;7c").await;
    }
    if contains_sequence(data, b"\x1b[14t") || contains_sequence(data, b"\x1b[16t") {
        let (pixel_height, pixel_width, cell_height, cell_width) = {
            let screen = screen.lock().unwrap();
            let font = screen.font_dimensions();
            (screen.height() * font.height, screen.width() * font.width, font.height, font.width)
        };
        if contains_sequence(data, b"\x1b[14t") {
            let _ = connection.com.send(format!("\x1b[4;{pixel_height};{pixel_width}t").as_bytes()).await;
        }
        if contains_sequence(data, b"\x1b[16t") {
            let _ = connection.com.send(format!("\x1b[6;{cell_height};{cell_width}t").as_bytes()).await;
        }
    }
    for channel in 0..audio_apc::CHANNELS {
        let query = format!("\x1b[=7;{channel}n");
        if contains_sequence(data, query.as_bytes()) {
            let active = u8::from(audio_apc::status().is_active(channel as u8));
            let _ = connection.com.send(format!("\x1b[=7;{channel};{active}n").as_bytes()).await;
        }
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
    if let Some(arguments) = payload.strip_prefix("SyncTERM:C;LoadJXLBlob;") {
        let Some((options, encoded)) = arguments.rsplit_once(';') else {
            return true;
        };
        let Some(buffer) = image_buffer(options) else {
            return true;
        };
        connection.media.pixel_buffers[buffer] = general_purpose::STANDARD
            .decode(encoded)
            .ok()
            .filter(|bytes| bytes.len() <= MAX_CACHED_MEDIA_SIZE);
        return true;
    }
    if let Some(options) = payload.strip_prefix("SyncTERM:P;Paste;") {
        let Some(buffer) = image_buffer(options) else {
            return true;
        };
        let Some(bytes) = connection.media.pixel_buffers[buffer].as_deref() else {
            return true;
        };
        let mut screen = screen.lock().unwrap();
        let font = screen.font_dimensions();
        let screen_size = Size::new(screen.width(), screen.height());
        if let Some((position, sixel)) = icy_engine::decode_image_blob(bytes, true, options, font, screen_size) {
            screen.add_sixel(position, sixel);
        }
        return true;
    }
    if payload == "SyncTERM:Q;JXL" {
        let _ = connection.com.send(b"\x1b[=1;1-n").await;
        return true;
    }
    if let Some(arguments) = payload.strip_prefix("SyncTERM:C;L") {
        let pattern = arguments.strip_prefix(';').unwrap_or("*");
        let listing = connection.media.list(pattern);
        let _ = connection.com.send(format!("\x1b_SyncTERM:C;L\n{listing}\x1b\\").as_bytes()).await;
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
    use std::time::Duration;

    use icy_net::connection::channel::ChannelConnection;

    use super::*;

    #[tokio::test]
    async fn local_terminal_forwards_keyboard_input_to_the_board() {
        let (ui_connection, mut board_connection) = ChannelConnection::create_pair();
        let screen = Arc::new(Mutex::new(TextScreen::new((80, 25))));
        let (handle, tx) = start_update_thread(Box::new(ui_connection), screen);

        tx.send(SendData::Data(b"q".to_vec())).await.unwrap();
        let mut input = [0; 1];
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), board_connection.read(&mut input))
                .await
                .unwrap()
                .unwrap(),
            1
        );
        assert_eq!(input, [b'q']);

        drop(tx);
        drop(board_connection);
        handle.join().unwrap();
    }

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
