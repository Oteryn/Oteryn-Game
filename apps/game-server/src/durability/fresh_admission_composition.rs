//! WP5 S3-B: sealed composition of the real fresh-admission owners.
//!
//! Every fact the Foundation fresh-admission verifier consumes comes from an
//! owning durable source, never from a caller-supplied claim:
//!
//! - Platform account security and fresh signing trust: the S2 native-source
//!   floors. `SourceObservation::semantic_facts` holds the exact authenticated
//!   S1 response body, which is re-decoded here with the exact S1 decoder
//!   against the registered source authority and the request binding;
//! - Character owner/world/lifecycle: #414 Character Authority;
//! - Channel runtime ownership/readiness: the #415 assignment plus the
//!   holder-attested Runtime guard.
//!
//! [`DurabilityRoot::publish_fresh_admission_sources`] publishes the Account,
//! SigningTrust and Character admission guards from those owners under the S2
//! custody fence. [`DurabilityRoot::compose_fresh_admission`] then returns a
//! sealed, already-resolved snapshot for the current assigned Channel holder;
//! it implements the Foundation source traits without database waits. The
//! durable commit (`FreshAdmissionStore::commit`) re-locks every guard, so a
//! snapshot never outlives a concurrent owner change.

use super::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition, encode_guard,
};
use super::character_authority::{
    CharacterAuthorityRecord, ReconciledCharacterAuthority, load_current_character,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::fresh_admission::{FreshAdmissionStore, OwnerRevalidation};
use super::native_admission_source::fence_custody;
use super::runtime_scope_assignment::{
    AssignmentState, NodeIncarnationProof, channel_scope, load_assignment,
    prove_current_incarnation, require_history_matches_high_water, scope_key, writer_high_water,
};
use super::{DurabilityError, DurabilityRoot, MAX_ADMISSION_GUARD_BYTES};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_game_server::admission_evidence::{Facts, Request, Response, decode_response};
use oteryn_game_server::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationCurrentSourceV1, AdmissionAuthorityPublicationErrorV1,
    AdmissionAuthorityPublicationV1, AdmissionClaimOwningSourceV1,
    AdmissionClaimTransitionEvidenceV1, AdmissionPublicationPreconditionV1,
    AdmissionPublicationPurposeV1, AdmissionPublicationSourceV1,
};
use oteryn_game_server::foundation::fnd04_verifier::{
    FreshAccountSecurityObservationV1, FreshCurrentEvidence, FreshDurabilityCurrentSourceV1,
    FreshDurabilityEvidenceSourceV1, FreshEvidenceProvenanceV1, FreshEvidencePurposeV1,
    FreshPublishedCurrentObservationV1, FreshSigningTrustObservationV1, fresh_source_sealed,
};
use oteryn_game_server::foundation::fresh_admission_durability::FreshAdmissionAuditBindingV1;
use oteryn_game_server::foundation::fresh_admission_durability::{
    FreshAdmissionCommitRequestV1, FreshAdmissionDurableOutcomeV1,
};
use oteryn_game_server::foundation::{
    ChannelId, CharacterId, Fnd04ConsumerError, Fnd04EvidenceError, Fnd04EvidenceScope,
    PRE_ADMISSION_PROFILE, RuntimeScopeRefV1, WorldId,
};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

/// Game-owned publisher identity for the Account and Character admission
/// guards. SigningTrust guards keep the Platform source identity itself.
pub const COMPOSITION_SOURCE_AUTHORITY: &str = "game:wp5-fresh-admission-composition";
const FRESH_ISSUER: &str = "urn:oteryn:platform:game-admission";
const FRESH_KEY_PURPOSE: &str = "fresh_admission";
const ACCOUNT_PURPOSE: &str = "platform_security";
const ACCOUNT_SCOPE: &str = "fresh_admission";

/// One fresh-admission subject: the Platform account, its Game Character, the
/// target Channel and the fresh signing key named by the grant header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshAdmissionSubject {
    pub account_id: String,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub signing_key_id: String,
}

