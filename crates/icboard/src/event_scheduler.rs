//! One scheduler per board/runtime. Main still exclusively owns the stop-services,
//! release-BoardLock, acknowledge, and restart handshake. No Online execution
//! requests that handshake or reloads a live board. Maintenance closes admission
//! and drains according to mode even while an Online job runs, but cannot request
//! service stop/lock release until that foreground task has completed.
//!
//! Startup scans forward from now (no retrospective offline catch-up). Observed
//! occurrences are retained while busy; an explicit end bound expires them.
//! For interval events only the latest due, unclaimed slot survives a backlog.
//! The journal claims each observed scheduled occurrence or manual run before
//! spawn. Pending recovery is interrupted, never retried. Journal errors latch
//! admission closed and require repair/restart, including errors AFTER a command.
//!
//! Commands own independent tasks and journal leases, so cancelling the scheduler
//! does not kill a shell and accidentally leave its descendants running. This is
//! not protection against whole-runtime/process termination or admin commands
//! that daemonize. Main/operator tooling must not start offline tools while
//! event_online_status is present and must await foreground work before shutdown.
//! There is intentionally no forced timeout: warning_minutes only logs Running
//! long (and marks the Online snapshot). A stalled command may hold maintenance
//! indefinitely; its waiting status is visible, and no new Online jobs start.

use std::{
    collections::{HashMap, VecDeque},
    process::Stdio,
    sync::Arc,
};

use chrono::{DateTime, Duration, Local, Utc};
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, BBSMessage, EventMaintenancePhase, EventMaintenanceStatus, OnlineEventStatus},
    events::{
        EventExecution, EventList, EventMode, EventWindow,
        event_history::{EventHistory, EventHistoryEntry, EventResult},
        event_window,
    },
    icb_config::EventOptions,
    icb_text::IceText,
    lock::BoardLock,
};
use tokio::sync::Mutex;

const TICK: std::time::Duration = std::time::Duration::from_secs(1);

fn warning_due(minutes: Option<u32>, elapsed: std::time::Duration) -> bool {
    minutes.is_some_and(|minutes| elapsed >= std::time::Duration::from_secs(u64::from(minutes) * 60))
}

fn publish_status(bbs: &mut BBS, status: Option<EventMaintenanceStatus>) {
    if bbs.event_maintenance_status == status {
        return;
    }
    match &status {
        Some(status) => match &status.phase {
            EventMaintenancePhase::ReloadFailed(error) => log::error!("Event '{}' remains offline: {error}", status.description),
            phase => log::info!("Event '{}': {phase:?}; admission closed", status.description),
        },
        None => log::info!("Event maintenance complete; admission reopened"),
    }
    bbs.event_maintenance_status = status;
}

async fn publish_phase(bbs: &Arc<Mutex<BBS>>, window: &EventWindow, phase: EventMaintenancePhase) {
    publish_status(
        &mut *bbs.lock().await,
        Some(EventMaintenanceStatus {
            description: window.event.description.clone(),
            phase,
        }),
    );
}

/// Retain occurrences, not just a query for the strictly-future next event.
/// The scan watermark survives listener/config restarts. Stable persisted IDs,
/// not file indices, identify retained occurrences. History is the restart barrier.
struct Schedule {
    scanned_until: DateTime<Local>,
    pending: Vec<(String, EventWindow)>,
    manual: VecDeque<EventWindow>,
}

struct Occurrence {
    window: EventWindow,
    manual: bool,
}

impl Schedule {
    fn new(now: DateTime<Local>) -> Self {
        Self {
            scanned_until: now - Duration::nanoseconds(1),
            pending: Vec::new(),
            manual: VecDeque::new(),
        }
    }

    fn refresh(&mut self, options: &EventOptions, events: &EventList, now: DateTime<Local>) {
        self.pending.retain_mut(|(id, window)| {
            let Some(event) = events.iter().find(|event| event.id == *id) else {
                return false;
            };
            if !options.enabled
                || !event.enabled
                || event.time != window.event.time
                || event.days != window.event.days
                || event.interval_minutes != window.event.interval_minutes
                || event.end_time != window.event.end_time
            {
                return false;
            }
            // Commands, descriptions, modes and restrictions may have changed on disk.
            *window = event_window(options, event.clone(), window.run_at);
            true
        });
        if options.enabled {
            for event in events.iter() {
                let mut cursor = self.scanned_until;
                while let Some(run_at) = event.next_occurrence(&cursor) {
                    if !self.pending.iter().any(|(id, window)| *id == event.id && window.run_at == run_at) {
                        self.pending.push((event.id.clone(), event_window(options, event.clone(), run_at)));
                    }
                    if run_at > now {
                        break;
                    }
                    cursor = run_at;
                }
            }
        }
        self.scanned_until = self.scanned_until.max(now);
        // File order is only a tie-breaker, never occurrence/history identity.
        self.pending
            .sort_by_key(|(id, window)| (window.run_at, events.iter().position(|event| event.id == *id)));
    }

    fn windows(&self) -> impl Iterator<Item = &EventWindow> {
        self.manual.iter().chain(self.pending.iter().map(|(_, w)| w))
    }

    fn discard(&mut self, now: DateTime<Local>, online: bool) -> Vec<(Occurrence, EventResult)> {
        let mut latest_due = HashMap::new();
        for (id, window) in &self.pending {
            if window.event.interval_minutes.is_some() && window.run_at <= now {
                latest_due
                    .entry(id.clone())
                    .and_modify(|latest: &mut DateTime<Local>| *latest = (*latest).max(window.run_at))
                    .or_insert(window.run_at);
            }
        }
        let mut discarded = Vec::new();
        self.pending.retain(|(id, w)| {
            let result = if w.event.expired(w.run_at, now) {
                Some(EventResult::Expired)
            } else if online && w.event.mode == EventMode::Idle && w.run_at <= now {
                Some(EventResult::SkippedBusy)
            } else if latest_due.get(id).is_some_and(|latest| w.run_at < *latest) {
                Some(EventResult::Superseded)
            } else {
                None
            };
            if let Some(result) = result {
                discarded.push((
                    Occurrence {
                        window: w.clone(),
                        manual: false,
                    },
                    result,
                ));
                false
            } else {
                true
            }
        });
        self.manual.retain(|w| {
            if online && w.event.mode == EventMode::Idle {
                discarded.push((
                    Occurrence {
                        window: w.clone(),
                        manual: true,
                    },
                    EventResult::SkippedBusy,
                ));
                false
            } else {
                true
            }
        });
        discarded
    }

    #[cfg(test)]
    fn skip_busy_idle(&mut self, now: DateTime<Local>, online: bool) {
        self.discard(now, online);
    }

    fn requires_maintenance(w: &EventWindow, now: DateTime<Local>) -> bool {
        w.event.execution == EventExecution::Maintenance && (w.is_suspended(&now) || w.run_at <= now)
    }

    fn maintenance_window(&self, now: DateTime<Local>) -> Option<&EventWindow> {
        self.windows().find(|w| Self::requires_maintenance(w, now))
    }

    fn forget_claimed(&mut self, history: &EventHistory) {
        self.pending.retain(|(_, w)| !history.contains(&w.event.id, w.run_at.with_timezone(&Utc)));
    }

    fn take_ready(&mut self, now: DateTime<Local>, sessions: bool, job_running: bool) -> Option<Occurrence> {
        if job_running {
            return None;
        }
        let maintenance = self.maintenance_due(now);
        let ready = |w: &EventWindow| {
            w.run_at <= now
                && match w.event.execution {
                    EventExecution::Maintenance => !sessions,
                    EventExecution::Online => !maintenance && (!sessions || w.event.mode == EventMode::Fixed),
                }
        };
        if let Some(index) = self.manual.iter().position(ready) {
            return Some(Occurrence {
                window: self.manual.remove(index).unwrap(),
                manual: true,
            });
        }
        let index = self.pending.iter().position(|(_, w)| ready(w))?;
        Some(Occurrence {
            window: self.pending.remove(index).1,
            manual: false,
        })
    }

    fn maintenance_due(&self, now: DateTime<Local>) -> bool {
        self.maintenance_window(now).is_some()
    }

    fn session_window(&self) -> Option<EventWindow> {
        // A sliding/idle event must not mask a later fixed event's time/upload limits.
        self.windows()
            .filter(|w| w.event.execution == EventExecution::Maintenance && w.event.mode == EventMode::Fixed)
            .min_by_key(|w| w.suspend_at)
            .or_else(|| self.windows().find(|w| w.event.execution == EventExecution::Maintenance))
            .cloned()
    }

    #[cfg(test)]
    fn take_due(&mut self, now: DateTime<Local>) -> Option<EventWindow> {
        let index = self.pending.iter().position(|(_, w)| w.run_at <= now)?;
        Some(self.pending.remove(index).1)
    }
}

