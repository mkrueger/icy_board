//! CLI orchestration; packet transactions and protocol receipts belong to the engine.

use std::{
    fs::{self, File, OpenOptions},
    future::Future,
    io,
    path::{Path, PathBuf},
};

use icy_board_cli::text;
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard,
        zconnect::{self as engine, TossReport, ZconnectConfig, ZconnectLink, poll::PollReport},
    },
};

fn message(key: &str) -> String {
    text("icbmailer", key)
}

pub(super) fn load(path: &Path) -> Res<IcyBoard> {
    let mut board = IcyBoard::load(&path)?;
    if board.config.paths.zconnect_file.as_os_str().is_empty() {
        return Err(message("zconnect-not-configured").into());
    }
    board.resolve_paths();
    if !board.zconnect.enabled {
        return Err(message("zconnect-disabled").into());
    }
    board.zconnect.validate()?;
    Ok(board)
}

pub(super) fn selected_links<'a>(config: &'a ZconnectConfig, wanted: Option<&str>) -> Res<Vec<&'a ZconnectLink>> {
    config.validate()?;
    let links: Vec<_> = config
        .links
        .iter()
        .filter(|link| wanted.is_none_or(|id| link.id.eq_ignore_ascii_case(id)))
        .collect();
    if links.is_empty() {
        return Err(format!("{}: {}", message("zconnect-no-links"), wanted.unwrap_or("-")).into());
    }
    Ok(links)
}

fn rooted(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { root.join(path) }
}

/// std's Unix file locks and the engine's fs4 locks both use flock. Never take
/// operation.lock here: scan/toss/ack take it internally. Offline operations
/// take poll.lock to exclude an online exchange, and hold it through archival.
/// Do NOT hold this guard across engine::poll, which takes poll.lock itself.
/// Keep the lock inode on disk; closing the handle releases it on every exit.
pub(super) fn offline_lock(config: &ZconnectConfig, root: &Path, link: &ZconnectLink) -> Res<File> {
    let dir = rooted(root, &config.outbound).join(&link.id);
    fs::create_dir_all(&dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("poll.lock"))?;
    file.try_lock().map_err(|err| format!("{}: {}: {err}", link.id, message("zconnect-busy")))?;
    Ok(file)
}

pub(super) fn links(board: &IcyBoard, wanted: Option<&str>) -> Res<()> {
    for link in selected_links(&board.zconnect, wanted)? {
        println!(
            "{:<16} {:<32} {}: {}",
            link.id,
            if link.host.is_empty() {
                message("zconnect-offline")
            } else {
                format!("{}:{}", link.host, link.port)
            },
            message("zconnect-areas"),
            link.areas.len()
        );
    }
    Ok(())
}

fn scan_link(config: &ZconnectConfig, root: &Path, link: &ZconnectLink) -> Res<()> {
    let _lock = offline_lock(config, root, link)?;
    let report = engine::scan(config, root, &link.id)?;
    match report.packet {
        Some(path) => println!("{}: {}: {}; {}", link.id, message("zconnect-messages"), report.messages, path.display()),
        None => println!("{}: {}", link.id, message("zconnect-no-messages")),
    }
    Ok(())
}

fn finish(errors: Vec<String>) -> Res<()> {
    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n").into()) }
}

pub(super) fn scan(board: &IcyBoard, wanted: Option<&str>) -> Res<()> {
    let mut errors = Vec::new();
    for link in selected_links(&board.zconnect, wanted)? {
        if let Err(err) = scan_link(&board.zconnect, &board.root_path, link) {
            errors.push(format!("{}: {err}", link.id));
        }
    }
    finish(errors)
}

/// Inspect only complete, regular ZIP files directly in this link's inbox.
/// Processed/retained subdirectories, temporary files and symlinks are excluded.
pub(super) fn inbound_packets(dir: &Path) -> Res<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut packets = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zip")) {
            packets.push(entry.path());
        }
    }
    packets.sort();
    Ok(packets)
}

