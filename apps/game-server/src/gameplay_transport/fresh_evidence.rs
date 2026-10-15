//! On-demand S2 evidence for one fresh admission attempt (OPS-NODE-BOOT-01 D4).
//!
//! The accepted source age is at most five seconds at the authorization
//! boundary, so each attempt fetches the account-security and signing-trust
//! observations it needs and hands them to S2 custody before composition.
//! Nothing here interprets the facts: S2 retains them and the composition and
//! commit decide.

use crate::admission_evidence::{Failure, Request, Response, decode_response, encode_request};
use crate::durability::DurabilityError;
use crate::durability::DurabilityRoot;
use crate::durability::native_admission_source::{
    NativeSourceOperation, NativeSourceSubject, SourceObservation,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::native_admission_source::descriptor::{Operation, ProducerDescriptor};
use crate::native_admission_source::{QueuePermit, SourceError, TransientCapacity, http1_mtls};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard, oneshot};
use tokio::task::JoinHandle;

const FRESH_ISSUER: &str = "urn:oteryn:platform:game-admission";
const FRESH_PROFILE: &str = "oteryn-pre-admission-v1";
const FRESH_KEY_PURPOSE: &str = "fresh_admission";
const ACCOUNT_PURPOSE: &str = "platform_security";
const ACCOUNT_SCOPE: &str = "fresh_admission";
const RECOVERY_ISSUER: &str = "urn:oteryn:platform:game-recovery";
const RECOVERY_PROFILE: &str = "oteryn-reauth-recovery-v1";
const RECOVERY_PURPOSE: &str = "existing_actor_recovery";

/// Bounded wait for one of the registered active exchange slots.
const ACTIVATION_ATTEMPTS: u32 = 100;
const ACTIVATION_BACKOFF: Duration = Duration::from_millis(10);
/// The two fixed durable NSRC-PENDING-PUBLICATION slots.
const PUBLICATION_SLOTS: usize = 2;
/// NSRC-QUEUE-WAIT bound for the node's single publication lane.
const PUBLICATION_QUEUE_WAIT: Duration = Duration::from_millis(1_000);
/// NSRC-PUBLISH: one publication operation, database acquisition included.
/// After an acceptance, the slot clear is that operation; its acquisition
/// waits for the caller's read of the accepted evidence, so hold and clear
/// together end within this bound and so does the task's ownership of the
/// lane, the reservation and the completion.
const NSRC_PUBLISH: Duration = Duration::from_millis(2_000);
/// The part of `NSRC_PUBLISH` always left to the clear pass itself. A read
/// that outlasts the rest only means the clear finds the holder busy and the
/// slot, whose outcome is already definite, is released by the next
/// reconciliation instead.
const CLEAR_RESERVE: Duration = Duration::from_millis(1_000);

/// Why a refresh did not complete; never a statement about the account.
///
/// Every field is a fixed token naming the step that failed and its cause, so
/// it can be logged as is: no identifier, credential or response byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvidenceUnavailable {
    step: &'static str,
    cause: &'static str,
    /// The cause of the last retained-slot replay of this demand that did not
    /// release its slot, when one preceded the failure.
    replay: Option<&'static str>,
}

impl EvidenceUnavailable {
    const fn new(step: &'static str, cause: &'static str) -> Self {
        Self {
            step,
            cause,
            replay: None,
        }
    }

    fn durability(step: &'static str, error: &DurabilityError) -> Self {
        Self::new(step, durability_cause(error))
    }

    const fn after(mut self, replay: Option<&'static str>) -> Self {
        if self.replay.is_none() {
            self.replay = replay;
        }
        self
    }
}

impl fmt::Display for EvidenceUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "step={} cause={}", self.step, self.cause)?;
        if let Some(replay) = self.replay {
            write!(f, " replay={replay}")?;
        }
        Ok(())
    }
}

