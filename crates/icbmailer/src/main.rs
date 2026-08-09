use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
};

use argh::FromArgs;
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        ftn::{
            FtnConfig, FtnLink,
            bundle::{is_bundle, unpack},
            packet::Packet,
            toss::{TossReport, TossTarget, scan_outbound, toss_inbound},
        },
        message_area::MessageArea,
    },
};
use icy_net::binkp::{BinkpIdentity, PollRequest};

mod zconnect_experiment;

#[derive(FromArgs)]
/// Exchange fidonet mail with the systems listed in ftn.toml
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    Links(Links),
    Poll(Poll),
    Scan(Scan),
    Show(Show),
    Toss(Toss),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "links")]
/// list the configured links and what is waiting for them
struct Links {
    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    config: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "poll")]
/// call a link, hand over what is waiting for it and take what it has
struct Poll {
    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    config: PathBuf,

    #[argh(positional)]
    /// the address to call, every link when left out
    address: Option<String>,

    #[argh(switch, short = 'k')]
    /// leave delivered files in the outbound instead of deleting them
    keep: bool,

    #[argh(switch, short = 'v')]
    /// report what the session is doing
    verbose: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
/// list what is inside a packet or a mail bundle
struct Show {
    #[argh(positional)]
    /// the packet or bundle to look into
    file: PathBuf,

    #[argh(switch, short = 't')]
    /// print the message text as well
    text: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "toss")]
/// read the mail waiting in the inbound into the message bases
struct Toss {
    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    config: PathBuf,

    #[argh(switch, short = 'v')]
    /// report what the tosser is doing
    verbose: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "scan")]
/// pack the mail written here into bundles for the links that carry its area
struct Scan {
    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    config: PathBuf,

    #[argh(switch, short = 'v')]
    /// report what the scanner is doing
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let arguments: Cli = argh::from_env();
    let result = match arguments.command {
        Command::Links(arguments) => list_links(&arguments.config),
        Command::Poll(arguments) => match load_and_log(&arguments.config, arguments.verbose) {
            Ok(mut board) => poll_links(&mut board, arguments.address.as_deref(), arguments.keep).await,
            Err(err) => Err(err),
        },
        Command::Show(arguments) => {
            set_up_logging(false);
            show(&arguments.file, arguments.text)
        }
        Command::Toss(arguments) => match load_and_log(&arguments.config, arguments.verbose) {
            Ok(mut board) => toss(&mut board),
            Err(err) => Err(err),
        },
        Command::Scan(arguments) => match load_and_log(&arguments.config, arguments.verbose) {
            Ok(board) => scan(&board),
            Err(err) => Err(err),
        },
    };
    if let Err(err) = result {
        eprintln!("{}", err);
        exit(1);
    }
}

/// The board has to be read before the logging can be set up, because it is
/// `ftn.toml` that says how much of it the sysop wants to see.
fn load_and_log(config: &Path, verbose: bool) -> Res<IcyBoard> {
    let board = load(config)?;
    set_up_logging(verbose || board.ftn.options.verbose_log);
    Ok(board)
}

fn set_up_logging(verbose: bool) {
    let level = if verbose { log::LevelFilter::Debug } else { log::LevelFilter::Warn };
    let _ = fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(level)
        .chain(std::io::stderr())
        .apply();
}

fn load(config: &Path) -> Res<IcyBoard> {
    let mut board = IcyBoard::load(&config)?;
    board.resolve_paths();
    if !board.ftn.is_configured() {
        return Err(format!("{} lists no ftn address, so there is nothing to introduce this board as", config.display()).into());
    }
    Ok(board)
}

fn list_links(config: &Path) -> Res<()> {
    let board = load(config)?;
    for link in &board.ftn.links {
        let waiting = outbound_files(&board.ftn.outbound_for(link))?;
        let bytes: u64 = waiting.iter().filter_map(|path| path.metadata().ok()).map(|data| data.len()).sum();
        println!(
            "{:<20} {}:{:<6} as {:<20} {} file(s), {} bytes waiting",
            link.to_5d(),
            link.host,
            link.port,
            board.ftn.aka_for(link).map(|aka| aka.to_5d()).unwrap_or_default(),
            waiting.len(),
            bytes
        );
    }
    if board.ftn.links.is_empty() {
        println!("No links configured.");
    }
    Ok(())
}

