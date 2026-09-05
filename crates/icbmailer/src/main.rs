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
            FtnConfig, FtnLink, FtnLogLevel,
            bundle::{is_bundle, unpack},
            freq,
            nodelist::Nodelist,
            packet::Packet,
            tic::{FileArea, toss_tics},
            toss::{EchoArea, TossReport, scan_outbound, toss_inbound},
        },
        message_area::MessageArea,
    },
};
use icy_net::binkp::{BinkpIdentity, PollRequest};

mod zconnect_experiment;

#[derive(FromArgs)]
/// Exchange fidonet mail with the systems listed in ftn.toml
struct Cli {
    /// print the version and exit
    #[argh(switch)]
    version: bool,

    #[argh(subcommand)]
    command: Option<Command>,
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
    if arguments.version {
        println!("icbmailer {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let Some(command) = arguments.command else {
        if let Err(err) = Cli::from_args(&["icbmailer"], &["--help"]) {
            eprintln!("{}", err.output);
        }
        exit(1);
    };
    let result = match command {
        Command::Links(arguments) => list_links(&arguments.config),
        Command::Poll(arguments) => match load_and_log(&arguments.config, arguments.verbose) {
            Ok(mut board) => poll_links(&mut board, arguments.address.as_deref(), arguments.keep).await,
            Err(err) => Err(err),
        },
        Command::Show(arguments) => {
            set_up_logging(FtnLogLevel::Normal);
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
    set_up_logging(if verbose { FtnLogLevel::Debug } else { board.ftn.options.log_level });
    Ok(board)
}

fn set_up_logging(level: FtnLogLevel) {
    let level = match level {
        FtnLogLevel::Normal => log::LevelFilter::Warn,
        FtnLogLevel::Detailed => log::LevelFilter::Info,
        FtnLogLevel::Debug => log::LevelFilter::Debug,
    };
    let _ = fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(level)
        .chain(std::io::stderr())
        .apply();
}

fn load(config: &Path) -> Res<IcyBoard> {
    let mut board = IcyBoard::load(&config)?;
    board.resolve_paths();
    if board.ftn.options.enabled && !board.ftn.is_configured() {
        return Err(format!(
            "{} lists no ftn address, so there is nothing to introduce this board as. Add an [[aka]] block with the node number your \
             coordinator gave you to the ftn file named by ftn_file, or fill it in under Message Networking in ICBSetup",
            config.display()
        )
        .into());
    }
    Ok(board)
}

fn list_links(config: &Path) -> Res<()> {
    let board = load(config)?;
    let mut links = board.ftn.links.clone();
    look_up_hosts(&board, &mut links)?;
    for link in &links {
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
    if !board.ftn.options.enabled {
        return Err("Fido processing is disabled. Enable it under Message Networking > Fido Configuration in ICBSetup".into());
    }
    if !board.ftn.options.dial_out {
        return Err("This board is set not to call out, see dial_out in ftn.toml".into());
    }
    let mut selected: Vec<FtnLink> = match address {
        Some(wanted) => board.ftn.links.iter().filter(|link| answers_to(link, wanted)).cloned().collect(),
        None => board.ftn.links.clone(),
    };
    if selected.is_empty() {
        return Err(match address {
            Some(wanted) => format!("No link named {} is configured", wanted).into(),
            None => "No links configured".into(),
        });
    }
    look_up_hosts(board, &mut selected)?;

    let mut failed = 0;
    let mut received = false;
    for link in &selected {
        if link.host.is_empty() {
            eprintln!(
                "{} has no host. Enter one under Message Networking > Node Configuration, or configure a nodelist that lists it with an IBN flag",
                link.to_5d()
            );
            failed += 1;
            continue;
        }
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

/// A link that names no host of its own is looked up in the nodelist.
fn look_up_hosts(board: &IcyBoard, links: &mut [FtnLink]) -> Res<()> {
    if board.ftn.nodelist.as_os_str().is_empty() || !links.iter().any(|link| link.host.is_empty()) {
        return Ok(());
    }
    let list = Nodelist::load(&board.ftn.nodelist)?;
    for link in links.iter_mut().filter(|link| link.host.is_empty()) {
        let Some(entry) = list.find(&link.address) else {
            println!("{} names no host and the nodelist does not list it", link.to_5d());
            continue;
        };
        match entry.binkp_address() {
            Some((host, port)) => {
                println!("{} is {} at {}:{}, from the nodelist", link.to_5d(), entry.name, host, port);
                link.host = host;
                link.port = port;
            }
            None => println!(
                "{} is {} in the nodelist, which does not say it takes binkp. Enter a host by hand if it does",
                link.to_5d(),
                entry.name
            ),
        }
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
        let path = if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("req")) {
            freq::identify_received(path, &link.address)?
        } else {
            path.clone()
        };
        println!("  received {}", path.display());
    }
    for path in &result.batch.sent {
        println!("  delivered {}", path.display());
        if !keep {
            fs::remove_file(path).map_err(|err| {
                format!(
                    "{} accepted {}, but it could not be removed from the outbound: {err}. Remove it by hand before polling again, or it may be offered twice",
                    link.to_5d(),
                    path.display()
                )
            })?;
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
    if !board.ftn.options.enabled {
        return Err("Fido processing is disabled. Enable it under Message Networking > Fido Configuration in ICBSetup".into());
    }
    if !board.ftn.options.process_in {
        return Err("Inbound processing is disabled, so no received mail was read. Turn on Process Inbound under Message Networking in ICBSetup, or set options.process_in = true in ftn.toml".into());
    }
    if !board.ftn.inbound.exists() {
        println!("No inbound directory at {}; there is no received mail to toss", board.ftn.inbound.display());
        return Ok(());
    }
    if !board.ftn.inbound.is_dir() {
        return Err(format!(
            "The configured inbound {} is not a directory. Change the Inbound path in ICBSetup or remove the file occupying that path",
            board.ftn.inbound.display()
        )
        .into());
    }
    let report = toss_inbound(&board.ftn, &echo_areas(board))?;

    println!(
        "{} echomail message(s) tossed, {} netmail, {} duplicate(s) dropped",
        report.imported, report.netmail, report.duplicates
    );
    if report.passed_through > 0 {
        println!("  {} echomail message(s) handed on", report.passed_through);
    }
    if report.routed > 0 {
        println!("  {} packet(s) routed to the next hop", report.routed);
    }
    if report.area_fix > 0 {
        println!("  {} AreaFix request(s) processed", report.area_fix);
    }
    if report.orphans > 0 {
        println!(
            "  {} packet(s) addressed to another system left in the inbound. Run with -v to see which address they name; add it as an aka, or turn on Toss Orphans to read them anyway",
            report.orphans
        );
    }
    for (source, count) in &report.untrusted_netmail {
        print_untrusted_netmail_advice(board, source, *count);
    }
    for (tag, count) in &report.unknown {
        println!(
            "  {} message(s) arrived for {}, which no area carries. Set ftn_area_tag = \"{}\" on the area that should hold it, or turn on Add Unknown Areas",
            count, tag, tag
        );
    }
    for (file, err, put_aside) in &report.failed {
        match put_aside {
            Some(target) => println!("  {} could not be read and was moved to {}: {}", file.display(), target.display(), err),
            None => println!("  left in the inbound, {}: {}", file.display(), err),
        }
    }
    toss_files(board)?;
    serve_requests(board)?;
    register_link_updates(board, &report)?;
    register_new_areas(board, &report)?;
    Ok(())
}

fn register_link_updates(board: &mut IcyBoard, report: &TossReport) -> Res<()> {
    if report.link_updates.is_empty() {
        return Ok(());
    }
    let path = board.config.paths.ftn_file.clone();
    let mut stored = FtnConfig::load(&path)?;
    for (index, areas) in &report.link_updates {
        if let Some(link) = board.ftn.links.get_mut(*index) {
            link.areas.clone_from(areas);
        }
        if let Some(link) = stored.links.get_mut(*index) {
            link.areas.clone_from(areas);
        }
    }
    stored.save(&path)?;
    Ok(())
}

fn print_untrusted_netmail_advice(board: &IcyBoard, source: &str, count: usize) {
    println!(
        "  WARNING: {} netmail message(s) from unconfigured node {} stored in {}",
        count,
        source,
        board.ftn.bad_netmail.display()
    );
    println!("    Reason: Secure Netmail is enabled, but the packet origin does not match any [[link]] address in ftn.toml.");
    println!(
        "    Fix: add [[link]] address = \"{}\" for a node you trust, or disable Secure Netmail (options.secure = false).",
        source
    );
}

/// Puts the files that arrived with a `.TIC` into the directories that carry
/// their area.
fn toss_files(board: &IcyBoard) -> Res<()> {
    let report = toss_tics(&board.ftn, &file_areas(board))?;
    for (file, area) in &report.arrived {
        println!("  {} put into {}", file, area);
    }
    for name in &report.replaced {
        println!("  {} was replaced and removed", name);
    }
    for (tag, count) in &report.unknown {
        println!(
            "  {} file(s) arrived for {}, which no file directory carries. Set Fido Area Tag = \"{}\" on the directory that should hold them",
            count, tag, tag
        );
    }
    for (file, err) in &report.failed {
        println!("  left in the inbound, {}: {}", file.display(), err);
    }
    Ok(())
}

/// The file directories that take part in the network, told apart by the tag
/// they carry there. A directory named after the echo is found by that name.
fn file_areas(board: &IcyBoard) -> Vec<FileArea> {
    let mut areas = Vec::new();
    for conference in board.conferences.iter() {
        let Some(list) = &conference.directories else {
            continue;
        };
        for directory in list.iter() {
            areas.push(FileArea {
                tag: directory.ftn_area_tag.clone(),
                name: directory.name.clone(),
                path: directory.path.clone(),
                metadata_path: directory.metadata_path.clone(),
            });
        }
    }
    areas
}

/// Puts what the nodes asked for into their outbound, so the next session
/// carries it.
fn serve_requests(board: &IcyBoard) -> Res<()> {
    let report = freq::serve(&board.ftn)?;
    if report.requests == 0 && report.unknown.is_empty() && report.failed.is_empty() {
        return Ok(());
    }
    println!("  {} file request(s), {} file(s) put in an outbound", report.requests, report.served);
    for (wanted, why) in &report.refused {
        println!("  {} was not sent: {}", wanted, why);
    }
    for name in &report.unknown {
        println!(
            "  {} was left alone because no configured link answers to the node that wrote it. Add that node under Message Networking > Node Configuration",
            name
        );
    }
    for (file, err) in &report.failed {
        println!("  left in the inbound, {}: {}", file.display(), err);
    }
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
    let areas = std::sync::Arc::make_mut(conference.areas.get_or_insert_with(Default::default));
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
    if !board.ftn.options.enabled {
        return Err("Fido processing is disabled. Enable it under Message Networking > Fido Configuration in ICBSetup".into());
    }
    if !board.ftn.options.process_out {
        return Err("Outbound scanning is disabled, so no locally written mail was packed. Turn on Scan Outbound under Message Networking in ICBSetup, or set options.process_out = true in ftn.toml".into());
    }
    if board.ftn.links.is_empty() {
        return Err("No mail can be scanned because ftn.toml configures no links. Add the uplink or downlink under Message Networking in ICBSetup".into());
    }
    let report = scan_outbound(&board.ftn, &echo_areas(board), &chrono::Local::now().naive_local())?;

    println!("{} message(s) packed into {} bundle(s)", report.exported, report.bundles.len());
    if report.netmail > 0 {
        println!("  {} netmail message(s) sent on their way", report.netmail);
    }
    for number in &report.unaddressed {
        println!(
            "  Netmail {} names no destination address and stays put. Reply to a message that came in from the network, so the address it carries is kept",
            number
        );
    }
    for (number, address) in &report.undeliverable {
        println!("  Netmail {} is addressed to {}, which no link carries and no route reaches", number, address);
    }
    for tag in &report.refused {
        println!(
            "  Area {} points at the netmail base and was left alone. Netmail is addressed to one node, so clear its Fido Area Tag",
            tag
        );
    }
    for bundle in &report.bundles {
        println!("  {}", bundle.display());
    }
    Ok(())
}

/// The areas that take part in the network, told apart by the tag they carry
/// there. An area without a tag is one this board keeps to itself.
fn echo_areas(board: &IcyBoard) -> Vec<EchoArea> {
    let mut areas = Vec::new();
    for conference in board.conferences.iter() {
        let Some(list) = &conference.areas else {
            continue;
        };
        for area in list.iter() {
            if !area.ftn_area_tag.is_empty() {
                areas.push(EchoArea {
                    tag: area.ftn_area_tag.clone(),
                    path: area.path.clone(),
                    origin: area.ftn_origin.clone(),
                });
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
    for entry in fs::read_dir(directory).map_err(|err| format!("Cannot read the outbound {}: {err}", directory.display()))? {
        let entry = entry.map_err(|err| format!("Cannot read an entry in the outbound {}: {err}", directory.display()))?;
        if entry
            .file_type()
            .map_err(|err| format!("Cannot inspect {} in the outbound: {err}", entry.path().display()))?
            .is_file()
        {
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