/// A fixed log token for a durability failure.
const fn durability_cause(error: &DurabilityError) -> &'static str {
    match error {
        DurabilityError::AdmissionGameSessionLedgerExhausted => "ledger_exhausted",
        DurabilityError::SessionUseAuthorization(_) => "session_use_authorization",
        DurabilityError::Unavailable => "unavailable",
        DurabilityError::Database(_) => "database",
        DurabilityError::Migration(_) => "migration",
        DurabilityError::SchemaIncompatible(_) => "schema_incompatible",
        DurabilityError::InvalidConfiguration => "invalid_configuration",
        DurabilityError::RootUnavailable => "root_unavailable",
        DurabilityError::RootPassDeadlineExceeded => "pass_deadline",
        DurabilityError::CommitRejected => "commit_rejected",
        DurabilityError::CommitOutcomeUnknown => "commit_unknown",
        DurabilityError::RootTaskFailed => "root_task_failed",
        DurabilityError::InvalidStoredState => "invalid_stored_state",
    }
}

/// A fixed log token for a failed source exchange.
fn source_cause(error: &SourceError) -> &'static str {
    match error {
        SourceError::InvalidDescriptor => "invalid_descriptor",
        SourceError::CapacityExceeded => "capacity_exceeded",
        SourceError::InvalidInput => "invalid_input",
        SourceError::Unauthorized => "unauthorized",
        SourceError::Unavailable => "unavailable",
        SourceError::OversizedOk => "oversized_ok",
        SourceError::Io(error) if error.kind() == std::io::ErrorKind::TimedOut => "io_timeout",
        SourceError::Io(_) => "io",
        SourceError::Tls(_) => "tls",
    }
}

/// A fixed log token for a decoded response that carries no observation.
const fn response_cause(response: &Response) -> &'static str {
    match response {
        Response::Observed(_) => "observed",
        Response::Failure(Failure::NotFound) => "not_found",
        Response::Failure(Failure::Unavailable) => "unavailable",
        Response::Failure(Failure::Unauthorized) => "unauthorized",
        Response::Failure(Failure::Unsupported) => "unsupported",
    }
}

/// One accepted observation whose publication slot is not cleared yet.
///
/// The caller reads the accepted evidence on the otherwise idle durability
/// holder, then releases it; only then does the publication task clear the
/// slot. Dropping it releases too, so a cancelled caller never strands the
/// lane, and the task stops waiting and clears within `NSRC_PUBLISH` of the
/// acceptance regardless.
#[must_use]
pub(crate) struct HeldEvidence {
    release: oneshot::Sender<()>,
    task: JoinHandle<Result<(), EvidenceUnavailable>>,
}

impl HeldEvidence {
    /// Lets the publication task clear the slot and waits for it to end.
    pub(crate) async fn release(self) -> Result<(), EvidenceUnavailable> {
        let Self { release, task } = self;
        drop(release);
        join_publication(task).await
    }
}

/// The Platform admission-evidence route and its registered transient capacity.
pub struct FreshEvidenceSource {
    descriptor: ProducerDescriptor,
    capacity: TransientCapacity,
    /// Serializes every S2 publication of this node. The guard is owned by the
    /// spawned publication task, so a cancelled caller cannot release it while
    /// its SQL work is still outstanding, and reconciliation never sees a live
    /// slot of this process.
    publication: Arc<Mutex<()>>,
    /// Slots reserved in this process for demands between their pre-exchange
    /// reconciliation and the end of their publication task.
    reserved: Arc<AtomicUsize>,
}