fn sync_dir(dir: &Path) -> Res<()> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Same-filesystem non-overwriting move: atomically create a second name for
/// the original inode, make it durable, then retire its inbox name. If any step
/// fails before retirement, the original is still in the inbox. No copy is
/// truncated and existing archives are never replaced, even on name collision.
pub(super) fn archive_packet(packet: &Path, retained: bool) -> Res<PathBuf> {
    if !fs::symlink_metadata(packet)?.file_type().is_file() {
        return Err(message("zconnect-unsafe-packet").into());
    }
    let parent = packet.parent().ok_or_else(|| message("zconnect-unsafe-packet"))?;
    let archive = parent.join(if retained { "retained" } else { "processed" });
    fs::create_dir_all(&archive)?;
    if !fs::symlink_metadata(&archive)?.file_type().is_dir() {
        return Err(message("zconnect-unsafe-packet").into());
    }
    let name = packet.file_name().ok_or_else(|| message("zconnect-unsafe-packet"))?;
    File::open(packet)?.sync_all()?;
    for counter in 0u64.. {
        let mut candidate = name.to_os_string();
        if counter != 0 {
            candidate.push(format!(".{counter}"));
        }
        let destination = archive.join(candidate);
        match fs::hard_link(packet, &destination) {
            Ok(()) => {
                sync_dir(&archive)?;
                sync_dir(parent)?;
                fs::remove_file(packet)?;
                sync_dir(parent)?;
                return Ok(destination);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!()
}

/// Keep this small boundary injectable so CLI retention/error policy can be
/// tested independently of the engine's ZIP and JAM fixture tests.
pub(super) fn toss_packets(dir: &Path, mut toss: impl FnMut(&Path) -> Res<TossReport>) -> Res<()> {
    let mut errors = Vec::new();
    for packet in inbound_packets(dir)? {
        let result: Res<()> = (|| {
            let report = toss(&packet)?;
            let retained = report.unsupported != 0 || report.unknown_boards != 0;
            let destination = archive_packet(&packet, retained)?;
            println!(
                "{}: {}: {}, {}: {}, {}: {}, {}: {}, {}: {}; {}",
                packet.display(),
                message("zconnect-imported"),
                report.imported,
                message("zconnect-duplicates"),
                report.duplicates,
                message("zconnect-loops"),
                report.loops,
                message("zconnect-unsupported"),
                report.unsupported,
                message("zconnect-unknown-boards"),
                report.unknown_boards,
                destination.display()
            );
            if retained {
                return Err(format!("{}: {}", message("zconnect-retained"), destination.display()).into());
            }
            Ok(())
        })();
        if let Err(err) = result {
            errors.push(format!("{}: {err}", packet.display()));
        }
    }
    finish(errors)
}

fn toss_link(config: &ZconnectConfig, root: &Path, link: &ZconnectLink) -> Res<()> {
    let _lock = offline_lock(config, root, link)?;
    let dir = rooted(root, &config.inbound).join(&link.id);
    toss_packets(&dir, |packet| engine::toss(config, root, &link.id, packet))
}

pub(super) fn toss(board: &IcyBoard, wanted: Option<&str>) -> Res<()> {
    let mut errors = Vec::new();
    for link in selected_links(&board.zconnect, wanted)? {
        if let Err(err) = toss_link(&board.zconnect, &board.root_path, link) {
            errors.push(format!("{}: {err}", link.id));
        }
    }
    finish(errors)
}

pub(super) fn ack(board: &IcyBoard, wanted: &str) -> Res<()> {
    // Deliberately no optional ID / all-links acknowledgement path. The sysop
    // must have verified delivery of this exact pending archive externally.
    let link = selected_links(&board.zconnect, Some(wanted))?[0];
    let _lock = offline_lock(&board.zconnect, &board.root_path, link)?;
    engine::acknowledge_outbound(&board.zconnect, &board.root_path, &link.id)?;
    println!("{}: {}", link.id, message("zconnect-acknowledged"));
    Ok(())
}

pub(super) async fn exchange_and_toss(exchange: impl Future<Output = Res<PollReport>>, toss: impl FnOnce() -> Res<()>) -> Res<()> {
    let mut errors = Vec::new();
    match exchange.await {
        Ok(report) => println!(
            "{}: {}; {}: {}",
            message("zconnect-uploaded"),
            report.uploaded,
            message("zconnect-downloaded"),
            report.downloaded.len()
        ),
        Err(err) => errors.push(format!("{}: {err}", message("zconnect-exchange-failed"))),
    }
    // A late handshake/disconnect error may follow durable downloads. Always
    // inspect the whole inbox, not merely the report's newly downloaded list.
    if let Err(err) = toss() {
        errors.push(format!("{}: {err}", message("zconnect-toss-failed")));
    }
    finish(errors)
}

pub(super) async fn poll(board: &IcyBoard, wanted: Option<&str>) -> Res<()> {
    let mut errors = Vec::new();
    for link in selected_links(&board.zconnect, wanted)? {
        let prepared = scan_link(&board.zconnect, &board.root_path, link);
        let exchange = async {
            prepared?;
            engine::poll(&board.zconnect, &board.root_path, &link.id).await
        };
        if let Err(err) = exchange_and_toss(exchange, || toss_link(&board.zconnect, &board.root_path, link)).await {
            errors.push(format!("{}: {err}", link.id));
        }
    }
    finish(errors)
}