impl FreshAdmissionSubject {
    fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
    }

    fn keys(&self) -> [AdmissionAuthorityGuardKeyV1; 4] {
        [
            AdmissionAuthorityGuardKeyV1::Account {
                account_id: self.account_id.clone(),
            },
            AdmissionAuthorityGuardKeyV1::Character(self.character_id),
            AdmissionAuthorityGuardKeyV1::Runtime(self.runtime_scope()),
            AdmissionAuthorityGuardKeyV1::SigningTrust {
                key_id: self.signing_key_id.clone(),
                profile: PRE_ADMISSION_PROFILE.into(),
            },
        ]
    }
}

/// Decoded current S2 floor for one fresh-admission operation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFloor {
    source_authority: String,
    source_revision: u64,
    decision_identity: String,
    observed_at: i64,
    clock_uncertainty_seconds: u64,
    facts: Facts,
}

fn length_bound_tuple(prefix: &str, values: &[&str]) -> String {
    let mut result = String::from(prefix);
    for value in values {
        result.push(':');
        result.push_str(&value.len().to_string());
        result.push(':');
        result.push_str(value);
    }
    result
}

/// Read the S2 floor and re-decode its exact stored response body. Any
/// mismatch between the stored metadata and the decoded body fails closed.
async fn load_floor(
    tx: &mut Transaction<'_, Postgres>,
    request: &Request<'_>,
    floor_subject: &str,
    expected_key_id: Option<&str>,
) -> Result<Option<SourceFloor>> {
    let registered: String = sqlx::query_scalar(
        "SELECT source_authority FROM game_durability_native_source_registration WHERE registration_id = 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    let Some(row) = sqlx::query(
        "SELECT source_revision::text, decision_identity, observed_at, semantic_facts, signing_key_id \
         FROM game_durability_native_source_floors \
         WHERE registration_id = 1 AND source_authority = $1 AND floor_subject = $2",
    )
    .bind(&registered)
    .bind(floor_subject)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let invalid = |_| DurabilityError::InvalidStoredState;
    let revision: u64 = row
        .try_get::<String, _>(0)
        .map_err(invalid)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decision: String = row.try_get(1).map_err(invalid)?;
    let observed_at: i64 = row.try_get(2).map_err(invalid)?;
    let body: Vec<u8> = row.try_get(3).map_err(invalid)?;
    let key_id: Option<String> = row.try_get(4).map_err(invalid)?;
    if key_id.as_deref() != expected_key_id {
        return Ok(None);
    }
    let Ok(Response::Observed(observation)) = decode_response(request, &registered, &body) else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if observation.source_authority.as_str() != registered
        || observation.source_revision != revision
        || observation.decision_identity.as_str() != decision
        || observation.source_observed_at != observed_at
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(SourceFloor {
        source_authority: registered,
        source_revision: revision,
        decision_identity: decision,
        observed_at,
        clock_uncertainty_seconds: observation.clock_uncertainty_seconds,
        facts: observation.facts,
    }))
}

async fn account_floor(
    tx: &mut Transaction<'_, Postgres>,
    account_id: &str,
) -> Result<Option<SourceFloor>> {
    let request = Request::Account {
        recovery: false,
        account_id,
        purpose: ACCOUNT_PURPOSE,
        scope: ACCOUNT_SCOPE,
    };
    load_floor(tx, &request, &format!("account:{account_id}"), None).await
}

async fn trust_floor(
    tx: &mut Transaction<'_, Postgres>,
    key_id: &str,
) -> Result<Option<SourceFloor>> {
    let request = Request::Trust {
        recovery: false,
        key_id,
        key_purpose: FRESH_KEY_PURPOSE,
    };
    let subject = length_bound_tuple(
        "trust",
        &[FRESH_ISSUER, PRE_ADMISSION_PROFILE, FRESH_KEY_PURPOSE],
    );
    load_floor(tx, &request, &subject, Some(key_id)).await
}

fn security_observation(
    account_id: &str,
    floor: &SourceFloor,
    publication_revision: u64,
) -> Result<FreshAccountSecurityObservationV1> {
    let Facts::Account {
        allowed,
        minimum_valid_generation,
    } = floor.facts
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    Ok(FreshAccountSecurityObservationV1 {
        account_id: account_id.into(),
        minimum_generation: minimum_valid_generation,
        allowed,
        provenance: FreshEvidenceProvenanceV1 {
            source_authority: floor.source_authority.clone(),
            purpose: FreshEvidencePurposeV1::PlatformSecurity,
            scope: Fnd04EvidenceScope::FreshAdmission,
            source_revision: floor.source_revision,
            accepted_source_revision: floor.source_revision,
            decision_identity: floor.decision_identity.clone(),
            accepted_decision_identity: floor.decision_identity.clone(),
            source_observed_at: floor.observed_at,
            clock_uncertainty_seconds: floor.clock_uncertainty_seconds,
            publication_revision,
        },
    })
}