/// Spawn exactly once for the lifetime of the shared board/BBS, NOT per listener
/// generation. Main services the two-phase restart handshake at call-wait.
pub async fn run_event_scheduler(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, board_lock: Arc<Mutex<Option<BoardLock>>>) {
    let root = board.lock().await.root_path.clone();
    let history = match EventHistory::open(&root, Utc::now()) {
        Ok(history) => Arc::new(Mutex::new(history)),
        Err(error) => {
            fail_closed(&bbs, error.to_string()).await;
            return;
        }
    };
    // Keep the journal lease even when a runtime error latches the board offline.
    if let Err(error) = scheduler_loop(board, bbs.clone(), board_lock, history.clone()).await {
        fail_closed(&bbs, error.to_string()).await;
    }
}

async fn fail_closed(bbs: &Arc<Mutex<BBS>>, error: String) {
    log::error!("Event scheduler failed closed: {error}; repair and restart required; no command retry");
    {
        let mut bbs = bbs.lock().await;
        bbs.event_maintenance = true;
        bbs.event_scheduler_error = Some(error.clone());
        publish_status(
            &mut bbs,
            Some(EventMaintenanceStatus {
                description: "Event journal/runtime".into(),
                phase: EventMaintenancePhase::ReloadFailed(error),
            }),
        );
    }
    // Never clear restart/admission flags or reload stale in-memory state here.
    loop {
        tokio::time::sleep(TICK).await;
    }
}

struct OnlineJob {
    id: String,
    task: tokio::task::JoinHandle<icy_board_engine::Res<()>>,
}

async fn scheduler_loop(
    board: Arc<Mutex<IcyBoard>>,
    bbs: Arc<Mutex<BBS>>,
    board_lock: Arc<Mutex<Option<BoardLock>>>,
    history: Arc<Mutex<EventHistory>>,
) -> icy_board_engine::Res<()> {
    let mut schedule = Schedule::new(Local::now());
    let mut online_job: Option<OnlineJob> = None;
    loop {
        let now = Local::now();
        if let Some(error) = bbs.lock().await.event_scheduler_error.clone() {
            return Err(error.into());
        }
        if online_job.as_ref().is_some_and(|job| job.task.is_finished()) {
            let job = online_job.take().unwrap();
            job.task.await??;
            let mut bbs = bbs.lock().await;
            if !schedule.manual.iter().any(|w| w.event.id == job.id) {
                bbs.event_active_ids.remove(&job.id);
            }
            bbs.event_online_status = None;
        }
        if bbs.lock().await.operator_maintenance {
            tokio::time::sleep(TICK).await;
            continue;
        }
        let (options, events) = {
            let board = board.lock().await;
            board.events.validate()?;
            (board.config.event.clone(), board.events.clone())
        };
        schedule.refresh(&options, &events, now);
        {
            let mut bbs = bbs.lock().await;
            bbs.event_active_ids.extend(schedule.manual.iter().map(|w| w.event.id.clone()));
            // Accepted manual requests are snapshots. Later editor changes do not
            // silently replace the command an operator explicitly confirmed.
            while let Some(id) = bbs.event_run_requests.pop_front() {
                let error = if bbs.event_active_ids.contains(&id) {
                    Some(format!("Event '{id}' is already active or queued"))
                } else if let Some(event) = events.iter().find(|event| event.id == id) {
                    bbs.event_active_ids.insert(id);
                    schedule.manual.push_back(event_window(&options, event.clone(), now));
                    None
                } else {
                    Some(format!("Unknown event ID '{id}'"))
                };
                if let Some(error) = error {
                    log::warn!("Manual event request refused: {error}");
                    bbs.event_request_error = Some(error);
                }
            }
        }
        schedule.forget_claimed(&*history.lock().await);
        let occurrence = {
            let mut bbs = bbs.lock().await;
            if bbs.operator_maintenance {
                continue;
            }
            bbs.clear_closed_connections().await;
            let sessions = bbs.open_connections.lock().await.iter().flatten().count();
            let online = sessions > 0;
            for (occurrence, result) in schedule.discard(now, online) {
                let mut history = history.lock().await;
                if let Some(entry) = history.claim(&occurrence.window.event, occurrence.window.run_at.with_timezone(&Utc), occurrence.manual)? {
                    history.finish(&entry.key, Utc::now(), result, None, None)?;
                }
                if occurrence.manual {
                    bbs.event_active_ids.remove(&occurrence.window.event.id);
                }
            }
            bbs.event_window = schedule.session_window();
            // Both checking online and closing admission happen under the same BBS
            // lock used by spawn_node. There is no online==0/admission race.
            bbs.event_maintenance = schedule.maintenance_due(now);
            let status = schedule.maintenance_window(now).map(|w| EventMaintenanceStatus {
                description: w.event.description.clone(),
                phase: if online {
                    EventMaintenancePhase::Draining(sessions)
                } else if online_job.is_some() {
                    EventMaintenancePhase::Waiting("Waiting for Online command to finish; board lock retained".into())
                } else {
                    EventMaintenancePhase::Waiting(w.run_at.format("%Y-%m-%d %H:%M:%S").to_string())
                },
            });
            let due = schedule.take_ready(now, online, online_job.is_some());
            if let Some(occurrence) = &due {
                bbs.event_active_ids.insert(occurrence.window.event.id.clone());
                if occurrence.window.event.execution == EventExecution::Maintenance {
                    bbs.event_maintenance = true;
                } else {
                    // Reserve Online work atomically with operator-maintenance
                    // checks, BEFORE releasing BBS to persist/claim or spawn.
                    // Main must inspect this field before entering offline tools.
                    bbs.event_online_status = Some(OnlineEventStatus {
                        event_id: occurrence.window.event.id.clone(),
                        description: occurrence.window.event.description.clone(),
                        started: Utc::now(),
                        log_file: None,
                        running_long: false,
                    });
                }
            }
            // A tied/idle occurrence may start immediately: do not report an
            // admission reopen between commands when the gate never opened.
            if due.is_none() {
                publish_status(&mut bbs, status);
            }
            due
        };
        // Fixed alone drains. Slide waits naturally; Idle was skipped above.
        if schedule.windows().any(|w| w.is_suspended(&now)) {
            clear_the_board(&board, &bbs).await;
        }
        if let Some(occurrence) = occurrence {
            let window = occurrence.window;
            // The durable journal, not an in-memory removal, owns the occurrence.
            let Some(entry) = history
                .lock()
                .await
                .claim(&window.event, window.run_at.with_timezone(&Utc), occurrence.manual)?
            else {
                let mut bbs = bbs.lock().await;
                if !schedule.manual.iter().any(|w| w.event.id == window.event.id) {
                    bbs.event_active_ids.remove(&window.event.id);
                }
                if window.event.execution == EventExecution::Online {
                    bbs.event_online_status = None;
                }
                continue;
            };
            bbs.lock().await.event_active_ids.insert(window.event.id.clone());
            if window.event.execution == EventExecution::Online {
                // Keep an extra BoardLock reference INSIDE the detached-safe task.
                // Even scheduler cancellation cannot release it while the shell runs.
                if board_lock.lock().await.is_none() {
                    return Err("Cannot run Online event without the board lock".into());
                }
                let lease = BoardLock::acquire(history.lock().await.root())?;
                online_job = Some(OnlineJob {
                    id: window.event.id.clone(),
                    task: tokio::spawn(run_event(history.clone(), bbs.clone(), window, entry, Some(lease))),
                });
                tokio::time::sleep(TICK).await;
                continue;
            }
            {
                let mut bbs = bbs.lock().await;
                bbs.event_window = Some(window.clone());
                bbs.event_restart_requested = true;
                publish_status(
                    &mut bbs,
                    Some(EventMaintenanceStatus {
                        description: window.event.description.clone(),
                        phase: EventMaintenancePhase::Stopping,
                    }),
                );
            }
            // Listeners AND the live admin service are stopped and joined before
            // allowing the external command to touch the board's disk state.
            while !bbs.lock().await.event_listeners_stopped {
                tokio::time::sleep(TICK).await;
            }
            // A Slide may have expired while sessions/services were draining.
            // Still reload/restart after main's stop handshake, but never spawn it.
            publish_phase(&bbs, &window, EventMaintenancePhase::Running).await;
            if !occurrence.manual && window.event.expired(window.run_at, Local::now()) {
                history.lock().await.finish(&entry.key, Utc::now(), EventResult::Expired, None, None)?;
            } else {
                // Dropping a JoinHandle detaches, it does not kill a shell and
                // orphan descendants. The task owns its journal lease to completion.
                tokio::spawn(run_event(history.clone(), bbs.clone(), window.clone(), entry, None)).await??;
            }
            publish_phase(&bbs, &window, EventMaintenancePhase::Reloading).await;
            // Never reopen with stale users/config after a failed reload: subsequent
            // saves could overwrite the event's changes. Retry loading, NOT command.
            loop {
                let (root, file) = {
                    let board = board.lock().await;
                    (board.root_path.clone(), board.file_name.clone())
                };
                let mut lock = board_lock.lock().await;
                if lock.is_none() {
                    match BoardLock::acquire(&root) {
                        Ok(acquired) => *lock = Some(acquired),
                        Err(err) => {
                            drop(lock);
                            publish_phase(
                                &bbs,
                                &window,
                                EventMaintenancePhase::ReloadFailed(format!("Cannot lock {}: {err}", root.display())),
                            )
                            .await;
                            tokio::time::sleep(TICK).await;
                            continue;
                        }
                    }
                }
                drop(lock);
                match reload_board(&board).await {
                    Ok(nodes) => {
                        let mut bbs = bbs.lock().await;
                        if bbs.resize_idle_nodes(nodes).await {
                            break;
                        }
                        publish_status(
                            &mut bbs,
                            Some(EventMaintenanceStatus {
                                description: window.event.description.clone(),
                                phase: EventMaintenancePhase::ReloadFailed("A live node still holds board state; waiting for it to finish and save".into()),
                            }),
                        );
                    }
                    Err(err) => {
                        // Permit an offline setup/repair tool to acquire the lock.
                        // Admission and every network service remain stopped.
                        drop(board_lock.lock().await.take());
                        publish_phase(&bbs, &window, EventMaintenancePhase::ReloadFailed(format!("{}: {err}", file.display()))).await;
                    }
                }
                tokio::time::sleep(TICK).await;
            }
            publish_phase(&bbs, &window, EventMaintenancePhase::Restarting).await;
            bbs.lock().await.event_restart_requested = false;
            while bbs.lock().await.event_listeners_stopped {
                tokio::time::sleep(TICK).await;
            }
            if !schedule.manual.iter().any(|w| w.event.id == window.event.id) {
                bbs.lock().await.event_active_ids.remove(&window.event.id);
            }
            // Keep admission closed until the next tick recomputes all tied/pending
            // occurrences. Refresh reads the NEW board; no scheduler restart/replay.
        } else {
            tokio::time::sleep(TICK).await;
        }
    }
}

