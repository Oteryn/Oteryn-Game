//! Bounded supervision, heartbeats, and diagnostics for background tasks.

use oteryn_error_codes::{Level, Line, PANIC, emit, filter};
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;

const MODULE: &str = "runtime";
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
static GLOBAL: OnceLock<Supervisor> = OnceLock::new();
static WATCHDOG_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn global() -> &'static Supervisor {
    GLOBAL.get_or_init(|| Supervisor::new(4_096))
}

fn start_watchdog() {
    if WATCHDOG_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let supervisor = global().clone();
        tokio::spawn(async move {
            supervisor
                .run_watchdog(Duration::from_secs(5), Duration::from_secs(30))
                .await;
        });
    }
}

/// Spawn a lifecycle-only supervised task without stall detection while preserving its output.
pub fn spawn_lifecycle_supervised<F>(
    name: &'static str,
    trace: Option<String>,
    future: F,
) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    start_watchdog();
    let guard = global().register(name, trace, TaskKind::Async, false);
    tokio::spawn(async move { guard.run_preserving(future).await })
}

/// Spawn a process-supervised task with an explicit progress heartbeat.
pub fn spawn_supervised_with_heartbeat<F, Fut>(
    name: &'static str,
    trace: Option<String>,
    make_future: F,
) -> JoinHandle<Fut::Output>
where
    F: FnOnce(Heartbeat) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    start_watchdog();
    let guard = global().register(name, trace, TaskKind::Async, true);
    let heartbeat = guard.heartbeat();
    tokio::spawn(async move { guard.run_preserving(make_future(heartbeat)).await })
}

/// Run blocking work on Tokio's blocking pool, supervised, while preserving its output and panics.
pub fn spawn_blocking_lifecycle_supervised<F, T>(
    name: &'static str,
    trace: Option<String>,
    operation: F,
) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let guard = global().register(name, trace, TaskKind::Blocking, false);
    tokio::task::spawn_blocking(move || guard.run_blocking_preserving(operation))
}

/// Run work on a dedicated OS thread named `name`, supervised, while preserving its output and
/// panics for [`std::thread::JoinHandle::join`].
///
/// # Errors
///
/// Returns the operating-system error when the thread cannot be created.
pub fn spawn_thread_supervised<F, T>(
    name: &'static str,
    stack_size: usize,
    operation: F,
) -> std::io::Result<std::thread::JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let guard = global().register(name, None, TaskKind::Thread, false);
    std::thread::Builder::new()
        .name(name.to_owned())
        .stack_size(stack_size)
        .spawn(move || guard.run_blocking_preserving(operation))
}

/// Return the process-wide live-task snapshot for operations health output.
#[must_use]
pub fn supervised_tasks() -> Vec<TaskInfo> {
    global().snapshot()
}

/// The execution mechanism used by a supervised task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// An asynchronous Tokio task.
    Async,
    /// Work executed on Tokio's blocking pool.
    Blocking,
    /// Work executed on a dedicated OS thread.
    Thread,
}

impl TaskKind {
    const fn token(self) -> &'static str {
        match self {
            Self::Async => "async",
            Self::Blocking => "blocking",
            Self::Thread => "thread",
        }
    }
}

/// A bounded-registry view of one live task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    /// Static diagnostic name of the task.
    pub name: &'static str,
    /// Optional correlation trace, which must not identify a player.
    pub trace: Option<String>,
    /// Milliseconds since the task was registered.
    pub age_ms: u64,
    /// Milliseconds since the task's latest heartbeat.
    pub since_beat_ms: u64,
    /// How the task is being executed.
    pub kind: TaskKind,
    /// Whether this task opted into progress-heartbeat stall detection.
    pub heartbeat_supervised: bool,
    /// Whether the task declared that its current wait is expected.
    pub expected_idle: bool,
}

impl TaskInfo {
    /// Whether this heartbeat-supervised task has exceeded `stall_after` while active.
    #[must_use]
    pub fn is_stalled(&self, stall_after: Duration) -> bool {
        self.heartbeat_supervised
            && !self.expected_idle
            && Duration::from_millis(self.since_beat_ms) >= stall_after
    }
}

#[derive(Debug)]
struct Entry {
    name: &'static str,
    trace: Option<String>,
    started_at: Instant,
    last_heartbeat: Instant,
    kind: TaskKind,
    heartbeat_supervised: bool,
    expected_idle: u64,
    stalled: bool,
}

