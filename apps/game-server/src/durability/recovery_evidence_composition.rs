//! Registered Recovery V2 credential verification from current acknowledged S2
//! owners. The sealed source exists only while custody/registration locks are
//! held; neither it nor a trust context can escape this module.
//!
//! Returned facts record credential verification, not current actor/controller
//! authority. PREPARE, COMMIT and controller adoption require their own current
//! locked owner resolution. This read does not certify a whole-database restore.
//!
//! ```compile_fail
//! use oteryn_game_server::durability::recovery_evidence_composition::RecoveryEvidenceComposition;
//! ```

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::native_admission_source::{NativeSourceOperation, NativeSourceSubject, fence_custody};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_game_server::admission_evidence::{Facts, Request, Response, decode_response};
use oteryn_game_server::foundation::fnd04_verifier::{
    FreshEvidenceProvenanceV1, FreshEvidencePurposeV1, MAX_COMPACT_JWS_BYTES,
    RecoveryAccountSecurityObservationV2, RecoveryCurrentEvidence,
    RecoveryDurabilityEvidenceSourceV2, RecoveryDurabilityTrustContextV2,
    RecoverySigningTrustObservationV2, VerifiedRecoveryDurabilityFactsV2, recovery_source_sealed,
    verify_recovery_grant_durability_v2,
};
use oteryn_game_server::foundation::{Fnd04ConsumerError, Fnd04EvidenceError, Fnd04EvidenceScope};
use sqlx::{Postgres, Row, Transaction};
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, DurabilityError>;
type CredentialResult = std::result::Result<VerifiedRecoveryDurabilityFactsV2, Fnd04ConsumerError>;
const ISSUER: &str = "urn:oteryn:platform:game-recovery";
const PROFILE: &str = "oteryn-reauth-recovery-v1";
const RECOVERY: &str = "existing_actor_recovery";

/// Expected credential bindings selected by the Game Recovery path. Syntax
/// validation does not prove eligibility, ownership or controller authority.
/// Source authority, ordinals and purpose are never caller-selected.
#[derive(Debug, Clone)]
pub struct RecoveryEvidenceSubject {
    account_id: String,
    signing_key_id: String,
}

impl RecoveryEvidenceSubject {
    pub fn new(account_id: impl Into<String>, signing_key_id: impl Into<String>) -> Result<Self> {
        let subject = Self {
            account_id: account_id.into(),
            signing_key_id: signing_key_id.into(),
        };
        // Existing S2 canonical AccountId/key-ID wire shapes, before cloning.
        if subject.account_id.len() != 36 || subject.signing_key_id.len() > 64 {
            return Err(DurabilityError::Unavailable);
        }
        subject.account()?;
        subject.trust()?;
        Ok(subject)
    }

    fn account(&self) -> Result<NativeSourceSubject> {
        NativeSourceSubject::account_security(self.account_id.clone())
    }

    fn trust(&self) -> Result<NativeSourceSubject> {
        NativeSourceSubject::signing_trust(ISSUER, PROFILE, RECOVERY, self.signing_key_id.clone())
    }
}

enum Verification {
    Grant(String),
    Revalidate(Box<VerifiedRecoveryDurabilityFactsV2>),
}

impl DurabilityRoot {
    /// Verify with independently current S2 floors. The inner result is the
    /// Foundation credential disposition; the outer result covers storage,
    /// custody and commit. Time is sampled after all asynchronous owner reads.
    /// `current` supplies inert expected signed bindings, not S2-owned actor facts.
    pub async fn verify_registered_recovery(
        &self,
        custody: &NodeIncarnationProof,
        subject: &RecoveryEvidenceSubject,
        token: &str,
        current: &RecoveryCurrentEvidence,
    ) -> Result<CredentialResult> {
        // Foundation's compact JWS bound; avoid retaining an unbounded caller
        // string while waiting for the accounted database pass.
        if token.len() > MAX_COMPACT_JWS_BYTES || !bounded_bindings(current) {
            return Ok(Err(Fnd04ConsumerError::RecoveryMalformed));
        }
        self.check_registered_recovery(
            custody,
            subject.clone(),
            current.clone(),
            Verification::Grant(token.into()),
        )
        .await
    }

