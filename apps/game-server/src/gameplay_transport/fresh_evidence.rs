//! On-demand S2 evidence for one fresh admission attempt (OPS-NODE-BOOT-01 D4).
//!
//! The accepted source age is at most five seconds at the authorization
//! boundary, so each attempt fetches the account-security and signing-trust
//! observations it needs and hands them to S2 custody before composition.
//! Nothing here interprets the facts: S2 retains them and the composition and
//! commit decide.

use crate::admission_evidence::{Request, Response, decode_response, encode_request};
use crate::durability::DurabilityError;
use crate::durability::DurabilityRoot;
use crate::durability::native_admission_source::{
    NativeSourceOperation, NativeSourceSubject, SourceObservation,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::native_admission_source::descriptor::{Operation, ProducerDescriptor};
use crate::native_admission_source::{QueuePermit, TransientCapacity, http1_mtls};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::time::Duration;

const FRESH_ISSUER: &str = "urn:oteryn:platform:game-admission";
const FRESH_PROFILE: &str = "oteryn-pre-admission-v1";
const FRESH_KEY_PURPOSE: &str = "fresh_admission";
const ACCOUNT_PURPOSE: &str = "platform_security";
const ACCOUNT_SCOPE: &str = "fresh_admission";

/// Bounded wait for one of the registered active exchange slots.
const ACTIVATION_ATTEMPTS: u32 = 100;
const ACTIVATION_BACKOFF: Duration = Duration::from_millis(10);

/// Why a refresh did not complete; never a statement about the account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceUnavailable {
    Capacity,
    Source,
    Custody,
}

/// The Platform admission-evidence route and its registered transient capacity.
pub struct FreshEvidenceSource {
    descriptor: ProducerDescriptor,
    capacity: TransientCapacity,
}

impl FreshEvidenceSource {
    #[must_use]
    pub fn new(descriptor: ProducerDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }

    /// Fetches the current account security of `account_id` into S2 custody.
    pub(crate) async fn refresh_account(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        account_id: &str,
    ) -> Result<(), EvidenceUnavailable> {
        let request = Request::Account {
            recovery: false,
            account_id,
            purpose: ACCOUNT_PURPOSE,
            scope: ACCOUNT_SCOPE,
        };
        self.fetch_and_accept(root, custody, &request, Operation::ReadAccountSecurityV1)
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

    async fn fetch_and_accept(
        &self,
        root: &DurabilityRoot,
        custody: &NodeIncarnationProof,
        request: &Request<'_>,
        operation: Operation,
    ) -> Result<(), EvidenceUnavailable> {
        let mut permit = self.activate().await?;
        let encoded = encode_request(request).map_err(|_| EvidenceUnavailable::Source)?;
        let raw = http1_mtls::exchange(&self.descriptor, operation, &encoded, &mut permit)
            .await
            .map_err(|_| EvidenceUnavailable::Source)?;
        let Ok(Response::Observed(observation)) =
            decode_response(request, self.descriptor.source_authority(), &raw)
        else {
            return Err(EvidenceUnavailable::Source);
        };
        let binding = PublicationBinding {
            source_authority: self.descriptor.source_authority().into(),
            subject: match request {
                Request::Account { account_id, .. } => {
                    BindingSubject::Account((*account_id).into())
                }
                Request::Trust { key_id, .. } => BindingSubject::FreshTrust((*key_id).into()),
            },
            source_revision: observation.source_revision,
            decision_identity: observation.decision_identity.as_str().into(),
            observed_at: observation.source_observed_at,
            semantic_facts: raw,
        };
        let checkpoint = binding.encode().ok_or(EvidenceUnavailable::Source)?;
        // Occupied slots are reconciled by their fixed identity before any
        // further work, so a running node recovers them without a restart.
        reconcile_pending_publications(root, custody).await?;
        let now = super::unix_seconds().ok_or(EvidenceUnavailable::Custody)?;
        // NSRC-PENDING-PUBLICATION: the exact binding is checkpointed before
        // SQL submission and the slot stays owned through an ambiguous outcome.
        let slot = root
            .checkpoint_native_source_publication(custody, checkpoint.clone(), now)
            .await
            .map_err(|_| EvidenceUnavailable::Custody)?;
        let submitted = binding.observation().ok_or(EvidenceUnavailable::Source)?;
        let outcome = root
            .accept_native_source_observation(custody, submitted)
            .await;
        if outcome_is_ambiguous(&outcome) {
            // Retained for restart reconciliation; never detached or reused.
            return Err(EvidenceUnavailable::Custody);
        }
        let _ = root
            .clear_native_source_publication(custody, slot, checkpoint)
            .await;
        drop(permit);
        outcome.map_err(|_| EvidenceUnavailable::Custody)
    }

    async fn activate(&self) -> Result<QueuePermit<'_>, EvidenceUnavailable> {
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| EvidenceUnavailable::Capacity)?;
        for attempt in 0..ACTIVATION_ATTEMPTS {
            if permit.try_activate().is_ok() {
                return Ok(permit);
            }
            if attempt + 1 < ACTIVATION_ATTEMPTS {
                tokio::time::sleep(ACTIVATION_BACKOFF).await;
            }
        }
        Err(EvidenceUnavailable::Capacity)
    }
}

/// Replays every retained publication through the idempotent acceptance and
/// releases its slot on a definite outcome. A slot whose binding cannot be read
/// back, or whose outcome is still unknown, stays occupied.
pub(crate) async fn reconcile_pending_publications(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
) -> Result<(), EvidenceUnavailable> {
    let pending = root
        .pending_native_source_publications(custody)
        .await
        .map_err(|_| EvidenceUnavailable::Custody)?;
    for publication in pending {
        let Some(observation) = PublicationBinding::decode(&publication.operation_binding)
            .and_then(|binding| binding.observation())
        else {
            continue;
        };
        let outcome = root
            .accept_native_source_observation(custody, observation)
            .await;
        if outcome_is_ambiguous(&outcome) {
            continue;
        }
        let _ = root
            .clear_native_source_publication(
                custody,
                publication.slot_id,
                publication.operation_binding,
            )
            .await;
    }
    Ok(())
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
        assert_eq!(PublicationBinding::decode(&encoded), Some(binding));
        let mut extra: serde_json::Value = serde_json::from_slice(&encoded).unwrap_or_default();
        if let Some(object) = extra.as_object_mut() {
            object.insert("x".into(), serde_json::Value::Null);
        }
        let extra = serde_json::to_vec(&extra).unwrap_or_default();
        assert_eq!(PublicationBinding::decode(&extra), None);
        assert_eq!(PublicationBinding::decode(b"not json"), None);
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
    }
}