type Writer = dyn Fn(&Line<'_>) + Send + Sync;

struct Inner {
    capacity: usize,
    tasks: Mutex<HashMap<u64, Entry>>,
    writer: Arc<Writer>,
    counters: Counters,
}

#[derive(Default)]
struct Counters {
    started: AtomicU64,
    ended_ok: AtomicU64,
    ended_err: AtomicU64,
    ended_cancelled: AtomicU64,
    panicked: AtomicU64,
    stalled: AtomicU64,
}

/// Monotonic task-supervision counters for an operations stats response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorCounters {
    /// Tasks passed to either spawn method.
    pub started: u64,
    /// Tasks that returned success.
    pub ended_ok: u64,
    /// Tasks that returned an error or panicked.
    pub ended_err: u64,
    /// Tasks cancelled before completion.
    pub ended_cancelled: u64,
    /// Panics caught at the task boundary.
    pub panicked: u64,
    /// Stall episodes observed by the watchdog.
    pub stalled: u64,
}

/// A cheap-to-clone owner of a bounded live-task registry.
#[derive(Clone)]
pub struct Supervisor {
    inner: Arc<Inner>,
}

impl Supervisor {
    /// Create a supervisor that retains at most `capacity` live task records.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::with_writer(capacity, |line| emit("oteryn", line))
    }

    fn with_writer(capacity: usize, writer: impl Fn(&Line<'_>) + Send + Sync + 'static) -> Self {
        Self {
            inner: Arc::new(Inner {
                capacity,
                tasks: Mutex::new(HashMap::with_capacity(capacity)),
                writer: Arc::new(writer),
                counters: Counters::default(),
            }),
        }
    }

    /// Spawn a lifecycle-only asynchronous operation returning `Some` only on success.
    pub fn spawn_lifecycle_supervised<F, T, E>(
        &self,
        name: &'static str,
        trace: Option<String>,
        future: F,
    ) -> JoinHandle<Option<T>>
    where
        F: Future<Output = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let guard = self.register(name, trace, TaskKind::Async, false);
        tokio::spawn(async move { guard.run(future).await })
    }

    /// Spawn a supervised asynchronous operation and pass it a heartbeat handle.
    pub fn spawn_supervised_with_heartbeat<F, Fut, T, E>(
        &self,
        name: &'static str,
        trace: Option<String>,
        make_future: F,
    ) -> JoinHandle<Option<T>>
    where
        F: FnOnce(Heartbeat) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let guard = self.register(name, trace, TaskKind::Async, true);
        let heartbeat = guard.heartbeat();
        tokio::spawn(async move { guard.run(make_future(heartbeat)).await })
    }

    /// Spawn supervised synchronous work on Tokio's blocking pool.
    pub fn spawn_blocking_supervised<F, T, E>(
        &self,
        name: &'static str,
        trace: Option<String>,
        operation: F,
    ) -> JoinHandle<Option<T>>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let guard = self.register(name, trace, TaskKind::Blocking, false);
        tokio::spawn(async move {
            match tokio::task::spawn_blocking(operation).await {
                Ok(Ok(value)) => guard.finish("ok", Some(value)),
                Ok(Err(_)) => guard.finish("err", None),
                Err(error) if error.is_panic() => {
                    guard.panic_line(error.into_panic().as_ref());
                    guard.finish("err", None)
                }
                Err(_) => guard.finish("cancelled", None),
            }
        })
    }

    /// Return a bounded point-in-time view of all registered live tasks.
    #[must_use]
    pub fn snapshot(&self) -> Vec<TaskInfo> {
        let now = Instant::now();
        let mut snapshot: Vec<_> = lock(&self.inner.tasks)
            .values()
            .map(|entry| TaskInfo {
                name: entry.name,
                trace: entry.trace.clone(),
                age_ms: elapsed_ms(entry.started_at, now),
                since_beat_ms: elapsed_ms(entry.last_heartbeat, now),
                kind: entry.kind,
                heartbeat_supervised: entry.heartbeat_supervised,
                expected_idle: entry.expected_idle != 0,
            })
            .collect();
        snapshot.sort_unstable_by_key(|task| task.name);
        snapshot
    }

    /// Return the current monotonic task counters.
    #[must_use]
    pub fn counters(&self) -> SupervisorCounters {
        let counters = &self.inner.counters;
        SupervisorCounters {
            started: counters.started.load(Ordering::Relaxed),
            ended_ok: counters.ended_ok.load(Ordering::Relaxed),
            ended_err: counters.ended_err.load(Ordering::Relaxed),
            ended_cancelled: counters.ended_cancelled.load(Ordering::Relaxed),
            panicked: counters.panicked.load(Ordering::Relaxed),
            stalled: counters.stalled.load(Ordering::Relaxed),
        }
    }

    /// Repeatedly inspect heartbeats and report each stall and recovery episode once.
    pub async fn run_watchdog(&self, period: Duration, stall_after: Duration) {
        loop {
            tokio::time::sleep(period).await;
            let now = Instant::now();
            let mut lines = Vec::new();
            for entry in lock(&self.inner.tasks).values_mut() {
                let is_stalled = entry.heartbeat_supervised
                    && entry.expected_idle == 0
                    && now.duration_since(entry.last_heartbeat) >= stall_after;
                if is_stalled != entry.stalled {
                    entry.stalled = is_stalled;
                    if is_stalled {
                        self.inner.counters.stalled.fetch_add(1, Ordering::Relaxed);
                    }
                    lines.push((
                        if is_stalled {
                            "task_stalled"
                        } else {
                            "task_recovered"
                        },
                        entry.name,
                        entry.trace.clone(),
                        elapsed_ms(entry.last_heartbeat, now),
                    ));
                }
            }
            for (event, name, trace, since_beat_ms) in lines {
                self.write(
                    Level::Warn,
                    event,
                    trace.as_deref(),
                    None,
                    format_args!("name={name} since_beat_ms={since_beat_ms}"),
                );
            }
        }
    }

    fn register(
        &self,
        name: &'static str,
        trace: Option<String>,
        kind: TaskKind,
        heartbeat_supervised: bool,
    ) -> TaskGuard {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
        self.inner.counters.started.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();
        let mut tasks = lock(&self.inner.tasks);
        let registered = tasks.len() < self.inner.capacity;
        if registered {
            tasks.insert(
                id,
                Entry {
                    name,
                    trace: trace.clone(),
                    started_at: now,
                    last_heartbeat: now,
                    kind,
                    heartbeat_supervised,
                    expected_idle: 0,
                    stalled: false,
                },
            );
        }
        drop(tasks);
        self.write(
            Level::Debug,
            "task_start",
            trace.as_deref(),
            None,
            format_args!("name={name} kind={}", kind.token()),
        );
        TaskGuard {
            supervisor: self.clone(),
            id,
            name,
            trace,
            started_at: now,
            registered,
            finished: false,
        }
    }

    fn write(
        &self,
        level: Level,
        event: &str,
        trace: Option<&str>,
        code: Option<oteryn_error_codes::ErrorCode>,
        detail: std::fmt::Arguments<'_>,
    ) {
        if !filter().allows(level, MODULE, code.is_some()) {
            return;
        }
        let detail = detail.to_string();
        let mut line = Line::new(level, MODULE, event).detail(&detail);
        if let Some(trace) = trace {
            line = line.trace(trace);
        }
        if let Some(code) = code {
            line = line.code(code);
        }
        (self.inner.writer)(&line);
    }
}

