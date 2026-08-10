use std::{net::SocketAddr, path::PathBuf, process::exit, sync::Arc};

use argh::FromArgs;
use icbadmin::{
    api::AppState,
    auth::{AuthState, random_hex},
    check_bind_address, serve,
    service::AdminService,
};

const TOKEN_ENV: &str = "ICBADMIN_TOKEN";

/// IcyBoard local web administration. Serves a small admin UI for a board
/// configuration. Binds to localhost only unless --allow-remote is given.
#[derive(FromArgs)]
struct Cli {
    /// address to listen on (default 127.0.0.1:8787)
    #[argh(option, short = 'b', default = "String::from(\"127.0.0.1:8787\")")]
    bind: String,

    /// DANGEROUS: allow binding to a non loopback address. Board administration
    /// will then be reachable from the network - only use behind a TLS reverse proxy
    #[argh(switch)]
    allow_remote: bool,

    /// path/file name of the icyboard.toml configuration file
    #[argh(positional)]
    file: PathBuf,
}

fn main() {
    let arguments: Cli = argh::from_env();

    if let Err(err) = init_logging() {
        eprintln!("Could not initialize logging: {err}");
    }

    let addr: SocketAddr = match arguments.bind.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("Invalid --bind value '{}', expected something like 127.0.0.1:8787", arguments.bind);
            exit(1);
        }
    };

    if let Err(err) = check_bind_address(&addr, arguments.allow_remote) {
        eprintln!("{err}");
        exit(1);
    }

    let service = match AdminService::open(&arguments.file) {
        Ok(service) => Arc::new(service),
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let (token, generated) = match std::env::var(TOKEN_ENV) {
        Ok(token) if !token.trim().is_empty() => (token, false),
        _ => (random_hex(24), true),
    };

    let state = AppState {
        backend: service.clone(),
        auth: Arc::new(AuthState::new(token.clone())),
    };

    println!("IcyBoard admin");
    println!("  board  : {}", service.board_file().display());
    println!("  listen : http://{addr}/");
    if generated {
        println!("  token  : {token}");
        println!("           (set {TOKEN_ENV} to use a fixed token instead)");
    } else {
        println!("  token  : taken from {TOKEN_ENV}");
    }
    if !addr.ip().is_loopback() {
        println!();
        println!("  WARNING: listening on a non loopback address. Anyone who can reach this");
        println!("           port and knows the token can change the board configuration.");
        println!("           Put a TLS terminating reverse proxy in front of it.");
    }
    println!();
    println!("  Note: do not run icbsetup or icbsysmgr against this board at the same time.");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("Could not start async runtime: {err}");
            exit(1);
        }
    };

    if let Err(err) = runtime.block_on(run_server(addr, state)) {
        eprintln!("Server error: {err}");
        exit(1);
    }
}

async fn run_server(addr: SocketAddr, state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    serve(addr, state).await?;
    Ok(())
}

fn init_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("[{}] {} {}", chrono::Local::now().format("%H:%M:%S"), record.level(), message)))
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