/// One reserved end-to-end publication slot, released when dropped by the
/// publication task (or by a demand that ends before submitting SQL).
struct SlotReservation(Arc<AtomicUsize>);
impl Drop for SlotReservation {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl FreshEvidenceSource {
    #[must_use]
    pub fn new(descriptor: ProducerDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
            publication: Arc::new(Mutex::new(())),
            reserved: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Fetches the current account security of `account_id` into S2 custody.
    pub(crate) async fn refresh_account(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        account_id: &str,
    ) -> Result<(), EvidenceUnavailable> {
        self.refresh_account_held(root, custody, account_id)
            .await?
            .release()
            .await
    }

    /// Like [`Self::refresh_account`], but returns as soon as the observation
    /// is accepted, before its slot clear, so the caller's decision is the
    /// next pass on the holder and reads the evidence one pass younger.
    pub(crate) async fn refresh_account_held(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        account_id: &str,
    ) -> Result<HeldEvidence, EvidenceUnavailable> {
        let request = Request::Account {
            recovery: false,
            account_id,
            purpose: ACCOUNT_PURPOSE,
            scope: ACCOUNT_SCOPE,
        };
        self.fetch_and_accept_held(root, custody, &request, Operation::ReadAccountSecurityV1)
            .await
    }

    /// Fetches the account security and the fresh signing trust of `key_id`.
    pub(crate) async fn refresh_fresh_admission(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        account_id: &str,
        key_id: &str,
    ) -> Result<(), EvidenceUnavailable> {
        self.refresh_account(root, custody, account_id).await?;
        let request = Request::Trust {
            recovery: false,
            key_id,
            key_purpose: FRESH_KEY_PURPOSE,
        };
        self.fetch_and_accept(root, custody, &request, Operation::ReadFreshSigningTrustV1)
            .await
    }

    /// Fetches the Recovery V2 account security and the recovery signing trust of
    /// `key_id` into S2 custody, for a same-session recovery decision (FND-04B §20).
    pub(crate) async fn refresh_recovery(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        account_id: &str,
        key_id: &str,
    ) -> Result<(), EvidenceUnavailable> {
        let account = Request::Account {
            recovery: true,
            account_id,
            purpose: ACCOUNT_PURPOSE,
            scope: RECOVERY_PURPOSE,
        };
        self.fetch_and_accept(
            root,
            custody,
            &account,
            Operation::ReadRecoveryAccountSecurityV2,
        )
        .await?;
        let trust = Request::Trust {
            recovery: true,
            key_id,
            key_purpose: RECOVERY_PURPOSE,
        };
        self.fetch_and_accept(root, custody, &trust, Operation::ReadRecoverySigningTrustV2)
            .await
    }

    async fn fetch_and_accept(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        request: &Request<'_>,
        operation: Operation,
    ) -> Result<(), EvidenceUnavailable> {
        self.fetch_and_accept_held(root, custody, request, operation)
            .await?
            .release()
            .await
    }

    async fn fetch_and_accept_held(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        request: &Request<'_>,
        operation: Operation,
    ) -> Result<HeldEvidence, EvidenceUnavailable> {
        // Retained slots are reconciled before any new source work, so no
        // exchange starts while no end-to-end publication slot is available.
        let reservation = self.reserve_before_exchange(root, custody).await?;
        let mut permit = self.activate().await?;
        let encoded =
            encode_request(request).map_err(|_| EvidenceUnavailable::new("encode", "request"))?;
        let raw = http1_mtls::exchange(&self.descriptor, operation, &encoded, &mut permit)
            .await
            .map_err(|error| EvidenceUnavailable::new("exchange", source_cause(&error)))?;
        let observation = match decode_response(request, self.descriptor.source_authority(), &raw) {
            Ok(Response::Observed(observation)) => observation,
            Ok(response) => {
                return Err(EvidenceUnavailable::new(
                    "response",
                    response_cause(&response),
                ));
            }
            Err(_) => return Err(EvidenceUnavailable::new("response", "undecodable")),
        };
        let binding = PublicationBinding {
            source_authority: self.descriptor.source_authority().into(),
            subject: match request {
                Request::Account {
                    recovery: false,
                    account_id,
                    ..
                } => BindingSubject::Account((*account_id).into()),
                Request::Account {
                    recovery: true,
                    account_id,
                    ..
                } => BindingSubject::RecoveryAccount((*account_id).into()),
                Request::Trust {
                    recovery: false,
                    key_id,
                    ..
                } => BindingSubject::FreshTrust((*key_id).into()),
                Request::Trust {
                    recovery: true,
                    key_id,
                    ..
                } => BindingSubject::RecoveryTrust((*key_id).into()),
            },
            source_revision: observation.source_revision,
            decision_identity: observation.decision_identity.as_str().into(),
            observed_at: observation.source_observed_at,
            semantic_facts: raw,
        };
        let checkpoint = binding
            .encode()
            .ok_or(EvidenceUnavailable::new("binding", "encode"))?;
        let lane = self.enter_publication_lane().await?;
        let (accepted, accepted_signal) = oneshot::channel();
        let (release, release_signal) = oneshot::channel();
        let task = root.spawn_task(publish(
            Publication {
                root: root.clone(),
                custody: custody.clone(),
                binding,
                checkpoint,
                lane,
                reservation,
            },
            accepted,
            release_signal,
        ));
        let held = accepted_signal.await.is_ok();
        drop(permit);
        if held {
            return Ok(HeldEvidence { release, task });
        }
        drop(release);
        match join_publication(task).await {
            Err(unavailable) => Err(unavailable),
            Ok(()) => Err(EvidenceUnavailable::new("publication", "not_held")),
        }
    }

    /// Boot reconciliation (OPS-NODE-BOOT-01 D3 step 4): inside the lane,
    /// replays every retained publication slot by its fixed identity. Returns
    /// the number of slots still retained.
    pub(crate) async fn reconcile_retained(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
    ) -> Result<usize, EvidenceUnavailable> {
        let lane = self.enter_publication_lane().await?;
        let (root_task, custody) = (root.clone(), custody.clone());
        run_on_root(root, async move {
            let _lane = lane;
            reconcile_pending_publications(&root_task, &custody)
                .await
                .map(|reconciled| reconciled.retained)
        })
        .await
    }

    /// Reconciles retained slots inside the lane and reserves one free slot
    /// before the lane is left, so concurrent demands never overbook.
    async fn reserve_before_exchange(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
    ) -> Result<SlotReservation, EvidenceUnavailable> {
        let lane = self.enter_publication_lane().await?;
        let (root_task, custody) = (root.clone(), custody.clone());
        let reserved = self.reserved.clone();
        run_on_root(root, async move {
            let _lane = lane;
            let reconciled = reconcile_pending_publications(&root_task, &custody).await?;
            if reconciled.retained + reserved.load(Ordering::Acquire) >= PUBLICATION_SLOTS {
                return Err(
                    EvidenceUnavailable::new("reserve", "slots_retained").after(reconciled.replay)
                );
            }
            reserved.fetch_add(1, Ordering::AcqRel);
            Ok(SlotReservation(reserved))
        })
        .await
    }

    async fn enter_publication_lane(&self) -> Result<OwnedMutexGuard<()>, EvidenceUnavailable> {
        tokio::time::timeout(
            PUBLICATION_QUEUE_WAIT,
            self.publication.clone().lock_owned(),
        )
        .await
        .map_err(|_| EvidenceUnavailable::new("lane", "queue_wait"))
    }

    async fn activate(&self) -> Result<QueuePermit<'_>, EvidenceUnavailable> {
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| EvidenceUnavailable::new("activate", "queue_full"))?;
        for attempt in 0..ACTIVATION_ATTEMPTS {
            if permit.try_activate().is_ok() {
                return Ok(permit);
            }
            if attempt + 1 < ACTIVATION_ATTEMPTS {
                tokio::time::sleep(ACTIVATION_BACKOFF).await;
            }
        }
        Err(EvidenceUnavailable::new("activate", "no_active_slot"))
    }
}

