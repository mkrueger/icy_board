//! Listener lifecycle tests using only temporary board data and ephemeral IPv4
//! loopback ports. No environment, current-directory, logger, or panic-hook changes.
//! SSH banner / secure-WebSocket TCP checks are NOT authenticated SSH / TLS tests.

use std::{
    fs::{File, OpenOptions, TryLockError},
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use icy_board_engine::icy_board::{
    IcyBoardSerializer,
    lock::{BoardLock, LOCK_FILE_NAME},
};
use icy_net::ConnectionType;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Instant, sleep, timeout},
};

use super::*;

const TELNET: usize = 0;
const SSH: usize = 1;
const WEBSOCKET: usize = 2;
const ADMIN: usize = 3;
const NAMES: [&str; 4] = ["Telnet", "SSH", "Secure WebSocket", "Web admin"];
const DEADLINE: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(10);

struct Fixture {
    dir: tempfile::TempDir,
    config_file: PathBuf,
    board: Arc<Mutex<IcyBoard>>,
    bbs: Arc<Mutex<BBS>>,
    addresses: [SocketAddr; 4],
    reservations: Vec<Option<TcpListener>>,
}

impl Fixture {
    async fn new(enabled: [bool; 4]) -> Self {
        // Reserve all four simultaneously, so this fixture never selects the same
        // ephemeral port twice. Release only immediately before preparing services.
        let mut reservations = Vec::new();
        let mut addresses = [SocketAddr::from((Ipv4Addr::LOCALHOST, 0)); 4];
        for address in &mut addresses {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
            *address = listener.local_addr().unwrap();
            reservations.push(Some(listener));
        }
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("board.toml");
        let mut board = IcyBoard::new();
        board.root_path = dir.path().to_path_buf();
        board.file_name = config_file.clone();
        board.config.board.num_nodes = 2;
        let login = &mut board.config.login_server;
        login.telnet.is_enabled = enabled[TELNET];
        login.telnet.address = Ipv4Addr::LOCALHOST.to_string();
        login.telnet.port = addresses[TELNET].port();
        login.ssh.is_enabled = enabled[SSH];
        login.ssh.address = Ipv4Addr::LOCALHOST.to_string();
        login.ssh.port = addresses[SSH].port();
        login.secure_websocket.is_enabled = enabled[WEBSOCKET];
        login.secure_websocket.address = Ipv4Addr::LOCALHOST.to_string();
        login.secure_websocket.port = addresses[WEBSOCKET].port();
        let admin = &mut board.config.board.web_admin;
        admin.enabled = enabled[ADMIN];
        admin.address = Ipv4Addr::LOCALHOST.to_string();
        admin.port = addresses[ADMIN].port();
        admin.allow_remote = false;
        board.config.save(&config_file).unwrap();
        let mut bbs = BBS::new(2);
        // Never permit the TCP smoke tests to spawn BBS threads (which install
        // process-wide panic hooks). Admission stays closed until scheduler ack.
        bbs.event_maintenance = true;
        Self {
            dir,
            config_file,
            board: Arc::new(Mutex::new(board)),
            bbs: Arc::new(Mutex::new(bbs)),
            addresses,
            reservations,
        }
    }

    fn release_except(&mut self, conflict: Option<usize>) {
        for (index, reservation) in self.reservations.iter_mut().enumerate() {
            if Some(index) != conflict {
                drop(reservation.take());
            }
        }
    }

    async fn start(&self, generation: &mut Generation) -> Res<Option<WebAdminInfo>> {
        start_connections(&self.bbs, &self.board, &self.config_file, generation.token.clone(), &mut generation.services).await
    }

    async fn assert_rebindable_except(&self, conflict: Option<usize>) {
        // Keep the probes alive together, checking every prepared socket was
        // released, not merely the failed endpoint or an empty JoinSet.
        let mut probes = Vec::new();
        for (index, address) in self.addresses.iter().enumerate() {
            if Some(index) != conflict {
                probes.push(
                    TcpListener::bind(address)
                        .await
                        .unwrap_or_else(|error| panic!("{} socket retained: {error}", NAMES[index])),
                );
            }
        }
    }

    async fn assert_no_nodes(&self) {
        let bbs = self.bbs.lock().await;
        assert!(bbs.open_connections.lock().await.iter().all(Option::is_none));
        assert!(bbs.bbs_channels.iter().all(Option::is_none));
    }

    async fn assert_restart_gate(&self) {
        let mut bbs = self.bbs.lock().await;
        assert!(bbs.event_maintenance);
        assert!(bbs.admissions_closed());
        assert!(bbs.event_listeners_stopped, "only the caller may acknowledge listener restart");
        assert!(!bbs.event_restart_requested);
        assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
        drop(bbs);
        self.assert_no_nodes().await;
    }
}

/// Own tasks throughout timeout/panic paths as well as success. Dropping a plain
/// JoinHandle would detach a retry with live listeners; this JoinSet aborts them.
struct Generation {
    token: CancellationToken,
    services: JoinSet<()>,
}