fn next_revision(value: u64) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)
}

fn successor_precondition(
    current: Option<&AdmissionAuthorityPublicationChangeV1>,
) -> Result<(AdmissionPublicationPreconditionV1, u64)> {
    Ok(match current {
        None => (
            AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            1,
        ),
        Some(current) => (
            AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: current.publication_revision,
            },
            next_revision(current.publication_revision)?,
        ),
    })
}

fn composition_source(
    purpose: AdmissionPublicationPurposeV1,
    current: Option<&AdmissionAuthorityPublicationChangeV1>,
    decision: &str,
    now: i64,
) -> Result<AdmissionPublicationSourceV1> {
    let source_revision = match current {
        None => 1,
        Some(current) if current.source.authority == COMPOSITION_SOURCE_AUTHORITY => {
            next_revision(current.source.source_revision)?
        }
        // Another publisher owns this guard; never impersonate it.
        Some(_) => return Err(DurabilityError::Unavailable),
    };
    Ok(AdmissionPublicationSourceV1 {
        authority: COMPOSITION_SOURCE_AUTHORITY.into(),
        purpose,
        source_revision,
        decision_identity: format!("{decision}:{source_revision}"),
        source_observed_at: now.max(current.map_or(0, |current| current.source.source_observed_at)),
        clock_uncertainty_seconds: 0,
    })
}

/// Sealed in-crate publisher registration for the resolved owner changes.
struct Resolved(Vec<AdmissionAuthorityPublicationChangeV1>);
impl fresh_source_sealed::Sealed for Resolved {}
impl AdmissionAuthorityOwningPublisherV1 for Resolved {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> std::result::Result<
        Vec<AdmissionAuthorityPublicationChangeV1>,
        AdmissionAuthorityPublicationErrorV1,
    > {
        Ok(self.0.clone())
    }
}

