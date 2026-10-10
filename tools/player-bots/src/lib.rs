//! Headless synthetic-player supervision over the existing real Oteryn dev-client session seam.
//!
//! This crate owns orchestration only. It does not implement a second protocol, admission path,
//! reconnect contract, gameplay authority, or privileged testing shortcut.

#[cfg(test)]
mod live_tests;

use oteryn_dev_client::{DevClientSession, JoinRequest, connect_session};
use oteryn_protocol_oteryn::CharacterId;
use oteryn_protocol_oteryn::actor_spell::SpellTarget;
use oteryn_protocol_oteryn::chat::ChatIntent;
use oteryn_protocol_oteryn::item_view::{ItemHandle, ItemMoveIntent};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use rustls::pki_types::CertificateDer;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, watch};
use tokio::task::JoinHandle;

pub type BotFuture<'a> = Pin<Box<dyn Future<Output = BotReport> + Send + 'a>>;

/// Everything one synthetic player needs to enter through the existing real TLS/session path.
///
/// Debug deliberately omits admission material, trust-root bytes and behavior seed.
pub struct BotSpec {
    pub bot_id: u64,
    pub profile: String,
    pub behavior_seed: [u8; 32],
    pub address: SocketAddr,
    pub server_name: String,
    pub root_certificate: CertificateDer<'static>,
    pub schema_revision: u32,
    pub character_id: CharacterId,
    pub admission_material: Vec<u8>,
    pub client_build_id: String,
    pub deadline: Duration,
}