async fn clear_the_board(board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>) {
    let message = {
        let board = board.lock().await;
        board
            .default_display_text
            .get_display_text(IceText::WaitingForEvent)
            .map(|entry| entry.text.replace('~', " "))
            .unwrap_or_default()
    };
    let channels = bbs.lock().await.bbs_channels.clone();
    for channel in channels.into_iter().flatten() {
        // A busy/full queue cannot deadlock the scheduler or a session needing BBS.
        // Retry next tick; never force-drop a live thread and race its final saves.
        match channel.try_send(BBSMessage::Shutdown(message.clone())) {
            Ok(()) | Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
        }
    }
}

async fn reload_board(board: &Arc<Mutex<IcyBoard>>) -> icy_board_engine::Res<usize> {
    let file = board.lock().await.file_name.clone();
    let mut loaded = IcyBoard::load(&file)?;
    loaded.resolve_paths();
    // Ordinary startup permits a missing event file. Maintenance reload must not
    // silently turn a command-damaged enabled schedule into an empty default.
    if loaded.config.event.enabled && !loaded.config.event.event_file.as_os_str().is_empty() {
        use icy_board_engine::icy_board::IcyBoardSerializer;
        loaded.events = EventList::load(&loaded.config.event.event_file)?;
    }
    let nodes = loaded.config.board.num_nodes as usize;
    // Preserve Arc identity for all consumers, replace every board-owned cache,
    // user/config/conference collection. Session-owned mail handles are gone.
    *board.lock().await = loaded;
    Ok(nodes)
}