/// Runs publication work detached from the caller, so cancelling the caller
/// cannot end it early or release its lane.
async fn run_on_root<T, F>(root: &DurabilityRoot, work: F) -> Result<T, EvidenceUnavailable>
where
    T: Send + 'static,
    F: Future<Output = Result<T, EvidenceUnavailable>> + Send + 'static,
{
    join_publication(root.spawn_task(work)).await
}

async fn join_publication<T>(
    task: JoinHandle<Result<T, EvidenceUnavailable>>,
) -> Result<T, EvidenceUnavailable> {
    match task.await {
        Ok(result) => result,
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(_) => Err(EvidenceUnavailable::new("publication", "task_failed")),
    }
}

/// What one publication task owns until it ends.
struct Publication {
    root: DurabilityRoot,
    custody: NodeIncarnationProof,
    binding: PublicationBinding,
    checkpoint: Vec<u8>,
    lane: OwnedMutexGuard<()>,
    reservation: SlotReservation,
}

/// One publication inside the lane: reconcile, checkpoint, accept, hand the
/// acceptance to the caller, clear.
async fn publish(
    publication: Publication,
    accepted: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
) -> Result<(), EvidenceUnavailable> {
    let Publication {
        root,
        custody,
        binding,
        checkpoint,
        lane,
        reservation,
    } = publication;
    let _lane = lane;
    // The reserved slot is released only when this task ends; an ambiguous
    // outcome leaves the durable slot occupied, which the next reconciliation
    // counts as retained.
    let _reservation = reservation;
    // Occupied slots are reconciled by their fixed identity before any
    // further work, so a running node recovers them without a restart. The
    // lane guarantees their original operation has ended in this process.
    let reconciled = reconcile_pending_publications(&root, &custody).await?;
    let now = super::unix_seconds().ok_or(EvidenceUnavailable::new("clock", "unavailable"))?;
    // NSRC-PENDING-PUBLICATION: the exact binding is checkpointed before
    // SQL submission and the slot stays owned through an ambiguous outcome.
    let slot = root
        .checkpoint_native_source_publication(&custody, checkpoint.clone(), now)
        .await
        .map_err(|error| {
            EvidenceUnavailable::durability("checkpoint", &error).after(reconciled.replay)
        })?;
    let submitted = binding
        .observation()
        .ok_or(EvidenceUnavailable::new("binding", "observation"))?;
    let outcome = root
        .accept_native_source_observation(&custody, submitted)
        .await;
    if let Err(error) = &outcome
        && outcome_is_ambiguous(&outcome)
    {
        // Retained for reconciliation; never detached or reused.
        return Err(EvidenceUnavailable::durability("accept", error).after(reconciled.replay));
    }
    let _ = if outcome.is_ok() {
        // Decide-time ordering: the accepted evidence is read before this
        // clear takes the holder. The slot already has its definite outcome.
        let bounds = HeldClear::starting(Instant::now());
        hand_over(accepted, release, bounds.hold_until).await;
        root.clear_native_source_publication_by(&custody, slot, checkpoint, bounds.clear_by)
            .await
    } else {
        root.clear_native_source_publication(&custody, slot, checkpoint)
            .await
    };
    outcome
        .map_err(|error| EvidenceUnavailable::durability("accept", &error).after(reconciled.replay))
}