/// RAII state that guarantees one terminal line when a supervised future is dropped.
pub struct TaskGuard {
    supervisor: Supervisor,
    id: u64,
    name: &'static str,
    trace: Option<String>,
    started_at: Instant,
    registered: bool,
    finished: bool,
}

impl TaskGuard {
    /// Obtain a cloneable heartbeat handle for a long-lived task loop.
    #[must_use]
    pub fn heartbeat(&self) -> Heartbeat {
        Heartbeat {
            inner: Arc::downgrade(&self.supervisor.inner),
            id: self.id,
        }
    }

    async fn run<F, T, E>(self, future: F) -> Option<T>
    where
        F: Future<Output = Result<T, E>>,
    {
        match (CatchUnwind {
            future: Box::pin(future),
        })
        .await
        {
            Ok(Ok(value)) => self.finish("ok", Some(value)),
            Ok(Err(_)) => self.finish("err", None),
            Err(payload) => {
                self.panic_line(payload.as_ref());
                self.finish("err", None)
            }
        }
    }

    async fn run_preserving<F: Future>(self, future: F) -> F::Output {
        match (CatchUnwind {
            future: Box::pin(future),
        })
        .await
        {
            Ok(value) => {
                self.finish_preserving("ok");
                value
            }
            Err(payload) => {
                self.panic_line(payload.as_ref());
                self.finish_preserving("err");
                std::panic::resume_unwind(payload)
            }
        }
    }