/// Own the child to completion. No forced timeout/kill-on-drop: terminating only
/// the shell could leave its descendants modifying data after a misleading finish.
/// Admin commands MUST remain foreground (no daemonization/background children).
async fn run_event(
    history: Arc<Mutex<EventHistory>>,
    bbs: Arc<Mutex<BBS>>,
    window: EventWindow,
    entry: EventHistoryEntry,
    _online_lock: Option<BoardLock>,
) -> icy_board_engine::Res<()> {
    if !entry.manual && window.event.expired(window.run_at, Local::now()) {
        return history.lock().await.finish(&entry.key, Utc::now(), EventResult::Expired, None, None);
    }
    let (root, log_path) = {
        let mut history = history.lock().await;
        let path = history.start(&entry.key, Utc::now())?;
        (history.root().to_path_buf(), path)
    };
    if window.event.execution == EventExecution::Online {
        if let Some(status) = &mut bbs.lock().await.event_online_status {
            status.log_file = Some(log_path.strip_prefix(&root)?.to_string_lossy().into_owned());
        }
    }
    // Includes time spent waiting for the journal/status locks after selection.
    if !entry.manual && window.event.expired(window.run_at, Local::now()) {
        return history.lock().await.finish(&entry.key, Utc::now(), EventResult::Expired, None, None);
    }
    let command = window.event.command.trim();
    log::info!(
        "Running event '{}' ({:?}); output: {}",
        window.event.description,
        window.event.execution,
        log_path.display()
    );
    enum SpawnOutcome {
        Empty,
        Expired,
        Child(tokio::process::Child),
    }
    let spawn = || -> std::io::Result<SpawnOutcome> {
        std::fs::create_dir_all(log_path.parent().unwrap())?;
        if std::fs::symlink_metadata(log_path.parent().unwrap())?.file_type().is_symlink() {
            return Err(std::io::Error::other("event_logs must be a directory within the board root, not a symlink"));
        }
        let mut log_options = std::fs::OpenOptions::new();
        log_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            log_options.mode(0o600);
        }
        let output = log_options.open(&log_path)?;
        let error = output.try_clone()?;
        if !entry.manual && window.event.expired(window.run_at, Local::now()) {
            return Ok(SpawnOutcome::Expired);
        }
        if command.is_empty() {
            return Ok(SpawnOutcome::Empty);
        }
        let mut shell = if cfg!(windows) {
            let mut shell = tokio::process::Command::new("cmd");
            shell.arg("/C");
            shell
        } else {
            let mut shell = tokio::process::Command::new("sh");
            shell.arg("-c");
            shell
        };
        shell
            .arg(command)
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(error))
            .kill_on_drop(false)
            .spawn()
            .map(SpawnOutcome::Child)
    };
    let (result, code, detail) = match spawn() {
        Ok(SpawnOutcome::Child(mut child)) => {
            let started = tokio::time::Instant::now();
            let mut warned = false;
            let status = loop {
                tokio::select! {
                    status = child.wait() => break status,
                    _ = tokio::time::sleep(TICK) => {
                        if !warned && warning_due(window.event.warning_minutes, started.elapsed()) {
                            warned = true;
                            log::warn!("Event '{}' Running long: exceeded {} minutes; no forced timeout; maintenance waits for foreground completion", window.event.description, window.event.warning_minutes.unwrap());
                            if window.event.execution == EventExecution::Online {
                                if let Some(status) = &mut bbs.lock().await.event_online_status { status.running_long = true; }
                            }
                        }
                    }
                }
            };
            match status {
                Ok(status) if status.success() => (EventResult::Success, status.code(), None),
                Ok(status) => (EventResult::NonzeroExit, status.code(), Some(status.to_string())),
                Err(error) => (EventResult::WaitError, None, Some(error.to_string())),
            }
        }
        Ok(SpawnOutcome::Empty) => (EventResult::Success, Some(0), Some("Empty command; no process spawned".into())),
        Ok(SpawnOutcome::Expired) => (EventResult::Expired, None, None),
        Err(error) => (EventResult::SpawnError, None, Some(error.to_string())),
    };
    log::info!(
        "Event '{}' finished: {result:?}, exit {code:?}, {detail:?}; no automatic retry",
        window.event.description
    );
    let wait_failed = result == EventResult::WaitError;
    let persisted = history.lock().await.finish(&entry.key, Utc::now(), result, code, detail);
    if wait_failed {
        // A wait error is not proof that the child stopped touching files.
        // Retain the Online lock and journal even if recording the error failed.
        fail_closed(&bbs, format!("Cannot confirm event process exited; remaining offline; journal: {persisted:?}")).await;
    }
    persisted?;
    if window.event.execution == EventExecution::Online {
        // Also clear status if the scheduler's JoinHandle was cancelled while
        // this independently owned foreground task finished safely.
        let mut bbs = bbs.lock().await;
        bbs.event_active_ids.remove(&window.event.id);
        bbs.event_online_status = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use icy_board_engine::{
        datetime::IcbTime,
        icy_board::{IcyBoardSerializer, events::BoardEvent},
    };

    fn at(hour: u32, minute: u32, second: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2024, 6, 3, hour, minute, second).unwrap()
    }

    fn options() -> EventOptions {
        EventOptions {
            enabled: true,
            suspend_minutes: 10,
            ..Default::default()
        }
    }

    fn events(mode: EventMode) -> EventList {
        let mut events = EventList::default();
        events.push(BoardEvent {
            time: IcbTime::parse("03:00:00"),
            mode,
            ..Default::default()
        });
        events
    }

    #[test]
    fn due_tick_is_retained_and_consumed_exactly_once() {
        let events = events(EventMode::Fixed);
        let mut schedule = Schedule::new(at(2, 49, 0));
        schedule.refresh(&options(), &events, at(2, 49, 0));
        assert!(!schedule.maintenance_due(at(2, 49, 59)));
        assert!(schedule.maintenance_due(at(2, 50, 0)));
        assert!(schedule.take_due(at(2, 59, 59)).is_none());
        schedule.refresh(&options(), &events, at(3, 0, 7));
        assert_eq!(schedule.session_window().unwrap().run_at, at(3, 0, 0));
        assert_eq!(schedule.take_due(at(3, 0, 7)).unwrap().run_at, at(3, 0, 0));
        schedule.refresh(&options(), &events, at(3, 0, 7));
        assert!(schedule.take_due(at(3, 0, 7)).is_none());
        schedule.refresh(&options(), &events, at(3, 2, 0));
        assert!(schedule.take_due(at(3, 2, 0)).is_none());
    }

    #[test]
    fn exact_startup_tick_and_identical_ties_run_in_file_order() {
        let mut events = events(EventMode::Fixed);
        let mut second = events[0].clone();
        second.id = BoardEvent::new_id();
        events.push(second);
        events[0].command = "first".into();
        events[1].command = "second".into();
        let mut schedule = Schedule::new(at(3, 0, 0));
        schedule.refresh(&options(), &events, at(3, 0, 0));
        assert_eq!(schedule.take_due(at(3, 0, 0)).unwrap().event.command, "first");
        // Reload changes a tied event's command without replaying the first event.
        events[1].command = "updated".into();
        schedule.refresh(&options(), &events, at(3, 0, 1));
        assert_eq!(schedule.take_due(at(3, 0, 1)).unwrap().event.command, "updated");
        assert!(schedule.take_due(at(3, 0, 1)).is_none());
    }

    #[test]
    fn slide_waits_without_fixed_restrictions_and_idle_skips_only_once() {
        let mut events = events(EventMode::Slide);
        let mut idle = events[0].clone();
        idle.id = BoardEvent::new_id();
        idle.mode = EventMode::Idle;
        events.push(idle);
        let mut schedule = Schedule::new(at(2, 40, 0));
        schedule.refresh(&options(), &events, at(2, 40, 0));
        assert!(!schedule.maintenance_due(at(2, 50, 0)));
        schedule.refresh(&options(), &events, at(3, 0, 1));
        schedule.skip_busy_idle(at(3, 0, 1), true);
        assert!(schedule.maintenance_due(at(3, 0, 1)));
        assert!(!schedule.session_window().unwrap().is_suspended(&at(3, 0, 1)));
        schedule.refresh(&options(), &events, at(4, 0, 0));
        assert_eq!(schedule.take_due(at(4, 0, 0)).unwrap().event.mode, EventMode::Slide);
        assert!(schedule.take_due(at(4, 0, 0)).is_none());
    }

    #[test]
    fn idle_runs_when_empty_and_slide_does_not_hide_later_fixed_limits() {
        let mut events = events(EventMode::Idle);
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, at(3, 0, 0));
        schedule.skip_busy_idle(at(3, 0, 0), false);
        assert_eq!(schedule.take_due(at(3, 0, 0)).unwrap().event.mode, EventMode::Idle);
        events[0].mode = EventMode::Slide;
        events.push(BoardEvent {
            time: IcbTime::parse("04:00:00"),
            ..Default::default()
        });
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, at(3, 0, 0));
        assert_eq!(schedule.session_window().unwrap().run_at, at(4, 0, 0));
    }

    #[test]
    fn disabled_reload_cancels_pending_and_backward_clock_does_not_replay() {
        let mut schedule = Schedule::new(at(2, 59, 0));
        let events = events(EventMode::Fixed);
        schedule.refresh(&options(), &events, at(3, 0, 0));
        schedule.take_due(at(3, 0, 0)).unwrap();
        schedule.refresh(&options(), &events, at(2, 59, 0));
        assert!(schedule.take_due(at(3, 0, 0)).is_none());
        schedule.refresh(&EventOptions::default(), &events, at(3, 1, 0));
        assert!(schedule.pending.is_empty());
    }

    #[test]
    fn reorder_and_command_edit_keep_occurrence_identity_and_journal_blocks_restart_replay() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = EventHistory::open(dir.path(), at(2, 59, 0).with_timezone(&Utc)).unwrap();
        let mut events = events(EventMode::Fixed);
        let first = events[0].clone();
        events.push(BoardEvent {
            description: "second".into(),
            ..first
        });
        events[1].id = BoardEvent::new_id();
        let first_id = events[0].id.clone();
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, at(3, 0, 0));
        let first = schedule.take_due(at(3, 0, 0)).unwrap();
        history.claim(&first.event, first.run_at.with_timezone(&Utc), false).unwrap().unwrap();
        events.swap(0, 1);
        events[1].command = "edited, not a new occurrence".into();
        schedule.refresh(&options(), &events, at(3, 0, 1));
        schedule.forget_claimed(&history);
        assert_ne!(schedule.take_due(at(3, 0, 1)).unwrap().event.id, first_id);
        assert!(schedule.take_due(at(3, 0, 1)).is_none());
        drop(history);
        let history = EventHistory::open(dir.path(), at(3, 0, 1).with_timezone(&Utc)).unwrap();
        assert_eq!(history.entries()[0].result, EventResult::Interrupted);
        let mut restarted = Schedule::new(at(3, 0, 0));
        restarted.refresh(&options(), &events, at(3, 0, 0));
        restarted.forget_claimed(&history);
        assert_ne!(restarted.take_due(at(3, 0, 0)).unwrap().event.id, first_id);
        assert!(restarted.take_due(at(3, 0, 0)).is_none());
    }

    #[test]
    fn expiry_and_busy_idle_are_journaled_once_manual_overrides_end_bound() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = EventHistory::open(dir.path(), at(3, 0, 0).with_timezone(&Utc)).unwrap();
        let mut events = events(EventMode::Slide);
        events[0].end_time = Some(IcbTime::parse("03:05:00"));
        let first = events[0].clone();
        events.push(BoardEvent {
            mode: EventMode::Idle,
            id: BoardEvent::new_id(),
            ..first
        });
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, at(3, 0, 0));
        let skipped = schedule.discard(at(3, 0, 0), true);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].1, EventResult::SkippedBusy);
        let expired = schedule.discard(at(3, 6, 0), true);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].1, EventResult::Expired);
        for (occurrence, result) in skipped.into_iter().chain(expired) {
            let entry = history
                .claim(&occurrence.window.event, occurrence.window.run_at.with_timezone(&Utc), false)
                .unwrap()
                .unwrap();
            history.finish(&entry.key, at(3, 6, 0).with_timezone(&Utc), result, None, None).unwrap();
        }
        let mut restarted = Schedule::new(at(3, 0, 0));
        restarted.refresh(&options(), &events, at(3, 0, 0));
        restarted.forget_claimed(&history);
        assert!(restarted.take_due(at(3, 6, 0)).is_none());
        schedule.manual.push_back(event_window(&options(), events[0].clone(), at(5, 0, 0)));
        assert!(schedule.discard(at(5, 0, 0), true).is_empty());
        assert!(schedule.take_ready(at(5, 0, 0), true, false).is_none());
        assert!(schedule.take_ready(at(5, 0, 0), false, false).unwrap().manual);
    }

    #[test]
    fn online_policy_and_fixed_priority_remain_responsive_while_a_job_runs() {
        let now = at(3, 0, 0);
        let mut schedule = Schedule::new(now);
        let online = BoardEvent {
            execution: EventExecution::Online,
            ..events(EventMode::Fixed)[0].clone()
        };
        schedule.manual.push_back(event_window(&options(), online.clone(), now));
        assert!(!schedule.maintenance_due(now));
        assert!(schedule.session_window().is_none());
        assert!(schedule.take_ready(now, true, true).is_none(), "one Online foreground at a time");
        assert_eq!(schedule.take_ready(now, true, false).unwrap().window.event.execution, EventExecution::Online);
        schedule.manual.push_back(event_window(&options(), online, now));
        let maintenance = events(EventMode::Fixed);
        schedule.refresh(&options(), &maintenance, now);
        assert!(schedule.maintenance_due(now));
        assert!(schedule.session_window().unwrap().is_suspended(&now));
        assert!(
            schedule.take_ready(now, false, true).is_none(),
            "maintenance must wait for the Online task before lock release"
        );
        assert!(schedule.take_ready(now, true, false).is_none(), "no more Online work while Fixed drains");
        assert_eq!(
            schedule.take_ready(now, false, false).unwrap().window.event.execution,
            EventExecution::Maintenance
        );
    }

    #[test]
    fn manual_fixed_slide_idle_policy_is_independent_of_calendar_and_enabled() {
        for mode in [EventMode::Fixed, EventMode::Slide, EventMode::Idle] {
            let event = BoardEvent {
                enabled: false,
                days: icy_board_engine::datetime::IcbDoW::new(0),
                mode,
                ..Default::default()
            };
            let mut schedule = Schedule::new(at(3, 0, 0));
            schedule.manual.push_back(event_window(&options(), event, at(3, 0, 0)));
            let discarded = schedule.discard(at(3, 0, 0), true);
            if mode == EventMode::Idle {
                assert_eq!(discarded.len(), 1);
                assert_eq!(discarded[0].1, EventResult::SkippedBusy);
            } else {
                assert!(schedule.maintenance_due(at(3, 0, 0)));
                assert_eq!(schedule.session_window().unwrap().is_suspended(&at(3, 0, 0)), mode == EventMode::Fixed);
                assert!(schedule.take_ready(at(3, 0, 0), true, false).is_none());
                assert!(schedule.take_ready(at(3, 0, 0), false, false).unwrap().manual);
            }
        }
    }

    #[test]
    fn warning_threshold_is_optional_positive_and_does_not_overflow() {
        use std::time::Duration;
        assert!(!warning_due(None, Duration::MAX));
        assert!(!warning_due(Some(2), Duration::from_secs(119)));
        assert!(warning_due(Some(2), Duration::from_secs(120)));
        assert!(!warning_due(Some(u32::MAX), Duration::from_secs(u64::from(u32::MAX))));
    }

    #[test]
    fn interval_backlog_stays_bounded_while_a_foreground_job_blocks_execution() {
        for execution in [EventExecution::Maintenance, EventExecution::Online] {
            for mode in [EventMode::Fixed, EventMode::Slide, EventMode::Idle] {
                let mut events = events(mode);
                events[0].execution = execution;
                events[0].interval_minutes = Some(30);
                let mut schedule = Schedule::new(at(2, 59, 0));
                let mut superseded = Vec::new();
                for (hour, minute) in [(3, 0), (3, 30), (4, 0), (4, 30)] {
                    let now = at(hour, minute, 0);
                    schedule.refresh(&options(), &events, now);
                    superseded.extend(schedule.discard(now, false));
                    assert!(schedule.take_ready(now, false, true).is_none());
                    assert_eq!(schedule.pending.iter().filter(|(_, w)| w.run_at <= now).count(), 1);
                    assert!(schedule.pending.len() <= 2, "only latest due and next future slot");
                }
                assert_eq!(superseded.len(), 3);
                assert!(superseded.iter().all(|(o, result)| !o.manual && *result == EventResult::Superseded));
                assert_eq!(schedule.take_ready(at(4, 30, 0), false, false).unwrap().window.run_at, at(4, 30, 0));
                assert!(schedule.take_ready(at(4, 30, 0), false, false).is_none());
                assert_eq!(schedule.pending[0].1.run_at, at(5, 0, 0));
            }
        }
    }

    #[test]
    fn sliding_intervals_keep_latest_due_after_callers_leave_and_future_does_not_supersede_it() {
        for execution in [EventExecution::Maintenance, EventExecution::Online] {
            let mut events = events(EventMode::Slide);
            events[0].execution = execution;
            events[0].interval_minutes = Some(30);
            let mut schedule = Schedule::new(at(2, 59, 0));
            schedule.refresh(&options(), &events, at(3, 0, 0));
            assert!(schedule.discard(at(3, 0, 0), true).is_empty());
            assert!(schedule.take_ready(at(3, 0, 0), true, false).is_none());
            schedule.refresh(&options(), &events, at(4, 45, 0));
            let discarded = schedule.discard(at(4, 45, 0), true);
            assert_eq!(discarded.len(), 3);
            assert!(discarded.iter().all(|(_, result)| *result == EventResult::Superseded));
            assert_eq!(schedule.maintenance_due(at(4, 45, 0)), execution == EventExecution::Maintenance);
            assert!(schedule.take_ready(at(4, 45, 0), true, false).is_none());
            assert_eq!(schedule.take_ready(at(4, 45, 0), false, false).unwrap().window.run_at, at(4, 30, 0));
            assert!(schedule.take_ready(at(4, 59, 59), false, false).is_none());
            assert!(schedule.discard(at(4, 59, 59), false).is_empty());
            assert_eq!(schedule.pending[0].1.run_at, at(5, 0, 0));
        }
    }

    #[test]
    fn coalescing_is_per_id_and_leaves_manual_and_non_interval_runs_alone() {
        let mut events = events(EventMode::Slide);
        events[0].interval_minutes = Some(30);
        let mut second = events[0].clone();
        second.id = BoardEvent::new_id();
        events.push(second);
        let mut daily = events[0].clone();
        daily.id = BoardEvent::new_id();
        daily.interval_minutes = None;
        events.push(daily);
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, at(4, 30, 0));
        schedule.manual.push_back(event_window(&options(), events[0].clone(), at(3, 0, 0)));
        let discarded = schedule.discard(at(4, 30, 0), false);
        assert_eq!(discarded.len(), 6);
        assert!(
            discarded
                .iter()
                .all(|(o, result)| o.window.event.id != events[2].id && *result == EventResult::Superseded)
        );
        assert!(schedule.take_ready(at(4, 30, 0), false, false).unwrap().manual);
        assert_eq!(schedule.take_ready(at(4, 30, 0), false, false).unwrap().window.event.id, events[2].id);
        for event in events.iter().take(2) {
            let occurrence = schedule.take_ready(at(4, 30, 0), false, false).unwrap();
            assert_eq!(occurrence.window.event.id, event.id);
            assert_eq!(occurrence.window.run_at, at(4, 30, 0));
        }
        assert!(schedule.take_ready(at(4, 30, 0), false, false).is_none());

        let now = at(4, 30, 0) + Duration::days(1);
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, now);
        schedule.discard(now, false);
        assert_eq!(
            schedule.pending.iter().filter(|(_, w)| w.event.id == events[2].id && w.run_at <= now).count(),
            2
        );
        assert_eq!(
            schedule.pending.iter().filter(|(_, w)| w.event.id == events[0].id && w.run_at <= now).count(),
            1
        );
    }

    #[test]
    fn interval_expiry_and_busy_idle_keep_their_existing_results() {
        let mut events = events(EventMode::Idle);
        events[0].interval_minutes = Some(30);
        events[0].end_time = Some(IcbTime::parse("04:30:00"));
        for (now, online, expected, count) in [
            (at(4, 30, 0), true, EventResult::SkippedBusy, 4),
            (at(4, 30, 1), true, EventResult::Expired, 4),
            (at(4, 30, 1), false, EventResult::Expired, 4),
            (at(4, 30, 0), false, EventResult::Superseded, 3),
        ] {
            let mut schedule = Schedule::new(at(2, 59, 0));
            schedule.refresh(&options(), &events, now);
            let discarded = schedule.discard(now, online);
            assert_eq!(discarded.len(), count);
            assert!(discarded.iter().all(|(_, result)| *result == expected));
            assert_eq!(schedule.take_ready(now, false, false).is_some(), expected == EventResult::Superseded);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn superseded_slots_are_durable_and_only_latest_command_runs() {
        use icy_board_engine::icy_board::events::event_history::LOG_DIRECTORY;

        let dir = tempfile::tempdir().unwrap();
        let now = at(4, 30, 0);
        let mut history = EventHistory::open(dir.path(), now.with_timezone(&Utc)).unwrap();
        let mut events = events(EventMode::Slide);
        events[0].interval_minutes = Some(30);
        events[0].command = "printf once >> runs".into();
        let mut schedule = Schedule::new(at(2, 59, 0));
        schedule.refresh(&options(), &events, now);
        for (occurrence, result) in schedule.discard(now, false) {
            let entry = history
                .claim(&occurrence.window.event, occurrence.window.run_at.with_timezone(&Utc), false)
                .unwrap()
                .unwrap();
            history.finish(&entry.key, now.with_timezone(&Utc), result, None, None).unwrap();
        }
        drop(history);
        let entries = EventHistory::read_entries(dir.path()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(
            entries
                .iter()
                .all(|e| e.result == EventResult::Superseded && e.start.is_none() && e.log_file.is_none() && e.finish.is_some())
        );
        assert!(!dir.path().join("runs").exists());
        assert!(!dir.path().join(LOG_DIRECTORY).exists());

        let history = Arc::new(Mutex::new(EventHistory::open(dir.path(), now.with_timezone(&Utc)).unwrap()));
        let mut restarted = Schedule::new(at(2, 59, 0));
        restarted.refresh(&options(), &events, now);
        restarted.forget_claimed(&*history.lock().await);
        assert!(restarted.discard(now, false).is_empty());
        let occurrence = restarted.take_ready(now, false, false).unwrap();
        assert_eq!(occurrence.window.run_at, now);
        let entry = history
            .lock()
            .await
            .claim(&occurrence.window.event, now.with_timezone(&Utc), false)
            .unwrap()
            .unwrap();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        run_event(history.clone(), bbs, occurrence.window, entry, None).await.unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("runs")).unwrap(), "once");
        assert_eq!(std::fs::read_dir(dir.path().join(LOG_DIRECTORY)).unwrap().count(), 1);
        drop(history);

        let history = EventHistory::open(dir.path(), now.with_timezone(&Utc)).unwrap();
        assert_eq!(history.entries().len(), 4);
        assert_eq!(history.entries()[3].result, EventResult::Success);
        let mut rolled_back = Schedule::new(at(2, 59, 0));
        rolled_back.refresh(&options(), &events, now);
        rolled_back.forget_claimed(&history);
        assert!(rolled_back.take_ready(now, false, false).is_none());
        assert!(rolled_back.discard(now, false).is_empty());
    }

    async fn wait_bbs(bbs: &Arc<Mutex<BBS>>, predicate: impl Fn(&BBS) -> bool) {
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                if predicate(&*bbs.lock().await) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("scheduler did not reach expected runtime state");
    }

    #[tokio::test]
    async fn real_scheduler_journal_failure_refuses_manual_command_and_admission() {
        let (dir, board) = fixture();
        let event = BoardEvent {
            command: "echo must-not-run > forbidden".into(),
            enabled: false,
            ..Default::default()
        };
        board.lock().await.events.push(event.clone());
        std::fs::write(dir.path().join("event_history.toml"), "broken = [").unwrap();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.request_event_run(event.id).unwrap();
        let lock = Arc::new(Mutex::new(Some(BoardLock::acquire(dir.path()).unwrap())));
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(run_event_scheduler(board, bbs.clone(), lock.clone()));
        wait_bbs(&bbs, |bbs| bbs.event_scheduler_error.is_some()).await;
        assert!(bbs.lock().await.admissions_closed());
        assert!(!bbs.lock().await.event_restart_requested);
        assert!(lock.lock().await.is_some());
        assert!(!dir.path().join("forbidden").exists());
        tasks.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn completion_persistence_failure_never_reloads_or_reopens_stale_board() {
        let (dir, board) = fixture();
        let mut replacement = board.lock().await.config.clone();
        replacement.board.name = "Command changed disk".into();
        replacement.save(&dir.path().join("replacement.toml")).unwrap();
        let original_name = board.lock().await.config.board.name.clone();
        let event = BoardEvent {
            enabled: false,
            command: "cp replacement.toml board.toml; printf once > command-runs; rm event_history.toml; mkdir event_history.toml".into(),
            ..Default::default()
        };
        board.lock().await.events.push(event.clone());
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.request_event_run(event.id).unwrap();
        let lock = Arc::new(Mutex::new(Some(BoardLock::acquire(dir.path()).unwrap())));
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(run_event_scheduler(board.clone(), bbs.clone(), lock.clone()));
        wait_bbs(&bbs, |bbs| bbs.event_restart_requested).await;
        drop(lock.lock().await.take());
        bbs.lock().await.event_listeners_stopped = true;
        wait_bbs(&bbs, |bbs| bbs.event_scheduler_error.is_some()).await;
        assert!(bbs.lock().await.admissions_closed());
        {
            let bbs = bbs.lock().await;
            assert!(bbs.event_restart_requested && bbs.event_listeners_stopped);
        }
        assert_eq!(board.lock().await.config.board.name, original_name);
        assert_eq!(std::fs::read_to_string(dir.path().join("command-runs")).unwrap(), "once");
        assert_eq!(
            icy_board_engine::icy_board::icb_config::IcbConfig::load(&dir.path().join("board.toml"))
                .unwrap()
                .board
                .name,
            "Command changed disk"
        );
        tasks.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runner_log_open_failure_is_recorded_and_start_persistence_failure_spawns_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let event = BoardEvent {
            command: "printf forbidden > forbidden".into(),
            ..Default::default()
        };
        let history = Arc::new(Mutex::new(EventHistory::open(dir.path(), Utc::now()).unwrap()));
        let entry = history.lock().await.claim(&event, Utc::now(), true).unwrap().unwrap();
        std::fs::write(dir.path().join("event_logs"), "not a directory").unwrap();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        run_event(history.clone(), bbs.clone(), event_window(&options(), event.clone(), Local::now()), entry, None)
            .await
            .unwrap();
        assert_eq!(history.lock().await.entries()[0].result, EventResult::SpawnError);
        assert!(history.lock().await.entries()[0].detail.is_some());
        assert!(!dir.path().join("forbidden").exists());
        std::fs::remove_file(dir.path().join("event_logs")).unwrap();
        let entry = history.lock().await.claim(&event, Utc::now(), true).unwrap().unwrap();
        std::fs::remove_file(dir.path().join("event_history.toml")).unwrap();
        std::fs::create_dir(dir.path().join("event_history.toml")).unwrap();
        assert!(
            run_event(history, bbs, event_window(&options(), event, Local::now()), entry, None)
                .await
                .is_err()
        );
        assert!(!dir.path().join("forbidden").exists());
        assert!(!dir.path().join("event_logs").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancelling_scheduler_does_not_kill_online_shell_or_release_its_leases() {
        use std::{fs::OpenOptions, io::Write};
        let (dir, board) = fixture();
        let root = dir.path();
        let event = BoardEvent {
            enabled: false,
            execution: EventExecution::Online,
            command: "mkfifo release; exec 3<>release; printf ready > started; read token <&3; printf finished > finished".into(),
            ..Default::default()
        };
        board.lock().await.events.push(event.clone());
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.request_event_run(event.id.clone()).unwrap();
        let lock = Arc::new(Mutex::new(Some(BoardLock::acquire(root).unwrap())));
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(icy_board_engine::icy_board::lock::LOCK_FILE_NAME))
            .unwrap();
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(run_event_scheduler(board, bbs.clone(), lock.clone()));
        wait_bbs(&bbs, |bbs| bbs.event_online_status.is_some() && root.join("started").exists()).await;
        struct Release(std::fs::File);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = self.0.write_all(b"go\n");
            }
        }
        let release = Release(OpenOptions::new().read(true).write(true).open(root.join("release")).unwrap());
        tasks.shutdown().await;
        drop(lock.lock().await.take());
        assert!(matches!(probe.try_lock(), Err(std::fs::TryLockError::WouldBlock)));
        assert!(EventHistory::open(root, Utc::now()).is_err());
        let entries = EventHistory::read_entries(root).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].result, EventResult::Pending);
        drop(release);
        wait_bbs(&bbs, |bbs| bbs.event_online_status.is_none()).await;
        assert!(!bbs.lock().await.event_active_ids.contains(&event.id));
        assert!(root.join("finished").exists());
        // Let the finishing task drop its leases after publishing its final status.
        let history = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                if let Ok(history) = EventHistory::open(root, Utc::now()) {
                    break history;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(history.entries()[0].result, EventResult::Success);
        probe.try_lock().unwrap();
        probe.unlock().unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn real_online_job_keeps_admission_and_lock_but_fixed_drains_before_stop_handshake() {
        use std::{fs::OpenOptions, io::Write};
        let (dir, board) = fixture();
        let root = dir.path();
        let online = BoardEvent {
            enabled: false, execution: EventExecution::Online,
            command: "mkfifo online-release; exec 3<>online-release; printf stdout; printf stderr >&2; printf ready > online-started; read token <&3; printf done > online-done".into(),
            ..Default::default()
        };
        let maintenance = BoardEvent {
            enabled: false,
            command: "test -f online-done && printf run >> maintenance-runs".into(),
            ..Default::default()
        };
        {
            let mut board = board.lock().await;
            board.events.extend([online.clone(), maintenance.clone()]);
            board.events.save(&root.join("events.toml")).unwrap();
        }
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let (finish_node, receive_finish) = std::sync::mpsc::channel();
        let (shutdown_seen, receive_shutdown) = tokio::sync::oneshot::channel();
        bbs.lock()
            .await
            .spawn_node(icy_net::ConnectionType::Channel, move |_, node| {
                let mut receiver = node.bbs_channel.take().unwrap();
                std::thread::Builder::new().spawn(move || {
                    assert!(matches!(receiver.blocking_recv(), Some(BBSMessage::Shutdown(_))));
                    shutdown_seen.send(()).unwrap();
                    receive_finish.recv_timeout(std::time::Duration::from_secs(15))?;
                    Ok(())
                })
            })
            .await
            .unwrap()
            .unwrap();
        let lock = Arc::new(Mutex::new(Some(BoardLock::acquire(root).unwrap())));
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(icy_board_engine::icy_board::lock::LOCK_FILE_NAME))
            .unwrap();
        bbs.lock().await.request_event_run(online.id.clone()).unwrap();
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(run_event_scheduler(board.clone(), bbs.clone(), lock.clone()));
        wait_bbs(&bbs, |bbs| bbs.event_online_status.is_some() && root.join("online-started").exists()).await;
        // Shell already has a read/write FIFO endpoint before publishing ready.
        // Drop releases it even when a following assertion panics.
        struct Release(std::fs::File);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = self.0.write_all(b"go\n");
            }
        }
        let release = Release(OpenOptions::new().read(true).write(true).open(root.join("online-release")).unwrap());
        assert!(!bbs.lock().await.admissions_closed());
        assert!(!bbs.lock().await.event_restart_requested);
        assert!(matches!(probe.try_lock(), Err(std::fs::TryLockError::WouldBlock)));
        assert!(bbs.lock().await.request_event_run(online.id.clone()).is_err());
        bbs.lock().await.request_event_run(maintenance.id.clone()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(15), receive_shutdown)
            .await
            .unwrap()
            .unwrap();
        assert!(bbs.lock().await.admissions_closed());
        assert!(!bbs.lock().await.event_restart_requested, "main may not stop/release while Online is active");
        assert!(!root.join("maintenance-runs").exists());
        finish_node.send(()).unwrap();
        wait_bbs(&bbs, |bbs| {
            bbs.event_maintenance_status
                .as_ref()
                .is_some_and(|s| matches!(&s.phase, EventMaintenancePhase::Waiting(text) if text.contains("Online")))
        })
        .await;
        assert!(lock.lock().await.is_some());
        drop(release);
        wait_bbs(&bbs, |bbs| bbs.event_restart_requested).await;
        assert!(root.join("online-done").exists());
        // Existing main contract, unchanged: drain services, release, then ack.
        drop(lock.lock().await.take());
        probe.try_lock().unwrap();
        probe.unlock().unwrap();
        bbs.lock().await.event_listeners_stopped = true;
        wait_bbs(&bbs, |bbs| !bbs.event_restart_requested && bbs.event_listeners_stopped).await;
        bbs.lock().await.event_listeners_stopped = false;
        wait_bbs(&bbs, |bbs| !bbs.admissions_closed() && bbs.event_online_status.is_none()).await;
        assert_eq!(std::fs::read_to_string(root.join("maintenance-runs")).unwrap(), "run");
        assert!(lock.lock().await.is_some());
        tasks.shutdown().await;
        let journal = EventHistory::open(root, Utc::now()).unwrap();
        assert_eq!(journal.entries().len(), 2);
        assert!(journal.entries().iter().all(|e| e.manual && e.result == EventResult::Success));
        let entry = journal.entries().iter().find(|e| e.event_id == online.id).unwrap();
        let output = std::fs::read_to_string(root.join(entry.log_file.as_ref().unwrap())).unwrap();
        assert!(output.contains("stdout") && output.contains("stderr"));
    }

    #[tokio::test]
    async fn full_shutdown_queue_never_blocks_the_scheduler() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.create_new_node(icy_net::ConnectionType::Channel).await;
        let channel = bbs.lock().await.bbs_channels[0].clone().unwrap();
        for _ in 0..32 {
            channel.try_send(BBSMessage::Broadcast("full".into())).unwrap();
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), clear_the_board(&board, &bbs))
            .await
            .unwrap();
        assert!(bbs.try_lock().is_ok());
        let nodes = bbs.lock().await.open_connections.clone();
        nodes.lock().await[0].as_mut().unwrap().bbs_channel.as_mut().unwrap().try_recv().unwrap();
        clear_the_board(&board, &bbs).await;
        let mut nodes = nodes.lock().await;
        let receiver = nodes[0].as_mut().unwrap().bbs_channel.as_mut().unwrap();
        for _ in 0..31 {
            receiver.try_recv().unwrap();
        }
        assert!(matches!(receiver.try_recv().unwrap(), BBSMessage::Shutdown(_)));
    }

    fn fixture() -> (tempfile::TempDir, Arc<Mutex<IcyBoard>>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut board = IcyBoard::new();
        board.root_path = root.to_path_buf();
        board.file_name = root.join("board.toml");
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
        board.config.save(&board.file_name).unwrap();
        (dir, Arc::new(Mutex::new(board)))
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scheduler_drains_joins_stops_runs_reloads_and_reopens_once() {
        use std::{fs::OpenOptions, io::Write, time::Duration as StdDuration};

        use icy_board_engine::{datetime::IcbDoW, icy_board::lock::LOCK_FILE_NAME};
        use icy_net::ConnectionType;

        const DEADLINE: StdDuration = StdDuration::from_secs(15);

        async fn phase(bbs: &Arc<Mutex<BBS>>, expected: EventMaintenancePhase) {
            tokio::time::timeout(DEADLINE, async {
                loop {
                    let bbs = bbs.lock().await;
                    if bbs.event_maintenance_status.as_ref().is_some_and(|status| status.phase == expected) {
                        assert!(bbs.admissions_closed());
                        break;
                    }
                    drop(bbs);
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("scheduler never reached {expected:?}"));
        }

        // Observe more than one scheduler tick, not just an instant before it runs.
        async fn remains_blocked(bbs: &Arc<Mutex<BBS>>, marker: &std::path::Path, expected: EventMaintenancePhase) {
            assert!(
                tokio::time::timeout(TICK * 2, async {
                    loop {
                        let mut bbs = bbs.lock().await;
                        assert!(bbs.admissions_closed());
                        assert_eq!(bbs.event_maintenance_status.as_ref().unwrap().phase, expected);
                        assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
                        assert!(!marker.exists(), "command ran before the drain/stop handshake completed");
                        drop(bbs);
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .is_err()
            );
        }

        let (dir, board) = fixture();
        let root = dir.path();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let nodes = bbs.lock().await.open_connections.clone();
        let board_lock = Arc::new(Mutex::new(Some(BoardLock::acquire(root).unwrap())));
        // Unlike BoardLock::acquire, a separate file description does not share
        // this process's refcounted lock. Verify actual release/reacquisition.
        let lock_probe = OpenOptions::new().read(true).write(true).open(root.join(LOCK_FILE_NAME)).unwrap();
        assert!(matches!(lock_probe.try_lock(), Err(std::fs::TryLockError::WouldBlock)));
        let marker = root.join("command-runs");
        let (shutdown_seen, shutdown_received) = tokio::sync::oneshot::channel();
        let (finish_node, finish_received) = std::sync::mpsc::channel();
        let saved = root.join("node-saved");
        assert_eq!(
            bbs.lock()
                .await
                .spawn_node(ConnectionType::Channel, move |_, node| {
                    let mut receiver = node.bbs_channel.take().unwrap();
                    std::thread::Builder::new().name("event-lifecycle-node".into()).spawn(move || {
                        assert!(matches!(receiver.blocking_recv(), Some(BBSMessage::Shutdown(_))));
                        // A closed receiver is not proof that the node finished saving.
                        drop(receiver);
                        shutdown_seen.send(()).unwrap();
                        finish_received.recv_timeout(DEADLINE)?;
                        std::fs::write(saved, "final node save\n")?;
                        Ok(())
                    })
                })
                .await
                .unwrap(),
            Some(0)
        );
        {
            let mut board = board.lock().await;
            board.config.board.num_nodes = 1;
            board.config.event.suspend_minutes = 0;
            board.config.save(&board.file_name).unwrap();
            let mut replacement = board.config.clone();
            replacement.board.name = "Reloaded by lifecycle command".into();
            replacement.board.num_nodes = 3;
            replacement.save(&root.join("replacement.toml")).unwrap();
            // All weekdays allow the near-future occurrence to cross midnight.
            let soon = Local::now() + Duration::seconds(2);
            board.events.push(BoardEvent {
                description: "Integrated lifecycle".into(),
                time: IcbTime::parse(&soon.format("%H:%M:%S").to_string()),
                days: IcbDoW::all(),
                mode: EventMode::Fixed,
                // The FIFO holds the real shell in Running without sleeps or ports.
                // Record every invocation, even one that violates the prerequisites.
                command: "printf 'run\\n' >> command-runs; \
                    test -f node-saved && test -f listeners-stopped && \
                    mkfifo command-release && printf 'started\\n' > command-started && \
                    read release < command-release && test \"$release\" = go && \
                    cp replacement.toml board.toml"
                    .into(),
                ..Default::default()
            });
            board.events.save(&root.join("events.toml")).unwrap();
        }
        // Scheduler cancellation never force-kills a foreground shell. The test
        // releases its FIFO before shutdown; commands must not detach descendants.
        let mut scheduler = tokio::task::JoinSet::new();
        scheduler.spawn(run_event_scheduler(board.clone(), bbs.clone(), board_lock.clone()));
        tokio::time::timeout(DEADLINE, shutdown_received).await.unwrap().unwrap();
        remains_blocked(&bbs, &marker, EventMaintenancePhase::Draining(1)).await;
        assert!(!root.join("node-saved").exists());
        assert!(!nodes.lock().await[0].as_ref().unwrap().handle.as_ref().unwrap().is_finished());
        assert!(!bbs.lock().await.event_restart_requested);

        finish_node.send(()).unwrap();
        phase(&bbs, EventMaintenancePhase::Stopping).await;
        assert_eq!(std::fs::read_to_string(root.join("node-saved")).unwrap(), "final node save\n");
        assert!(nodes.lock().await.iter().all(Option::is_none), "scheduler must reap/join the real node thread");
        assert!(bbs.lock().await.bbs_channels.iter().all(Option::is_none));
        assert!(bbs.lock().await.event_restart_requested);
        assert!(!bbs.lock().await.event_listeners_stopped);
        remains_blocked(&bbs, &marker, EventMaintenancePhase::Stopping).await;

        // Simulated main: listeners/admin joined, then release BoardLock, then ack.
        // Retain only the mutex guard to pause reacquisition at Reloading.
        let mut held_lock = board_lock.lock().await;
        drop(held_lock.take());
        lock_probe
            .try_lock()
            .expect("main must release the actual board file lock before acknowledging stop");
        lock_probe.unlock().unwrap();
        std::fs::write(root.join("listeners-stopped"), "ack\n").unwrap();
        bbs.lock().await.event_listeners_stopped = true;
        phase(&bbs, EventMaintenancePhase::Running).await;
        tokio::time::timeout(DEADLINE, async {
            while !root.join("command-started").exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("shell did not reach its FIFO after observing the final save and listener ack");
        {
            let mut bbs = bbs.lock().await;
            assert!(bbs.admissions_closed());
            assert!(bbs.event_restart_requested && bbs.event_listeners_stopped);
            assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
        }
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "run\n");
        assert_eq!(board.lock().await.config.board.num_nodes, 1);
        // Opening both ends avoids blocking the async test while the shell opens
        // its reader. No helper thread/process is left waiting on the FIFO.
        let mut release = OpenOptions::new().read(true).write(true).open(root.join("command-release")).unwrap();
        release.write_all(b"go\n").unwrap();
        phase(&bbs, EventMaintenancePhase::Reloading).await;
        // Keep a FIFO endpoint alive until the shell consumes the buffered token.
        drop(release);
        assert!(bbs.lock().await.event_restart_requested);
        assert!(bbs.lock().await.event_listeners_stopped);
        assert!(bbs.lock().await.try_create_new_node(ConnectionType::Channel).await.is_none());
        assert_eq!(board.lock().await.config.board.num_nodes, 1, "reload is held behind lock reacquisition");
        drop(held_lock);

        phase(&bbs, EventMaintenancePhase::Restarting).await;
        tokio::time::timeout(DEADLINE, async {
            while bbs.lock().await.event_restart_requested {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("scheduler did not request listener restart");
        assert!(!bbs.lock().await.event_restart_requested);
        assert!(bbs.lock().await.event_listeners_stopped);
        assert!(board_lock.lock().await.is_some());
        assert!(matches!(lock_probe.try_lock(), Err(std::fs::TryLockError::WouldBlock)));
        assert_eq!(board.lock().await.config.board.name, "Reloaded by lifecycle command");
        assert_eq!(board.lock().await.config.board.num_nodes, 3);
        assert!(Arc::ptr_eq(&nodes, &bbs.lock().await.open_connections));
        assert_eq!(nodes.lock().await.len(), 3);
        assert!(nodes.lock().await.iter().all(Option::is_none));
        assert_eq!(bbs.lock().await.bbs_channels.len(), 3);
        assert!(bbs.lock().await.try_create_new_node(ConnectionType::Channel).await.is_none());

        // Simulated main has now restarted its services; only this ack may reopen.
        bbs.lock().await.event_listeners_stopped = false;
        tokio::time::timeout(DEADLINE, async {
            while bbs.lock().await.admissions_closed() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("scheduler did not reopen after restart acknowledgement");
        assert!(
            tokio::time::timeout(TICK * 2, async {
                loop {
                    let bbs = bbs.lock().await;
                    assert!(!bbs.admissions_closed());
                    assert!(!bbs.event_restart_requested && !bbs.event_listeners_stopped);
                    assert!(bbs.event_maintenance_status.is_none());
                    assert!(bbs.event_window.as_ref().unwrap().run_at > Local::now());
                    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "run\n", "reload must not replay the occurrence");
                    drop(bbs);
                    tokio::task::yield_now().await;
                }
            })
            .await
            .is_err()
        );
        scheduler.shutdown().await;
        assert!(scheduler.is_empty());
        assert!(nodes.lock().await.iter().all(Option::is_none));
        drop(board_lock.lock().await.take());
        drop(lock_probe);
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn session_event_cap_is_total_fixed_only_and_prefers_past_retained_window() {
        use icy_board_engine::icy_board::state::IcyBoardState;
        use icy_net::{ConnectionType, channel::ChannelConnection};

        let (dir, board) = fixture();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (_peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs.clone(), board.clone(), nodes, node, Box::new(connection)).await;
        let now = Local::now();
        let options = EventOptions {
            disallow_uploads: true,
            minutes_uploads_disallowed: 10,
            ..options()
        };
        let fixed = events(EventMode::Fixed)[0].clone();
        state.session.login_date = (now - Duration::minutes(20)).with_timezone(&chrono::Utc);
        state.session.time_limit = 120;
        bbs.lock().await.event_window = Some(event_window(&options, fixed.clone(), now + Duration::minutes(30)));
        state.limit_time_for_event().await;
        assert_eq!(state.session.time_limit, 40, "20 elapsed + 20 remaining, not a total allowance of 20");
        assert!(state.session.time_adjusted_for_event);
        state.limit_time_for_event().await;
        assert_eq!(state.session.time_limit, 40, "conference changes must not subtract elapsed time twice");

        for mode in [EventMode::Slide, EventMode::Idle] {
            for limit in [0, 120] {
                state.session.time_limit = limit;
                state.session.time_adjusted_for_event = false;
                bbs.lock().await.event_window = Some(event_window(&options, events(mode)[0].clone(), now + Duration::minutes(30)));
                state.limit_time_for_event().await;
                assert_eq!(state.session.time_limit, limit, "{mode:?} must preserve finite and unlimited allowances");
                assert!(!state.session.time_adjusted_for_event, "{mode:?} must not set the event adjustment flag");
            }
        }

        // A future-only query would return a later occurrence and lose both bans.
        let mut later = fixed.clone();
        later.time = IcbTime::parse(&(now + Duration::minutes(30)).format("%H:%M:%S").to_string());
        {
            let mut board = board.lock().await;
            board.config.event = options.clone();
            board.events.push(later);
        }
        bbs.lock().await.event_window = None;
        let future = state.event_window().await.unwrap();
        assert!(future.run_at > now);
        assert!(!future.is_suspended(&now));
        assert!(!future.uploads_blocked(&now));
        let past = event_window(&options, fixed, now - Duration::minutes(1));
        bbs.lock().await.event_window = Some(past.clone());
        let retained = state.event_window().await.unwrap();
        assert_eq!(retained, past);
        assert!(retained.is_suspended(&now));
        assert!(retained.uploads_blocked(&now));
        drop(state);
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn reload_replaces_shared_board_and_resolves_new_paths_without_saving_stale_state() {
        let (dir, board) = fixture();
        let old_http = board.lock().await.ppl_http_service.clone();
        let file = dir.path().join("board.toml");
        let mut config = icy_board_engine::icy_board::icb_config::IcbConfig::load(&file).unwrap();
        config.board.name = "Changed by event".into();
        config.board.num_nodes = 7;
        config.paths.email_msgbase = "newmail/email".into();
        config.save(&file).unwrap();
        let mut conferences = icy_board_engine::icy_board::conferences::ConferenceBase::default();
        conferences.push(icy_board_engine::icy_board::conferences::Conference {
            name: "New conference".into(),
            ..Default::default()
        });
        conferences.save(&dir.path().join("conferences.toml")).unwrap();
        let mut new_events = events(EventMode::Slide);
        new_events[0].command = "new command from disk".into();
        new_events.save(&dir.path().join("events.toml")).unwrap();
        assert_eq!(reload_board(&board).await.unwrap(), 7);
        let board = board.lock().await;
        assert_eq!(board.config.board.name, "Changed by event");
        assert_eq!(board.config.paths.email_msgbase, dir.path().join("newmail/email"));
        assert!(!Arc::ptr_eq(&old_http, &board.ppl_http_service));
        assert_eq!(board.conferences[0].name, "New conference");
        assert_eq!(board.events[0].command, "new command from disk");
        assert_eq!(icy_board_engine::icy_board::icb_config::IcbConfig::load(&file).unwrap().board.num_nodes, 7);
    }

    #[tokio::test]
    async fn invalid_reload_keeps_old_board_and_never_overwrites_disk() {
        let (dir, board) = fixture();
        board.lock().await.config.board.name = "Keep old in memory".into();
        let file = dir.path().join("board.toml");
        std::fs::write(&file, "not valid TOML [").unwrap();
        assert!(reload_board(&board).await.is_err());
        assert_eq!(board.lock().await.config.board.name, "Keep old in memory");
        assert_eq!(std::fs::read_to_string(file).unwrap(), "not valid TOML [");
    }

    #[tokio::test]
    async fn damaged_enabled_schedule_is_not_silently_replaced_by_defaults() {
        let (dir, board) = fixture();
        std::fs::write(dir.path().join("events.toml"), "invalid = [").unwrap();
        assert!(reload_board(&board).await.is_err());
        assert_eq!(std::fs::read_to_string(dir.path().join("events.toml")).unwrap(), "invalid = [");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn command_uses_board_root_null_input_and_reload_handles_nonzero_exit_changes() {
        let (dir, board) = fixture();
        let mut event = events(EventMode::Fixed)[0].clone();
        // Even a failing command may have changed files; its changes are reloaded.
        let mut config = board.lock().await.config.clone();
        config.board.name = "Written by command".into();
        config.save(&dir.path().join("replacement.toml")).unwrap();
        event.command = "cat > stdin.txt; cp replacement.toml board.toml; exit 7".into();
        let history = Arc::new(Mutex::new(EventHistory::open(dir.path(), Utc::now()).unwrap()));
        let entry = history.lock().await.claim(&event, at(3, 0, 0).with_timezone(&Utc), false).unwrap().unwrap();
        run_event(
            history.clone(),
            Arc::new(Mutex::new(BBS::new(1))),
            event_window(&options(), event, at(3, 0, 0)),
            entry,
            None,
        )
        .await
        .unwrap();
        assert_eq!(history.lock().await.entries()[0].result, EventResult::NonzeroExit);
        assert_eq!(history.lock().await.entries()[0].exit_code, Some(7));
        assert!(std::fs::read(dir.path().join("stdin.txt")).unwrap().is_empty());
        reload_board(&board).await.unwrap();
        assert_eq!(board.lock().await.config.board.name, "Written by command");
    }
}
