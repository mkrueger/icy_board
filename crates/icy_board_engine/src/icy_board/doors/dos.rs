use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use fatfs::{FileSystem, FsOptions};
use tokio::sync::mpsc;
use x86::{Image, ImageKind, Machine, MachineConfig, ModemStatus, NativeBackend, RunOptions};

use crate::{Res, icy_board::doors::Door};

const POWEROFF_COM: &[u8] = &[
    0xBA, 0x04, 0xB0, // mov dx, b004h
    0xB8, 0x00, 0x20, // mov ax, 2000h
    0xEF, // out dx, ax
    0xF4, 0xEB, 0xFD, // hlt; jmp hlt
];

pub struct DosSession {
    pub input: mpsc::UnboundedSender<Vec<u8>>,
    pub output: mpsc::UnboundedReceiver<Vec<u8>>,
    pub finished: tokio::sync::oneshot::Receiver<Res<()>>,
    cancel: Arc<AtomicBool>,
}

impl Drop for DosSession {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl DosSession {
    pub async fn stop(&mut self) -> bool {
        self.cancel.store(true, Ordering::Release);
        tokio::time::timeout(std::time::Duration::from_secs(5), &mut self.finished).await.is_ok()
    }
}

struct PartitionFile {
    file: File,
    offset: u64,
    position: u64,
}

impl PartitionFile {
    fn open(path: &Path) -> Res<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut sector = [0; 512];
        file.read_exact(&mut sector)?;
        if sector[510..512] != [0x55, 0xAA] {
            return Err("DOS disk image has no valid MBR signature".into());
        }
        let first_lba = u32::from_le_bytes(sector[454..458].try_into().unwrap()) as u64;
        if first_lba == 0 {
            return Err("DOS disk image has no first partition".into());
        }
        Ok(Self {
            file,
            offset: first_lba * 512,
            position: 0,
        })
    }
}

impl Read for PartitionFile {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.offset + self.position))?;
        let count = self.file.read(buffer)?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Write for PartitionFile {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.offset + self.position))?;
        let count = self.file.write(buffer)?;
        self.position += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Seek for PartitionFile {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.position = match position {
            SeekFrom::Start(position) => position,
            SeekFrom::Current(delta) => self
                .position
                .checked_add_signed(delta)
                .ok_or_else(|| std::io::Error::other("seek before partition"))?,
            SeekFrom::End(_) => return Err(std::io::Error::other("partition-relative end seek is unsupported")),
        };
        Ok(self.position)
    }
}

pub fn inject_session_files(image: &Path, files: &[(String, Vec<u8>)], run_batch: &str) -> Res<()> {
    let partition = PartitionFile::open(image)?;
    let file_system = FileSystem::new(partition, FsOptions::new())?;
    {
        let root = file_system.root_dir();
        if root.open_dir("ICB").is_err() {
            root.create_dir("ICB")?;
        }
        if root.open_dir("DOOR").is_err() {
            root.create_dir("DOOR")?;
        }
        for directory_name in ["ICB", "DOOR"] {
            let directory = root.open_dir(directory_name)?;
            for (name, contents) in files {
                let mut file = directory.create_file(name)?;
                file.truncate()?;
                file.write_all(contents)?;
            }
        }
        let mut file = root.open_dir("ICB")?.create_file("RUN.BAT")?;
        file.truncate()?;
        file.write_all(normalize_dos_text(run_batch).as_bytes())?;
        let mut poweroff = root.open_dir("ICB")?.create_file("POWEROFF.COM")?;
        poweroff.truncate()?;
        poweroff.write_all(POWEROFF_COM)?;
        let mut startup = root.create_file("FDAUTO.BAT")?;
        startup.truncate()?;
        startup.write_all(
            b"@ECHO OFF\r\nSET DOSDIR=C:\\FREEDOS\r\nSET PATH=%DOSDIR%\\BIN\r\nCTTY COM1\r\nCALL C:\\ICB\\RUN.BAT\r\nECHO Returning to Icy Board...\r\nC:\\ICB\\POWEROFF.COM\r\n",
        )?;
    }
    file_system.unmount()?;
    Ok(())
}