    fn run_blocking_preserving<F: FnOnce() -> T, T>(self, operation: F) -> T {
        match std::panic::catch_unwind(AssertUnwindSafe(operation)) {
            Ok(value) => {
                self.finish_preserving("ok");
                value
            }
            Err(payload) => {
                self.panic_line(payload.as_ref());
                self.finish_preserving("err");
                std::panic::resume_unwind(payload)
            }
        }
    }

    fn finish_preserving(mut self, outcome: &str) {
        self.finished = true;
        self.write_end(outcome);
    }

    fn panic_line(&self, payload: &(dyn Any + Send)) {
        self.supervisor
            .inner
            .counters
            .panicked
            .fetch_add(1, Ordering::Relaxed);
        let payload = payload
            .downcast_ref::<&'static str>()
            .copied()
            .unwrap_or("formatted");
        self.supervisor.write(
            Level::Error,
            "task_panic",
            self.trace.as_deref(),
            Some(PANIC),
            format_args!("task={} payload={payload}", self.name),
        );
    }

    fn finish<T>(mut self, outcome: &str, value: Option<T>) -> Option<T> {
        self.finished = true;
        self.write_end(outcome);
        value
    }

    fn write_end(&self, outcome: &str) {
        let counter = match outcome {
            "ok" => &self.supervisor.inner.counters.ended_ok,
            "err" => &self.supervisor.inner.counters.ended_err,
            _ => &self.supervisor.inner.counters.ended_cancelled,
        };
        counter.fetch_add(1, Ordering::Relaxed);
        let level = if outcome == "ok" {
            Level::Info
        } else {
            Level::Warn
        };
        self.supervisor.write(
            level,
            "task_end",
            self.trace.as_deref(),
            None,
            format_args!(
                "name={} outcome={outcome} ms={}",
                self.name,
                elapsed_ms(self.started_at, Instant::now())
            ),
        );
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if self.registered {
            lock(&self.supervisor.inner.tasks).remove(&self.id);
        }
        if !self.finished {
            self.write_end("cancelled");
        }
    }
}

/// A cloneable handle that refreshes a supervised task's liveness timestamp.
#[derive(Clone)]
pub struct Heartbeat {
    inner: std::sync::Weak<Inner>,
    id: u64,
}

impl Heartbeat {
    /// Record current liveness; this is a no-op after the task has ended.
    pub fn beat(&self) {
        if let Some(inner) = self.inner.upgrade()
            && let Some(entry) = lock(&inner.tasks).get_mut(&self.id)
        {
            entry.last_heartbeat = Instant::now();
        }
    }