    /// Resolve current owners again; retained verified facts cannot certify
    /// that N remains current after an acknowledged N+1 denial or key change.
    pub async fn revalidate_registered_recovery(
        &self,
        custody: &NodeIncarnationProof,
        verified: &VerifiedRecoveryDurabilityFactsV2,
        current: &RecoveryCurrentEvidence,
    ) -> Result<CredentialResult> {
        if !bounded_bindings(current)
            || verified.security().account_id.len() != 36
            || verified.signing().key_id.len() > 64
        {
            return Ok(Err(Fnd04ConsumerError::RecoveryMalformed));
        }
        let subject = RecoveryEvidenceSubject::new(
            verified.security().account_id.clone(),
            verified.signing().key_id.clone(),
        )?;
        self.check_registered_recovery(
            custody,
            subject,
            current.clone(),
            Verification::Revalidate(Box::new(verified.clone())),
        )
        .await
    }

    async fn check_registered_recovery(
        &self,
        custody: &NodeIncarnationProof,
        subject: RecoveryEvidenceSubject,
        current: RecoveryCurrentEvidence,
        verification: Verification,
    ) -> Result<CredentialResult> {
        let custody = custody.clone();
        self.try_issue_semantic_pass()?
            .run(move |pass, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(pass, deadline).await?;
                    // Same order as every S2 writer: Node current/proof, source
                    // registration, Account floor, set-wide Recovery trust floor.
                    fence_custody(&mut tx, &custody).await?;
                    let authority = registered_authority(&mut tx).await?;
                    let source = resolve_current(&mut tx, &authority, &subject).await?;
                    let now = server_now()?;
                    let result = {
                        let context = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
                        // Synchronous bounded verifier; no IO or nested writer pass.
                        match verification {
                            Verification::Grant(token) => {
                                verify_recovery_grant_durability_v2(&token, now, &context, &current)
                            }
                            Verification::Revalidate(verified) => {
                                verified.revalidate(now, &context, &current)
                            }
                        }
                    };
                    drop(source);
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}

fn bounded_bindings(current: &RecoveryCurrentEvidence) -> bool {
    // Necessary wire bounds already enforced by Foundation's canonical UUID
    // and valid_revision codecs; this does not grant authority to these DTOs.
    current.account_id.len() == 36
        && [
            &current.ruleset_revision,
            &current.content_revision,
            &current.map_revision,
            &current.world_policy_revision,
        ]
        .iter()
        .all(|revision| (1..=64).contains(&revision.len()))
}

fn server_now() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DurabilityError::Unavailable)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| DurabilityError::Unavailable)
}