async fn poll_links(board: &mut IcyBoard, address: Option<&str>, keep: bool) -> Res<()> {
    if !board.ftn.options.dial_out {
        return Err("This board is set not to call out, see dial_out in ftn.toml".into());
    }
    let selected: Vec<FtnLink> = match address {
        Some(wanted) => board.ftn.links.iter().filter(|link| answers_to(link, wanted)).cloned().collect(),
        None => board.ftn.links.clone(),
    };
    if selected.is_empty() {
        return Err(match address {
            Some(wanted) => format!("No link named {} is configured", wanted).into(),
            None => "No links configured".into(),
        });
    }

    let mut failed = 0;
    let mut received = false;
    for link in &selected {
        match poll_link(&board.ftn, &identity_for(board, link)?, link, keep).await {
            Ok(files) => received |= files,
            Err(err) => {
                eprintln!("{}: {}", link.to_5d(), err);
                failed += 1;
            }
        }
    }
    if received && board.ftn.options.import_after_xfer {
        toss(board)?;
    }
    if failed > 0 {
        return Err(format!("{} of the calls did not get through", failed).into());
    }
    Ok(())
}

/// Answers whether the call brought anything back, which is what decides
/// whether there is a point in tossing afterwards.
async fn poll_link(ftn: &FtnConfig, identity: &BinkpIdentity, link: &FtnLink, keep: bool) -> Res<bool> {
    let outbound = outbound_files(&ftn.outbound_for(link))?;
    println!(
        "Calling {} at {}:{} with {} file(s) to hand over",
        link.to_5d(),
        link.host,
        link.port,
        outbound.len()
    );

    let request = PollRequest {
        host: link.host.clone(),
        port: link.port,
        identity: identity.clone(),
        called: link.to_5d(),
        password: link.password.clone(),
        outbound,
        inbound: ftn.inbound.clone(),
        ..Default::default()
    };
    let result = icy_net::binkp::poll(&request).await?;

    println!(
        "  {} answered, running {}{}",
        if result.remote.system_name.is_empty() {
            link.to_5d()
        } else {
            result.remote.system_name.clone()
        },
        result.remote.mailer,
        if result.remote.secure { "" } else { " (unsecure session)" }
    );
    for path in &result.batch.received {
        println!("  received {}", path.display());
    }
    for path in &result.batch.sent {
        println!("  delivered {}", path.display());
        if !keep {
            fs::remove_file(path)?;
        }
    }
    for path in &result.batch.skipped {
        println!("  held back for the next call: {}", path.display());
    }
    Ok(!result.batch.received.is_empty())
}

/// An address may be given with or without its network, so both spellings count.
fn answers_to(link: &FtnLink, wanted: &str) -> bool {
    link.address.to_string().eq_ignore_ascii_case(wanted) || link.to_5d().eq_ignore_ascii_case(wanted)
}

fn toss(board: &mut IcyBoard) -> Res<()> {
    let target = TossTarget {
        sysop: board.config.sysop.name.clone(),
        users: board.users.iter().map(|user| user.get_name().to_string()).collect(),
    };
    let report = toss_inbound(&board.ftn, &echo_areas(board), &target)?;

    println!(
        "{} message(s) tossed, {} netmail, {} duplicate(s) dropped",
        report.imported, report.netmail, report.duplicates
    );
    if report.passed_through > 0 {
        println!("  {} message(s) handed on in {} bundle(s)", report.passed_through, report.bundles.len());
    }
    if report.orphans > 0 {
        println!("  {} packet(s) for another system left in the inbound", report.orphans);
    }
    for (tag, count) in &report.unknown {
        println!("  {} message(s) arrived for {}, which no area carries", count, tag);
    }
    for (file, err) in &report.failed {
        println!("  left in the inbound, {}: {}", file.display(), err);
    }
    register_new_areas(board, &report)?;
    Ok(())
}