impl Generation {
    fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            services: JoinSet::new(),
        }
    }

    async fn stop(&mut self) {
        timeout(DEADLINE, stop_connections(&self.token, &mut self.services))
            .await
            .expect("listener tasks did not drain after cancellation");
        assert!(self.token.is_cancelled());
        assert!(self.services.is_empty());
    }
}

impl Drop for Generation {
    fn drop(&mut self) {
        self.token.cancel();
        self.services.abort_all();
    }
}

fn start_error(result: Res<Option<WebAdminInfo>>) -> String {
    match result {
        Err(error) => error.to_string(),
        // WebAdminInfo includes a secret: never Debug-print a successful result.
        Ok(_) => panic!("listener preparation unexpectedly succeeded"),
    }
}

fn lock_probe(root: &Path) -> File {
    // BoardLock::acquire shares an in-process lease. A separate file description
    // instead tests the actual OS lock, including release and reacquisition.
    OpenOptions::new().read(true).write(true).open(root.join(LOCK_FILE_NAME)).unwrap()
}

fn assert_locked(probe: &File) {
    assert!(matches!(probe.try_lock(), Err(TryLockError::WouldBlock)), "board lock must remain held");
}

async fn wait_bbs(bbs: &Arc<Mutex<BBS>>, predicate: impl Fn(&BBS) -> bool) {
    timeout(DEADLINE, async {
        loop {
            if predicate(&*bbs.lock().await) {
                break;
            }
            sleep(POLL).await;
        }
    })
    .await
    .expect("BBS did not reach the expected listener/scheduler state");
}

async fn occupied_endpoint_fails_without_tasks(index: usize) {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(Some(index));
    let mut generation = Generation::new();
    let error = start_error(fixture.start(&mut generation).await);
    assert!(error.contains(NAMES[index]), "wrong service in error: {error}");
    assert!(error.contains(&fixture.addresses[index].port().to_string()), "missing failed port: {error}");
    assert!(generation.services.is_empty(), "no task may spawn before all enabled listeners bind");
    assert!(
        fixture.bbs.lock().await.runtime_listeners.is_empty(),
        "failed preparation must not publish running diagnostics"
    );
    assert!(!generation.token.is_cancelled(), "preparation failure must not poison the caller's token");
    fixture.assert_no_nodes().await;
    // In particular, SSH/admin failures must drop the earlier Telnet listener;
    // a late admin failure must also drop PreparedSsh and secure WebSocket.
    fixture.assert_rebindable_except(Some(index)).await;
    assert!(
        TcpListener::bind(fixture.addresses[index]).await.is_err(),
        "the conflict itself must still exist"
    );
    generation.stop().await;
    fixture.release_except(None);
    fixture.assert_rebindable_except(None).await;
}

#[tokio::test]
async fn occupied_telnet_port_fails_before_spawning() {
    occupied_endpoint_fails_without_tasks(TELNET).await;
}

#[tokio::test]
async fn occupied_ssh_port_releases_prepared_telnet() {
    occupied_endpoint_fails_without_tasks(SSH).await;
}

#[tokio::test]
async fn occupied_secure_websocket_port_rolls_back_earlier_listeners() {
    occupied_endpoint_fails_without_tasks(WEBSOCKET).await;
}

#[tokio::test]
async fn occupied_admin_port_rolls_back_all_three_prepared_protocols() {
    occupied_endpoint_fails_without_tasks(ADMIN).await;
}

#[tokio::test]
async fn invalid_admin_configuration_is_ignored_only_when_disabled() {
    // The wildcard case is rejected by policy before any bind; it never exposes
    // a test listener outside loopback, even briefly.
    for address in ["not-an-ip-address", "0.0.0.0"] {
        let mut fixture = Fixture::new([true, true, true, false]).await;
        fixture.release_except(None);
        fixture.board.lock().await.config.board.web_admin.address = address.into();
        let mut generation = Generation::new();
        assert!(fixture.start(&mut generation).await.unwrap().is_none());
        assert_eq!(generation.services.len(), 3);
        generation.stop().await;
        fixture.assert_rebindable_except(None).await;

        fixture.board.lock().await.config.board.web_admin.enabled = true;
        let mut generation = Generation::new();
        let error = start_error(fixture.start(&mut generation).await);
        assert!(error.contains("Web admin"), "wrong service in error: {error}");
        if address == "0.0.0.0" {
            assert!(error.contains("not a loopback"));
        }
        assert!(generation.services.is_empty());
        fixture.assert_rebindable_except(None).await;
        generation.stop().await;
    }
}

#[tokio::test]
async fn missing_admin_backend_is_an_error_not_a_disabled_service() {
    let mut fixture = Fixture::new([false; 4]).await;
    fixture.release_except(None);
    std::fs::remove_file(&fixture.config_file).unwrap();
    let mut generation = Generation::new();
    assert!(fixture.start(&mut generation).await.unwrap().is_none());
    assert!(generation.services.is_empty());
    fixture.board.lock().await.config.board.web_admin.enabled = true;
    let error = start_error(fixture.start(&mut generation).await);
    assert!(error.contains("Web admin backend"));
    assert!(generation.services.is_empty());
    fixture.assert_rebindable_except(None).await;
    generation.stop().await;
}

