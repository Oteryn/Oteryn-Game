//! Wires the runtime-status reporter into the serve lifecycle
//! (`oteryn-game-native-runtime-status-v1` §8). Nothing here gates boot,
//! serving or shutdown.

use crate::durability::runtime_scope_assignment::{
    AssignmentError, AssignmentState, NodeRegistrationFact, RuntimeScopeAssignment,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardStateV1, AdmissionAuthorityPublicationV1,
};
use crate::native_admission_source::runtime_status::{
    self, Gate, HeartbeatGate, MtlsSink, Publication, ReportClock, ReportSink,
    RuntimeStatusDescriptor, SystemClock,
};
use oteryn_foundation::CancellationToken;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// The assignment is re-read at most this often. Between reads the snapshot
/// verified by the `ready = true` commit (or by the last read) stands, so no
/// heartbeat takes a durability transaction of its own.
const RECHECK: Duration = Duration::from_secs(10);
/// While a re-read finds the single durability connection in use, the last
/// verification still stands this long. `RECHECK + BUSY_GRACE` equals the
/// proposed freshness bound F (15 s): a root that stays unreachable stops
/// heartbeats within it.
const BUSY_GRACE: Duration = Duration::from_secs(5);

/// One non-blocking assignment read, classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe {
    Current,
    Lost,
    /// The connection is in use (or being re-established): not a verdict.
    Busy,
    NotReady,
}

pub(super) fn classify(
    read: Result<Option<RuntimeScopeAssignment>, AssignmentError>,
    fact: NodeRegistrationFact,
    generation: u64,
) -> Probe {
    match read {
        Ok(Some(current))
            if current.state == AssignmentState::Assigned
                && current.holder == Some(fact)
                && current.ownership_generation == generation =>
        {
            Probe::Current
        }
        Ok(_) => Probe::Lost,
        Err(AssignmentError::Unavailable(DurabilityError::RootUnavailable)) => Probe::Busy,
        Err(_) => Probe::NotReady,
    }
}

pub(super) trait AssignmentProbe: Send {
    fn probe(&mut self) -> impl Future<Output = Probe> + Send;
}

/// Reads through `try_issue_semantic_pass`: it never waits for the
/// connection, so a busy connection is `Busy` at once.
struct RootProbe {
    root: DurabilityRoot,
    scope: RuntimeScopeRefV1,
    fact: NodeRegistrationFact,
    generation: u64,
}

impl AssignmentProbe for RootProbe {
    async fn probe(&mut self) -> Probe {
        classify(
            self.root.read_runtime_scope_assignment(self.scope).await,
            self.fact,
            self.generation,
        )
    }
}

/// §8.2 heartbeat conditions. `Lost` is final for this reporter.
struct AssignmentGate<C, P> {
    clock: C,
    probe: P,
    serving: CancellationToken,
    verified_at: Duration,
    lost: bool,
}

impl<C: ReportClock, P: AssignmentProbe> HeartbeatGate for AssignmentGate<C, P> {
    async fn check(&mut self) -> Gate {
        if self.serving.is_cancelled() {
            return Gate::NotReady;
        }
        if self.lost {
            return Gate::Lost;
        }
        let age = self.clock.elapsed().saturating_sub(self.verified_at);
        if age < RECHECK {
            return Gate::Holds;
        }
        match self.probe.probe().await {
            Probe::Current => {
                self.verified_at = self.clock.elapsed();
                Gate::Holds
            }
            Probe::Lost => {
                self.lost = true;
                Gate::Lost
            }
            Probe::Busy if age < RECHECK + BUSY_GRACE => Gate::Holds,
            Probe::Busy => Gate::Busy,
            Probe::NotReady => Gate::NotReady,
        }
    }
}