impl DurabilityRoot {
    /// Publish the Account (security), SigningTrust and Character admission
    /// guards for one subject from the real owners, under the S2 custody fence.
    /// Unchanged owner facts are not republished; a concurrent owner change
    /// makes the guard CAS reject rather than overwrite.
    pub async fn publish_fresh_admission_sources(
        &self,
        character: &ReconciledCharacterAuthority<'_, '_>,
        custody: &NodeIncarnationProof,
        subject: &FreshAdmissionSubject,
        now: i64,
    ) -> Result<GuardPublicationDisposition> {
        let recovery = character
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let custody = custody.clone();
        let subject = subject.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    fence_custody(&mut tx, &custody).await?;
                    lock_admission_relations(&mut tx).await?;
                    let store = AdmissionGuardStore::from_root(root);
                    let [account_key, character_key, _, trust_key] = subject.keys();
                    let account = account_floor(&mut tx, &subject.account_id)
                        .await?
                        .ok_or(DurabilityError::Unavailable)?;
                    let trust = trust_floor(&mut tx, &subject.signing_key_id)
                        .await?
                        .ok_or(DurabilityError::Unavailable)?;
                    let record = load_current_character(
                        &mut tx,
                        &recovery,
                        *subject.character_id.as_bytes(),
                    )
                    .await?
                    .ok_or(DurabilityError::Unavailable)?;
                    if !owned_by(&record, &subject) {
                        return Err(DurabilityError::Unavailable);
                    }
                    let mut changes = Vec::with_capacity(3);
                    let mut current = Vec::with_capacity(3);

                    let prior = store.load_locked(&mut tx, &account_key).await?;
                    let unchanged = matches!(
                        prior.as_ref().map(|prior| &prior.state),
                        Some(AdmissionAuthorityGuardStateV1::Account { security, .. })
                            if security.provenance.source_authority == account.source_authority
                                && security.provenance.source_revision == account.source_revision
                    );
                    if !unchanged {
                        let presence = match prior.as_ref().map(|prior| &prior.state) {
                            None => None,
                            Some(AdmissionAuthorityGuardStateV1::Account { presence, .. }) => {
                                *presence
                            }
                            Some(_) => return Err(DurabilityError::InvalidStoredState),
                        };
                        let (precondition, publication_revision) =
                            successor_precondition(prior.as_ref())?;
                        changes.push(AdmissionAuthorityPublicationChangeV1 {
                            key: account_key,
                            source: composition_source(
                                AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
                                prior.as_ref(),
                                "account-security",
                                now,
                            )?,
                            precondition,
                            publication_revision,
                            state: AdmissionAuthorityGuardStateV1::Account {
                                security: security_observation(
                                    &subject.account_id,
                                    &account,
                                    publication_revision,
                                )?,
                                presence,
                            },
                        });
                        current.push(prior);
                    }

                    let prior = store.load_locked(&mut tx, &character_key).await?;
                    match prior.as_ref().map(|prior| &prior.state) {
                        None => {
                            changes.push(AdmissionAuthorityPublicationChangeV1 {
                                key: character_key,
                                source: composition_source(
                                    AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
                                    None,
                                    "character-authority",
                                    now,
                                )?,
                                precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                                    restored_publication_high_water: Some(0),
                                },
                                publication_revision: 1,
                                state: AdmissionAuthorityGuardStateV1::Character {
                                    account_id: subject.account_id.clone(),
                                    world_id: subject.world_id,
                                    // Only an active (lifecycle 1) Character is loaded.
                                    eligible: true,
                                    lease_generation: 1,
                                    holder: None,
                                },
                            });
                            current.push(None);
                        }
                        // Character ownership/world are immutable in this slice.
                        Some(AdmissionAuthorityGuardStateV1::Character {
                            account_id,
                            world_id,
                            ..
                        }) if *account_id == subject.account_id
                            && *world_id == subject.world_id => {}
                        Some(_) => return Err(DurabilityError::InvalidStoredState),
                    }

                    let Facts::Trust {
                        trusted,
                        public_key,
                    } = trust.facts
                    else {
                        return Err(DurabilityError::InvalidStoredState);
                    };
                    let prior = store.load_locked(&mut tx, &trust_key).await?;
                    let unchanged = prior.as_ref().is_some_and(|prior| {
                        prior.source.authority == trust.source_authority
                            && prior.source.source_revision == trust.source_revision
                    });
                    if !unchanged {
                        let (precondition, publication_revision) =
                            successor_precondition(prior.as_ref())?;
                        changes.push(AdmissionAuthorityPublicationChangeV1 {
                            key: trust_key,
                            // The fixed fresh-trust guard carries the Platform
                            // provenance itself; the verifier binds it exactly.
                            source: AdmissionPublicationSourceV1 {
                                authority: trust.source_authority.clone(),
                                purpose: AdmissionPublicationPurposeV1::FixedFreshSigningTrust,
                                source_revision: trust.source_revision,
                                decision_identity: trust.decision_identity.clone(),
                                source_observed_at: trust.observed_at,
                                clock_uncertainty_seconds: trust.clock_uncertainty_seconds,
                            },
                            precondition,
                            publication_revision,
                            state: AdmissionAuthorityGuardStateV1::SigningTrust {
                                public_key,
                                trusted,
                            },
                        });
                        current.push(prior);
                    }