#[tokio::test]
async fn start_requires_fresh_token_and_drained_service_set() {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(None);
    let mut cancelled = Generation::new();
    cancelled.token.cancel();
    assert!(start_error(fixture.start(&mut cancelled).await).contains("fresh cancellation token"));
    assert!(cancelled.services.is_empty());
    fixture.assert_rebindable_except(None).await;
    cancelled.stop().await;

    let mut busy = Generation::new();
    let token = busy.token.clone();
    busy.services.spawn(async move { token.cancelled().await });
    assert!(start_error(fixture.start(&mut busy).await).contains("drained service set"));
    assert_eq!(busy.services.len(), 1, "must not disturb the existing task or spawn new listeners");
    assert!(!busy.token.is_cancelled());
    fixture.assert_rebindable_except(None).await;
    busy.stop().await;
}

#[tokio::test]
async fn start_rejects_preexisting_sticky_failure_and_releases_all_prepared_ports() {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(None);
    let status = set_restarting(&fixture).await;
    fixture.bbs.lock().await.event_active_ids.insert("active-command".into());
    crate::latch_scheduler_failure(&fixture.bbs, "sticky runtime failure".into()).await;
    let mut generation = Generation::new();

    let error = start_error(timeout(DEADLINE, fixture.start(&mut generation)).await.unwrap());
    assert!(error.contains("blocked by runtime failure"), "wrong start error: {error}");
    assert!(error.contains("sticky runtime failure"), "missing sticky failure: {error}");
    assert!(generation.services.is_empty(), "a sticky failure must prevent every service spawn");
    assert!(!generation.token.is_cancelled(), "rejection must not poison the fresh token");
    // All four endpoints were enabled and available: rejection must drop every
    // prepared listener, including SSH and the last-prepared admin socket.
    fixture.assert_rebindable_except(None).await;
    fixture.assert_restart_gate().await;
    {
        let bbs = fixture.bbs.lock().await;
        assert_eq!(bbs.event_scheduler_error.as_deref(), Some("sticky runtime failure"));
        assert_eq!(bbs.event_maintenance_status.as_ref(), Some(&status));
        assert_eq!(bbs.event_active_ids.len(), 1);
        assert!(bbs.event_active_ids.contains("active-command"));
    }
    generation.stop().await;
}

async fn http_get(address: SocketAddr, path: &str) -> String {
    timeout(DEADLINE, async {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let request = format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        // Bounded response size; Connection: close also exercises request drain.
        let mut bytes = Vec::new();
        stream.take(256 * 1024).read_to_end(&mut bytes).await.unwrap();
        String::from_utf8(bytes).unwrap()
    })
    .await
    .expect("admin HTTP request timed out")
}

async fn tcp_and_http_smoke_with_closed_admission(fixture: &Fixture) {
    assert!(fixture.bbs.lock().await.admissions_closed());
    for index in [TELNET, WEBSOCKET] {
        timeout(DEADLINE, async {
            let mut stream = TcpStream::connect(fixture.addresses[index]).await.unwrap();
            // The closed admission gate drops these sockets before creating a
            // session or reading any TLS certificate. This is TCP smoke ONLY.
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).await.unwrap(), 0, "{} admitted a gated connection", NAMES[index]);
        })
        .await
        .expect("gated TCP accept did not close the socket");
    }
    timeout(DEADLINE, async {
        let mut stream = TcpStream::connect(fixture.addresses[SSH]).await.unwrap();
        let mut prefix = [0; 4];
        stream.read_exact(&mut prefix).await.unwrap();
        assert_eq!(&prefix, b"SSH-", "SSH transport did not publish its banner");
        // No key exchange/authentication/channel; stop must also drain this
        // generation's incomplete SSH transports without admitting a BBS node.
    })
    .await
    .expect("SSH TCP/banner smoke timed out");
    let health = http_get(fixture.addresses[ADMIN], "/api/health").await;
    assert!(health.starts_with("HTTP/1.1 200 "));
    assert!(health.contains("\"service\":\"icbadmin\""));
    assert!(health.contains("\"status\":\"ok\""));
    let login = http_get(fixture.addresses[ADMIN], "/login").await;
    assert!(login.starts_with("HTTP/1.1 200 "));
    assert!(login.contains("<form"));
    let protected = http_get(fixture.addresses[ADMIN], "/api/overview").await;
    assert!(protected.starts_with("HTTP/1.1 401 "), "protected API must not allow anonymous access");
    fixture.assert_no_nodes().await;
    assert!(fixture.bbs.lock().await.event_scheduler_error.is_none());
}