/// Field-by-field projection of the one committed Runtime change (§4).
fn project(
    committed: &AdmissionAuthorityPublicationV1,
    world_id: &str,
    channel_id: &str,
    node_id: &str,
    epoch: u64,
) -> Option<Publication> {
    let [change] = committed.changes() else {
        return None;
    };
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready,
        route_revision,
        runtime_observation_revision,
        protocol_major,
        transport_profile,
        ruleset_revision,
        content_revision,
        map_revision,
        world_policy_revision,
        offer_revision,
    } = &change.state
    else {
        return None;
    };
    Some(Publication {
        source_authority: change.source.authority.clone(),
        world_id: world_id.to_owned(),
        channel_id: channel_id.to_owned(),
        node_id: node_id.to_owned(),
        assignment_epoch: epoch,
        scope_ownership_generation: *ownership_generation,
        source_revision: change.source.source_revision,
        decision_identity: change.source.decision_identity.clone(),
        ready: *ready,
        published_at: change.source.source_observed_at,
        protocol_major: *protocol_major,
        transport_profile: *transport_profile,
        route_revision: route_revision.clone(),
        runtime_observation_revision: runtime_observation_revision.clone(),
        ruleset_revision: ruleset_revision.clone(),
        content_revision: content_revision.clone(),
        map_revision: map_revision.clone(),
        world_policy_revision: world_policy_revision.clone(),
        offer_revision: offer_revision.clone(),
    })
}

/// Identity of the reported scope and incarnation.
pub(super) struct Reported {
    pub(super) world_id: String,
    pub(super) channel_id: String,
    pub(super) node_id: String,
    pub(super) epoch: u64,
}

pub(super) struct Reporter<C> {
    clock: C,
    latest: watch::Sender<Option<Publication>>,
    serving: CancellationToken,
    task: tokio::task::JoinHandle<()>,
    reported: Reported,
}

/// The production reporter: Platform over mTLS, the real durability root.
#[allow(clippy::too_many_arguments)]
pub(super) fn start_platform(
    descriptor: Arc<RuntimeStatusDescriptor>,
    reported: Reported,
    root: DurabilityRoot,
    scope: RuntimeScopeRefV1,
    fact: NodeRegistrationFact,
    generation: u64,
    committed: &AdmissionAuthorityPublicationV1,
) -> Reporter<SystemClock> {
    let probe = RootProbe {
        root,
        scope,
        fact,
        generation,
    };
    Reporter::start(
        SystemClock::default(),
        MtlsSink::new(descriptor),
        probe,
        reported,
        committed,
    )
}