async fn registered_authority(tx: &mut Transaction<'_, Postgres>) -> Result<String> {
    // Custody already holds this row FOR UPDATE. Check installed descriptor
    // against immutable history and its independently issued authorization.
    let row = sqlx::query(
        "SELECT r.source_authority, \
         EXISTS (SELECT 1 FROM game_durability_native_source_descriptor_history h \
           JOIN game_native_source_descriptor_issuances i USING (descriptor_revision) \
           WHERE h.registration_id=r.registration_id AND h.descriptor_revision=r.descriptor_revision \
             AND h.descriptor_facts=r.descriptor_facts AND i.descriptor_facts=h.descriptor_facts \
             AND i.installed_at=h.installed_at AND i.source_authority=r.source_authority) \
         AND NOT EXISTS (SELECT 1 FROM game_durability_native_source_descriptor_history h \
           WHERE h.registration_id=r.registration_id AND h.descriptor_revision>r.descriptor_revision) \
         FROM game_durability_native_source_registration r WHERE registration_id=1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    if !row.try_get::<bool, _>(1)? {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(row.try_get(0)?)
}

async fn load_floor(
    tx: &mut Transaction<'_, Postgres>,
    authority: &str,
    operation: NativeSourceOperation,
    subject: &NativeSourceSubject,
    request: &Request<'_>,
    purpose: FreshEvidencePurposeV1,
) -> Result<(Facts, FreshEvidenceProvenanceV1)> {
    let row = sqlx::query(
        "SELECT f.source_revision::text, f.operation, f.observation_subject, f.signing_key_id, \
         f.decision_identity, f.observed_at, f.semantic_facts, \
         EXISTS (SELECT 1 FROM game_durability_native_source_observation_history h \
           WHERE h.registration_id=f.registration_id AND h.source_authority=f.source_authority \
             AND h.floor_subject=f.floor_subject AND h.source_revision=f.source_revision \
             AND h.operation=f.operation AND h.observation_subject=f.observation_subject \
             AND h.signing_key_id IS NOT DISTINCT FROM f.signing_key_id \
             AND h.decision_identity=f.decision_identity AND h.observed_at=f.observed_at \
             AND h.semantic_facts=f.semantic_facts) \
         AND NOT EXISTS (SELECT 1 FROM game_durability_native_source_observation_history h \
           WHERE h.registration_id=f.registration_id AND h.source_authority=f.source_authority \
             AND h.floor_subject=f.floor_subject AND h.source_revision>f.source_revision) \
         FROM game_durability_native_source_floors f \
         WHERE registration_id=1 AND source_authority=$1 AND floor_subject=$2 FOR SHARE",
    )
    .bind(authority)
    .bind(subject.floor_key())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    if !row.try_get::<bool, _>(7)? {
        return Err(DurabilityError::InvalidStoredState);
    }
    if row.try_get::<String, _>(1)? != operation.as_str()
        || row.try_get::<String, _>(2)? != subject.observation_key()
        || row.try_get::<Option<String>, _>(3)?.as_deref() != subject.signing_key_id()
    {
        // Fresh Account and different signing keys share their respective
        // floors. Their older Recovery history must never become current.
        return Err(DurabilityError::Unavailable);
    }
    let revision = row
        .try_get::<String, _>(0)?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decision: String = row.try_get(4)?;
    let observed_at: i64 = row.try_get(5)?;
    let body: Vec<u8> = row.try_get(6)?;
    let Ok(Response::Observed(observation)) = decode_response(request, authority, &body) else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if revision == 0
        || observation.source_revision != revision
        || observation.decision_identity.as_str() != decision
        || observation.source_observed_at != observed_at
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok((
        observation.facts,
        FreshEvidenceProvenanceV1 {
            source_authority: authority.into(),
            purpose,
            scope: Fnd04EvidenceScope::ExistingActorRecovery,
            source_revision: observation.source_revision,
            accepted_source_revision: revision,
            decision_identity: observation.decision_identity.as_str().into(),
            accepted_decision_identity: decision,
            source_observed_at: observed_at,
            clock_uncertainty_seconds: observation.clock_uncertainty_seconds,
            // One scoped local publication per acknowledged S2 ordinal.
            // Numeric projection is explicit; authority domains remain distinct.
            // Exact replay does not allocate, re-age or mutate a source ordinal.
            publication_revision: revision,
        },
    ))
}

async fn resolve_current(
    tx: &mut Transaction<'_, Postgres>,
    authority: &str,
    subject: &RecoveryEvidenceSubject,
) -> Result<RecoveryEvidenceComposition> {
    let (account, provenance) = load_floor(
        tx,
        authority,
        NativeSourceOperation::ReadRecoveryAccountSecurityV2,
        &subject.account()?,
        &Request::Account {
            recovery: true,
            account_id: &subject.account_id,
            purpose: "platform_security",
            scope: RECOVERY,
        },
        FreshEvidencePurposeV1::PlatformSecurity,
    )
    .await?;
    let Facts::Account {
        allowed,
        minimum_valid_generation,
    } = account
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let security = RecoveryAccountSecurityObservationV2 {
        account_id: subject.account_id.clone(),
        minimum_generation: minimum_valid_generation,
        allowed,
        provenance,
    };
    let (trust, provenance) = load_floor(
        tx,
        authority,
        NativeSourceOperation::ReadRecoverySigningTrustV2,
        &subject.trust()?,
        &Request::Trust {
            recovery: true,
            key_id: &subject.signing_key_id,
            key_purpose: RECOVERY,
        },
        FreshEvidencePurposeV1::SigningTrust,
    )
    .await?;
    let Facts::Trust {
        trusted,
        public_key,
    } = trust
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    Ok(RecoveryEvidenceComposition {
        security,
        signing: RecoverySigningTrustObservationV2 {
            key_id: subject.signing_key_id.clone(),
            public_key,
            trusted,
            provenance,
        },
    })
}

// Private, non-Clone and never returned. Trait reads are safe only within the
// locked synchronous verification above; no cached owning capability exists.
struct RecoveryEvidenceComposition {
    security: RecoveryAccountSecurityObservationV2,
    signing: RecoverySigningTrustObservationV2,
}
impl recovery_source_sealed::Sealed for RecoveryEvidenceComposition {}
impl RecoveryDurabilityEvidenceSourceV2 for RecoveryEvidenceComposition {
    fn signing_trust(
        &self,
        key_id: &str,
        _now: i64,
    ) -> std::result::Result<RecoverySigningTrustObservationV2, Fnd04EvidenceError> {
        if key_id != self.signing.key_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.signing.clone())
    }
    fn account_security(
        &self,
        account_id: &str,
        _now: i64,
    ) -> std::result::Result<RecoveryAccountSecurityObservationV2, Fnd04EvidenceError> {
        if account_id != self.security.account_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.security.clone())
    }
}