/// An area the tosser created for a tag nobody carried is of no use until a
/// conference offers it to the users.
fn register_new_areas(board: &mut IcyBoard, report: &TossReport) -> Res<()> {
    if report.added.is_empty() {
        return Ok(());
    }
    let number = board.ftn.options.auto_add_conference;
    let Some(conference) = board.conferences.get_mut(number) else {
        return Err(format!("Conference {}, which new areas are added to, does not exist", number).into());
    };
    let areas = conference.areas.get_or_insert_with(Default::default);
    for (tag, path) in &report.added {
        println!("  {} is new, added to {}", tag, conference.name);
        areas.push(MessageArea {
            name: tag.clone(),
            ftn_area_tag: tag.clone(),
            path: path.clone(),
            ..Default::default()
        });
    }
    areas.save(&conference.area_file)?;
    Ok(())
}

fn scan(board: &IcyBoard) -> Res<()> {
    let report = scan_outbound(&board.ftn, &echo_areas(board), &chrono::Local::now().naive_local())?;

    println!("{} message(s) packed into {} bundle(s)", report.exported, report.bundles.len());
    for bundle in &report.bundles {
        println!("  {}", bundle.display());
    }
    Ok(())
}

/// The areas that take part in the network, told apart by the tag they carry
/// there. An area without a tag is one this board keeps to itself.
fn echo_areas(board: &IcyBoard) -> Vec<(String, PathBuf)> {
    let mut areas = Vec::new();
    for conference in board.conferences.iter() {
        let Some(list) = &conference.areas else {
            continue;
        };
        for area in list.iter() {
            if !area.ftn_area_tag.is_empty() {
                areas.push((area.ftn_area_tag.clone(), area.path.clone()));
            }
        }
    }
    areas
}

fn identity_for(board: &IcyBoard, link: &FtnLink) -> Res<BinkpIdentity> {
    let Some(aka) = board.ftn.aka_for(link) else {
        return Err(format!("No address of this board belongs to the network of {}", link.to_5d()).into());
    };
    Ok(BinkpIdentity {
        addresses: vec![aka.to_5d()],
        system_name: board.config.board.name.clone(),
        sysop: board.config.sysop.name.clone(),
        location: board.config.board.location.clone(),
        ..Default::default()
    })
}

fn outbound_files(directory: &Path) -> Res<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !directory.is_dir() {
        return Ok(files);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    // Bundles are named so that the oldest sorts first, and that is the order they should travel in.
    files.sort();
    Ok(files)
}

fn show(file: &Path, with_text: bool) -> Res<()> {
    let name = file.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if !is_bundle(name) {
        return show_packet(file, with_text);
    }

    let unpacked = tempfile::tempdir()?;
    let packets = unpack(file, unpacked.path())?;
    println!("{} holds {} file(s)", name, packets.len());
    for packet in packets {
        println!();
        show_packet(&packet, with_text)?;
    }
    Ok(())
}

fn show_packet(file: &Path, with_text: bool) -> Res<()> {
    let packet = Packet::load(file)?;
    println!(
        "{}: {} -> {}, written {}, {} message(s)",
        file.file_name().unwrap_or_default().to_string_lossy(),
        packet.header.orig,
        packet.header.dest,
        packet.header.created.format("%Y-%m-%d %H:%M:%S"),
        packet.messages.len()
    );
    for (index, message) in packet.messages.iter().enumerate() {
        println!(
            "  {:>3}. {:<12} {} -> {}: {}",
            index + 1,
            message.area().unwrap_or("netmail"),
            message.from,
            message.to,
            message.subject
        );
        if with_text {
            for line in message.text.split('\r') {
                println!("       {}", line);
            }
        }
    }
    Ok(())
}
