//! GM-1 executor. Authentication precedes typed decoding; the current session,
//! trusted time, chain successor, effect and audit share the semantic transaction.
pub use oteryn_gameplay_social_rules::staff as rules;
mod executor;
mod expiry;
mod retained;
mod trust;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_item_only_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::staff::{SignedEnvelope, StaffResult, encode_result};
use sqlx::Row;
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
use trust::{RECEIPT_NAMESPACE, REQUEST_NAMESPACE, decode, digest};
pub use trust::{StaffConfig, StaffTrust, TrustError};

pub type TrustedStaffClock = fn() -> Option<(u64, u64)>;
static VERIFIERS: OnceLock<Arc<Semaphore>> = OnceLock::new();
#[derive(Clone)]
struct Authenticated {
    envelope: SignedEnvelope,
    roster: String,
    fingerprint: String,
    message: Message,
}
#[derive(Clone)]
enum Message {
    Request(rules::Request),
    Receipt(rules::Receipt),
}
impl Authenticated {
    fn operation(&self) -> &str {
        match &self.message {
            Message::Request(r) => &r.operation_id,
            Message::Receipt(r) => &r.operation_id,
        }
    }
    fn seq(&self) -> Option<u64> {
        rules::decimal(match &self.message {
            Message::Request(r) => &r.control_seq,
            Message::Receipt(r) => &r.control_seq,
        })
    }
    fn previous(&self) -> &str {
        match &self.message {
            Message::Request(r) => &r.previous_link_sha256,
            Message::Receipt(r) => &r.previous_link_sha256,
        }
    }
    fn activation(&self) -> &str {
        match &self.message {
            Message::Request(r) => &r.activation_sha256,
            Message::Receipt(r) => &r.activation_sha256,
        }
    }
}
impl StaffTrust {
    async fn authenticate_async(
        self: Arc<Self>,
        envelope: SignedEnvelope,
        receipt: bool,
        seq: u64,
    ) -> Option<(String, rules::StaffRole, String)> {
        let permit = VERIFIERS
            .get_or_init(|| Arc::new(Semaphore::new(4)))
            .clone()
            .try_acquire_owned()
            .ok()?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            self.authenticate_at(
                &envelope,
                if receipt {
                    RECEIPT_NAMESPACE
                } else {
                    REQUEST_NAMESPACE
                },
                seq,
            )
            .ok()
        })
        .await
        .ok()?
    }
    async fn authenticate_message(
        self: Arc<Self>,
        envelope: SignedEnvelope,
        receipt: bool,
    ) -> Option<Authenticated> {
        let permit = VERIFIERS
            .get_or_init(|| Arc::new(Semaphore::new(4)))
            .clone()
            .try_acquire_owned()
            .ok()?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let namespace = if receipt {
                RECEIPT_NAMESPACE
            } else {
                REQUEST_NAMESPACE
            };
            self.authenticate_any(&envelope, namespace).ok()?;
            // Only after raw signature verification do we parse the closed object.
            // Re-verify against exactly the revision pinned for its signed sequence.
            let seq = if receipt {
                rules::decimal(
                    &decode::<rules::Receipt>(&envelope.canonical)
                        .ok()?
                        .control_seq,
                )?
            } else {
                rules::decimal(
                    &decode::<rules::Request>(&envelope.canonical)
                        .ok()?
                        .control_seq,
                )?
            };
            let (roster, role, fingerprint) =
                self.authenticate_at(&envelope, namespace, seq).ok()?;
            let message = if receipt {
                let r: rules::Receipt = decode(&envelope.canonical).ok()?;
                if r.environment != self.environment()
                    || r.roster_sha256 != roster
                    || !rules::hex(&r.operation_id, 16)
                    || !rules::hex(&r.previous_link_sha256, 32)
                    || !rules::hex(&r.request_sha256, 32)
                    || !rules::hex(&r.outcome_sha256, 32)
                {
                    return None;
                }
                Message::Receipt(r)
            } else {
                let r: rules::Request = decode(&envelope.canonical).ok()?;
                if !r.valid(role)
                    || r.environment != self.environment()
                    || r.roster_sha256 != roster
                {
                    return None;
                }
                Message::Request(r)
            };
            let auth = Authenticated {
                envelope,
                roster,
                fingerprint,
                message,
            };
            let (activation, at_roster) = self.activation_at(auth.seq()?)?;
            if activation != auth.activation() || at_roster != auth.roster {
                return None;
            }
            Some(auth)
        })
        .await
        .ok()?
    }
}
impl DurabilityRoot {
    pub async fn configure_staff(
        &self,
        config: Option<StaffConfig>,
    ) -> Result<(), DurabilityError> {
        let trust = match config {
            Some(config) => Some(Arc::new(
                tokio::task::spawn_blocking(move || StaffTrust::load(config))
                    .await
                    .map_err(|_| DurabilityError::InvalidStoredState)?
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )),
            None => None,
        };
        if let Some(pinned) = trust.clone() {
            let valid = self.try_issue_semantic_pass()?
                .run(move |holder, deadline| Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let head: Option<(i64, String, String)> = sqlx::query_as(
                        "SELECT control_seq,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment=$1 FOR UPDATE",
                    ).bind(pinned.environment()).fetch_optional(&mut *tx).await?;
                    let Some((seq, digest, activation)) = head else { return Ok(false); };
                    let valid = activation == pinned.activation()
                        && executor::verify_chain(&mut tx, &pinned, seq, &digest).await?
                        && retained::verify_and_retain(&mut tx, &pinned, seq).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(valid)
                })).await?;
            if !valid {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        self.staff_trust
            .set(trust)
            .map_err(|_| DurabilityError::InvalidStoredState)
    }
    pub fn staff_moderation_configured(&self) -> bool {
        self.staff_trust.get().is_some()
    }
    fn staff_trust(&self) -> Option<Arc<StaffTrust>> {
        self.staff_trust.get().and_then(Clone::clone)
    }