#[tokio::test]
async fn all_four_listeners_survive_three_start_stop_rebind_cycles_tcp_and_http_only() {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(None);
    let started_at = fixture.bbs.lock().await.started_at;
    for _ in 0..3 {
        let mut generation = Generation::new();
        let before_start = chrono::Utc::now();
        let admin = fixture.start(&mut generation).await.unwrap().expect("admin must be enabled");
        let after_start = chrono::Utc::now();
        assert_eq!(admin.url, format!("http://{}/", fixture.addresses[ADMIN]));
        // Do not print, compare, or depend on a generated/environment admin token.
        assert_eq!(generation.services.len(), 4);
        {
            let bbs = fixture.bbs.lock().await;
            assert_eq!(bbs.started_at, started_at, "listener restarts must preserve uptime");
            assert_eq!(bbs.runtime_listeners.len(), 4);
            for (index, status) in bbs.runtime_listeners.iter().enumerate() {
                assert_eq!(status.name, NAMES[index]);
                assert_eq!(status.address, fixture.addresses[index]);
                assert!(status.running);
                assert!(status.changed_at >= before_start && status.changed_at <= after_start);
            }
        }
        tcp_and_http_smoke_with_closed_admission(&fixture).await;
        // Keep a separate, incomplete SSH transport alive across cancellation.
        // A successful bind after stop alone would not detect a leaked transport.
        let mut pending_ssh = timeout(DEADLINE, TcpStream::connect(fixture.addresses[SSH])).await.unwrap().unwrap();
        let mut prefix = [0; 4];
        timeout(DEADLINE, pending_ssh.read_exact(&mut prefix)).await.unwrap().unwrap();
        assert_eq!(&prefix, b"SSH-");
        let before_stop = chrono::Utc::now();
        generation.stop().await;
        let after_stop = chrono::Utc::now();
        let stopped = fixture.bbs.lock().await.runtime_listeners.clone();
        assert!(stopped.iter().all(|status| status.changed_at >= before_stop && status.changed_at <= after_stop));
        timeout(DEADLINE, async {
            let mut buffer = [0; 512];
            while pending_ssh.read(&mut buffer).await.unwrap() != 0 {}
        })
        .await
        .expect("stopped SSH generation retained an incomplete transport");
        // Stop is idempotent and normal cancellation must not latch a failure.
        generation.stop().await;
        assert_eq!(
            fixture.bbs.lock().await.runtime_listeners,
            stopped,
            "idempotent stop must retain transition timestamps"
        );
        assert!(fixture.bbs.lock().await.event_scheduler_error.is_none());
        assert!(fixture.bbs.lock().await.runtime_listeners.iter().all(|status| !status.running));
        assert!(fixture.bbs.lock().await.event_maintenance);
        fixture.assert_no_nodes().await;
        fixture.assert_rebindable_except(None).await;
    }
}

#[tokio::test]
async fn port_zero_publishes_actual_bound_endpoints_and_failed_restart_keeps_stopped_diagnostics() {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(None);
    {
        let mut board = fixture.board.lock().await;
        board.config.login_server.telnet.port = 0;
        board.config.login_server.ssh.port = 0;
        board.config.login_server.secure_websocket.port = 0;
        board.config.board.web_admin.port = 0;
    }
    let mut generation = Generation::new();
    let admin = timeout(DEADLINE, fixture.start(&mut generation)).await.unwrap().unwrap().unwrap();
    {
        let bbs = fixture.bbs.lock().await;
        assert_eq!(bbs.runtime_listeners.len(), 4);
        for (index, status) in bbs.runtime_listeners.iter().enumerate() {
            assert_eq!(status.name, NAMES[index]);
            assert!(status.running);
            assert!(status.address.ip().is_loopback());
            assert_ne!(status.address.port(), 0, "must publish the allocated port, not configuration");
            fixture.addresses[index] = status.address;
        }
    }
    assert_eq!(admin.url, format!("http://{}/", fixture.addresses[ADMIN]));
    // Proves published addresses actually serve; closed admission still permits
    // listener readiness and must not be inferred from the diagnostics vector.
    tcp_and_http_smoke_with_closed_admission(&fixture).await;
    generation.stop().await;
    let previous = fixture.bbs.lock().await.runtime_listeners.clone();
    assert!(previous.iter().all(|status| !status.running));
    let conflict = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    fixture.board.lock().await.config.board.web_admin.port = conflict.local_addr().unwrap().port();
    let mut next = Generation::new();
    assert!(start_error(fixture.start(&mut next).await).contains("Web admin"));
    assert!(next.services.is_empty());
    assert_eq!(
        fixture.bbs.lock().await.runtime_listeners,
        previous,
        "failed preparation must not publish a partial generation"
    );
    next.stop().await;
}