                    if changes.is_empty() {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(GuardPublicationDisposition::Existing);
                    }
                    let Ok(request) =
                        AdmissionAuthorityPublicationV1::prepare(&Resolved(changes), now)
                    else {
                        return Err(DurabilityError::Unavailable);
                    };
                    if let Err(error) = request.validate_locked(&current) {
                        return Ok(if error == AdmissionAuthorityPublicationErrorV1::Stale {
                            GuardPublicationDisposition::Stale
                        } else {
                            GuardPublicationDisposition::Conflict
                        });
                    }
                    if !store
                        .successor_history_available(&mut tx, request.changes(), &current)
                        .await?
                    {
                        return Ok(GuardPublicationDisposition::Conflict);
                    }
                    let encoded: Vec<_> = request
                        .changes()
                        .iter()
                        .map(|change| encode_guard(change, MAX_ADMISSION_GUARD_BYTES))
                        .collect::<Result<_>>()?;
                    store
                        .persist_locked(&mut tx, request.changes(), &current, &encoded)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(GuardPublicationDisposition::Applied)
                })
            })
            .await
    }

    /// Resolve the sealed fresh-admission composition for the current
    /// assigned Channel holder. Every published guard must equal the current
    /// owner facts (S2 floors, #414 Character, #415 assignment generation);
    /// otherwise the caller must republish first.
    pub async fn compose_fresh_admission(
        &self,
        character: &ReconciledCharacterAuthority<'_, '_>,
        holder: &NodeIncarnationProof,
        subject: &FreshAdmissionSubject,
    ) -> Result<FreshAdmissionComposition> {
        let recovery = character
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let holder = holder.clone();
        let subject = subject.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |pass, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(pass, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    let store = AdmissionGuardStore::from_root(root);
                    let composition =
                        resolve_current(&mut tx, &store, &recovery, &holder, subject).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(composition)
                })
            })
            .await
    }
}

impl DurabilityRoot {
    /// Durable fresh-admission commit that revalidates every owner inside the
    /// commit transaction: the sealed composition is recomputed from the
    /// current S2 floors, #414 Character (under the recovery fence), #415
    /// assignment and guards, and the commit is rejected as stale authority
    /// unless it is exactly the composition the request was prepared from.
    pub async fn commit_composed_fresh_admission(
        &self,
        character: &ReconciledCharacterAuthority<'_, '_>,
        holder: &NodeIncarnationProof,
        composition: &FreshAdmissionComposition,
        request: &FreshAdmissionCommitRequestV1,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        let recovery = character
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let holder = holder.clone();
        let expected = composition.clone();
        let store = AdmissionGuardStore::from_root(self.clone());
        let revalidate: OwnerRevalidation = Box::new(move |tx| {
            Box::pin(async move {
                match resolve_current(tx, &store, &recovery, &holder, expected.subject.clone())
                    .await
                {
                    Ok(current) => Ok(current == expected),
                    Err(DurabilityError::Unavailable) => Ok(false),
                    Err(error) => Err(error),
                }
            })
        });
        FreshAdmissionStore::from_root(self.clone())
            .commit_revalidated(request, Some(revalidate))
            .await
    }
}

/// Resolve the sealed composition from the current owners inside `tx`. The
/// caller holds the admission relation locks.
async fn resolve_current(
    tx: &mut Transaction<'_, Postgres>,
    store: &AdmissionGuardStore,
    recovery: &CharacterRecoveryFenceV1,
    holder: &NodeIncarnationProof,
    subject: FreshAdmissionSubject,
) -> Result<FreshAdmissionComposition> {
    if !prove_current_incarnation(tx, holder).await? {
        return Err(DurabilityError::Unavailable);
    }
    let (world_id, channel_id) =
        channel_scope(subject.runtime_scope()).map_err(|_| DurabilityError::Unavailable)?;
    let high_water = writer_high_water(tx, false).await?;
    require_history_matches_high_water(tx, high_water).await?;
    let assignment = load_assignment(tx, &scope_key(world_id, channel_id), false)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    if assignment.state != AssignmentState::Assigned || assignment.holder != Some(holder.fact()) {
        return Err(DurabilityError::Unavailable);
    }
    let mut rows = Vec::with_capacity(4);
    for key in subject.keys() {
        rows.push(
            store
                .load_locked(tx, &key)
                .await?
                .ok_or(DurabilityError::Unavailable)?,
        );
    }
    let account = account_floor(tx, &subject.account_id)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let trust = trust_floor(tx, &subject.signing_key_id)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let record = load_current_character(tx, recovery, *subject.character_id.as_bytes())
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    FreshAdmissionComposition::resolve(
        subject,
        rows,
        &account,
        &trust,
        &record,
        assignment.ownership_generation,
    )
}

/// #414 owner/world binding: the Character belongs to exactly this Platform
/// AccountId (full 128-bit value) and the subject's World.
fn owned_by(record: &CharacterAuthorityRecord, subject: &FreshAdmissionSubject) -> bool {
    canonical_uuid(record.account_id.as_bytes()) == subject.account_id
        && record.world_id.as_bytes() == subject.world_id.as_bytes()
        && record.character_id.as_bytes() == subject.character_id.as_bytes()
}