    pub async fn submit_staff(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        envelope: SignedEnvelope,
        receipt: bool,
        clock: TrustedStaffClock,
    ) -> Result<Option<Vec<u8>>, DurabilityError> {
        let Some(trust) = self.staff_trust() else {
            return Ok(None);
        };
        let Some(auth) = trust.clone().authenticate_message(envelope, receipt).await else {
            return Ok(None);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let retained_trust = trust.clone();
        let retained_auth = auth.clone();
        let result = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    let result = executor::execute(&mut tx, &trust, &auth, fence, clock).await?;
                    if result.is_some() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?;
        if result.is_some() {
            // Retain only after commit, and before exposing the result. A failure is
            // unknown to the transport; retry re-verifies and repairs retained evidence.
            retained_trust.retain(retained_auth).await?;
        }
        Ok(result)
    }

    /// The gate reads durable truth on each chat/cast, including sessions without chat state.
    pub async fn staff_mute_for_session(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<u64>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let environment = self
            .staff_trust()
            .map(|trust| trust.environment().to_owned());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                    return Ok(None);
                }
                // The pinned environment isolates a configured operator node. A node
                // without operator trust conservatively honours all sanctions in its
                // database; it cannot silently bypass moderation on reconnect.
                let until: Option<i64> = sqlx::query_scalar(
                    "SELECT ends_at_ms FROM game_character_sanctions WHERE character_id=encode($1,'hex')::uuid AND ($2::text IS NULL OR environment=$2) AND lifted_at_ms IS NULL ORDER BY ends_at_ms DESC LIMIT 1",
                ).bind(fence.character_id.as_bytes().as_slice()).bind(environment)
                    .fetch_optional(&mut *tx).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Some(until.unwrap_or(0).try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?))
            })).await
    }
}