#[tokio::test]
async fn joined_child_marks_only_matching_listener_stopped_before_failure_latch() {
    for mode in 0..4 {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let changed_at = chrono::Utc::now() - chrono::Duration::seconds(60);
        bbs.lock().await.runtime_listeners = vec![
            ListenerStatus {
                name: "Observed service".into(),
                address,
                running: true,
                changed_at,
            },
            ListenerStatus {
                name: "Other service".into(),
                address,
                running: true,
                changed_at,
            },
        ];
        let mut generation = Generation::new();
        let (finish, ready) = tokio::sync::oneshot::channel();
        spawn_service(
            &mut generation.services,
            "Observed service",
            bbs.clone(),
            generation.token.clone(),
            async move {
                let _listener = listener;
                ready.await.unwrap();
                match mode {
                    0 | 3 => Ok(()),
                    1 => Err("synthetic failure".into()),
                    _ => panic!("synthetic listener panic"),
                }
            },
        );
        assert!(bbs.lock().await.runtime_listeners.iter().all(|status| status.running));
        if mode == 3 {
            generation.token.cancel();
        }
        let before_stop = chrono::Utc::now();
        finish.send(()).unwrap();
        timeout(DEADLINE, generation.services.join_next()).await.unwrap().unwrap().unwrap();
        {
            let bbs = bbs.lock().await;
            assert!(!bbs.runtime_listeners[0].running);
            assert_eq!(bbs.runtime_listeners[0].address, address);
            assert!(bbs.runtime_listeners[0].changed_at >= before_stop);
            assert!(bbs.runtime_listeners[0].changed_at <= chrono::Utc::now());
            assert_eq!(bbs.runtime_listeners[1].changed_at, changed_at);
            assert!(bbs.runtime_listeners[1].running, "another service must not be marked stopped");
            assert_eq!(bbs.event_scheduler_error.is_some(), mode != 3);
            assert_eq!(bbs.admissions_closed(), mode != 3);
        }
        generation.stop().await;
        assert!(
            TcpListener::bind(address).await.is_ok(),
            "child must release its listener before publishing stopped"
        );
    }
}

async fn set_restarting(fixture: &Fixture) -> EventMaintenanceStatus {
    let status = EventMaintenanceStatus {
        description: "Listener restart test".into(),
        phase: EventMaintenancePhase::Restarting,
    };
    let mut bbs = fixture.bbs.lock().await;
    bbs.event_maintenance = true;
    bbs.event_listeners_stopped = true;
    bbs.event_restart_requested = false;
    bbs.event_maintenance_status = Some(status.clone());
    status
}

/// Drive the borrowed retry future with select, never a detached task. On any
/// assertion/timeout it is dropped BEFORE its owning Generation is dropped.
async fn recover_admin_conflict(fixture: &mut Fixture, generation: &mut Generation, probe: &File, command_marker: Option<&Path>) {
    let conflict = fixture.reservations[ADMIN].take().expect("admin port must be occupied");
    let previous = fixture.bbs.lock().await.event_maintenance_status.clone();
    {
        let restart = restart_connections(
            &fixture.bbs,
            &fixture.board,
            &fixture.config_file,
            &mut generation.token,
            &mut generation.services,
        );
        tokio::pin!(restart);
        tokio::select! {
            _ = &mut restart => panic!("restart succeeded despite the occupied admin port"),
            _ = wait_bbs(&fixture.bbs, |bbs| {
                bbs.event_maintenance_status.as_ref().is_some_and(|status| {
                    matches!(&status.phase, EventMaintenancePhase::ListenerFailed(error) if error.contains("Web admin"))
                })
            }) => {}
        }
        fixture.assert_rebindable_except(Some(ADMIN)).await;
        // Observe past another complete retry interval, while actually polling
        // restart, so a one-shot pending future cannot satisfy this regression.
        tokio::select! {
            _ = &mut restart => panic!("blocked retry unexpectedly returned"),
            _ = async {
                let until = Instant::now() + RETRY_DELAY + Duration::from_millis(250);
                while Instant::now() < until {
                    fixture.assert_restart_gate().await;
                    assert_locked(probe);
                    let status = fixture.bbs.lock().await.event_maintenance_status.clone().unwrap();
                    assert_eq!(status.description, previous.as_ref().unwrap().description);
                    assert!(matches!(status.phase, EventMaintenancePhase::ListenerFailed(_)));
                    if let Some(marker) = command_marker {
                        assert_eq!(std::fs::read_to_string(marker).unwrap(), "run\n", "listener retry replayed the command");
                    }
                    sleep(POLL).await;
                }
            } => {}
        }
        drop(conflict);
        let admin = timeout(DEADLINE, &mut restart).await.expect("restart did not recover after releasing the port");
        assert!(admin.is_some());
    }
    assert_eq!(generation.services.len(), 4);
    assert!(!generation.token.is_cancelled());
    assert_eq!(
        fixture.bbs.lock().await.event_maintenance_status,
        previous,
        "restore the caller's phase after recovery"
    );
    fixture.assert_restart_gate().await;
    assert_locked(probe);
    tcp_and_http_smoke_with_closed_admission(fixture).await;
}

#[tokio::test]
async fn restart_retries_with_real_lock_and_gate_retained_then_listens_without_acknowledging() {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(Some(ADMIN));
    let lock = BoardLock::acquire(fixture.dir.path()).unwrap();
    let probe = lock_probe(fixture.dir.path());
    set_restarting(&fixture).await;
    let mut generation = Generation::new();
    generation.token.cancel();
    let old_token = generation.token.clone();
    recover_admin_conflict(&mut fixture, &mut generation, &probe, None).await;
    assert!(old_token.is_cancelled(), "restart must replace, not reuse, the stopped generation's token");
    generation.stop().await;
    fixture.assert_rebindable_except(None).await;
    fixture.assert_restart_gate().await;
    assert_locked(&probe);
    drop(lock);
    probe.try_lock().unwrap();
    probe.unlock().unwrap();
}