fn canonical_uuid(value: &[u8; 16]) -> String {
    let hex: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Sealed, already-resolved owner facts for one fresh admission. There is no
/// public constructor; only [`DurabilityRoot::compose_fresh_admission`]
/// creates it from the locked owners.
#[derive(Debug, Clone, PartialEq)]
pub struct FreshAdmissionComposition {
    subject: FreshAdmissionSubject,
    rows: Vec<AdmissionAuthorityPublicationChangeV1>,
    security: FreshAccountSecurityObservationV1,
    signing: FreshSigningTrustObservationV1,
    current: FreshPublishedCurrentObservationV1,
}

impl FreshAdmissionComposition {
    fn resolve(
        subject: FreshAdmissionSubject,
        rows: Vec<AdmissionAuthorityPublicationChangeV1>,
        account: &SourceFloor,
        trust: &SourceFloor,
        record: &CharacterAuthorityRecord,
        assignment_generation: u64,
    ) -> Result<Self> {
        let stale = DurabilityError::Unavailable;
        let [account_row, character_row, runtime_row, trust_row] = rows.as_slice() else {
            return Err(DurabilityError::InvalidStoredState);
        };
        // Account guard must carry exactly the current S2 account floor.
        let AdmissionAuthorityGuardStateV1::Account { security, presence } = &account_row.state
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if *security
            != security_observation(
                &subject.account_id,
                account,
                account_row.publication_revision,
            )?
        {
            return Err(stale);
        }
        // SigningTrust guard must carry exactly the current S2 trust floor.
        let (
            AdmissionAuthorityGuardStateV1::SigningTrust {
                public_key,
                trusted,
            },
            Facts::Trust {
                trusted: floor_trusted,
                public_key: floor_key,
            },
        ) = (&trust_row.state, trust.facts)
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if trust_row.source.authority != trust.source_authority
            || trust_row.source.source_revision != trust.source_revision
            || trust_row.source.decision_identity != trust.decision_identity
            || trust_row.source.source_observed_at != trust.observed_at
            || trust_row.source.clock_uncertainty_seconds != trust.clock_uncertainty_seconds
            || *public_key != floor_key
            || *trusted != floor_trusted
        {
            return Err(stale);
        }
        // Character guard must match #414 ownership, world and lifecycle.
        let AdmissionAuthorityGuardStateV1::Character {
            account_id,
            world_id,
            eligible,
            lease_generation,
            holder,
        } = &character_row.state
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if !owned_by(record, &subject)
            || *account_id != subject.account_id
            || *world_id != subject.world_id
            || !*eligible
        {
            return Err(stale);
        }
        // Runtime guard must be for the current #415 assignment generation.
        let AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            route_revision,
            runtime_observation_revision,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
            offer_revision,
            ..
        } = &runtime_row.state
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if *ownership_generation != assignment_generation {
            return Err(stale);
        }
        let proposed_lease_generation = lease_generation
            .checked_add(1)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let signing = FreshSigningTrustObservationV1 {
            key_id: subject.signing_key_id.clone(),
            public_key: *public_key,
            trusted: *trusted,
            provenance: FreshEvidenceProvenanceV1 {
                source_authority: trust.source_authority.clone(),
                purpose: FreshEvidencePurposeV1::SigningTrust,
                scope: Fnd04EvidenceScope::FreshAdmission,
                source_revision: trust.source_revision,
                accepted_source_revision: trust.source_revision,
                decision_identity: trust.decision_identity.clone(),
                accepted_decision_identity: trust.decision_identity.clone(),
                source_observed_at: trust.observed_at,
                clock_uncertainty_seconds: trust.clock_uncertainty_seconds,
                publication_revision: trust_row.publication_revision,
            },
        };
        let current = FreshPublishedCurrentObservationV1 {
            facts: FreshCurrentEvidence {
                account_id: subject.account_id.clone(),
                character_id: subject.character_id,
                world_id: subject.world_id,
                channel_id: subject.channel_id,
                character_lease_generation: *lease_generation,
                route_revision: route_revision.clone(),
                runtime_observation_revision: runtime_observation_revision.clone(),
                scope_ownership_generation: *ownership_generation,
                ruleset_revision: ruleset_revision.clone(),
                content_revision: content_revision.clone(),
                map_revision: map_revision.clone(),
                world_policy_revision: world_policy_revision.clone(),
                offer_revision: offer_revision.clone(),
            },
            account_publication_revision: account_row.publication_revision,
            character_publication_revision: character_row.publication_revision,
            runtime_publication_revision: runtime_row.publication_revision,
            expected_lease_generation: *lease_generation,
            proposed_lease_generation,
            account_presence_available: presence.is_none(),
            character_eligible: *eligible && holder.is_none(),
            runtime_ready: *ready,
        };
        Ok(Self {
            security: security.clone(),
            rows: vec![
                account_row.clone(),
                character_row.clone(),
                runtime_row.clone(),
                trust_row.clone(),
            ],
            subject,
            signing,
            current,
        })
    }
}