pub fn configure_base_image(image: &Path) -> Res<()> {
    let partition = PartitionFile::open(image)?;
    let file_system = FileSystem::new(partition, FsOptions::new())?;
    {
        let root = file_system.root_dir();
        if root.open_dir("ICB").is_err() {
            root.create_dir("ICB")?;
        }
        let mut startup = root.create_file("FDAUTO.BAT")?;
        startup.truncate()?;
        startup.write_all(
            b"@ECHO OFF\r\nSET DOSDIR=C:\\FREEDOS\r\nSET PATH=%DOSDIR%\\BIN\r\nCTTY COM1\r\nCALL C:\\ICB\\RUN.BAT\r\nECHO Returning to Icy Board...\r\nC:\\ICB\\POWEROFF.COM\r\n",
        )?;
        let mut run_batch = root.open_dir("ICB")?.create_file("RUN.BAT")?;
        run_batch.truncate()?;
        run_batch.write_all(b"@ECHO OFF\r\nECHO No DOS door configured. > COM1\r\n")?;
        let mut poweroff = root.open_dir("ICB")?.create_file("POWEROFF.COM")?;
        poweroff.truncate()?;
        poweroff.write_all(POWEROFF_COM)?;
        let mut config = root.create_file("FDCONFIG.SYS")?;
        config.truncate()?;
        config.write_all(
            b"!COUNTRY=001,858:C:\\FREEDOS\\BIN\\COUNTRY.SYS\r\n!LASTDRIVE=Z\r\n!BUFFERS=20\r\n!FILES=40\r\nSHELL=C:\\FREEDOS\\BIN\\COMMAND.COM C:\\FREEDOS\\BIN /E:1024 /P=C:\\FDAUTO.BAT\r\n",
        )?;
    }
    file_system.unmount()?;
    Ok(())
}

pub fn replace_image_startup(image: &Path, contents: &[u8]) -> Res<Vec<u8>> {
    let file_system = FileSystem::new(PartitionFile::open(image)?, FsOptions::new())?;
    let mut previous = Vec::new();
    {
        let root = file_system.root_dir();
        let mut startup = root.open_file("FDAUTO.BAT")?;
        startup.read_to_end(&mut previous)?;
        startup.seek(SeekFrom::Start(0))?;
        startup.truncate()?;
        startup.write_all(contents)?;
    }
    file_system.unmount()?;
    Ok(previous)
}

const DOS_ASSETS: [(&str, &str, &str); 3] = [
    (
        "freedos.img",
        "https://download.freedos.org/1.4/FD14-LiteUSB.zip",
        "857dcd2ebf9d3d094320154db5fb5b830acba6fb98f981a95a0ca7ab3350338b",
    ),
    (
        "seabios.bin",
        "https://raw.githubusercontent.com/copy/v86/master/bios/seabios.bin",
        "73e3f359102e3a9982c35fce98eb7cd08f18303ac7f1ba6ebfbe6cdc1c244d98",
    ),
    (
        "vgabios.bin",
        "https://raw.githubusercontent.com/copy/v86/master/bios/vgabios.bin",
        "a4bc0d80cc3ca028c73dafa8fee396b8d054ce87ebd8abfbd31b06b437607880",
    ),
];

pub fn dos_assets_ready(assets: &Path) -> bool {
    DOS_ASSETS.iter().all(|(name, _, _)| assets.join(name).is_file())
}

pub fn prepare_dos_assets(assets: &Path) -> Res<()> {
    prepare_dos_assets_with(assets, |url, expected| {
        log::info!("Downloading DOS asset from {url}");
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let mut bytes = Vec::new();
        client.get(url).send()?.error_for_status()?.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(format!("DOS asset download is too large: {url}").into());
        }
        verify_dos_asset(&bytes, expected)?;
        Ok(bytes)
    })
}

fn verify_dos_asset(bytes: &[u8], expected: &str) -> Res<()> {
    use sha2::{Digest, Sha256};
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!("DOS asset checksum mismatch: expected {expected}, got {actual}").into());
    }
    Ok(())
}

