pub mod api;
pub mod auth;
pub mod backup;
pub mod dto;
pub mod error;
pub mod service;
pub mod ui;

use std::net::SocketAddr;

use crate::api::{AppState, router};

/// Binding to anything but loopback exposes board administration to the network,
/// so it has to be requested explicitly.
pub fn check_bind_address(addr: &SocketAddr, allow_remote: bool) -> Result<(), String> {
    if addr.ip().is_loopback() || allow_remote {
        Ok(())
    } else {
        Err(format!(
            "refusing to bind to {addr} because it is not a loopback address - pass --allow-remote if that is really intended"
        ))
    }
}

/// Run the admin HTTP server until the listener fails.
pub async fn serve(addr: SocketAddr, state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state).into_make_service_with_connect_info::<SocketAddr>()).await
}

/// Stop accepting and finish every in-flight request before returning. The board
/// event runner must not race a detached live-admin save with its disk command.
pub async fn serve_until(addr: SocketAddr, state: AppState, shutdown: impl std::future::Future<Output = ()> + Send + 'static) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown)
        .await
}