/// The NSRC-PUBLISH budget of the clear that follows an acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeldClear {
    /// The caller's read of the accepted evidence is awaited until here.
    hold_until: Instant,
    /// The clear pass, acquisition included, ends by here.
    clear_by: Instant,
}

impl HeldClear {
    fn starting(accepted_at: Instant) -> Self {
        let clear_by = accepted_at + NSRC_PUBLISH;
        Self {
            hold_until: clear_by - CLEAR_RESERVE,
            clear_by,
        }
    }
}

/// Tells the caller its observation is accepted, then waits until the caller
/// releases it, drops it, or `hold_until` passes. Returns at once when the
/// caller is gone.
async fn hand_over(
    accepted: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
    hold_until: Instant,
) {
    if accepted.send(()).is_ok() {
        let _ = tokio::time::timeout_at(tokio::time::Instant::from_std(hold_until), release).await;
    }
}

/// Retained slots after one reconciliation, and the cause of its last replay
/// that kept a slot, if any.
struct Reconciled {
    retained: usize,
    replay: Option<&'static str>,
}

/// Replays every retained publication through the idempotent acceptance and
/// releases its slot on a definite outcome. A slot whose binding cannot be read
/// back, or whose outcome is still unknown, stays occupied.
async fn reconcile_pending_publications(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
) -> Result<Reconciled, EvidenceUnavailable> {
    let pending = root
        .pending_native_source_publications(custody)
        .await
        .map_err(|error| EvidenceUnavailable::durability("pending", &error))?;
    let mut reconciled = Reconciled {
        retained: 0,
        replay: None,
    };
    for publication in pending {
        let Some(observation) = PublicationBinding::decode(&publication.operation_binding)
            .and_then(|binding| binding.observation())
        else {
            reconciled.retained += 1;
            reconciled.replay = Some("binding_unreadable");
            continue;
        };
        let outcome = root
            .accept_native_source_observation(custody, observation)
            .await;
        if let Err(error) = &outcome
            && outcome_is_ambiguous(&outcome)
        {
            reconciled.retained += 1;
            reconciled.replay = Some(durability_cause(error));
            continue;
        }
        let cleared = root
            .clear_native_source_publication(
                custody,
                publication.slot_id,
                publication.operation_binding,
            )
            .await;
        if let Err(error) = cleared {
            reconciled.retained += 1;
            reconciled.replay = Some(durability_cause(&error));
        }
    }
    Ok(reconciled)
}