impl<C: ReportClock + Clone + 'static> Reporter<C> {
    /// Only a committed publication starts reporting (§8.1): `committed` is
    /// what `Readiness::publish` returns after `Applied` or `Existing`.
    pub(super) fn start<S, P>(
        clock: C,
        sink: S,
        probe: P,
        reported: Reported,
        committed: &AdmissionAuthorityPublicationV1,
    ) -> Self
    where
        S: ReportSink + 'static,
        P: AssignmentProbe + 'static,
    {
        let first = project(
            committed,
            &reported.world_id,
            &reported.channel_id,
            &reported.node_id,
            reported.epoch,
        );
        let (latest, receiver) = watch::channel(first);
        let serving = CancellationToken::new();
        let gate = AssignmentGate {
            clock: clock.clone(),
            probe,
            serving: serving.clone(),
            verified_at: clock.elapsed(),
            lost: false,
        };
        let task = tokio::spawn(runtime_status::run(
            clock.clone(),
            receiver,
            runtime_status::HEARTBEAT,
            gate,
            sink,
        ));
        Self {
            clock,
            latest,
            serving,
            task,
            reported,
        }
    }

    /// Shutdown began: no further heartbeat (§8.3).
    pub(super) fn stop_heartbeats(&self) {
        self.serving.cancel();
    }

    /// The shutdown withdrawal outcome: only a committed `ready = false`
    /// publication is reported; a failed withdrawal reports nothing.
    pub(super) fn withdrawn(&self, committed: Option<&AdmissionAuthorityPublicationV1>) {
        let reported = &self.reported;
        if let Some(publication) = committed.and_then(|committed| {
            project(
                committed,
                &reported.world_id,
                &reported.channel_id,
                &reported.node_id,
                reported.epoch,
            )
        }) {
            self.latest.send_replace(Some(publication));
        }
    }

    /// Best effort within the remaining shutdown budget: the pending report
    /// is sent, then the reporter ends or is abandoned.
    pub(super) async fn finish(self, budget: Duration) {
        self.serving.cancel();
        drop(self.latest);
        let mut task = self.task;
        if runtime_status::first_of(&mut task, self.clock.sleep(budget))
            .await
            .is_none()
        {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityOwningPublisherV1,
        AdmissionAuthorityPublicationChangeV1, AdmissionAuthorityPublicationErrorV1,
        AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
        AdmissionPublicationSourceV1,
    };
    use crate::foundation::{ChannelId, NodeId, WorldId};
    use crate::native_admission_source::runtime_status::{Delivery, NotDelivered};
    use std::pin::pin;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Committed(AdmissionAuthorityPublicationChangeV1);
    impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for Committed {}
    impl AdmissionAuthorityOwningPublisherV1 for Committed {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(vec![self.0.clone()])
        }
    }

    const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
    const CHANNEL: &str = "01934f10-7c03-7001-805b-3b1122334401";
    const NODE: &str = "01934f10-7c04-7001-805b-3b1122334401";
    const PUBLISHED_AT: i64 = 1_790_000_000;
    const F: Duration = Duration::from_secs(15);

    fn uuid(text: &str) -> [u8; 16] {
        super::super::uuid_bytes(text).expect("uuid")
    }

    fn scope() -> RuntimeScopeRefV1 {
        RuntimeScopeRefV1::channel(
            WorldId::decode(&uuid(WORLD)).expect("world"),
            ChannelId::decode(&uuid(CHANNEL)).expect("channel"),
        )
    }

    fn committed(ready: bool, source_revision: u64) -> AdmissionAuthorityPublicationV1 {
        let change = AdmissionAuthorityPublicationChangeV1 {
            key: AdmissionAuthorityGuardKeyV1::Runtime(scope()),
            source: AdmissionPublicationSourceV1 {
                authority: "oteryn:runtime:world-1:channel-1".into(),
                purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
                source_revision,
                decision_identity: format!("runtime-readiness:0a:3:{source_revision}:{ready}"),
                source_observed_at: PUBLISHED_AT,
                clock_uncertainty_seconds: 0,
            },
            precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            publication_revision: 1,
            state: AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation: 3,
                ready,
                route_revision: "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e".into(),
                runtime_observation_revision: "observation-1".into(),
                protocol_major: 1,
                transport_profile: 1,
                ruleset_revision: "ruleset-1".into(),
                content_revision: "content-1".into(),
                map_revision: "map-1".into(),
                world_policy_revision: "policy-1".into(),
                offer_revision: "offer-1".into(),
            },
        };
        AdmissionAuthorityPublicationV1::prepare(&Committed(change), PUBLISHED_AT)
            .expect("committed publication")
    }

    fn reported() -> Reported {
        Reported {
            world_id: WORLD.into(),
            channel_id: CHANNEL.into(),
            node_id: NODE.into(),
            epoch: 2,
        }
    }

    #[test]
    fn the_report_equals_the_committed_publication_field_by_field() {
        let report = project(&committed(true, 7), WORLD, CHANNEL, NODE, 2).expect("runtime change");
        assert_eq!(
            report,
            Publication {
                source_authority: "oteryn:runtime:world-1:channel-1".into(),
                world_id: WORLD.into(),
                channel_id: CHANNEL.into(),
                node_id: NODE.into(),
                assignment_epoch: 2,
                scope_ownership_generation: 3,
                source_revision: 7,
                decision_identity: "runtime-readiness:0a:3:7:true".into(),
                ready: true,
                published_at: PUBLISHED_AT,
                protocol_major: 1,
                transport_profile: 1,
                route_revision: "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e".into(),
                runtime_observation_revision: "observation-1".into(),
                ruleset_revision: "ruleset-1".into(),
                content_revision: "content-1".into(),
                map_revision: "map-1".into(),
                world_policy_revision: "policy-1".into(),
                offer_revision: "offer-1".into(),
            }
        );
        assert!(runtime_status::encode(&report, PUBLISHED_AT).is_ok());
    }

    #[test]
    fn the_real_assignment_read_is_classified_one_fact_at_a_time() {
        let fact = NodeRegistrationFact::new(NodeId::decode(&uuid(NODE)).expect("node"), 4);
        let current = RuntimeScopeAssignment {
            scope: scope(),
            ownership_generation: 3,
            state: AssignmentState::Assigned,
            holder: Some(fact),
            source_revision: 9,
            decision_identity: "assignment".into(),
            decided_at: PUBLISHED_AT,
        };
        let with = |change: fn(&mut RuntimeScopeAssignment)| {
            let mut changed = current.clone();
            change(&mut changed);
            classify(Ok(Some(changed)), fact, 3)
        };
        assert_eq!(classify(Ok(Some(current.clone())), fact, 3), Probe::Current);
        assert_eq!(with(|a| a.ownership_generation = 4), Probe::Lost);
        assert_eq!(
            with(|a| a.holder = Some(NodeRegistrationFact::new(a.holder.unwrap().node_id(), 5))),
            Probe::Lost
        );
        assert_eq!(with(|a| a.holder = None), Probe::Lost);
        assert_eq!(with(|a| a.state = AssignmentState::Revoked), Probe::Lost);
        assert_eq!(classify(Ok(None), fact, 3), Probe::Lost);
        assert_eq!(
            classify(Err(DurabilityError::RootUnavailable.into()), fact, 3),
            Probe::Busy
        );
        assert_eq!(
            classify(
                Err(DurabilityError::RootPassDeadlineExceeded.into()),
                fact,
                3
            ),
            Probe::NotReady
        );
        assert_eq!(
            classify(Err(AssignmentError::NotCurrentHolder), fact, 3),
            Probe::NotReady
        );
    }

    /// Virtual time: sleeps complete only when the test advances the clock.
    #[derive(Clone, Default)]
    struct ManualClock {
        now: Arc<Mutex<Duration>>,
        tick: Arc<tokio::sync::Notify>,
    }

    impl ReportClock for ManualClock {
        fn elapsed(&self) -> Duration {
            *self.now.lock().unwrap()
        }
        fn unix_now(&self) -> i64 {
            PUBLISHED_AT + i64::try_from(self.elapsed().as_secs()).unwrap()
        }
        fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
            let (clock, until) = (self.clone(), self.elapsed() + duration);
            async move {
                loop {
                    let mut notified = pin!(clock.tick.notified());
                    notified.as_mut().enable();
                    let reached = clock.elapsed() >= until;
                    if reached {
                        return;
                    }
                    notified.await;
                }
            }
        }
    }

    impl ManualClock {
        async fn advance(&self, total: Duration) {
            let step = Duration::from_millis(50);
            let mut left = total;
            // Let every task reach its next wait before time moves.
            for _ in 0..32 {
                tokio::task::yield_now().await;
            }
            while !left.is_zero() {
                *self.now.lock().unwrap() += step;
                left = left.saturating_sub(step);
                self.tick.notify_waiters();
                for _ in 0..32 {
                    tokio::task::yield_now().await;
                }
            }
        }
    }

    type Sent = Arc<Mutex<Vec<(Duration, Publication)>>>;

    struct RecordingSink {
        clock: ManualClock,
        sent: Sent,
    }

    impl ReportSink for RecordingSink {
        async fn send(
            &mut self,
            publication: &Publication,
            _observed_at: i64,
        ) -> Result<Delivery, NotDelivered> {
            self.sent
                .lock()
                .unwrap()
                .push((self.clock.elapsed(), publication.clone()));
            Ok(Delivery::Accepted)
        }
    }

    /// The durability connection is busy during `busy`; the assignment is
    /// replaced at `lost_at`. Each call is one non-blocking try.
    struct ScriptedProbe {
        clock: ManualClock,
        busy: std::ops::Range<Duration>,
        lost_at: Duration,
        tries: Arc<Mutex<Vec<Duration>>>,
    }

    impl AssignmentProbe for ScriptedProbe {
        async fn probe(&mut self) -> Probe {
            let now = self.clock.elapsed();
            self.tries.lock().unwrap().push(now);
            if self.busy.contains(&now) {
                Probe::Busy
            } else if now >= self.lost_at {
                Probe::Lost
            } else {
                Probe::Current
            }
        }
    }

    fn secs(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
    }

    #[test]
    fn busy_connection_never_opens_a_gap_beyond_f_and_lost_assignment_stops() {
        runtime().block_on(async {
            let clock = ManualClock::default();
            let sent = Sent::default();
            let tries = Arc::new(Mutex::new(Vec::new()));
            let sink = RecordingSink {
                clock: clock.clone(),
                sent: sent.clone(),
            };
            let probe = ScriptedProbe {
                clock: clock.clone(),
                busy: secs(10)..secs(22),
                lost_at: secs(40),
                tries: tries.clone(),
            };
            assert!(sent.lock().unwrap().is_empty());
            let reporter =
                Reporter::start(clock.clone(), sink, probe, reported(), &committed(true, 1));
            clock.advance(secs(70)).await;
            let times: Vec<_> = sent.lock().unwrap().iter().map(|(at, _)| *at).collect();
            // The committed ready=true publication is sent at once.
            assert_eq!(times[0], Duration::ZERO);
            assert!(
                sent.lock()
                    .unwrap()
                    .iter()
                    .all(|(_, p)| p.ready && p.source_revision == 1)
            );
            for pair in times.windows(2) {
                assert!(pair[1] - pair[0] <= F, "gap {:?}", pair[1] - pair[0]);
            }
            // A sent heartbeat once the busy connection is free again.
            assert!(times.iter().any(|at| (secs(22)..secs(23)).contains(at)));
            // Lost at the first re-check after 40 s: nothing after it.
            let last = *times.last().unwrap();
            assert!(last >= secs(40) && last < secs(46), "{last:?}");
            // Busy tries are non-blocking and bounded by backoff; outside the
            // busy window the assignment is read at most every RECHECK.
            let tries = tries.lock().unwrap().clone();
            let during = tries
                .iter()
                .filter(|at| (secs(10)..secs(23)).contains(*at))
                .count();
            assert!(during <= 20, "{during}");
            let outside: Vec<_> = tries.iter().filter(|at| **at >= secs(23)).collect();
            for pair in outside.windows(2) {
                assert!(*pair[1] - *pair[0] >= RECHECK - Duration::from_millis(100));
            }
            reporter.finish(secs(10)).await;
        });
    }

    #[test]
    fn shutdown_reports_only_a_committed_withdrawal_within_the_budget() {
        for withdrawal_committed in [true, false] {
            runtime().block_on(async {
                let clock = ManualClock::default();
                let sent = Sent::default();
                let sink = RecordingSink {
                    clock: clock.clone(),
                    sent: sent.clone(),
                };
                let probe = ScriptedProbe {
                    clock: clock.clone(),
                    busy: Duration::ZERO..Duration::ZERO,
                    lost_at: secs(1_000),
                    tries: Arc::default(),
                };
                let reporter =
                    Reporter::start(clock.clone(), sink, probe, reported(), &committed(true, 1));
                clock.advance(secs(12)).await;
                reporter.stop_heartbeats();
                let before = sent.lock().unwrap().len();
                clock.advance(secs(12)).await;
                assert_eq!(
                    sent.lock().unwrap().len(),
                    before,
                    "no heartbeat after stop"
                );
                let withdrawal = committed(false, 2);
                reporter.withdrawn(withdrawal_committed.then_some(&withdrawal));
                let finished = Arc::new(AtomicUsize::new(0));
                let done = finished.clone();
                let finish = tokio::spawn(async move {
                    reporter.finish(secs(10)).await;
                    done.store(1, Ordering::SeqCst);
                });
                clock.advance(Duration::from_millis(100)).await;
                assert_eq!(finished.load(Ordering::SeqCst), 1);
                finish.await.unwrap();
                let sent = sent.lock().unwrap();
                assert_eq!(sent.len(), before + usize::from(withdrawal_committed));
                if withdrawal_committed {
                    let (_, last) = sent.last().unwrap();
                    assert!(!last.ready);
                    assert_eq!(last.source_revision, 2);
                }
            });
        }
    }
}
