//! Prepare every enabled listener before starting any service. A failed attempt
//! drops all bound sockets; maintenance retries listeners, never the command.
use std::{future::Future, net::SocketAddr, path::Path, sync::Arc, time::Duration};

use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard,
        bbs::{BBS, EventMaintenancePhase, EventMaintenanceStatus, ListenerStatus},
    },
};
use tokio::{net::TcpListener, sync::Mutex, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::{WEB_ADMIN_TOKEN_ENV, bbs, node_monitoring_screen::WebAdminInfo};

const RETRY_DELAY: Duration = Duration::from_secs(2);

struct PreparedAdmin {
    listener: TcpListener,
    state: icbadmin::api::AppState,
    info: WebAdminInfo,
    from_env: bool,
}

async fn bind(name: &str, address: &str, port: u16) -> Res<TcpListener> {
    let address = if address.trim().is_empty() { "0.0.0.0" } else { address };
    TcpListener::bind((address, port))
        .await
        .map_err(|err| format!("{name} {address}:{port}: {err}").into())
}

async fn prepare_admin(board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, config_file: &Path) -> Res<Option<PreparedAdmin>> {
    let config = board.lock().await.config.board.web_admin.clone();
    if !config.enabled {
        return Ok(None);
    }
    let addr: SocketAddr = format!("{}:{}", config.address.trim(), config.port)
        .parse()
        .map_err(|err| format!("Web admin {}:{}: {err}", config.address, config.port))?;
    icbadmin::check_bind_address(&addr, config.allow_remote).map_err(|err| format!("Web admin: {err}"))?;
    let backend = icbadmin::service::LiveAdminBackend::with_bbs(config_file, board.clone(), bbs.clone()).map_err(|err| format!("Web admin backend: {err}"))?;
    let listener = TcpListener::bind(addr).await.map_err(|err| format!("Web admin {addr}: {err}"))?;
    if !addr.ip().is_loopback() {
        log::warn!("web admin is listening on a non-loopback address; put a TLS reverse proxy in front of it");
    }
    let (token, from_env) = match std::env::var(WEB_ADMIN_TOKEN_ENV) {
        Ok(token) if !token.trim().is_empty() => (token, true),
        _ => (icbadmin::auth::random_hex(24), false),
    };
    let info = WebAdminInfo {
        url: format!("http://{}/", listener.local_addr()?),
        token: token.clone(),
    };
    let state = icbadmin::api::AppState {
        backend: Arc::new(backend),
        auth: Arc::new(icbadmin::auth::AuthState::new(token)),
    };
    Ok(Some(PreparedAdmin {
        listener,
        state,
        info,
        from_env,
    }))
}

pub(crate) async fn stop_connections(token: &CancellationToken, services: &mut JoinSet<()>) {
    token.cancel();
    while let Some(result) = services.join_next().await {
        if let Err(err) = result {
            log::error!("Listener task failed: {err}");
        }
    }
}

fn spawn_service(
    services: &mut JoinSet<()>,
    name: &'static str,
    bbs: Arc<Mutex<BBS>>,
    token: CancellationToken,
    run: impl Future<Output = Res<()>> + Send + 'static,
) {
    services.spawn(async move {
        // Own the child task while also observing panics, not only Result errors.
        let mut task = JoinSet::new();
        task.spawn(run);
        let result = task.join_next().await.unwrap();
        // Capture completion before waiting for the diagnostics lock.
        let stopped_at = chrono::Utc::now();
        {
            let mut bbs = bbs.lock().await;
            // Unique service names within a generation; start requires the old
            // supervisor set to be drained. Release the lock before latching
            // failure (which takes it again); never join a child while held.
            if let Some(status) = bbs.runtime_listeners.iter_mut().find(|status| status.name == name) {
                status.running = false;
                status.changed_at = stopped_at;
            }
        }
        if !token.is_cancelled() {
            let detail = match result {
                Ok(Ok(())) => "unexpected completion".into(),
                Ok(Err(err)) => err.to_string(),
                Err(err) => format!("task failed: {err}"),
            };
            crate::latch_scheduler_failure(&bbs, format!("{name} listener stopped: {detail}")).await;
        }
    });
}

/// Success means every enabled endpoint is bound and its configuration prepared.
/// No serving task exists on failure; previously prepared sockets drop via RAII.
pub(crate) async fn start_connections(
    bbs: &Arc<Mutex<BBS>>,
    board: &Arc<Mutex<IcyBoard>>,
    config_file: &Path,
    token: CancellationToken,
    services: &mut JoinSet<()>,
) -> Res<Option<WebAdminInfo>> {
    if !services.is_empty() || token.is_cancelled() {
        return Err("Listener start requires a drained service set and a fresh cancellation token".into());
    }
    let config = board.lock().await.config.login_server.clone();
    let telnet = if config.telnet.is_enabled {
        Some(bind("Telnet", &config.telnet.address, config.telnet.port).await?)
    } else {
        None
    };
    let ssh = if config.ssh.is_enabled {
        let address = format!("{}:{}", config.ssh.address, config.ssh.port);
        Some(bbs::ssh::PreparedSsh::bind(config.ssh).await.map_err(|err| format!("SSH {address}: {err}"))?)
    } else {
        None
    };
    let websocket = if config.secure_websocket.is_enabled {
        Some(bind("Secure WebSocket", &config.secure_websocket.address, config.secure_websocket.port).await?)
    } else {
        None
    };
    let admin = prepare_admin(board, bbs, config_file).await?;

    // Query actual addresses before publication. Address-query failures roll
    // back all prepared sockets just like bind failures, without a partial view.
    let mut statuses = Vec::new();
    for (name, address) in [
        ("Telnet", telnet.as_ref().map(TcpListener::local_addr)),
        ("SSH", ssh.as_ref().map(bbs::ssh::PreparedSsh::local_addr)),
        ("Secure WebSocket", websocket.as_ref().map(TcpListener::local_addr)),
        ("Web admin", admin.as_ref().map(|admin| admin.listener.local_addr())),
    ] {
        if let Some(address) = address {
            statuses.push(ListenerStatus {
                name: name.into(),
                address: address.map_err(|error| format!("{name} local address: {error}"))?,
                running: true,
                changed_at: chrono::Utc::now(),
            });
        }
    }

    // Preparation can await DNS/binding. Recheck the sticky safety latch before
    // publishing any services, atomically with failure publication and admission.
    let mut admission = bbs.lock().await;
    if let Some(error) = &admission.event_scheduler_error {
        return Err(format!("Listener start blocked by runtime failure: {error}").into());
    }
    if token.is_cancelled() {
        return Err("Listener start cancelled during preparation".into());
    }
    admission.runtime_listeners = statuses;
    if let Some(listener) = telnet {
        let run = bbs::serve_telnet_connections(listener, board.clone(), bbs.clone());
        let cancel = token.clone();
        spawn_service(services, "Telnet", bbs.clone(), token.clone(), async move {
            tokio::select! { result = run => result, _ = cancel.cancelled() => Ok(()) }
        });
    }
    if let Some(ssh) = ssh {
        let run = ssh.run(board.clone(), bbs.clone(), token.child_token());
        spawn_service(services, "SSH", bbs.clone(), token.clone(), run);
    }
    if let Some(listener) = websocket {
        let run = bbs::serve_securewebsocket_connections(listener, board.clone(), bbs.clone());
        let cancel = token.clone();
        spawn_service(services, "Secure WebSocket", bbs.clone(), token.clone(), async move {
            tokio::select! { result = run => result, _ = cancel.cancelled() => Ok(()) }
        });
    }
    if let Some(admin) = admin {
        log::info!("web admin listening on {}", admin.info.url);
        if admin.from_env {
            log::info!("web admin token taken from {WEB_ADMIN_TOKEN_ENV}");
        } else {
            log::info!("web admin token: {}", admin.info.token);
        }
        let cancel = token.clone();
        spawn_service(services, "Web admin", bbs.clone(), token, async move {
            icbadmin::serve_listener_until(admin.listener, admin.state, cancel.cancelled_owned())
                .await
                .map_err(Into::into)
        });
        return Ok(Some(admin.info));
    }
    Ok(None)
}

/// Called with admission closed and BoardLock retained. The caller owns the
/// restart acknowledgement; never clear it or replay/reload the event here.
pub(crate) async fn restart_connections(
    bbs: &Arc<Mutex<BBS>>,
    board: &Arc<Mutex<IcyBoard>>,
    config_file: &Path,
    token: &mut CancellationToken,
    services: &mut JoinSet<()>,
) -> Option<WebAdminInfo> {
    let previous_status = bbs.lock().await.event_maintenance_status.clone();
    let mut last_error = None;
    loop {
        // A sticky runtime failure must not be mistaken for a bind retry.
        if bbs.lock().await.event_scheduler_error.is_some() {
            tokio::time::sleep(RETRY_DELAY).await;
            continue;
        }
        *token = CancellationToken::new();
        match start_connections(bbs, board, config_file, token.clone(), services).await {
            Ok(admin) => {
                bbs.lock().await.event_maintenance_status = previous_status;
                return admin;
            }
            Err(error) => {
                let error = error.to_string();
                if last_error.as_ref() != Some(&error) {
                    log::error!("Listener restart failed: {error}; admission closed, board lock retained; retrying listeners only");
                    last_error = Some(error.clone());
                }
                bbs.lock().await.event_maintenance_status = Some(EventMaintenanceStatus {
                    description: previous_status
                        .as_ref()
                        .map_or_else(|| icy_board_tui::get_text("event_runtime_title"), |s| s.description.clone()),
                    phase: EventMaintenancePhase::ListenerFailed(error),
                });
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

#[cfg(test)]
mod tests;
