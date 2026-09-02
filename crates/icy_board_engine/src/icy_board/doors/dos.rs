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

pub fn expand_run_batch(door: &Door, node: usize, drop_file: &str) -> String {
    let command = door
        .dos_command
        .replace("{dropFile}", drop_file)
        .replace("{dropfile}", drop_file)
        .replace("{node}", &node.to_string())
        .replace("{baud}", "57600");
    format!("@ECHO OFF\nCD C:\\DOOR\n{command}")
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

    #[test]
    fn normalizes_batch_files_and_expands_tokens() {
        let mut door = Door::default();
        door.dos_command = "COPY C:\\ICB\\{dropFile} C:\\DOOR\nGAME {node} {baud}".into();
        assert_eq!(
            normalize_dos_text(&expand_run_batch(&door, 3, "DOOR.SYS")),
            "@ECHO OFF\r\nCD C:\\DOOR\r\nCOPY C:\\ICB\\DOOR.SYS C:\\DOOR\r\nGAME 3 57600"
        );
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
}