/// A committed outcome is definite only when the store says so: an unknown
/// commit or an expired pass may have landed and keeps its slot.
fn outcome_is_ambiguous(outcome: &Result<(), DurabilityError>) -> bool {
    matches!(
        outcome,
        Err(DurabilityError::CommitOutcomeUnknown
            | DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::RootTaskFailed)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingSubject {
    Account(String),
    FreshTrust(String),
    RecoveryAccount(String),
    RecoveryTrust(String),
}

/// The exact immutable observation a slot owns, enough to replay the
/// idempotent acceptance after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublicationBinding {
    source_authority: String,
    subject: BindingSubject,
    source_revision: u64,
    decision_identity: String,
    observed_at: i64,
    semantic_facts: Vec<u8>,
}

impl PublicationBinding {
    fn encode(&self) -> Option<Vec<u8>> {
        let (kind, id) = match &self.subject {
            BindingSubject::Account(id) => ("account", id),
            BindingSubject::FreshTrust(id) => ("fresh_trust", id),
            BindingSubject::RecoveryAccount(id) => ("recovery_account", id),
            BindingSubject::RecoveryTrust(id) => ("recovery_trust", id),
        };
        serde_json::to_vec(&serde_json::json!({
            "v": 1,
            "authority": self.source_authority,
            "kind": kind,
            "id": id,
            "revision": self.source_revision.to_string(),
            "decision": self.decision_identity,
            "observed_at": self.observed_at,
            "facts": STANDARD.encode(&self.semantic_facts),
        }))
        .ok()
    }

    /// Read back a retained checkpoint for reconciliation.
    pub(crate) fn decode(bytes: &[u8]) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let object = value.as_object()?;
        if object.len() != 8 || object.get("v")?.as_u64()? != 1 {
            return None;
        }
        let text = |key: &str| object.get(key)?.as_str().map(str::to_owned);
        let id = text("id")?;
        let subject = match object.get("kind")?.as_str()? {
            "account" => BindingSubject::Account(id),
            "fresh_trust" => BindingSubject::FreshTrust(id),
            "recovery_account" => BindingSubject::RecoveryAccount(id),
            "recovery_trust" => BindingSubject::RecoveryTrust(id),
            _ => return None,
        };
        Some(Self {
            source_authority: text("authority")?,
            subject,
            source_revision: text("revision")?.parse().ok()?,
            decision_identity: text("decision")?,
            observed_at: object.get("observed_at")?.as_i64()?,
            semantic_facts: STANDARD.decode(object.get("facts")?.as_str()?).ok()?,
        })
    }

    /// The observation submitted for this binding, rebuilt identically on replay.
    pub(crate) fn observation(&self) -> Option<SourceObservation> {
        let (operation, subject) = match &self.subject {
            BindingSubject::Account(id) => (
                NativeSourceOperation::ReadAccountSecurityV1,
                NativeSourceSubject::account_security(id.clone()).ok()?,
            ),
            BindingSubject::FreshTrust(id) => (
                NativeSourceOperation::ReadFreshSigningTrustV1,
                NativeSourceSubject::signing_trust(
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    FRESH_KEY_PURPOSE,
                    id.clone(),
                )
                .ok()?,
            ),
            BindingSubject::RecoveryAccount(id) => (
                NativeSourceOperation::ReadRecoveryAccountSecurityV2,
                NativeSourceSubject::account_security(id.clone()).ok()?,
            ),
            BindingSubject::RecoveryTrust(id) => (
                NativeSourceOperation::ReadRecoverySigningTrustV2,
                NativeSourceSubject::signing_trust(
                    RECOVERY_ISSUER,
                    RECOVERY_PROFILE,
                    RECOVERY_PURPOSE,
                    id.clone(),
                )
                .ok()?,
            ),
        };
        Some(SourceObservation {
            source_authority: self.source_authority.clone(),
            operation,
            subject,
            source_revision: self.source_revision,
            decision_identity: self.decision_identity.clone(),
            observed_at: self.observed_at,
            semantic_facts: self.semantic_facts.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_binding_round_trips_exactly_and_rejects_extras() {
        let binding = PublicationBinding {
            source_authority: "platform".into(),
            subject: BindingSubject::FreshTrust("key-1".into()),
            source_revision: u64::MAX,
            decision_identity: "decision:1".into(),
            observed_at: 1_700_000_000,
            semantic_facts: br#"{"version":1}"#.to_vec(),
        };
        let encoded = binding.encode();
        assert!(encoded.is_some());
        let encoded = encoded.unwrap_or_default();
        assert_eq!(PublicationBinding::decode(&encoded), Some(binding.clone()));
        let mut extra: serde_json::Value = serde_json::from_slice(&encoded).unwrap_or_default();
        if let Some(object) = extra.as_object_mut() {
            object.insert("x".into(), serde_json::Value::Null);
        }
        let extra = serde_json::to_vec(&extra).unwrap_or_default();
        assert_eq!(PublicationBinding::decode(&extra), None);
        assert_eq!(PublicationBinding::decode(b"not json"), None);
        // Recovery bindings replay as their own Recovery V2 operations and subjects.
        for (subject, operation) in [
            (
                BindingSubject::RecoveryAccount("01934f10-7c00-7000-8000-000000000001".into()),
                NativeSourceOperation::ReadRecoveryAccountSecurityV2,
            ),
            (
                BindingSubject::RecoveryTrust("recovery-1".into()),
                NativeSourceOperation::ReadRecoverySigningTrustV2,
            ),
        ] {
            let binding = PublicationBinding {
                subject,
                ..binding.clone()
            };
            let encoded = binding.encode().unwrap_or_default();
            assert_eq!(PublicationBinding::decode(&encoded), Some(binding.clone()));
            assert_eq!(
                binding
                    .observation()
                    .map(|observation| observation.operation),
                Some(operation)
            );
        }
    }

    #[test]
    fn only_unknown_commit_outcomes_keep_the_slot() {
        assert!(outcome_is_ambiguous(&Err(
            DurabilityError::CommitOutcomeUnknown
        )));
        assert!(outcome_is_ambiguous(&Err(
            DurabilityError::RootPassDeadlineExceeded
        )));
        assert!(!outcome_is_ambiguous(&Ok(())));
        assert!(!outcome_is_ambiguous(&Err(DurabilityError::Unavailable)));
        // A pass that was never issued submitted nothing: definite.
        assert!(!outcome_is_ambiguous(&Err(
            DurabilityError::RootUnavailable
        )));
    }

    #[test]
    fn failure_reasons_are_fixed_log_tokens() {
        let causes = [
            durability_cause(&DurabilityError::AdmissionGameSessionLedgerExhausted),
            durability_cause(&DurabilityError::Unavailable),
            durability_cause(&DurabilityError::InvalidConfiguration),
            durability_cause(&DurabilityError::RootUnavailable),
            durability_cause(&DurabilityError::RootPassDeadlineExceeded),
            durability_cause(&DurabilityError::CommitRejected),
            durability_cause(&DurabilityError::CommitOutcomeUnknown),
            durability_cause(&DurabilityError::RootTaskFailed),
            durability_cause(&DurabilityError::InvalidStoredState),
            source_cause(&SourceError::InvalidDescriptor),
            source_cause(&SourceError::CapacityExceeded),
            source_cause(&SourceError::InvalidInput),
            source_cause(&SourceError::Unauthorized),
            source_cause(&SourceError::Unavailable),
            source_cause(&SourceError::OversizedOk),
            source_cause(&SourceError::Io(std::io::ErrorKind::TimedOut.into())),
            source_cause(&SourceError::Io(std::io::ErrorKind::ConnectionReset.into())),
            response_cause(&Response::Failure(Failure::NotFound)),
            response_cause(&Response::Failure(Failure::Unavailable)),
            response_cause(&Response::Failure(Failure::Unauthorized)),
            response_cause(&Response::Failure(Failure::Unsupported)),
        ];
        for cause in causes {
            assert!(!cause.is_empty());
            assert!(
                cause
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
                "{cause}"
            );
        }
        assert_eq!(
            durability_cause(&DurabilityError::RootPassDeadlineExceeded),
            "pass_deadline"
        );
        assert_eq!(
            durability_cause(&DurabilityError::CommitOutcomeUnknown),
            "commit_unknown"
        );
        assert_eq!(
            source_cause(&SourceError::Io(std::io::ErrorKind::TimedOut.into())),
            "io_timeout"
        );
    }

    #[test]
    fn unavailable_names_its_step_cause_and_prior_replay() {
        let plain =
            EvidenceUnavailable::durability("checkpoint", &DurabilityError::RootUnavailable);
        assert_eq!(plain.to_string(), "step=checkpoint cause=root_unavailable");
        let after = plain
            .after(Some("commit_unknown"))
            .after(Some("pass_deadline"));
        assert_eq!(
            after.to_string(),
            "step=checkpoint cause=root_unavailable replay=commit_unknown"
        );
        assert_eq!(
            EvidenceUnavailable::new("lane", "queue_wait")
                .after(None)
                .to_string(),
            "step=lane cause=queue_wait"
        );
    }

    fn runtime() -> Option<tokio::runtime::Runtime> {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .ok()
    }

    #[test]
    fn hand_over_defers_the_clear_until_the_caller_releases() {
        let runtime = runtime();
        assert!(runtime.is_some());
        let Some(runtime) = runtime else {
            return;
        };
        runtime.block_on(async {
            // Released by the caller: returns once the caller lets go.
            let (accepted, accepted_signal) = oneshot::channel();
            let (release, release_signal) = oneshot::channel::<()>();
            let task = tokio::spawn(hand_over(
                accepted,
                release_signal,
                Instant::now() + Duration::from_secs(60),
            ));
            assert!(accepted_signal.await.is_ok());
            tokio::task::yield_now().await;
            assert!(!task.is_finished());
            drop(release);
            assert!(
                tokio::time::timeout(Duration::from_secs(5), task)
                    .await
                    .is_ok()
            );

            // Caller gone before the hand-over: no wait at all.
            let (accepted, accepted_signal) = oneshot::channel();
            let (_release, release_signal) = oneshot::channel::<()>();
            drop(accepted_signal);
            assert!(
                tokio::time::timeout(
                    Duration::from_secs(5),
                    hand_over(
                        accepted,
                        release_signal,
                        Instant::now() + Duration::from_secs(60)
                    ),
                )
                .await
                .is_ok()
            );

            // A caller that never releases is bounded by the hold.
            let (accepted, accepted_signal) = oneshot::channel();
            let (_release, release_signal) = oneshot::channel::<()>();
            let started = Instant::now();
            hand_over(
                accepted,
                release_signal,
                started + Duration::from_millis(20),
            )
            .await;
            assert!(accepted_signal.await.is_ok());
            assert!(started.elapsed() >= Duration::from_millis(20));
        });
    }

    #[test]
    fn held_clear_ends_within_one_publication_operation() {
        assert_eq!(NSRC_PUBLISH, Duration::from_millis(2_000));
        assert!(CLEAR_RESERVE > Duration::ZERO && CLEAR_RESERVE < NSRC_PUBLISH);
        let accepted_at = Instant::now();
        let bounds = HeldClear::starting(accepted_at);
        // Hold and clear (acquisition included) end within NSRC-PUBLISH of
        // the acceptance, and the clear always keeps its reserve.
        assert_eq!(bounds.clear_by, accepted_at + NSRC_PUBLISH);
        assert!(bounds.hold_until > accepted_at);
        assert_eq!(bounds.hold_until + CLEAR_RESERVE, bounds.clear_by);
        assert!(bounds.hold_until - accepted_at + CLEAR_RESERVE <= NSRC_PUBLISH);
    }
}