async fn sticky_failure_prevents_restart(fail_during_retry: bool) {
    let mut fixture = Fixture::new([true; 4]).await;
    fixture.release_except(if fail_during_retry { Some(ADMIN) } else { None });
    let lock = BoardLock::acquire(fixture.dir.path()).unwrap();
    let probe = lock_probe(fixture.dir.path());
    set_restarting(&fixture).await;
    let conflict = fixture.reservations[ADMIN].take();
    let mut generation = Generation::new();
    if !fail_during_retry {
        crate::latch_scheduler_failure(&fixture.bbs, "sticky runtime failure".into()).await;
    }
    {
        let restart = restart_connections(
            &fixture.bbs,
            &fixture.board,
            &fixture.config_file,
            &mut generation.token,
            &mut generation.services,
        );
        tokio::pin!(restart);
        if fail_during_retry {
            tokio::select! {
                _ = &mut restart => panic!("occupied restart returned before failure could be latched"),
                _ = wait_bbs(&fixture.bbs, |bbs| {
                    bbs.event_maintenance_status.as_ref().is_some_and(|s| matches!(s.phase, EventMaintenancePhase::ListenerFailed(_)))
                }) => {}
            }
            crate::latch_scheduler_failure(&fixture.bbs, "sticky runtime failure".into()).await;
        }
        drop(conflict);
        let status = fixture.bbs.lock().await.event_maintenance_status.clone();
        tokio::select! {
            _ = &mut restart => panic!("sticky failure must prevent reopening even with every port available"),
            _ = async {
                let until = Instant::now() + RETRY_DELAY * 2 + Duration::from_millis(250);
                while Instant::now() < until {
                    fixture.assert_restart_gate().await;
                    assert_locked(&probe);
                    let bbs = fixture.bbs.lock().await;
                    assert_eq!(bbs.event_scheduler_error.as_deref(), Some("sticky runtime failure"));
                    assert_eq!(bbs.event_maintenance_status, status);
                    drop(bbs);
                    fixture.assert_rebindable_except(None).await;
                    sleep(POLL).await;
                }
            } => {}
        }
    }
    assert!(generation.services.is_empty());
    generation.stop().await;
    fixture.assert_restart_gate().await;
    drop(lock);
    probe.try_lock().unwrap();
    probe.unlock().unwrap();
}

#[tokio::test]
async fn restart_never_binds_when_scheduler_failure_is_already_latched() {
    sticky_failure_prevents_restart(false).await;
}

#[tokio::test]
async fn scheduler_failure_latched_during_bind_retry_prevents_recovery() {
    sticky_failure_prevents_restart(true).await;
}

#[tokio::test]
async fn service_supervisor_latches_error_or_unexpected_completion_and_preserves_first_failure() {
    for returns_error in [true, false] {
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let status = EventMaintenanceStatus {
            description: "In-flight maintenance".into(),
            phase: EventMaintenancePhase::Restarting,
        };
        {
            let mut bbs = bbs.lock().await;
            bbs.event_restart_requested = true;
            bbs.event_listeners_stopped = true;
            bbs.event_active_ids.insert("active-command".into());
            bbs.event_maintenance_status = Some(status.clone());
        }
        let mut generation = Generation::new();
        spawn_service(&mut generation.services, "Test service", bbs.clone(), generation.token.clone(), async move {
            if returns_error { Err("synthetic listener error".into()) } else { Ok(()) }
        });
        timeout(DEADLINE, generation.services.join_next()).await.unwrap().unwrap().unwrap();
        let expected = if returns_error {
            "Test service listener stopped: synthetic listener error"
        } else {
            "Test service listener stopped: unexpected completion"
        };
        {
            let bbs = bbs.lock().await;
            assert!(bbs.event_maintenance && bbs.admissions_closed());
            assert_eq!(bbs.event_scheduler_error.as_deref(), Some(expected));
        }
        spawn_service(&mut generation.services, "Later service", bbs.clone(), generation.token.clone(), async {
            Err("later error".into())
        });
        timeout(DEADLINE, generation.services.join_next()).await.unwrap().unwrap().unwrap();
        generation.stop().await;
        let bbs = bbs.lock().await;
        assert_eq!(bbs.event_scheduler_error.as_deref(), Some(expected));
        assert!(bbs.event_restart_requested && bbs.event_listeners_stopped);
        assert!(bbs.event_active_ids.contains("active-command"));
        assert_eq!(bbs.event_maintenance_status.as_ref(), Some(&status));
    }
}