    /// Mark a deliberate idle wait as exempt from stall detection until the guard is dropped.
    #[must_use]
    pub fn expected_idle(&self) -> ExpectedIdle {
        if let Some(inner) = self.inner.upgrade()
            && let Some(entry) = lock(&inner.tasks).get_mut(&self.id)
        {
            entry.expected_idle = entry.expected_idle.saturating_add(1);
        }
        ExpectedIdle {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

/// A scoped exemption for an intentional idle wait in a heartbeat-supervised task.
pub struct ExpectedIdle {
    inner: std::sync::Weak<Inner>,
    id: u64,
}

impl Drop for ExpectedIdle {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade()
            && let Some(entry) = lock(&inner.tasks).get_mut(&self.id)
        {
            entry.expected_idle = entry.expected_idle.saturating_sub(1);
            entry.last_heartbeat = Instant::now();
        }
    }
}

struct CatchUnwind<F> {
    future: Pin<Box<F>>,
}

impl<F: Future> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Box<dyn Any + Send>>;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match std::panic::catch_unwind(AssertUnwindSafe(|| self.future.as_mut().poll(context))) {
            Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload)),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn elapsed_ms(start: Instant, end: Instant) -> u64 {
    u64::try_from(end.duration_since(start).as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn fixture(capacity: usize) -> (Supervisor, Arc<Mutex<Vec<String>>>) {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let output = Arc::clone(&lines);
        let supervisor = Supervisor::with_writer(capacity, move |line| {
            lock(&output).push(line.render("oteryn", oteryn_error_codes::unix_ms()))
        });
        (supervisor, lines)
    }

    fn matching(lines: &Mutex<Vec<String>>, event: &str) -> Vec<String> {
        lock(lines)
            .iter()
            .filter(|line| line.contains(&format!("event={event}")))
            .cloned()
            .collect()
    }

    #[test]
    fn default_writer_emits_structured_terminal_line() {
        let supervisor = Supervisor::new(1);
        let lines = oteryn_error_codes::capture(|| {
            let _guard = supervisor.register("captured", None, TaskKind::Async, false);
        });
        assert!(lines.iter().any(|line| {
            line.contains("module=runtime event=task_end")
                && line.contains("name=captured")
                && line.contains("outcome=cancelled")
        }));
    }

    #[tokio::test]
    async fn ok_err_cancel_and_panic_each_write_one_end_line() {
        let (supervisor, lines) = fixture(8);
        assert_eq!(
            supervisor
                .spawn_lifecycle_supervised("ok", None, async { Ok::<_, ()>(7) })
                .await
                .unwrap(),
            Some(7)
        );
        assert_eq!(
            supervisor
                .spawn_lifecycle_supervised("err", None, async { Err::<(), _>(()) })
                .await
                .unwrap(),
            None
        );

        let cancelled = supervisor.spawn_lifecycle_supervised("cancel", None, async {
            std::future::pending::<()>().await;
            Ok::<(), ()>(())
        });
        tokio::task::yield_now().await;
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());

        let panic =
            supervisor.spawn_lifecycle_supervised("panic", Some("trace-1".to_owned()), async {
                std::panic::resume_unwind(Box::new("static panic"));
                #[allow(unreachable_code)]
                Ok::<(), ()>(())
            });
        assert_eq!(panic.await.unwrap(), None);

        let ends = matching(&lines, "task_end");
        assert_eq!(ends.len(), 4);
        for outcome in ["ok", "err", "cancelled", "err"] {
            assert_eq!(
                ends.iter()
                    .filter(|line| line.contains(&format!("outcome={outcome}")))
                    .count(),
                if outcome == "err" { 2 } else { 1 }
            );
        }
        let panics = matching(&lines, "task_panic");
        assert_eq!(panics.len(), 1);
        assert!(panics[0].contains("code=E4001 name=PANIC"));
        assert!(panics[0].contains("payload=static panic"));
        assert_eq!(
            supervisor.counters(),
            SupervisorCounters {
                started: 4,
                ended_ok: 1,
                ended_err: 2,
                ended_cancelled: 1,
                panicked: 1,
                stalled: 0,
            }
        );
    }

    #[tokio::test]
    async fn formatted_panic_payload_is_withheld() {
        let (supervisor, lines) = fixture(1);
        let task = supervisor.spawn_lifecycle_supervised("panic", None, async {
            std::panic::resume_unwind(Box::new("private runtime value".to_owned()));
            #[allow(unreachable_code)]
            Ok::<(), ()>(())
        });
        assert_eq!(task.await.unwrap(), None);
        let panics = matching(&lines, "task_panic");
        assert_eq!(panics.len(), 1);
        assert!(panics[0].contains("payload=formatted"));
        assert!(!panics[0].contains("private runtime value"));
    }

    #[tokio::test]
    async fn blocking_work_reports_ok_err_and_panic() {
        let (supervisor, lines) = fixture(3);
        assert_eq!(
            supervisor
                .spawn_blocking_supervised("blocking_ok", None, || Ok::<_, ()>(9))
                .await
                .unwrap(),
            Some(9)
        );
        assert_eq!(
            supervisor
                .spawn_blocking_supervised("blocking_err", None, || Err::<(), _>(()))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            supervisor
                .spawn_blocking_supervised("blocking_panic", None, || {
                    std::panic::resume_unwind(Box::new("blocking panic"));
                    #[allow(unreachable_code)]
                    Ok::<(), ()>(())
                })
                .await
                .unwrap(),
            None
        );
        assert_eq!(matching(&lines, "task_end").len(), 3);
        assert_eq!(matching(&lines, "task_panic").len(), 1);
        assert_eq!(supervisor.counters().ended_ok, 1);
        assert_eq!(supervisor.counters().ended_err, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn watchdog_reports_one_stall_and_recovery_per_episode() {
        let (supervisor, lines) = fixture(1);
        let heartbeat_slot = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&heartbeat_slot);
        let task =
            supervisor.spawn_supervised_with_heartbeat("loop", None, move |heartbeat| async move {
                *lock(&slot) = Some(heartbeat);
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            });
        tokio::task::yield_now().await;
        let watchdog = tokio::spawn({
            let supervisor = supervisor.clone();
            async move {
                supervisor
                    .run_watchdog(Duration::from_secs(1), Duration::from_secs(3))
                    .await
            }
        });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;
        assert_eq!(matching(&lines, "task_stalled").len(), 1);
        assert!(supervisor.snapshot()[0].is_stalled(Duration::from_secs(3)));
        lock(&heartbeat_slot).as_ref().unwrap().beat();
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(matching(&lines, "task_recovered").len(), 1);
        assert_eq!(supervisor.counters().stalled, 1);
        watchdog.abort();
        task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn lifecycle_only_task_idle_past_stall_bound_is_not_stalled() {
        let (supervisor, lines) = fixture(1);
        let task = supervisor.spawn_lifecycle_supervised("lifecycle", None, async {
            std::future::pending::<()>().await;
            Ok::<(), ()>(())
        });
        tokio::task::yield_now().await;
        let watchdog = tokio::spawn({
            let supervisor = supervisor.clone();
            async move {
                supervisor
                    .run_watchdog(Duration::from_secs(1), Duration::from_secs(3))
                    .await
            }
        });
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;

        assert!(matching(&lines, "task_stalled").is_empty());
        assert!(!supervisor.snapshot()[0].is_stalled(Duration::from_secs(3)));
        watchdog.abort();
        task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn expected_idle_wait_does_not_stall_heartbeat_task() {
        let (supervisor, lines) = fixture(1);
        let task = supervisor.spawn_supervised_with_heartbeat(
            "waiting",
            None,
            move |heartbeat| async move {
                let _idle = heartbeat.expected_idle();
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            },
        );
        tokio::task::yield_now().await;
        let watchdog = tokio::spawn({
            let supervisor = supervisor.clone();
            async move {
                supervisor
                    .run_watchdog(Duration::from_secs(1), Duration::from_secs(3))
                    .await
            }
        });
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;

        assert!(matching(&lines, "task_stalled").is_empty());
        assert!(!supervisor.snapshot()[0].is_stalled(Duration::from_secs(3)));
        watchdog.abort();
        task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn registry_is_bounded_and_snapshot_has_task_times() {
        let (supervisor, _lines) = fixture(1);
        let entered = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&entered);
        let first = supervisor.spawn_lifecycle_supervised(
            "first",
            Some("trace-2".to_owned()),
            async move {
                marker.store(true, Ordering::SeqCst);
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            },
        );
        while !entered.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
        let second = supervisor.spawn_lifecycle_supervised("second", None, async {
            std::future::pending::<()>().await;
            Ok::<(), ()>(())
        });
        tokio::time::advance(Duration::from_millis(12)).await;
        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "first");
        assert_eq!(snapshot[0].trace.as_deref(), Some("trace-2"));
        assert_eq!(snapshot[0].age_ms, 12);
        assert_eq!(snapshot[0].since_beat_ms, 12);
        assert_eq!(snapshot[0].kind, TaskKind::Async);
        assert!(!snapshot[0].heartbeat_supervised);
        assert!(!snapshot[0].expected_idle);
        first.abort();
        second.abort();
    }

    #[tokio::test]
    async fn global_blocking_task_preserves_value_and_panic() {
        let value = spawn_blocking_lifecycle_supervised("blocking_value", None, || 7_u8);
        assert_eq!(value.await.unwrap(), 7);
        let panic = spawn_blocking_lifecycle_supervised("blocking_panic", None, || {
            std::panic::resume_unwind(Box::new("static panic"))
        });
        assert!(panic.await.unwrap_err().is_panic());
    }

    #[test]
    fn global_thread_keeps_name_registers_and_preserves_panic() {
        let (entered, wait_entered) = std::sync::mpsc::channel();
        let (release, wait_release) = std::sync::mpsc::channel::<()>();
        let thread = spawn_thread_supervised("supervised-thread", 1 << 20, move || {
            entered.send(()).unwrap();
            wait_release.recv().unwrap();
            std::thread::current().name().map(str::to_owned)
        })
        .unwrap();
        wait_entered.recv().unwrap();
        assert!(
            supervised_tasks()
                .iter()
                .any(|task| task.name == "supervised-thread" && task.kind == TaskKind::Thread)
        );
        release.send(()).unwrap();
        assert_eq!(thread.join().unwrap().as_deref(), Some("supervised-thread"));
        assert!(
            !supervised_tasks()
                .iter()
                .any(|task| task.name == "supervised-thread")
        );
        let panic = spawn_thread_supervised("supervised-panic", 1 << 20, || {
            std::panic::resume_unwind(Box::new("static panic"))
        })
        .unwrap();
        assert!(panic.join().is_err());
    }
}