fn prepare_dos_assets_with(assets: &Path, mut download: impl FnMut(&str, &str) -> Res<Vec<u8>>) -> Res<()> {
    if dos_assets_ready(assets) {
        return Ok(());
    }
    std::fs::create_dir_all(assets)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(assets.join(".prepare.lock"))?;
    lock.lock()?;
    let mut prepared = Vec::new();
    for (name, url, checksum) in DOS_ASSETS {
        let destination = assets.join(name);
        if destination.try_exists()? {
            if !destination.is_file() {
                return Err(format!("DOS asset is not a file: {}", destination.display()).into());
            }
            continue;
        }
        let bytes = download(url, checksum).map_err(|error| format!("Could not prepare {name}: {error}"))?;
        let mut staged = tempfile::NamedTempFile::new_in(assets)?;
        if name == "freedos.img" {
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
            let mut image = archive.by_name("FD14LITE.img")?;
            if image.size() > 64 * 1024 * 1024 {
                return Err("FreeDOS image is too large".into());
            }
            std::io::copy(&mut image, &mut staged)?;
            staged.flush()?;
            configure_base_image(staged.path())?;
        } else {
            staged.write_all(&bytes)?;
        }
        staged.as_file().sync_all()?;
        prepared.push((staged, destination));
    }
    for (staged, destination) in prepared {
        match staged.persist_noclobber(&destination) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists && destination.is_file() => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub fn install_x00_fossil(image: &Path, archive: &Path) -> Res<std::path::PathBuf> {
    let mut archive = zip::ZipArchive::new(File::open(archive)?)?;
    if archive.len() > 256 {
        return Err("X00 archive contains too many entries".into());
    }
    let mut files = std::collections::BTreeMap::new();
    let mut total_size = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_ascii_uppercase();
        if name.is_empty() || name == "." || name == ".." || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)) || entry.is_dir()
        {
            return Err(format!("Expected an original, flat X00 ZIP archive with DOS filenames; invalid entry: {name}").into());
        }
        let remaining = 16 * 1024 * 1024 - total_size;
        if entry.size() > remaining {
            return Err("X00 archive is too large".into());
        }
        let mut bytes = Vec::new();
        (&mut entry).take(remaining + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > remaining {
            return Err("X00 archive is too large".into());
        }
        total_size += bytes.len() as u64;
        if files.insert(name, bytes).is_some() {
            return Err("Duplicate filename in X00 archive".into());
        }
    }
    for name in ["X00.SYS", "LICENSE.TXT", "X00USER.DOC", "X00REF.DOC"] {
        if !files.get(name).is_some_and(|bytes| !bytes.is_empty()) {
            return Err(format!("X00 archive is missing {name}; supply the complete original distribution").into());
        }
    }
    let mut backup_name = image.file_name().ok_or("Missing image filename")?.to_os_string();
    backup_name.push(".pre-fossil.bak");
    let backup = image.with_file_name(backup_name);
    if backup.try_exists()? {
        return Err(format!("Backup already exists: {}; keep it safe before installing again", backup.display()).into());
    }
    let parent = image.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let staged = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::copy(image, staged.path())?;
    let file_system = FileSystem::new(PartitionFile::open(staged.path())?, FsOptions::new())?;
    {
        let root = file_system.root_dir();
        if root.open_dir("FOSSIL").is_ok() {
            return Err("A FOSSIL directory already exists; existing drivers were not changed".into());
        }
        let mut config = Vec::new();
        root.open_file("FDCONFIG.SYS")?.read_to_end(&mut config)?;
        let text = String::from_utf8_lossy(&config).to_ascii_uppercase();
        if ["X00.SYS", "X00.EXE", "BNU.SYS", "BNU.COM", "FOSSIL"].iter().any(|name| text.contains(name)) {
            return Err("FDCONFIG.SYS already references a FOSSIL driver; review it before installing".into());
        }
        let directory = root.create_dir("FOSSIL")?;
        for (name, bytes) in &files {
            directory.create_file(name)?.write_all(bytes)?;
        }
        if config.last() == Some(&0x1a) {
            config.pop();
        }
        if !config.ends_with(b"\n") {
            config.extend_from_slice(b"\r\n");
        }
        config.extend_from_slice(b"DEVICE=C:\\FOSSIL\\X00.SYS E B,0,57600\r\n");
        let mut output = root.create_file("FDCONFIG.SYS")?;
        output.truncate()?;
        output.write_all(&config)?;
    }
    file_system.unmount()?;
    staged.as_file().sync_all()?;
    let saved = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::copy(image, saved.path())?;
    saved.as_file().sync_all()?;
    saved.persist_noclobber(&backup)?;
    staged.persist(image)?;
    Ok(backup)
}

pub fn copy_file_into_image(image: &Path, source: &Path, destination: &str) -> Res<()> {
    let partition = PartitionFile::open(image)?;
    let file_system = FileSystem::new(partition, FsOptions::new())?;
    {
        let root = file_system.root_dir();
        let destination = destination.trim_start_matches(['/', '\\']).replace('\\', "/");
        let (directory, _) = destination.rsplit_once('/').unwrap_or(("", destination.as_str()));
        let mut current = String::new();
        for component in directory.split('/').filter(|component| !component.is_empty()) {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(component);
            if root.open_dir(&current).is_err() {
                root.create_dir(&current)?;
            }
        }
        let mut output = root.create_file(&destination)?;
        output.truncate()?;
        output.write_all(&std::fs::read(source)?)?;
    }
    file_system.unmount()?;
    Ok(())
}

pub(crate) fn read_editor_file(image: &Path, name: &str, limit: usize) -> Res<Vec<u8>> {
    let partition = PartitionFile::open(image)?;
    let file_system = FileSystem::new(partition, FsOptions::new())?;
    let mut bytes = Vec::new();
    {
        let root = file_system.root_dir();
        root.open_file(&format!("DOOR/{name}"))?.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    }
    file_system.unmount()?;
    if bytes.len() > limit {
        return Err("DOS editor output exceeds the configured limit".into());
    }
    Ok(bytes)
}