#[tokio::test]
async fn service_supervisor_latches_panic_offline_and_preserves_active_reservations() {
    let bbs = Arc::new(Mutex::new(BBS::new(2)));
    let status = EventMaintenanceStatus {
        description: "In-flight maintenance".into(),
        phase: EventMaintenancePhase::Restarting,
    };
    {
        let mut bbs = bbs.lock().await;
        assert!(!bbs.admissions_closed());
        bbs.event_restart_requested = true;
        bbs.event_listeners_stopped = true;
        bbs.event_active_ids.insert("active-command".into());
        bbs.event_active_ids.insert("another-active-command".into());
        bbs.event_maintenance_status = Some(status.clone());
    }
    let mut generation = Generation::new();
    spawn_service(&mut generation.services, "Panicking service", bbs.clone(), generation.token.clone(), async {
        panic!("synthetic listener panic");
    });
    // The supervisor must finish normally after observing its owned child's
    // panic; a JoinError escaping the supervisor is not a latched failure.
    timeout(DEADLINE, generation.services.join_next()).await.unwrap().unwrap().unwrap();
    assert!(generation.services.is_empty());
    assert!(!generation.token.is_cancelled());
    let first_failure = {
        let mut bbs = bbs.lock().await;
        assert!(bbs.event_maintenance && bbs.admissions_closed());
        assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
        let error = bbs.event_scheduler_error.clone().expect("service panic must latch a runtime failure");
        assert!(error.contains("Panicking service listener stopped: task failed"), "wrong panic error: {error}");
        assert!(error.contains("synthetic listener panic"), "missing panic detail: {error}");
        error
    };
    spawn_service(&mut generation.services, "Later service", bbs.clone(), generation.token.clone(), async {
        Err("later error".into())
    });
    timeout(DEADLINE, generation.services.join_next()).await.unwrap().unwrap().unwrap();
    generation.stop().await;
    let bbs = bbs.lock().await;
    assert!(bbs.event_maintenance && bbs.admissions_closed());
    assert_eq!(bbs.event_scheduler_error.as_deref(), Some(first_failure.as_str()));
    assert!(bbs.event_restart_requested && bbs.event_listeners_stopped);
    assert_eq!(bbs.event_active_ids.len(), 2);
    assert!(bbs.event_active_ids.contains("active-command"));
    assert!(bbs.event_active_ids.contains("another-active-command"));
    assert_eq!(bbs.event_maintenance_status.as_ref(), Some(&status));
    assert!(bbs.open_connections.lock().await.iter().all(Option::is_none));
    assert!(bbs.bbs_channels.iter().all(Option::is_none));
}

#[tokio::test]
async fn service_supervisor_does_not_latch_completion_or_error_after_cancellation() {
    for returns_error in [true, false] {
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let mut generation = Generation::new();
        let cancel = generation.token.clone();
        spawn_service(
            &mut generation.services,
            "Cancelled service",
            bbs.clone(),
            generation.token.clone(),
            async move {
                cancel.cancelled().await;
                if returns_error { Err("shutdown error".into()) } else { Ok(()) }
            },
        );
        generation.stop().await;
        let bbs = bbs.lock().await;
        assert!(bbs.event_scheduler_error.is_none());
        assert!(!bbs.admissions_closed());
        assert!(bbs.event_maintenance_status.is_none());
    }
}