impl fresh_source_sealed::Sealed for FreshAdmissionComposition {}

impl FreshDurabilityEvidenceSourceV1 for FreshAdmissionComposition {
    fn signing_trust(
        &self,
        key_id: &str,
        _now: i64,
    ) -> std::result::Result<FreshSigningTrustObservationV1, Fnd04EvidenceError> {
        if key_id != self.subject.signing_key_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.signing.clone())
    }

    fn account_security(
        &self,
        account_id: &str,
        _now: i64,
    ) -> std::result::Result<FreshAccountSecurityObservationV1, Fnd04EvidenceError> {
        if account_id != self.subject.account_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.security.clone())
    }
}

impl FreshDurabilityCurrentSourceV1 for FreshAdmissionComposition {
    fn current(
        &self,
        account_id: &str,
        character_id: CharacterId,
        _now: i64,
    ) -> std::result::Result<FreshPublishedCurrentObservationV1, Fnd04ConsumerError> {
        if account_id != self.subject.account_id || character_id != self.subject.character_id {
            return Err(Fnd04ConsumerError::FreshAccountCharacterConflict);
        }
        Ok(self.current.clone())
    }
}

impl AdmissionAuthorityPublicationCurrentSourceV1 for FreshAdmissionComposition {
    fn current_publications(
        &self,
        keys: &[AdmissionAuthorityGuardKeyV1],
    ) -> std::result::Result<
        Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
        AdmissionAuthorityPublicationErrorV1,
    > {
        Ok(keys
            .iter()
            .map(|key| self.rows.iter().find(|row| &row.key == key).cloned())
            .collect())
    }
}

impl AdmissionClaimOwningSourceV1 for FreshAdmissionComposition {
    fn prepare_fresh_claim(
        &self,
        binding: &FreshAdmissionAuditBindingV1,
        now: i64,
    ) -> std::result::Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>
    {
        use AdmissionAuthorityPublicationErrorV1::Invalid;
        let predecessors = self.rows[..2].to_vec();
        let mut successors = predecessors.clone();
        for row in &mut successors {
            if row.source.authority != COMPOSITION_SOURCE_AUTHORITY {
                return Err(Invalid);
            }
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: row.publication_revision,
            };
            row.publication_revision = row.publication_revision.checked_add(1).ok_or(Invalid)?;
            row.source.source_revision =
                row.source.source_revision.checked_add(1).ok_or(Invalid)?;
            row.source.decision_identity = format!("fresh-claim:{}", row.source.source_revision);
            row.source.source_observed_at = now;
            row.source.clock_uncertainty_seconds = 0;
            match &mut row.state {
                AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                    security.provenance.publication_revision = row.publication_revision;
                    *presence = Some((self.subject.character_id, binding.candidate_session));
                }
                AdmissionAuthorityGuardStateV1::Character {
                    lease_generation,
                    holder,
                    ..
                } => {
                    *lease_generation = lease_generation.checked_add(1).ok_or(Invalid)?;
                    *holder = Some(binding.candidate_session);
                }
                _ => return Err(Invalid),
            }
        }
        Ok(AdmissionClaimTransitionEvidenceV1 {
            predecessors,
            successors,
            prepared_at: now,
        })
    }
}