impl Debug for BotSpec {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BotSpec")
            .field("bot_id", &self.bot_id)
            .field("profile", &self.profile)
            .field("address", &self.address)
            .field("server_name", &self.server_name)
            .field("schema_revision", &self.schema_revision)
            .field("client_build_id", &self.client_build_id)
            .field("deadline", &self.deadline)
            .field("root_certificate", &"<redacted>")
            .field("character_id", &"<redacted>")
            .field("admission_material", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl BotSpec {
    #[must_use]
    pub fn join_request(&self) -> JoinRequest<'_> {
        JoinRequest {
            address: self.address,
            server_name: &self.server_name,
            root_certificate: &self.root_certificate,
            schema_revision: self.schema_revision,
            character_id: self.character_id,
            admission_material: &self.admission_material,
            client_build_id: &self.client_build_id,
            deadline: self.deadline,
        }
    }
}

/// The currently supported normal player-facing actions. Attack/target support belongs to KAN-35B.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotAction {
    Idle(Duration),
    Step(StepDirection),
    UseObject {
        placement: Vec<u8>,
        expected_revision: u64,
    },
    Cast {
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    },
    UseItem(ItemHandle),
    MoveItem(ItemMoveIntent),
    Chat(ChatIntent),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BotScenario {
    pub actions: Vec<BotAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BotRunConfig {
    pub concurrency: NonZeroUsize,
    pub liveness_slice: Duration,
}

impl BotRunConfig {
    pub fn new(
        concurrency: NonZeroUsize,
        liveness_slice: Duration,
    ) -> Result<Self, SupervisorError> {
        if liveness_slice.is_zero() {
            return Err(SupervisorError::ZeroLivenessSlice);
        }
        Ok(Self {
            concurrency,
            liveness_slice,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BotCommandClass {
    Step,
    UseObject,
    Cast,
    UseItem,
    MoveItem,
    Chat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LatencyStats {
    pub samples: u64,
    pub total_nanos: u128,
    pub max_nanos: u128,
}

impl LatencyStats {
    fn record(&mut self, elapsed: Duration) {
        let nanos = elapsed.as_nanos();
        self.samples = self.samples.saturating_add(1);
        self.total_nanos = self.total_nanos.saturating_add(nanos);
        self.max_nanos = self.max_nanos.max(nanos);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotFailureKind {
    Connect,
    Command,
    Liveness,
    Task,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotTerminal {
    Completed,
    ShutdownRequested,
    Failed(BotFailureKind),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BotMetrics {
    pub admitted: bool,
    pub connect_failures: u64,
    pub commands: BTreeMap<BotCommandClass, u64>,
    pub command_failures: BTreeMap<BotCommandClass, u64>,
    pub command_latency: BTreeMap<BotCommandClass, LatencyStats>,
    pub liveness_cycles: u64,
    pub liveness_failures: u64,
    pub events_drained: u64,
}

impl BotMetrics {
    fn record_command(&mut self, class: BotCommandClass, elapsed: Duration, success: bool) {
        let commands = self.commands.entry(class).or_default();
        *commands = commands.saturating_add(1);
        self.command_latency
            .entry(class)
            .or_default()
            .record(elapsed);
        if !success {
            let failures = self.command_failures.entry(class).or_default();
            *failures = failures.saturating_add(1);
        }
    }

    fn drain_events(&mut self, session: &mut DevClientSession) {
        let drained = u64::try_from(session.take_events().len()).unwrap_or(u64::MAX);
        self.events_drained = self.events_drained.saturating_add(drained);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotReport {
    pub bot_id: u64,
    pub profile: String,
    pub terminal: BotTerminal,
    pub metrics: BotMetrics,
}

impl BotReport {
    fn completed(spec: &BotSpec, metrics: BotMetrics) -> Self {
        Self {
            bot_id: spec.bot_id,
            profile: spec.profile.clone(),
            terminal: BotTerminal::Completed,
            metrics,
        }
    }

    fn shutdown(spec: &BotSpec, metrics: BotMetrics) -> Self {
        Self {
            bot_id: spec.bot_id,
            profile: spec.profile.clone(),
            terminal: BotTerminal::ShutdownRequested,
            metrics,
        }
    }

    fn failed(spec: &BotSpec, kind: BotFailureKind, metrics: BotMetrics) -> Self {
        Self {
            bot_id: spec.bot_id,
            profile: spec.profile.clone(),
            terminal: BotTerminal::Failed(kind),
            metrics,
        }
    }

    fn task_failed(bot_id: u64, profile: String) -> Self {
        Self {
            bot_id,
            profile,
            terminal: BotTerminal::Failed(BotFailureKind::Task),
            metrics: BotMetrics::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorError {
    DuplicateBotId(u64),
    ZeroLivenessSlice,
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBotId(bot_id) => {
                write!(formatter, "duplicate synthetic bot id {bot_id}")
            }
            Self::ZeroLivenessSlice => formatter.write_str("liveness slice must be non-zero"),
        }
    }
}

impl Error for SupervisorError {}

pub struct BotRunContext {
    shutdown: watch::Receiver<bool>,
    liveness_slice: Duration,
}

impl BotRunContext {
    #[must_use]
    pub fn shutdown_requested(&self) -> bool {
        *self.shutdown.borrow()
    }

    #[must_use]
    pub const fn liveness_slice(&self) -> Duration {
        self.liveness_slice
    }
}

/// Injectable execution boundary. Production uses LiveBotRunner; tests can inject a runner
/// without introducing a fake protocol implementation.
pub trait BotRunner: Send + Sync + 'static {
    fn run<'a>(
        &'a self,
        spec: BotSpec,
        scenario: Arc<BotScenario>,
        context: BotRunContext,
    ) -> BotFuture<'a>;
}

#[derive(Debug, Default)]
pub struct LiveBotRunner;

impl BotRunner for LiveBotRunner {
    fn run<'a>(
        &'a self,
        spec: BotSpec,
        scenario: Arc<BotScenario>,
        mut context: BotRunContext,
    ) -> BotFuture<'a> {
        Box::pin(async move {
            let mut metrics = BotMetrics::default();
            if context.shutdown_requested() {
                return BotReport::shutdown(&spec, metrics);
            }

            let Some(connection) =
                unless_shutdown(&mut context.shutdown, connect_session(spec.join_request())).await
            else {
                return BotReport::shutdown(&spec, metrics);
            };
            let mut session = match connection {
                Ok(session) => session,
                Err(_) => {
                    metrics.connect_failures = metrics.connect_failures.saturating_add(1);
                    return BotReport::failed(&spec, BotFailureKind::Connect, metrics);
                }
            };
            metrics.admitted = true;
            metrics.drain_events(&mut session);

            for action in &scenario.actions {
                if context.shutdown_requested() {
                    return BotReport::shutdown(&spec, metrics);
                }
                match action {
                    BotAction::Idle(duration) => {
                        if !service_idle(
                            &mut session,
                            *duration,
                            context.liveness_slice,
                            &mut context.shutdown,
                            &mut metrics,
                        )
                        .await
                        {
                            return if context.shutdown_requested() {
                                BotReport::shutdown(&spec, metrics)
                            } else {
                                BotReport::failed(&spec, BotFailureKind::Liveness, metrics)
                            };
                        }
                    }
                    BotAction::Step(direction) => {
                        let started = Instant::now();
                        let Some(result) =
                            unless_shutdown(&mut context.shutdown, session.step(*direction)).await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::Step,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                    BotAction::UseObject {
                        placement,
                        expected_revision,
                    } => {
                        let started = Instant::now();
                        let Some(result) = unless_shutdown(
                            &mut context.shutdown,
                            session.use_object(placement, *expected_revision),
                        )
                        .await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::UseObject,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                    BotAction::Cast {
                        spell,
                        target,
                        aim_at_target,
                    } => {
                        let started = Instant::now();
                        let Some(result) = unless_shutdown(
                            &mut context.shutdown,
                            session.cast_spell(*spell, *target, *aim_at_target),
                        )
                        .await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::Cast,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                    BotAction::UseItem(handle) => {
                        let started = Instant::now();
                        let Some(result) =
                            unless_shutdown(&mut context.shutdown, session.use_item(*handle)).await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::UseItem,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                    BotAction::MoveItem(intent) => {
                        let started = Instant::now();
                        let Some(result) =
                            unless_shutdown(&mut context.shutdown, session.move_item(intent)).await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::MoveItem,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                    BotAction::Chat(intent) => {
                        let started = Instant::now();
                        let Some(result) =
                            unless_shutdown(&mut context.shutdown, session.chat(intent)).await
                        else {
                            return BotReport::shutdown(&spec, metrics);
                        };
                        metrics.record_command(
                            BotCommandClass::Chat,
                            started.elapsed(),
                            result.is_ok(),
                        );
                        if result.is_err() {
                            return BotReport::failed(&spec, BotFailureKind::Command, metrics);
                        }
                    }
                }
                // A command result may precede its pushed deltas. Give the real
                // session one bounded read window before draining/completing.
                if !matches!(action, BotAction::Idle(_))
                    && !service_idle(
                        &mut session,
                        context.liveness_slice,
                        context.liveness_slice,
                        &mut context.shutdown,
                        &mut metrics,
                    )
                    .await
                {
                    return if context.shutdown_requested() {
                        BotReport::shutdown(&spec, metrics)
                    } else {
                        BotReport::failed(&spec, BotFailureKind::Liveness, metrics)
                    };
                }
            }

            BotReport::completed(&spec, metrics)
        })
    }
}

// Cancellation drops the in-flight exchange and the caller then drops the
// session. Never reuse a session after cancelling a partially consumed frame.
async fn unless_shutdown<F: Future>(
    shutdown: &mut watch::Receiver<bool>,
    operation: F,
) -> Option<F::Output> {
    if *shutdown.borrow() {
        return None;
    }
    let mut changed = std::pin::pin!(shutdown.changed());
    let mut operation = std::pin::pin!(operation);
    std::future::poll_fn(|cx| {
        if changed.as_mut().poll(cx).is_ready() {
            return Poll::Ready(None);
        }
        operation.as_mut().poll(cx).map(Some)
    })
    .await
}

async fn service_idle(
    session: &mut DevClientSession,
    duration: Duration,
    liveness_slice: Duration,
    shutdown: &mut watch::Receiver<bool>,
    metrics: &mut BotMetrics,
) -> bool {
    let mut remaining = duration;
    while !remaining.is_zero() {
        if *shutdown.borrow() {
            return false;
        }
        let current = remaining.min(liveness_slice);
        let Some(result) = unless_shutdown(shutdown, session.service_liveness(current)).await
        else {
            return false;
        };
        if result.is_err() {
            metrics.liveness_failures = metrics.liveness_failures.saturating_add(1);
            return false;
        }
        metrics.liveness_cycles = metrics.liveness_cycles.saturating_add(1);
        metrics.drain_events(session);
        remaining = remaining.saturating_sub(current);
    }
    true
}

pub struct BotSupervisor {
    runner: Arc<dyn BotRunner>,
}

impl Default for BotSupervisor {
    fn default() -> Self {
        Self::live()
    }
}

impl BotSupervisor {
    #[must_use]
    pub fn live() -> Self {
        Self {
            runner: Arc::new(LiveBotRunner),
        }
    }

    #[must_use]
    pub fn with_runner(runner: Arc<dyn BotRunner>) -> Self {
        Self { runner }
    }

    pub fn start(
        &self,
        specs: Vec<BotSpec>,
        scenario: BotScenario,
        config: BotRunConfig,
    ) -> Result<BotRun, SupervisorError> {
        if config.liveness_slice.is_zero() {
            return Err(SupervisorError::ZeroLivenessSlice);
        }
        let mut ids = BTreeSet::new();
        for spec in &specs {
            if !ids.insert(spec.bot_id) {
                return Err(SupervisorError::DuplicateBotId(spec.bot_id));
            }
        }

        let semaphore = Arc::new(Semaphore::new(config.concurrency.get()));
        let scenario = Arc::new(scenario);
        let (shutdown, receiver) = watch::channel(false);
        let mut handles = Vec::with_capacity(specs.len());

        for spec in specs {
            let runner = Arc::clone(&self.runner);
            let scenario = Arc::clone(&scenario);
            let semaphore = Arc::clone(&semaphore);
            let context = BotRunContext {
                shutdown: receiver.clone(),
                liveness_slice: config.liveness_slice,
            };
            let identity = (spec.bot_id, spec.profile.clone());
            let handle = tokio::spawn(async move {
                let permit = match semaphore.acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return BotReport::failed(
                            &spec,
                            BotFailureKind::Task,
                            BotMetrics::default(),
                        );
                    }
                };
                let report = runner.run(spec, scenario, context).await;
                drop(permit);
                report
            });
            handles.push((identity, handle));
        }

        Ok(BotRun { shutdown, handles })
    }
}

#[derive(Clone)]
pub struct BotShutdown {
    sender: watch::Sender<bool>,
}

impl BotShutdown {
    /// Requests cooperative shutdown. Returns false only when every worker receiver is already gone.
    pub fn request(&self) -> bool {
        self.sender.send(true).is_ok()
    }
}

#[must_use = "BotRun must be finished or shut down so worker tasks are joined"]
pub struct BotRun {
    shutdown: watch::Sender<bool>,
    handles: Vec<((u64, String), JoinHandle<BotReport>)>,
}

impl Drop for BotRun {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        for (_, handle) in &self.handles {
            handle.abort();
        }
    }
}

impl BotRun {
    #[must_use]
    pub fn shutdown_handle(&self) -> BotShutdown {
        BotShutdown {
            sender: self.shutdown.clone(),
        }
    }

    pub async fn finish(mut self) -> Vec<BotReport> {
        // Keep handles owned by self while awaiting: cancelling collection must
        // still let Drop abort every worker rather than detach its JoinHandle.
        collect_reports(&mut self.handles).await
    }

    pub async fn shutdown(mut self) -> Vec<BotReport> {
        let _ = self.shutdown.send(true);
        collect_reports(&mut self.handles).await
    }
}

async fn collect_reports(handles: &mut [((u64, String), JoinHandle<BotReport>)]) -> Vec<BotReport> {
    let mut reports = Vec::with_capacity(handles.len());
    for ((bot_id, profile), handle) in handles {
        match handle.await {
            Ok(report) => reports.push(report),
            Err(_) => reports.push(BotReport::task_failed(*bot_id, profile.clone())),
        }
    }
    reports.sort_by_key(|report| report.bot_id);
    reports
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Barrier;

    #[derive(Clone)]
    struct MockRunner {
        active: Arc<AtomicUsize>,
        maximum_active: Arc<AtomicUsize>,
        finished: Arc<AtomicUsize>,
        barrier: Option<Arc<Barrier>>,
        fail_bot: Option<u64>,
        wait_for_shutdown: bool,
    }

    impl MockRunner {
        fn new(
            barrier: Option<Arc<Barrier>>,
            fail_bot: Option<u64>,
            wait_for_shutdown: bool,
        ) -> Self {
            Self {
                active: Arc::new(AtomicUsize::new(0)),
                maximum_active: Arc::new(AtomicUsize::new(0)),
                finished: Arc::new(AtomicUsize::new(0)),
                barrier,
                fail_bot,
                wait_for_shutdown,
            }
        }
    }

    impl BotRunner for MockRunner {
        fn run<'a>(
            &'a self,
            spec: BotSpec,
            _scenario: Arc<BotScenario>,
            mut context: BotRunContext,
        ) -> BotFuture<'a> {
            Box::pin(async move {
                let active = self.active.fetch_add(1, Ordering::SeqCst).saturating_add(1);
                self.maximum_active.fetch_max(active, Ordering::SeqCst);

                if let Some(barrier) = &self.barrier {
                    barrier.wait().await;
                }

                if self.wait_for_shutdown {
                    while !context.shutdown_requested() {
                        if context.shutdown.changed().await.is_err() {
                            break;
                        }
                    }
                } else {
                    tokio::task::yield_now().await;
                }

                self.active.fetch_sub(1, Ordering::SeqCst);
                self.finished.fetch_add(1, Ordering::SeqCst);

                if self.fail_bot == Some(spec.bot_id) {
                    BotReport::failed(&spec, BotFailureKind::Command, BotMetrics::default())
                } else if self.wait_for_shutdown {
                    BotReport::shutdown(&spec, BotMetrics::default())
                } else {
                    BotReport::completed(&spec, BotMetrics::default())
                }
            })
        }
    }

    fn uuid_v7(last: u8) -> [u8; 16] {
        [0x01, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, last]
    }

    fn fixture_spec(bot_id: u64) -> Result<BotSpec, Box<dyn Error>> {
        let last = u8::try_from(bot_id.saturating_add(1))
            .map_err(|_| io::Error::other("fixture bot id is too large"))?;
        let character_id = CharacterId::decode(&uuid_v7(last))
            .map_err(|_| io::Error::other("fixture character id rejected"))?;
        Ok(BotSpec {
            bot_id,
            profile: format!("profile-{bot_id}"),
            behavior_seed: [last; 32],
            address: SocketAddr::from(([127, 0, 0, 1], 7171)),
            server_name: "localhost".into(),
            root_certificate: CertificateDer::from(vec![1_u8, 2, 3, 4]),
            schema_revision: 1,
            character_id,
            admission_material: format!("grant-secret-{bot_id}").into_bytes(),
            client_build_id: "player-bot-test".into(),
            deadline: Duration::from_secs(1),
        })
    }

    fn specs(count: u64) -> Result<Vec<BotSpec>, Box<dyn Error>> {
        (0..count).map(fixture_spec).collect()
    }

    fn runtime() -> Result<tokio::runtime::Runtime, io::Error> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
    }

    #[test]
    fn twenty_sessions_run_concurrently_and_complete() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(
                Some(Arc::new(Barrier::new(20))),
                None,
                false,
            ));
            let supervisor = BotSupervisor::with_runner(runner.clone());
            let config = BotRunConfig::new(
                NonZeroUsize::new(20).ok_or_else(|| io::Error::other("nonzero"))?,
                Duration::from_millis(25),
            )?;
            let run = supervisor.start(specs(20)?, BotScenario::default(), config)?;
            let reports = run.finish().await;
            assert_eq!(reports.len(), 20);
            assert!(
                reports
                    .iter()
                    .all(|report| report.terminal == BotTerminal::Completed)
            );
            assert_eq!(runner.maximum_active.load(Ordering::SeqCst), 20);
            assert_eq!(runner.finished.load(Ordering::SeqCst), 20);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn one_failed_bot_does_not_terminate_peers() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(
                Some(Arc::new(Barrier::new(20))),
                Some(7),
                false,
            ));
            let supervisor = BotSupervisor::with_runner(runner);
            let config = BotRunConfig::new(
                NonZeroUsize::new(20).ok_or_else(|| io::Error::other("nonzero"))?,
                Duration::from_millis(25),
            )?;
            let reports = supervisor
                .start(specs(20)?, BotScenario::default(), config)?
                .finish()
                .await;
            let failures = reports
                .iter()
                .filter(|report| report.terminal == BotTerminal::Failed(BotFailureKind::Command))
                .count();
            let successes = reports
                .iter()
                .filter(|report| report.terminal == BotTerminal::Completed)
                .count();
            assert_eq!(failures, 1);
            assert_eq!(successes, 19);
            assert_eq!(reports[7].bot_id, 7);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn cooperative_shutdown_joins_every_worker() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(
                Some(Arc::new(Barrier::new(20))),
                None,
                true,
            ));
            let supervisor = BotSupervisor::with_runner(runner.clone());
            let config = BotRunConfig::new(
                NonZeroUsize::new(20).ok_or_else(|| io::Error::other("nonzero"))?,
                Duration::from_millis(25),
            )?;
            let reports = supervisor
                .start(specs(20)?, BotScenario::default(), config)?
                .shutdown()
                .await;
            assert!(
                reports
                    .iter()
                    .all(|report| report.terminal == BotTerminal::ShutdownRequested)
            );
            assert_eq!(runner.finished.load(Ordering::SeqCst), 20);
            assert_eq!(runner.active.load(Ordering::SeqCst), 0);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn supervisor_respects_the_run_concurrency_bound() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(None, None, false));
            let supervisor = BotSupervisor::with_runner(runner.clone());
            let config = BotRunConfig::new(
                NonZeroUsize::new(4).ok_or_else(|| io::Error::other("nonzero"))?,
                Duration::from_millis(25),
            )?;
            let reports = supervisor
                .start(specs(20)?, BotScenario::default(), config)?
                .finish()
                .await;
            assert_eq!(reports.len(), 20);
            assert!(runner.maximum_active.load(Ordering::SeqCst) <= 4);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn duplicate_bot_ids_fail_before_any_worker_starts() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(None, None, false));
            let supervisor = BotSupervisor::with_runner(runner.clone());
            let first = fixture_spec(1)?;
            let second = fixture_spec(1)?;
            let config = BotRunConfig::new(
                NonZeroUsize::new(2).ok_or_else(|| io::Error::other("nonzero"))?,
                Duration::from_millis(25),
            )?;
            assert_eq!(
                supervisor
                    .start(vec![first, second], BotScenario::default(), config)
                    .err(),
                Some(SupervisorError::DuplicateBotId(1))
            );
            assert_eq!(runner.active.load(Ordering::SeqCst), 0);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn debug_output_never_contains_join_secrets_or_seed() -> Result<(), Box<dyn Error>> {
        let spec = fixture_spec(3)?;
        let rendered = format!("{spec:?}");
        assert!(!rendered.contains("grant-secret-3"));
        assert!(!rendered.contains(&format!("{:?}", spec.behavior_seed)));
        assert!(!rendered.contains("1, 2, 3, 4"));
        assert!(rendered.contains("<redacted>"));
        Ok(())
    }

    #[test]
    fn zero_liveness_slice_is_rejected() {
        let concurrency = NonZeroUsize::new(1);
        assert_eq!(
            concurrency.and_then(|value| BotRunConfig::new(value, Duration::ZERO).ok()),
            None
        );
    }

    #[test]
    fn supervisor_rejects_zero_slice_in_a_direct_config() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            let runner = Arc::new(MockRunner::new(None, None, false));
            let supervisor = BotSupervisor::with_runner(runner.clone());
            let config = BotRunConfig {
                concurrency: NonZeroUsize::new(1).ok_or_else(|| io::Error::other("nonzero"))?,
                liveness_slice: Duration::ZERO,
            };
            assert_eq!(
                supervisor
                    .start(specs(1)?, BotScenario::default(), config)
                    .err(),
                Some(SupervisorError::ZeroLivenessSlice)
            );
            assert_eq!(runner.active.load(Ordering::SeqCst), 0);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    struct PendingRunner {
        started: tokio::sync::mpsc::UnboundedSender<u64>,
        stopped: tokio::sync::mpsc::UnboundedSender<u64>,
    }

    struct WorkerDrop {
        bot_id: u64,
        stopped: tokio::sync::mpsc::UnboundedSender<u64>,
    }

    impl Drop for WorkerDrop {
        fn drop(&mut self) {
            let _ = self.stopped.send(self.bot_id);
        }
    }

    impl BotRunner for PendingRunner {
        fn run<'a>(
            &'a self,
            spec: BotSpec,
            _scenario: Arc<BotScenario>,
            _context: BotRunContext,
        ) -> BotFuture<'a> {
            Box::pin(async move {
                let _guard = WorkerDrop {
                    bot_id: spec.bot_id,
                    stopped: self.stopped.clone(),
                };
                let _ = self.started.send(spec.bot_id);
                std::future::pending().await
            })
        }
    }

    #[test]
    fn cancelling_finish_or_shutdown_aborts_every_worker() -> Result<(), Box<dyn Error>> {
        let runtime = runtime()?;
        runtime.block_on(async {
            for shutdown in [false, true] {
                let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
                let (stopped, mut stops) = tokio::sync::mpsc::unbounded_channel();
                let supervisor =
                    BotSupervisor::with_runner(Arc::new(PendingRunner { started, stopped }));
                let config = BotRunConfig::new(
                    NonZeroUsize::new(2).ok_or_else(|| io::Error::other("nonzero"))?,
                    Duration::from_millis(25),
                )?;
                let run = supervisor.start(specs(2)?, BotScenario::default(), config)?;
                for _ in 0..2 {
                    assert!(
                        tokio::time::timeout(Duration::from_secs(2), starts.recv())
                            .await?
                            .is_some()
                    );
                }
                let (polled, first_poll) = tokio::sync::oneshot::channel();
                let joining = tokio::spawn(async move {
                    let collecting = async move {
                        if shutdown {
                            run.shutdown().await
                        } else {
                            run.finish().await
                        }
                    };
                    let mut collecting = std::pin::pin!(collecting);
                    let mut polled = Some(polled);
                    std::future::poll_fn(move |cx| {
                        let result = collecting.as_mut().poll(cx);
                        if let Some(sender) = polled.take() {
                            let _ = sender.send(());
                        }
                        result
                    })
                    .await
                });
                tokio::time::timeout(Duration::from_secs(2), first_poll).await??;
                joining.abort();
                assert!(joining.await.is_err());
                let mut stopped_ids = BTreeSet::new();
                for _ in 0..2 {
                    let id = tokio::time::timeout(Duration::from_secs(2), stops.recv())
                        .await?
                        .ok_or_else(|| io::Error::other("worker drop channel closed"))?;
                    stopped_ids.insert(id);
                }
                assert_eq!(stopped_ids, BTreeSet::from([0, 1]));
            }
            Ok::<(), Box<dyn Error>>(())
        })
    }
}
