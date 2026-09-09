use std::{
    fmt::Display,
    io::{stderr, stdout},
    path::PathBuf,
    process::{self, Command, exit},
    sync::Arc,
};

use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use clap::Parser;
use connections::{restart_connections, start_connections, stop_connections};
use crossterm::{
    ExecutableCommand, execute,
    style::{Attribute, Print, SetAttribute, SetForegroundColor},
    terminal::Clear,
};
use icy_board_engine::{
    Res,
    icy_board::{IcyBoard, bbs::BBS, lock::BoardLock, state::PPEExecute},
};

use node_monitoring_screen::{NodeMonitoringScreenMessage, WebAdminInfo};
use ratatui::{Terminal, backend::Backend};
use semver::Version;
use system_statistics_screen::{SystemStatisticsScreen, SystemStatisticsScreenMessage};
use tokio::{sync::Mutex, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tui::{Tui, print_exit_screen};

const WEB_ADMIN_TOKEN_ENV: &str = "ICBADMIN_TOKEN";

pub mod bbs;
mod call_wait_screen;
mod connections;
mod cws_chrome;
mod event_scheduler;
mod event_screen;
mod log_screen;
pub mod menu_runner;
mod node_monitoring_screen;
mod system_statistics_screen;
mod system_status_screen;
mod terminal_thread;
mod tui;

#[cfg(test)]
mod tests;

static mut SHOW_TOTAL_STATS: bool = true;

#[derive(Parser)]
#[command(name = "icboard", disable_version_flag = true, about = icy_board_cli::text("icboard", "about"))]
struct Cli {
    /// Internal worker: PPE path, output directory, original archive name.
    #[arg(long, hide = true, num_args = 3, allow_hyphen_values = true, value_names = ["PPE", "OUTPUT", "ARCHIVE"])]
    upload_advertisement_ppe: Vec<String>,

    #[arg(long = "full-screen", short = 'f', help = icy_board_cli::text("icboard", "full-screen"))]
    full_screen: bool,

    #[arg(long = "localon", help = icy_board_cli::text("icboard", "localon"))]
    localon: bool,

    #[arg(long = "ppe", help = icy_board_cli::text("icboard", "ppe"))]
    ppe: Option<PathBuf>,

    #[arg(long = "runppe", help = icy_board_cli::text("icboard", "runppe"))]
    runppe: Option<String>,

    #[arg(long = "key", help = icy_board_cli::text("icboard", "key"))]
    key: Option<String>,

    #[arg(long = "version", help = icy_board_cli::text("icboard", "version"))]
    version: bool,

    #[arg(help = icy_board_cli::text("icboard", "file"))]
    file: Option<PathBuf>,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn cli_defaults_and_options() {
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>(["icboard"]).unwrap();
        assert!(!cli.full_screen && !cli.localon && !cli.version);
        assert!(cli.ppe.is_none() && cli.runppe.is_none() && cli.key.is_none() && cli.file.is_none());
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>([
            "icboard",
            "-f",
            "--localon",
            "--ppe",
            "test.ppe",
            "--runppe",
            "first;last",
            "--key",
            "abc",
            "--version",
            "board.toml",
        ])
        .unwrap();
        assert!(cli.full_screen && cli.localon && cli.version);
        assert_eq!(cli.ppe, Some(PathBuf::from("test.ppe")));
        assert_eq!(cli.runppe.as_deref(), Some("first;last"));
        assert_eq!(cli.key.as_deref(), Some("abc"));
        assert_eq!(cli.file, Some(PathBuf::from("board.toml")));
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icboard", "--localon=true"]).is_err());
    }
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// Set by build.rs, empty when the binary was not built from a checkout.
const GIT_HASH: &str = env!("GIT_HASH");
/// evlevlelvelvelv`

#[tokio::main]
async fn main() -> Res<()> {
    let arguments = icy_board_cli::parse::<Cli>();
    if !arguments.upload_advertisement_ppe.is_empty() {
        let args = &arguments.upload_advertisement_ppe;
        return icy_board_engine::icy_board::upload_advertisement::run_advertisement_ppe(
            std::path::Path::new(&args[0]),
            std::path::Path::new(&args[1]),
            &args[2],
        )
        .await;
    }
    if arguments.version {
        println!("{}", icy_board_cli::version_line("icboard", &*VERSION, GIT_HASH));
        return Ok(());
    }
    let file = match icy_board_engine::resolve_icyboard_file(&arguments.file) {
        Ok(file) => file,
        Err(icy_board_engine::IcyBoardFileLookupError::FileNotFound(path)) => {
            icy_board_tui::print_board_config_not_found("icboard", &path);
            exit(1);
        }
    };

    if let Err(err) = start_icy_board(&arguments, file).await {
        print_error(err);
        exit(1);
    }
    Ok(())
}

async fn start_icy_board(arguments: &Cli, file: PathBuf) -> Res<()> {
    let stuffed = arguments.key.clone().unwrap_or_default();
    let config_file = file.with_extension("toml");
    let log_file = config_file.with_extension("log");
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Info)
        // - and per-module overrides
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("x86_native", log::LevelFilter::Debug)
        // Output to stdout, files, and other Dispatch configurations
        .chain(fern::log_file(&log_file).map_err(|err| std::io::Error::new(err.kind(), format!("Can't open log file {}: {err}", log_file.display())))?)
        // Apply globally
        .apply()
        .map_err(|err| format!("Can't initialize logging: {err}"))?;
    let _ = x86::set_native_log_handler(|message| log::debug!(target: "x86_native", "{message}"));
    match IcyBoard::load(&config_file) {
        Ok(mut icy_board) => {
            icy_board.resolve_paths();
            // Operator screens follow the palette the board owns, like the tools do.
            icy_board_tui::theme::set_admin_theme(&icy_board.config.sysop.config_color_theme, &icy_board.config.sysop.config_color_configuration);
            let board_lock = Arc::new(Mutex::new(Some(BoardLock::acquire(&icy_board.root_path)?)));
            let recovered = icy_board_engine::icy_board::upload_quarantine::UploadQuarantine::new(icy_board.config.upload_processing.quarantine_path.clone())
                .recover_interrupted()?;
            if recovered > 0 {
                log::warn!("Returned {recovered} interrupted uploads to SysOp review");
            }
            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
            let board: Arc<Mutex<IcyBoard>> = Arc::new(tokio::sync::Mutex::new(icy_board));
            if arguments.localon || arguments.ppe.is_some() {
                let mut terminal = init_terminal()?;
                let cmd = if let Some(ppe) = &arguments.ppe {
                    CallWaitMessage::RunPPE(ppe.clone(), None, None, None)
                } else {
                    CallWaitMessage::User
                };
                run_message(cmd, &mut terminal, &board, &mut bbs, arguments.full_screen, stuffed, None).await?;
                restore_terminal()?;
                return Ok(());
            }

            // Handle /runppe parameter
            if let Some(runppe_params) = &arguments.runppe {
                match handle_runppe(runppe_params).await {
                    Ok(cmd) => {
                        let mut terminal = init_terminal()?;
                        run_message(cmd, &mut terminal, &board, &mut bbs, arguments.full_screen, stuffed, None).await?;
                        restore_terminal()?;
                    }
                    Err(err) => {
                        print_error(err.to_string());
                        exit(99);
                    }
                }
                exit(0);
            }

            let mut connection_token = CancellationToken::new();
            let mut services = JoinSet::new();
            // Keep the scheduler (and its occurrence watermark) alive across every
            // listener restart. Dropping this set on exit cancels it explicitly.
            let mut scheduler = JoinSet::new();
            // Every fallible foreground UI/tool path passes this boundary before
            // dropping the runtime. An error must not abandon a live shell.
            let result: Res<()> = async {
                let mut web_admin = start_connections(&bbs, &board, &config_file, connection_token.clone(), &mut services).await?;
                scheduler.spawn(supervise_event_scheduler(board.clone(), bbs.clone(), board_lock.clone()));
                let mut app = CallWaitScreen::new(&board).await?;
                let mut terminal = init_terminal()?;
                loop {
                    terminal.clear()?;
                    app.reset(&board).await;
                    match app.run(&mut terminal, &board, &bbs, arguments.full_screen).await {
                        Ok(msg) => {
                            if matches!(msg, CallWaitMessage::EventRestart) {
                                let (restarted_admin, restarted_app) = app
                                    .during_event(&mut terminal, &bbs, arguments.full_screen, async {
                                        // Normally guaranteed by the scheduler; never release
                                        // BoardLock if an Online reservation is still present.
                                        while bbs.lock().await.event_online_status.is_some() {
                                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                        }
                                        log::info!("Event maintenance: stopping listeners and draining admin requests");
                                        stop_connections(&connection_token, &mut services).await;
                                        // Maintenance utilities launched by the command need
                                        // the cross-process board lock after all writers drain.
                                        drop(board_lock.lock().await.take());
                                        bbs.lock().await.event_listeners_stopped = true;
                                        log::info!("Event maintenance: services drained, board lock released");
                                        // No timeout may reopen a board with a live writer,
                                        // blocked command or failed reload. Redraw independently.
                                        while {
                                            let bbs = bbs.lock().await;
                                            bbs.event_restart_requested || bbs.event_scheduler_error.is_some()
                                        } {
                                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                        }
                                        let admin = restart_connections(&bbs, &board, &config_file, &mut connection_token, &mut services).await;
                                        let app = CallWaitScreen::new(&board).await?;
                                        bbs.lock().await.event_listeners_stopped = false;
                                        log::info!("Event maintenance: listeners ready; scheduler retains admission gate");
                                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((admin, app))
                                    })
                                    .await??;
                                web_admin = restarted_admin;
                                app = restarted_app;
                                continue;
                            }
                            let launches_board_tool = matches!(&msg, CallWaitMessage::SystemManager | CallWaitMessage::Setup | CallWaitMessage::IcbText);
                            if launches_board_tool {
                                let can_start = {
                                    let mut bbs_guard = bbs.lock().await;
                                    bbs_guard.clear_closed_connections().await;
                                    let online = bbs_guard.open_connections.lock().await.iter().any(Option::is_some);
                                    if online || event_screen::runtime_busy(&bbs_guard) {
                                        false
                                    } else {
                                        bbs_guard.operator_maintenance = true;
                                        true
                                    }
                                };
                                if !can_start {
                                    app.show_error(icy_board_tui::get_text("event_runtime_tools_blocked"));
                                    continue;
                                }
                                stop_connections(&connection_token, &mut services).await;
                                drop(board_lock.lock().await.take());
                            }

                            let result = run_message(msg, &mut terminal, &board, &mut bbs, arguments.full_screen, String::new(), web_admin.clone()).await;

                            if launches_board_tool {
                                match BoardLock::acquire(&board.lock().await.root_path) {
                                    Ok(lock) => *board_lock.lock().await = Some(lock),
                                    Err(err) => {
                                        log::error!("could not reacquire board lock after running tool: {err}");
                                        app.show_error(format!("Could not reacquire the board lock: {err}"));
                                        // Fail closed: another process may own the board.
                                        continue;
                                    }
                                }
                            }

                            match result {
                                Ok(reload) => {
                                    if reload {
                                        let mut loaded = IcyBoard::load(&config_file)?;
                                        loaded.resolve_paths();
                                        let nodes = loaded.config.board.num_nodes as usize;
                                        *board.lock().await = loaded;
                                        bbs.lock().await.resize_idle_nodes(nodes).await;
                                        app = CallWaitScreen::new(&board).await?;
                                        web_admin = app
                                            .during_event(
                                                &mut terminal,
                                                &bbs,
                                                arguments.full_screen,
                                                restart_connections(&bbs, &board, &config_file, &mut connection_token, &mut services),
                                            )
                                            .await?;
                                        bbs.lock().await.operator_maintenance = false;
                                        continue;
                                    }
                                }
                                Err(err) => {
                                    log::error!("while processing call wait screen message: {}", err);
                                    app.show_error(err.to_string());
                                    if launches_board_tool {
                                        web_admin = app
                                            .during_event(
                                                &mut terminal,
                                                &bbs,
                                                arguments.full_screen,
                                                restart_connections(&bbs, &board, &config_file, &mut connection_token, &mut services),
                                            )
                                            .await?;
                                        bbs.lock().await.operator_maintenance = false;
                                    }
                                    continue;
                                }
                            }
                        }
                        Err(err) => {
                            restore_terminal()?;
                            log::error!("while running call wait screen: {}", err);
                            return Err(err);
                        }
                    }
                }
            }
            .await;
            if result.is_err() {
                retain_runtime_until_event_safe(&bbs).await;
                stop_connections(&connection_token, &mut services).await;
            }
            result
        }
        Err(err) => {
            log::error!("while loading icy board configuration: {}", err);
            Err(err)
        }
    }
}

/// Exactly one scheduler lifetime, including while local sessions/tools own the
/// foreground. Catch task panic/early return here, not only at call-wait ticks.
async fn supervise_event_scheduler(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, board_lock: Arc<Mutex<Option<BoardLock>>>) {
    let result = tokio::spawn(event_scheduler::run_event_scheduler(board, bbs.clone(), board_lock)).await;
    let message = match result {
        Ok(()) => icy_board_tui::get_text("event_runtime_scheduler_stopped"),
        Err(error) => format!("{}: {error}", icy_board_tui::get_text("event_runtime_scheduler_stopped")),
    };
    latch_scheduler_failure(&bbs, message).await;
}

async fn latch_scheduler_failure(bbs: &Arc<Mutex<BBS>>, message: String) {
    log::error!("{message}");
    let mut bbs = bbs.lock().await;
    bbs.event_maintenance = true;
    bbs.event_scheduler_error.get_or_insert(message);
    // Do not clear active IDs/status or restart flags: a detached foreground
    // command may still be writing. Only explicit repair/restart can resolve it.
}

async fn retain_runtime_until_event_safe(bbs: &Arc<Mutex<BBS>>) {
    bbs.lock().await.operator_maintenance = true;
    loop {
        let busy = {
            let bbs = bbs.lock().await;
            bbs.event_online_status.is_some() || !bbs.event_active_ids.is_empty()
        };
        if !busy {
            break;
        }
        log::error!("Foreground UI failed; retaining runtime and board lock until event work finishes. A latched scheduler failure requires operator repair.");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn reserve_operator_exit(bbs: &Arc<Mutex<BBS>>) -> Res<()> {
    let mut bbs = bbs.lock().await;
    if event_screen::runtime_busy(&bbs) {
        return Err(icy_board_tui::get_text("event_runtime_exit_blocked").into());
    }
    // Atomic with the scheduler's pre-spawn check. Applies to BOTH Exit variants.
    bbs.operator_maintenance = true;
    Ok(())
}

#[cfg(test)]
mod event_operator_tests {
    use super::*;

    #[tokio::test]
    async fn unexpected_scheduler_stop_is_sticky_and_preserves_active_work() {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.event_active_ids.insert("active".into());
        latch_scheduler_failure(&bbs, "original failure".into()).await;
        latch_scheduler_failure(&bbs, "later failure".into()).await;
        let bbs = bbs.lock().await;
        assert!(bbs.admissions_closed());
        assert_eq!(bbs.event_scheduler_error.as_deref(), Some("original failure"));
        assert!(bbs.event_active_ids.contains("active"));
        assert!(bbs.event_maintenance_status.is_none());
        assert!(event_screen::runtime_busy(&bbs));
    }

    #[tokio::test]
    async fn exit_reservation_rejects_queued_active_online_and_failed_runtime() {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.request_event_run("queued".into()).unwrap();
        assert!(reserve_operator_exit(&bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            bbs.event_run_requests.clear();
            bbs.event_active_ids.insert("active".into());
        }
        assert!(reserve_operator_exit(&bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            bbs.event_active_ids.clear();
            bbs.event_online_status = Some(icy_board_engine::icy_board::bbs::OnlineEventStatus {
                event_id: "online".into(),
                description: "Online".into(),
                started: chrono::Utc::now(),
                log_file: None,
                running_long: false,
            });
        }
        assert!(reserve_operator_exit(&bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            bbs.event_online_status = None;
            bbs.event_scheduler_error = Some("failure without status".into());
        }
        assert!(reserve_operator_exit(&bbs).await.is_err());
        bbs.lock().await.event_scheduler_error = None;
        reserve_operator_exit(&bbs).await.unwrap();
        assert!(bbs.lock().await.operator_maintenance);
    }

    #[tokio::test]
    async fn exit_rejects_before_touching_terminal_or_process() {
        let mut bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.request_event_run("queued".into()).unwrap();
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        let result = run_message(CallWaitMessage::Exit, &mut terminal, &board, &mut bbs, false, String::new(), None).await;
        assert_eq!(result.unwrap_err().to_string(), icy_board_tui::get_text("event_runtime_exit_blocked"));
    }
}

async fn run_message<B: Backend>(
    msg: CallWaitMessage,
    terminal: &mut Terminal<B>,
    board: &Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: &mut Arc<Mutex<BBS>>,
    full_screen: bool,
    stuffed_chars: String,
    web_admin: Option<WebAdminInfo>,
) -> Res<bool>
where
    B::Error: Send + Sync + 'static,
{
    match msg {
        CallWaitMessage::EventRestart => {} // Handled by the service owner above.
        CallWaitMessage::User => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
            match Tui::local_mode(board, bbs, false, None, stuffed_chars).await {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, board).await {
                        log::error!("while running board in local mode: {}", err);
                        return Err(err);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        CallWaitMessage::RunPPE(ppe, name_opt, pw_opt, params_opt) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
            match Tui::local_mode(
                board,
                bbs,
                false,
                Some(PPEExecute {
                    ppe,
                    user_name: name_opt,
                    password: pw_opt,
                    args: params_opt.unwrap_or_default(),
                }),
                stuffed_chars,
            )
            .await
            {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, board).await {
                        log::error!("while running board in local mode: {}", err);
                        return Err(err);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        CallWaitMessage::Sysop => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
            match Tui::local_mode(board, bbs, true, None, stuffed_chars).await {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, board).await {
                        log::error!("while running board in local mode: {}", err);
                        return Err(err);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        CallWaitMessage::Exit => {
            reserve_operator_exit(bbs).await?;
            restore_terminal()?;
            print_exit_screen();
            process::exit(0);
        }
        CallWaitMessage::EventMonitor => {
            event_screen::run(terminal, board, bbs, full_screen).await?;
        }
        CallWaitMessage::LogViewer => {
            log_screen::run(terminal, board, bbs, full_screen).await?;
        }
        CallWaitMessage::SystemStatus => {
            system_status_screen::run(terminal, board, bbs, full_screen).await?;
        }
        CallWaitMessage::Monitor => {
            let mut app = node_monitoring_screen::NodeMonitoringScreen::new(board).await;
            match app.run(terminal, board, bbs, full_screen, web_admin.as_ref()).await {
                Ok(msg) => {
                    if let NodeMonitoringScreenMessage::EnterNode(node) = msg {
                        if let Some(mut tui) = Tui::sysop_mode(bbs, node).await? {
                            if let Err(err) = tui.run(bbs, board).await {
                                log::error!("while running board in local mode: {}", err);
                                return Err(err);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("while running node monitoring screen: {}", err);
                    return Err(err);
                }
            }
        }
        CallWaitMessage::ToggleCallLog => {
            let config = &mut board.lock().await.config;
            config.options.call_log = !config.options.call_log;
        }
        CallWaitMessage::TogglePageBell => {
            let config = &mut board.lock().await.config;
            config.options.page_bell = !config.options.page_bell;
        }
        CallWaitMessage::ToggleAlarm => {
            let config = &mut board.lock().await.config;
            config.options.alarm = !config.options.alarm;
        }
        CallWaitMessage::SystemManager => {
            let path = std::env::current_exe()?.with_file_name("icbsm");
            let board_file = board.lock().await.file_name.clone();
            run_board_tool(&path, &board_file)?;
            return Ok(true);
        }
        CallWaitMessage::Setup => {
            let path = std::env::current_exe()?.with_file_name("icbsetup");
            let board_file = board.lock().await.file_name.clone();
            run_board_tool(&path, &board_file)?;
            return Ok(true);
        }
        CallWaitMessage::IcbText => {
            let icbtxt_path = board.lock().await.config.paths.icbtext.clone();
            let icbtxt_path = board.lock().await.resolve_file(&icbtxt_path);

            let path = std::env::current_exe()?.with_file_name("mkicbtxt");
            run_board_tool(&path, &icbtxt_path)?;
            return Ok(true);
        }
        CallWaitMessage::ToggleStatistics => unsafe {
            SHOW_TOTAL_STATS = !SHOW_TOTAL_STATS;
        },
        CallWaitMessage::ShowStatistics => {
            let mut app = SystemStatisticsScreen::new(board).await;
            match app.run(terminal, full_screen, bbs).await {
                Ok(msg) => {
                    if msg == SystemStatisticsScreenMessage::Reset {
                        IcyBoard::write_statistics(board, move |statistics| *statistics = Default::default()).await?;
                    }
                    // just exit
                }
                Err(err) => {
                    log::error!("while running system statistics screen: {}", err);
                    return Err(err);
                }
            }
        }
    }
    Ok(false)
}

fn run_board_tool(path: &std::path::Path, argument: &std::path::Path) -> Res<()> {
    let status = icy_board_tui::term::with_terminal(|| Command::new(path).arg(argument.as_os_str()).status())
        .map_err(|err| format!("Can't run {}: {err}", path.display()))?;
    check_board_tool_status(path, status)
}

fn check_board_tool_status(path: &std::path::Path, status: std::process::ExitStatus) -> Res<()> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with {}", path.display(), status).into())
    }
}

fn init_terminal() -> Res<icy_board_tui::TerminalType> {
    let terminal = icy_board_tui::term::init()?;
    install_panic_hook();
    Ok(terminal)
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("unnamed");
        let location = panic_info
            .location()
            .map(|location| format!("{}:{}:{}", location.file(), location.line(), location.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("panic in thread '{thread}' at {location}: {message}\n{backtrace}");
        ratatui::restore();
        original(panic_info);
    }));
}

pub fn restore_terminal() -> Res<()> {
    Ok(icy_board_tui::term::restore()?)
}

pub fn print_error<A: Display>(error: A) {
    execute!(
        stderr(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(crossterm::style::Color::Red),
        //Print(gettext("error_cmd_line_label")),
        Print("error:"),
        Print(" "),
        SetAttribute(Attribute::Reset),
        SetAttribute(Attribute::Bold),
        Print(error),
        Print("\n"),
        SetAttribute(Attribute::Reset)
    )
    .unwrap();
}

async fn handle_runppe(params: &str) -> Res<CallWaitMessage> {
    // Parse semicolon-separated parameters
    let parts: Vec<&str> = params.split(';').collect();

    if parts.len() < 4 {
        return Err("Insufficient parameters. Format: first;last;PWRD:password;PPE:file.ppe".into());
    }

    let mut first_name = String::new();
    let mut last_name = String::new();

    let mut ppe_params = Vec::new();
    let mut idx = 0;

    // Parse user name (might be 2 or 3 parts for Jr./Sr./III etc.)
    while idx < parts.len() && !parts[idx].to_uppercase().starts_with("PWRD:") {
        if first_name.is_empty() {
            first_name = parts[idx].to_string();
        } else if last_name.is_empty() {
            last_name = parts[idx].to_string();
        } else {
            // Handle suffixes like Jr., Sr., III
            last_name.push(' ');
            last_name.push_str(parts[idx]);
        }
        idx += 1;
    }

    // Parse password
    if idx >= parts.len() || !parts[idx].to_uppercase().starts_with("PWRD:") {
        return Err("Error in Password - missing PWRD: prefix".into());
    }
    let password = parts[idx][5..].to_string();
    idx += 1;

    // Parse PPE file
    if idx >= parts.len() || !parts[idx].to_uppercase().starts_with("PPE:") {
        return Err("PPE Name is missing - missing PPE: prefix".into());
    }
    let ppe_file = PathBuf::from(&parts[idx][4..]);
    idx += 1;

    // Remaining parts are PPE parameters
    while idx < parts.len() {
        ppe_params.push(parts[idx].to_string());
        idx += 1;
    }

    // Validate PPE file exists
    if !ppe_file.exists() {
        return Err(format!("PPE Name is missing - file not found: {}", ppe_file.display()).into());
    }

    let name = if last_name.is_empty() {
        first_name
    } else {
        format!("{} {}", first_name, last_name)
    };

    Ok(CallWaitMessage::RunPPE(ppe_file, Some(name), Some(password), Some(ppe_params)))
}