pub fn create_door_image(base_image: &Path, door_image: &Path, source_directory: &Path) -> Res<bool> {
    if door_image.exists() {
        return Ok(false);
    }
    if !source_directory.is_dir() {
        return Err(format!("DOS door path is not a directory: {}", source_directory.display()).into());
    }
    if let Some(parent) = door_image.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(base_image, door_image)?;
    let result = copy_directory_into_image(door_image, source_directory, "DOOR");
    if result.is_err() {
        let _ = std::fs::remove_file(door_image);
    }
    result.map(|()| true)
}

fn copy_directory_into_image(image: &Path, source: &Path, destination: &str) -> Res<()> {
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        let destination = format!("{}/{}", destination, relative.to_string_lossy().replace('\\', "/"));
        copy_file_into_image(image, entry.path(), &destination)?;
    }
    Ok(())
}

pub fn normalize_dos_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

pub fn image_file_name(door_name: &str) -> String {
    let name = door_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{}.img", if name.is_empty() { "door" } else { &name })
}

pub fn expand_run_batch(door: &Door, node: usize, drop_file: &str) -> Res<String> {
    let expand = |value: &str| {
        value
            .replace("{dropFile}", drop_file)
            .replace("{dropfile}", drop_file)
            .replace("{node}", &(node + 1).to_string())
            .replace("{baud}", "57600")
    };
    let mut command = expand(&door.dos_command);
    if !door.args.is_empty() {
        command.truncate(command.trim_end().len());
    }
    for argument in &door.args {
        let argument = expand(argument);
        if argument.contains(['\r', '\n', '"', '%']) {
            return Err("DOS arguments cannot contain line breaks, double quotes or percent signs; use the command field for batch syntax".into());
        }
        command.push(' ');
        if argument.is_empty() || argument.chars().any(|character| character.is_whitespace() || matches!(character, '&' | '|' | '<' | '>')) {
            command.push('"');
            command.push_str(&argument);
            command.push('"');
        } else {
            command.push_str(&argument);
        }
    }
    Ok(format!("@ECHO OFF\nCD C:\\DOOR\n{command}"))
}

pub fn validate_simple_command(source_directory: &Path, command: &str) -> Res<()> {
    let command = command.trim();
    if command.is_empty() || command.contains(['\r', '\n', ' ', '\t']) {
        return Ok(());
    }
    let extension = Path::new(command).extension().and_then(|extension| extension.to_str()).unwrap_or_default();
    if !matches!(extension.to_ascii_lowercase().as_str(), "bat" | "com" | "exe") {
        return Ok(());
    }
    let found = std::fs::read_dir(source_directory)?
        .filter_map(Result::ok)
        .any(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()) && entry.file_name().to_string_lossy().eq_ignore_ascii_case(command));
    if found {
        Ok(())
    } else {
        Err(format!(
            "DOS command '{}' was not found in {}. Install/configure the door before launching it",
            command,
            source_directory.display()
        )
        .into())
    }
}

pub(crate) fn editor_run_batch(command: &str) -> String {
    format!(
        "@ECHO OFF\nCD C:\\DOOR\nCALL {command}\nIF ERRORLEVEL 3 GOTO ICBERR\nIF ERRORLEVEL 2 GOTO ICBTIME\nIF ERRORLEVEL 1 GOTO ICBABORT\nECHO 0>C:\\DOOR\\ICBEDIT.RC\nGOTO ICBEND\n:ICBABORT\nECHO 1>C:\\DOOR\\ICBEDIT.RC\nGOTO ICBEND\n:ICBTIME\nECHO 2>C:\\DOOR\\ICBEDIT.RC\nGOTO ICBEND\n:ICBERR\nECHO 3>C:\\DOOR\\ICBEDIT.RC\n:ICBEND\n"
    )
}