/// Same minimal persisted component set as event_scheduler's fixture, with the
/// loopback listener configuration preserved for the real post-command reload.
#[cfg(unix)]
async fn persist_reload_fixture(fixture: &Fixture) {
    let root = fixture.dir.path();
    let mut board = fixture.board.lock().await;
    let paths = &mut board.config.paths;
    paths.user_file = "users.toml".into();
    paths.conferences = "conferences.toml".into();
    paths.icbtext = "text.toml".into();
    paths.language_file = "languages.toml".into();
    paths.protocol_data_file = "protocols.toml".into();
    paths.pwrd_sec_level_file = "security.toml".into();
    paths.command_file = "commands.toml".into();
    paths.statistics_file = "statistics.toml".into();
    paths.group_file = "groups.toml".into();
    paths.ftn_file = Default::default();
    paths.qwknet_file = Default::default();
    paths.zconnect_file = Default::default();
    paths.email_msgbase = "mail/email".into();
    board.config.event.event_file = "events.toml".into();
    board.config.event.enabled = true;
    board.config.event.suspend_minutes = 0;
    board.users.save(&root.join("users.toml")).unwrap();
    board.conferences.save(&root.join("conferences.toml")).unwrap();
    board.default_display_text.save(&root.join("text.toml")).unwrap();
    board.languages.save(&root.join("languages.toml")).unwrap();
    board.protocols.save(&root.join("protocols.toml")).unwrap();
    board.sec_levels.save(&root.join("security.toml")).unwrap();
    board.commands.save(&root.join("commands.toml")).unwrap();
    board.statistics.save(&root.join("statistics.toml")).unwrap();
    board.groups.save(&root.join("groups.toml")).unwrap();
    board.events.save(&root.join("events.toml")).unwrap();
    board.config.save(&fixture.config_file).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn real_scheduler_restarts_network_after_admin_conflict_without_replaying_manual_command() {
    use icy_board_engine::icy_board::events::{
        BoardEvent, EventExecution,
        event_history::{EventHistory, EventResult},
    };

    let mut fixture = Fixture::new([true; 4]).await;
    let event = BoardEvent {
        enabled: false, // Explicit manual run only: no wall-clock scheduling race.
        description: "Real listener recovery".into(),
        execution: EventExecution::Maintenance,
        command: "printf 'run\\n' >> command-runs".into(),
        ..Default::default()
    };
    fixture.board.lock().await.events.push(event.clone());
    persist_reload_fixture(&fixture).await;
    let persisted_name = fixture.board.lock().await.config.board.name.clone();
    fixture.board.lock().await.config.board.name = format!("{persisted_name} (unsaved test sentinel)");
    fixture.release_except(None);
    let board_lock = Arc::new(Mutex::new(Some(BoardLock::acquire(fixture.dir.path()).unwrap())));
    let probe = lock_probe(fixture.dir.path());
    let marker = fixture.dir.path().join("command-runs");
    let mut generation = Generation::new();
    assert!(fixture.start(&mut generation).await.unwrap().is_some());
    assert_eq!(generation.services.len(), 4);
    tcp_and_http_smoke_with_closed_admission(&fixture).await;
    fixture.bbs.lock().await.request_event_run(event.id.clone()).unwrap();
    let mut scheduler = JoinSet::new();
    scheduler.spawn(crate::event_scheduler::run_event_scheduler(
        fixture.board.clone(),
        fixture.bbs.clone(),
        board_lock.clone(),
    ));
    wait_bbs(&fixture.bbs, |bbs| {
        bbs.event_restart_requested
            && bbs
                .event_maintenance_status
                .as_ref()
                .is_some_and(|s| s.phase == EventMaintenancePhase::Stopping)
    })
    .await;
    assert!(!marker.exists(), "command must wait for main's stop acknowledgement");
    assert_locked(&probe);

    // Simulated main, REAL services: join all four before releasing the actual
    // board lock and acknowledging stop. Occupy admin only after stop completes.
    generation.stop().await;
    fixture.assert_rebindable_except(None).await;
    fixture.reservations[ADMIN] = Some(TcpListener::bind(fixture.addresses[ADMIN]).await.unwrap());
    drop(board_lock.lock().await.take());
    probe.try_lock().expect("stop must release the actual OS lock");
    probe.unlock().unwrap();
    fixture.bbs.lock().await.event_listeners_stopped = true;

    wait_bbs(&fixture.bbs, |bbs| {
        !bbs.event_restart_requested
            && bbs.event_listeners_stopped
            && bbs
                .event_maintenance_status
                .as_ref()
                .is_some_and(|s| s.phase == EventMaintenancePhase::Restarting)
    })
    .await;
    assert!(board_lock.lock().await.is_some());
    assert_locked(&probe);
    assert_eq!(
        fixture.board.lock().await.config.board.name,
        persisted_name,
        "scheduler must really reload the persisted board"
    );
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "run\n");
    let before_retry = EventHistory::read_entries(fixture.dir.path()).unwrap();
    assert_eq!(before_retry.len(), 1);
    assert_eq!(before_retry[0].event_id, event.id);
    assert!(before_retry[0].manual);
    assert_eq!(before_retry[0].execution, EventExecution::Maintenance);
    assert_eq!(before_retry[0].result, EventResult::Success);
    assert_eq!(before_retry[0].exit_code, Some(0));
    assert!(before_retry[0].start.is_some() && before_retry[0].finish.is_some());
    assert!(fixture.bbs.lock().await.event_active_ids.contains(&event.id));

    recover_admin_conflict(&mut fixture, &mut generation, &probe, Some(&marker)).await;
    assert_eq!(EventHistory::read_entries(fixture.dir.path()).unwrap(), before_retry);
    // Listener success is not acknowledgement. The scheduler still waits, the
    // lock remains real, and even a fully listening generation admits no nodes.
    fixture.assert_restart_gate().await;
    assert!(fixture.bbs.lock().await.event_active_ids.contains(&event.id));
    fixture.bbs.lock().await.event_listeners_stopped = false;
    wait_bbs(&fixture.bbs, |bbs| !bbs.admissions_closed() && !bbs.event_active_ids.contains(&event.id)).await;
    let until = Instant::now() + Duration::from_millis(2200);
    while Instant::now() < until {
        let bbs = fixture.bbs.lock().await;
        assert!(!bbs.event_restart_requested && !bbs.event_listeners_stopped && !bbs.admissions_closed());
        assert!(bbs.event_maintenance_status.is_none());
        assert!(bbs.event_scheduler_error.is_none());
        drop(bbs);
        assert_locked(&probe);
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "run\n");
        assert_eq!(EventHistory::read_entries(fixture.dir.path()).unwrap(), before_retry);
        sleep(POLL).await;
    }
    // No protocol clients are opened once admission reopens. Close it again for
    // teardown, stop/join the real listeners, and abort/join the idle scheduler.
    fixture.bbs.lock().await.operator_maintenance = true;
    generation.stop().await;
    scheduler.shutdown().await;
    assert!(scheduler.is_empty());
    fixture.assert_no_nodes().await;
    fixture.assert_rebindable_except(None).await;
    drop(board_lock.lock().await.take());
    probe.try_lock().unwrap();
    probe.unlock().unwrap();
    let history = EventHistory::open(fixture.dir.path(), chrono::Utc::now()).unwrap();
    assert_eq!(
        history.entries(),
        before_retry.as_slice(),
        "scheduler shutdown must release its journal lease without altering success"
    );
}