pub fn start_session(image_path: &Path, bios_path: &Path, vga_bios_path: &Path, memory_mb: u32, max_runtime: std::time::Duration) -> Res<DosSession> {
    let image_path = image_path.to_path_buf();
    let bios_path = bios_path.to_path_buf();
    let vga_bios_path = vga_bios_path.to_path_buf();
    let (input, mut input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (output_tx, output) = mpsc::unbounded_channel::<Vec<u8>>();
    let (finished_tx, finished) = tokio::sync::oneshot::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = Arc::clone(&cancel);

    std::thread::Builder::new().name("icy-board-dos-door".into()).spawn(move || {
        let result = (|| -> Res<()> {
            let config = MachineConfig::default()
                .with_ram_bytes(memory_mb.max(1) as u64 * 1024 * 1024)
                .with_vga_memory_bytes(2 * 1024 * 1024);
            let mut machine = Machine::new(config);
            machine.set_bios(Image::from_file(ImageKind::Bios, &bios_path)?)?;
            machine.set_vga_bios(Image::from_file(ImageKind::VgaBios, &vga_bios_path)?)?;
            machine.set_disk(Image::from_file(ImageKind::RawDisk, &image_path)?)?;
            machine.attach_backend(NativeBackend::new().with_instructions_per_step(10_000));
            machine.prepare()?;
            machine.set_modem_status(0, ModemStatus::default())?;
            let mut serial = vec![0; 32 * 1024];
            let started = std::time::Instant::now();
            while !thread_cancel.load(Ordering::Acquire) && started.elapsed() < max_runtime {
                while let Ok(bytes) = input_rx.try_recv() {
                    machine.serial_input(0, &bytes)?;
                }
                let report = machine.run(RunOptions {
                    max_steps: Some(1),
                    ..RunOptions::default()
                })?;
                let count = machine.serial_output(0, &mut serial)?;
                if count > 0 && output_tx.send(serial[..count].to_vec()).is_err() {
                    break;
                }
                if report.halted {
                    break;
                }
            }
            let timed_out = started.elapsed() >= max_runtime;
            if timed_out {
                log::warn!("native DOS emulator reached its hard runtime limit of {} seconds", max_runtime.as_secs());
            }
            if !thread_cancel.load(Ordering::Acquire) && !timed_out {
                let snapshot = machine.hard_disk_snapshot(0)?;
                crate::icy_board::write_atomic(&image_path, &snapshot)?;
            }
            Ok(())
        })();
        let _ = finished_tx.send(result);
    })?;

    Ok(DosSession {
        input,
        output,
        finished,
        cancel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dos_image(path: &Path) {
        let mut partition = std::io::Cursor::new(vec![0; 8 * 1024 * 1024]);
        fatfs::format_volume(&mut partition, fatfs::FormatVolumeOptions::new()).unwrap();
        let mut disk = vec![0; 512];
        disk[454..458].copy_from_slice(&1u32.to_le_bytes());
        disk[510..512].copy_from_slice(&[0x55, 0xaa]);
        disk.extend(partition.into_inner());
        std::fs::write(path, disk).unwrap();
        configure_base_image(path).unwrap();
    }

    fn test_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, bytes) in files {
            writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn fossil_install_preserves_config_and_keeps_backup_and_license() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("door.img");
        test_dos_image(&image);
        let config = b"REM custom \x84\r\nFILES=80\r\n";
        let host_config = directory.path().join("config.sys");
        std::fs::write(&host_config, config).unwrap();
        copy_file_into_image(&image, &host_config, "FDCONFIG.SYS").unwrap();
        let original = std::fs::read(&image).unwrap();
        let archive = directory.path().join("x00.zip");
        let files: &[(&str, &[u8])] = &[
            ("X00.SYS", b"driver"),
            ("LICENSE.TXT", b"terms"),
            ("X00USER.DOC", b"user"),
            ("X00REF.DOC", b"ref"),
        ];
        test_zip(&archive, files);
        let backup = install_x00_fossil(&image, &archive).unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        let file_system = FileSystem::new(PartitionFile::open(&image).unwrap(), FsOptions::new()).unwrap();
        {
            let root = file_system.root_dir();
            let mut saved_config = Vec::new();
            root.open_file("FDCONFIG.SYS").unwrap().read_to_end(&mut saved_config).unwrap();
            assert_eq!(saved_config, [config.as_slice(), b"DEVICE=C:\\FOSSIL\\X00.SYS E B,0,57600\r\n"].concat());
            for (name, bytes) in files {
                let mut actual = Vec::new();
                root.open_file(&format!("FOSSIL/{name}")).unwrap().read_to_end(&mut actual).unwrap();
                assert_eq!(&actual, bytes);
            }
        }
        file_system.unmount().unwrap();
        let installed = std::fs::read(&image).unwrap();
        assert!(install_x00_fossil(&image, &archive).is_err());
        assert_eq!(std::fs::read(&image).unwrap(), installed);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
    }

    #[test]
    fn fossil_install_rejects_incomplete_or_unsafe_archives_without_touching_image() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("door.img");
        std::fs::write(&image, b"unchanged").unwrap();
        let archive = directory.path().join("bad.zip");
        for name in ["X00.SYS", "../X00.SYS", "C:X00.SYS"] {
            test_zip(&archive, &[(name, b"driver")]);
            assert!(install_x00_fossil(&image, &archive).is_err());
            assert_eq!(std::fs::read(&image).unwrap(), b"unchanged");
            assert!(!directory.path().join("door.img.pre-fossil.bak").exists());
        }
    }

    #[test]
    fn dos_assets_prepare_and_patch_a_fresh_image() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.img");
        test_dos_image(&source);
        let archive = directory.path().join("dos.zip");
        test_zip(&archive, &[("FD14LITE.img", &std::fs::read(&source).unwrap())]);
        let assets = directory.path().join("assets");
        prepare_dos_assets_with(&assets, |url, _| {
            if url.ends_with(".zip") {
                Ok(std::fs::read(&archive)?)
            } else {
                Ok(b"bios".to_vec())
            }
        })
        .unwrap();
        let file_system = FileSystem::new(PartitionFile::open(&assets.join("freedos.img")).unwrap(), FsOptions::new()).unwrap();
        {
            let root = file_system.root_dir();
            let mut startup = String::new();
            root.open_file("FDAUTO.BAT").unwrap().read_to_string(&mut startup).unwrap();
            assert!(startup.contains("CALL C:\\ICB\\RUN.BAT"));
            assert!(root.open_file("ICB/POWEROFF.COM").is_ok());
        }
        file_system.unmount().unwrap();
        assert!(dos_assets_ready(&assets));
    }

    #[test]
    fn dos_assets_preserve_existing_files_without_downloading() {
        let directory = tempfile::tempdir().unwrap();
        for (name, _, _) in DOS_ASSETS {
            std::fs::write(directory.path().join(name), b"custom asset").unwrap();
        }
        prepare_dos_assets_with(directory.path(), |_, _| panic!("must not download existing assets")).unwrap();
        for (name, _, _) in DOS_ASSETS {
            assert_eq!(std::fs::read(directory.path().join(name)).unwrap(), b"custom asset");
        }
    }

    #[test]
    fn dos_assets_failed_download_is_not_published_and_can_retry() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("freedos.img"), b"custom image").unwrap();
        let mut requests = 0;
        let result = prepare_dos_assets_with(directory.path(), |_, _| {
            requests += 1;
            if requests == 2 { Err("network failure".into()) } else { Ok(b"bios".to_vec()) }
        });
        assert!(result.unwrap_err().to_string().contains("network failure"));
        assert!(!directory.path().join("seabios.bin").exists());
        assert!(!directory.path().join("vgabios.bin").exists());
        prepare_dos_assets_with(directory.path(), |_, _| Ok(b"bios".to_vec())).unwrap();
        assert!(dos_assets_ready(directory.path()));
        assert_eq!(std::fs::read(directory.path().join("freedos.img")).unwrap(), b"custom image");
    }

    #[test]
    fn dos_assets_reject_bad_checksums_and_invalid_archives() {
        assert!(verify_dos_asset(b"bad download", DOS_ASSETS[0].2).is_err());
        verify_dos_asset(b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad").unwrap();
        let directory = tempfile::tempdir().unwrap();
        assert!(prepare_dos_assets_with(directory.path(), |_, _| Ok(b"not a ZIP".to_vec())).is_err());
        assert!(!directory.path().join("freedos.img").exists());
    }

    #[test]
    fn dos_node_placeholder_is_one_based_in_commands_and_arguments() {
        let door = Door {
            dos_command: "START.BAT {node}".into(),
            args: vec!["/N{node}".into()],
            ..Door::default()
        };
        for node in [0, 1, 3] {
            assert_eq!(
                expand_run_batch(&door, node, "PCBOARD.SYS").unwrap(),
                format!("@ECHO OFF\nCD C:\\DOOR\nSTART.BAT {} /N{}", node + 1, node + 1)
            );
        }
    }

    #[test]
    fn normalizes_batch_files_and_expands_tokens() {
        let mut door = Door::default();
        door.dos_command = "COPY C:\\ICB\\{dropFile} C:\\DOOR\nGAME {node} {baud}".into();
        assert_eq!(
            normalize_dos_text(&expand_run_batch(&door, 3, "DOOR.SYS").unwrap()),
            "@ECHO OFF\r\nCD C:\\DOOR\r\nCOPY C:\\ICB\\DOOR.SYS C:\\DOOR\r\nGAME 4 57600"
        );
    }

    #[test]
    fn dos_arguments_follow_the_command_and_preserve_dos_paths() {
        let door = Door {
            dos_command: "GAME.EXE /LOCAL".into(),
            args: vec!["/N{node}".into(), "{dropFile}".into(), "C:\\DOOR\\GAME DATA".into(), String::new()],
            ..Door::default()
        };
        assert_eq!(
            expand_run_batch(&door, 3, "DOOR.SYS").unwrap(),
            "@ECHO OFF\nCD C:\\DOOR\nGAME.EXE /LOCAL /N4 DOOR.SYS \"C:\\DOOR\\GAME DATA\" \"\""
        );
    }

    #[test]
    fn dos_arguments_reject_batch_expansion_and_quote_operators() {
        let mut door = Door {
            dos_command: "GAME.EXE".into(),
            args: vec!["a&b|c<d>e".into(), "{baud}".into()],
            ..Door::default()
        };
        assert_eq!(expand_run_batch(&door, 0, "").unwrap(), "@ECHO OFF\nCD C:\\DOOR\nGAME.EXE \"a&b|c<d>e\" 57600");
        for argument in ["%PATH%", "quoted\"value", "first\nsecond", "first\rsecond"] {
            door.args = vec![argument.into()];
            assert!(expand_run_batch(&door, 0, "").is_err(), "accepted {argument:?}");
        }
    }

    #[tokio::test]
    async fn stopping_a_session_waits_for_worker_completion() {
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (finished_tx, finished) = tokio::sync::oneshot::channel();
        let (input, _input_rx) = mpsc::unbounded_channel();
        let (_output_tx, output) = mpsc::unbounded_channel();
        let worker = std::thread::spawn(move || {
            while !worker_cancel.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            let _ = finished_tx.send(Ok(()));
        });
        let mut session = DosSession {
            input,
            output,
            finished,
            cancel,
        };

        assert!(session.stop().await);
        worker.join().unwrap();
    }

    #[tokio::test]
    #[ignore = "requires ICB_DOS_IMAGE, ICB_DOS_BIOS, and ICB_DOS_VGA_BIOS"]
    async fn freedos_poweroff_finishes_the_session() {
        let image = std::env::var_os("ICB_DOS_IMAGE").expect("set ICB_DOS_IMAGE");
        let bios = std::env::var_os("ICB_DOS_BIOS").expect("set ICB_DOS_BIOS");
        let vga_bios = std::env::var_os("ICB_DOS_VGA_BIOS").expect("set ICB_DOS_VGA_BIOS");
        let directory = tempfile::tempdir().unwrap();
        let session_image = directory.path().join("session.img");
        std::fs::copy(image, &session_image).unwrap();

        let run_batch = std::env::var("ICB_DOS_RUN_BATCH").unwrap_or_else(|_| "@ECHO OFF\nECHO No DOS door configured. > COM1".into());
        let max_runtime = std::env::var("ICB_DOS_MAX_RUNTIME_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or_else(|| std::time::Duration::from_secs(30));
        inject_session_files(&session_image, &[], &run_batch).unwrap();
        let mut session = start_session(&session_image, Path::new(&bios), Path::new(&vga_bios), 8, max_runtime).unwrap();
        let mut serial_output = Vec::new();
        let mut output_open = true;
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                tokio::select! {
                    result = &mut session.finished => return result.unwrap(),
                    output = session.output.recv(), if output_open => {
                        match output {
                            Some(output) => serial_output.extend_from_slice(&output),
                            None => output_open = false,
                        }
                    },
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "FreeDOS poweroff did not finish the DOS worker; serial={:?}",
                String::from_utf8_lossy(&serial_output)
            )
        });
        result.unwrap();
        if std::env::var_os("ICB_DOS_RUN_BATCH").is_some() {
            eprintln!("DOS serial output: {}", String::from_utf8_lossy(&serial_output));
        }
    }

    #[test]
    fn simple_dos_commands_must_exist_in_the_door_directory() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("BRE.EXE"), []).unwrap();

        validate_simple_command(directory.path(), "bre.exe").unwrap();
        let error = validate_simple_command(directory.path(), "BRE.BAT").unwrap_err();
        assert!(error.to_string().contains("Install/configure the door"));
    }

    #[test]
    #[ignore = "requires ICB_DOS_ASSETS; probes the native VGA console keyboard"]
    fn freedos_console_accepts_keyboard_commands() {
        let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
        let root = tempfile::tempdir().unwrap();
        let image = root.path().join("console.img");
        std::fs::copy(assets.join("freedos.img"), &image).unwrap();
        inject_session_files(&image, &[], "").unwrap();
        let startup = root.path().join("FDAUTO.BAT");
        std::fs::write(
            &startup,
            b"@ECHO OFF\r\nSET DOSDIR=C:\\FREEDOS\r\nSET PATH=%DOSDIR%\\BIN\r\nCTTY CON\r\nCD C:\\DOOR\r\nECHO CONSOLE-READY\r\n",
        )
        .unwrap();
        copy_file_into_image(&image, &startup, "FDAUTO.BAT").unwrap();
        let mut machine = Machine::new(MachineConfig::default().with_ram_bytes(64 * 1024 * 1024).with_vga_memory_bytes(2 * 1024 * 1024));
        machine
            .set_bios(Image::from_file(ImageKind::Bios, assets.join("seabios.bin")).unwrap())
            .unwrap();
        machine
            .set_vga_bios(Image::from_file(ImageKind::VgaBios, assets.join("vgabios.bin")).unwrap())
            .unwrap();
        machine.set_disk(Image::from_file(ImageKind::RawDisk, &image).unwrap()).unwrap();
        machine.attach_backend(NativeBackend::new().with_instructions_per_step(10_000));
        machine.prepare().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut injected = false;
        let mut command = "ECHO CONSOLE-OK > CONSOLE.TXT\nC:\\ICB\\POWEROFF.COM\n".chars();
        let mut next_key = std::time::Instant::now();
        let mut screen = String::new();
        while std::time::Instant::now() < deadline {
            let report = machine
                .run(RunOptions {
                    max_steps: Some(1),
                    ..Default::default()
                })
                .unwrap();
            if report.halted {
                assert!(injected);
                std::fs::write(&image, machine.hard_disk_snapshot(0).unwrap()).unwrap();
                let output = read_editor_file(&image, "CONSOLE.TXT", 128).unwrap();
                assert!(String::from_utf8_lossy(&output).contains("CONSOLE-OK"));
                return;
            }
            if let Some((_, _, cells)) = machine.vga_text_snapshot() {
                screen = cells.chunks_exact(2).map(|cell| cell[0] as char).collect();
                if !injected && screen.contains("CONSOLE-READY") {
                    injected = true;
                }
            }
            if injected && std::time::Instant::now() >= next_key {
                if let Some(character) = command.next() {
                    assert_eq!(machine.inject_text(&character.to_string()).unwrap(), 1);
                }
                next_key = std::time::Instant::now() + std::time::Duration::from_millis(10);
            }
        }
        panic!("console command did not finish; injected={injected}; screen={screen:?}");
    }

    #[tokio::test]
    #[ignore = "requires ICB_DOS_ASSETS with X00 installed in freedos.img"]
    async fn installed_fossil_initializes_and_transmits() {
        let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
        let root = tempfile::tempdir().unwrap();
        let image = root.path().join("fossil.img");
        std::fs::copy(assets.join("freedos.img"), &image).unwrap();
        let marker = b"FOSSIL-OK\r\n";
        let mut program = vec![0x31, 0xd2, 0xb4, 0x04, 0xcd, 0x14, 0x3d, 0x54, 0x19, 0x75, (marker.len() * 5 + 4) as u8];
        for byte in marker {
            program.extend_from_slice(&[0xb8, *byte, 0x01, 0xcd, 0x14]);
        }
        program.extend_from_slice(&[0xb4, 0x08, 0xcd, 0x14, 0xb8, 0x00, 0x4c, 0xcd, 0x21]);
        inject_session_files(&image, &[("FOSSTEST.COM".into(), program)], "@ECHO OFF\nC:\\DOOR\\FOSSTEST.COM").unwrap();
        let mut session = start_session(
            &image,
            &assets.join("seabios.bin"),
            &assets.join("vgabios.bin"),
            64,
            std::time::Duration::from_secs(25),
        )
        .unwrap();
        let mut output = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                tokio::select! {
                    packet = session.output.recv() => {
                        if let Some(packet) = packet { output.extend(packet); }
                        else { break; }
                    },
                    result = &mut session.finished => {
                        result.unwrap().unwrap();
                        while let Ok(packet) = session.output.try_recv() { output.extend(packet); }
                        break;
                    },
                }
            }
        })
        .await
        .unwrap();
        assert!(
            output.windows(marker.len()).any(|bytes| bytes == marker),
            "{:?}",
            String::from_utf8_lossy(&output)
        );
    }

    #[tokio::test]
    #[ignore = "requires ICB_DOS_ASSETS; probes the pinned emulator UART"]
    async fn freedos_uart_thre_interrupt_is_acknowledged() {
        let assets = std::path::PathBuf::from(std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS"));
        let root = tempfile::tempdir().unwrap();
        let image = root.path().join("uart.img");
        std::fs::copy(assets.join("freedos.img"), &image).unwrap();
        // Disable interrupts, enable THRE, read IIR twice, disable THRE, print both results.
        let program = vec![
            0xfa, 0xba, 0xfb, 0x03, 0xb0, 0x03, 0xee, 0xba, 0xfa, 0x03, 0x30, 0xc0, 0xee, 0xba, 0xf9, 0x03, 0xb0, 0x02, 0xee, 0x42, 0xec, 0x88, 0xc3, 0xec,
            0x88, 0xc7, 0x4a, 0x30, 0xc0, 0xee, 0xfb, 0x80, 0xc3, 0x30, 0x80, 0xc7, 0x30, 0xba, 0xf8, 0x03, 0x88, 0xd8, 0xee, 0x88, 0xf8, 0xee, 0xb8, 0x00,
            0x4c, 0xcd, 0x21,
        ];
        inject_session_files(&image, &[("UART.COM".into(), program)], "@ECHO OFF\nC:\\DOOR\\UART.COM").unwrap();
        let mut session = start_session(
            &image,
            &assets.join("seabios.bin"),
            &assets.join("vgabios.bin"),
            8,
            std::time::Duration::from_secs(15),
        )
        .unwrap();
        let mut output = Vec::new();
        loop {
            tokio::select! {
                packet = session.output.recv() => {
                    if let Some(packet) = packet { output.extend(packet); }
                    else { break; }
                },
                result = &mut session.finished => {
                    result.unwrap().unwrap();
                    while let Ok(packet) = session.output.try_recv() { output.extend(packet); }
                    break;
                },
            }
        }
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("21"), "THRE must clear after IIR acknowledgement, serial={output:?}");
    }
}
