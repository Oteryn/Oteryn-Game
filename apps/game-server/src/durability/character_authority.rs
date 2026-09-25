//! Canonical first-slice Character owner/world authority.

use super::character_authority_audit::{
    EVENT_SCHEMA_REVISION, EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED, RETENTION_PROFILE,
    encode_bootstrap,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::{
    CharacterRecoveryFenceV1, CharacterRecoveryTransition, SealedCharacterRecoveryFence,
    recovery_record_digest,
};
use oteryn_game_server::admission_evidence::{Facts, Request, Response, decode_response};
use oteryn_game_server::character_bootstrap_intent::{
    CharacterBootstrapIntentV1, CharacterInterpretationV1,
};
use oteryn_game_server::domain::{AccountId, CharacterId, CharacterRevision, WorldId};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterAuthorityError>;
const MAX_CONTEXT_BYTES: usize = 128;
const MAX_OPERATION_BINDING_BYTES: usize = 1024;
/// Current S1/S2 Platform account-security evidence consumed as an independent
/// bootstrap prerequisite: the fresh-admission account observation.
const ACCOUNT_SECURITY_OPERATION: &str = "ReadAccountSecurityV1";
const ACCOUNT_SECURITY_PURPOSE: &str = "platform_security";
const ACCOUNT_SECURITY_SCOPE: &str = "fresh_admission";
/// Existing FND-04 fresh security freshness window (seconds, less the source's
/// declared clock uncertainty); see `fnd04_verifier::fresh_source_deadline`.
const ACCOUNT_SECURITY_FRESH_SECONDS: i64 = 5;
const MAX_AUDIT_BATCH: u16 = 64;
/// Bounded identity of this server build, recorded with every audit event so a
/// redelivery after an upgrade keeps the originating EventEnvelope value.
pub const SERVER_BUILD_ID: &str = match option_env!("OTERYN_SERVER_BUILD_ID") {
    Some(build) => build,
    None => concat!("oteryn-game-server/", env!("CARGO_PKG_VERSION")),
};

#[derive(Debug)]
pub enum CharacterAuthorityError {
    Rejected,
    Conflict,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for CharacterAuthorityError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

/// Authority capability issued only after the sealed recovery fence matched the
/// database and the complete Character integrity check passed in this process.
pub struct ReconciledCharacterAuthority<'f, 's> {
    record: CharacterRecoveryFenceV1,
    root_identity: usize,
    root_liveness: std::sync::Weak<dyn std::any::Any + Send + Sync>,
    _seal: &'f SealedCharacterRecoveryFence<'s>,
}

impl ReconciledCharacterAuthority<'_, '_> {
    /// The capability is valid only on the exact root whose database was checked.
    pub(super) fn record_for(&self, root: &DurabilityRoot) -> Result<CharacterRecoveryFenceV1> {
        if self.root_identity != root.root_identity() || self.root_liveness.strong_count() == 0 {
            return Err(CharacterAuthorityError::Rejected);
        }
        Ok(self.record.clone())
    }
}

fn uuid_v7_shape(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuthorityRecord {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub revision: CharacterRevision,
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    /// Registered audit payload bytes. They are the deterministic encoding of
    /// the retained authority identities, pinned byte for byte for schema
    /// revision 1 by `registered_payload_bytes_are_pinned_for_schema_revision_one`.
    /// An exact retry therefore returns the same bytes even after ordinary
    /// expiry deleted the event, without retaining the payload past P90D.
    pub payload: Vec<u8>,
}

/// One committed audit event awaiting at-least-once publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuditDelivery {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at: i64,
    /// Originating server build for the EventEnvelope, captured at commit.
    pub server_build_id: String,
    pub payload: Vec<u8>,
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CONTEXT_BYTES
}

impl DurabilityRoot {
    /// Consume one authenticated `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1`
    /// decision. In one transaction, or not at all: the sealed recovery fence,
    /// the current #415 incarnation, the current allowed S1/S2 account-security
    /// floor for the intent's AccountId and the unexpired intent are proven; the
    /// intent source high-water advances; and the Character root, receipt with
    /// the complete intent binding, audit event and pending outbox commit.
    /// The intent's world and interpretation are requested context: the world
    /// must currently have an assigned Channel (#415) and the revisions must
    /// equal the current Game-owned interpretation, which is resolved from the
    /// durable configuration inside this transaction, never from the caller.
    /// An exact retry returns the committed result; changed reuse conflicts.
    pub async fn bootstrap_character(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        intent: &CharacterBootstrapIntentV1,
    ) -> Result<CharacterAuthorityRecord> {
        let operation = intent.operation_id();
        let intent_binding = intent.binding();
        if !uuid_v7_shape(&operation)
            || intent_binding.len() > MAX_OPERATION_BINDING_BYTES
            || !intent.interpretation().into_iter().all(bounded)
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let requested = intent.clone();
        let proof = proof.clone();
        let account = *intent.account_id().as_bytes();
        let world = *intent.target_world_id().as_bytes();
        let [profile, ruleset, content, starter] = intent.interpretation().map(str::to_owned);
        let decision = intent.issuer_decision_id();
        let source_revision = intent.source_revision();
        let issued_at = intent.issued_at_source();
        let expires_at = intent.expires_at_source();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            // The write must come from the currently registered process incarnation.
            if !prove_current_incarnation(&mut tx, &proof).await? {
                return Err(DurabilityError::Unavailable);
            }
            // Serialize the operation identity before any account-scoped lock, so a
            // concurrent reuse with another account observes the committed receipt.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:' || encode($1, 'hex'), 0))")
                .bind(operation.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES (encode($1, 'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(account.as_slice()).execute(&mut *tx).await?;
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id = encode($1, 'hex')::uuid FOR UPDATE")
                .bind(account.as_slice()).fetch_one(&mut *tx).await?;
            if let Some(row) = sqlx::query("SELECT o.command_binding, o.character_id::text, o.event_id::text, o.transaction_id::text FROM game_character_operation_receipts o WHERE o.operation_id = encode($1, 'hex')::uuid")
                .bind(operation.as_slice()).fetch_optional(&mut *tx).await? {
                // Reconciliation of the committed decision, never a new authorization.
                let old: Vec<u8> = row.try_get("command_binding")?;
                if old != intent_binding { return Ok(Err(CharacterAuthorityError::Conflict)); }
                let record = read_record_row(&row, account, world)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(record));
            }
            // Source-time validity against trusted Game time; arrival or retry
            // time never refreshes the intent.
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
                .fetch_one(&mut *tx).await?;
            if issued_at > now || expires_at <= now {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Game-owned current interpretation, share-locked against a
            // concurrent reconfiguration.
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:character-interpretation', 0))")
                .execute(&mut *tx).await?;
            match current_interpretation(&mut tx).await? {
                Some((_, current)) if current.admits(&requested) => {}
                _ => return Ok(Err(CharacterAuthorityError::Rejected)),
            }
            // Game-owned current world: at least one currently assigned Channel,
            // share-locked so a concurrent revocation serializes with this commit.
            if sqlx::query("SELECT scope_key FROM game_runtime_scope_assignments WHERE world_id = encode($1, 'hex')::uuid AND state = 1 LIMIT 1 FOR SHARE")
                .bind(world.as_slice()).fetch_optional(&mut *tx).await?.is_none() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Independent prerequisite: current allowed Platform account security.
            if !current_account_security_allows(&mut tx, &account, now).await? {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Retained issuer/variant source high-water: a lower revision is stale
            // and an equal one is another decision (no receipt matched this one).
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-bootstrap-intent-floor', 0))")
                .execute(&mut *tx).await?;
            let floor: Option<i64> = sqlx::query_scalar("SELECT source_revision FROM game_character_bootstrap_intent_floors WHERE issuer_scope = 1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?;
            if floor.is_some_and(|floor| source_revision <= floor) {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope, source_revision, issuer_decision_id, intent_binding) VALUES (1, $1, encode($2,'hex')::uuid, $3) ON CONFLICT (issuer_scope) DO UPDATE SET source_revision = EXCLUDED.source_revision, issuer_decision_id = EXCLUDED.issuer_decision_id, intent_binding = EXCLUDED.intent_binding")
                .bind(source_revision).bind(decision.as_slice()).bind(&intent_binding).execute(&mut *tx).await?;
            let ids = sqlx::query("SELECT game_character_uuid_v7()::text AS character_id, game_character_uuid_v7()::text AS event_id, game_character_uuid_v7()::text AS transaction_id, floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
                .fetch_one(&mut *tx).await?;
            let character = uuid_text(ids.try_get("character_id")?)?;
            let event = uuid_text(ids.try_get("event_id")?)?;
            let transaction = uuid_text(ids.try_get("transaction_id")?)?;
            let occurred_at: i64 = ids.try_get("occurred_at")?;
            let payload = encode_bootstrap(&account, &character, &world);
            sqlx::query("INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, 1, $4, $5, $6, $7)")
                .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).bind(profile).bind(ruleset).bind(content).bind(starter).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_audit_outbox(event_id, transaction_id, transaction_ordinal, transaction_count, event_type_id, schema_revision, retention_profile_id, character_id, occurred_at, expires_at, payload, payload_sha256, server_build_id, publication_state) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, encode($6,'hex')::uuid, $9, $9 + 7776000000, $7, sha256($7), $8, 1)")
                .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED)).bind(i64::from(EVENT_SCHEMA_REVISION)).bind(RETENTION_PROFILE).bind(character.as_slice()).bind(&payload).bind(SERVER_BUILD_ID).bind(occurred_at).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id, server_build_id, occurred_at, issuer_decision_id, intent_source_revision, issued_at_source, expires_at_source) VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, 1, encode($6,'hex')::uuid, encode($7,'hex')::uuid, $8, $9, encode($10,'hex')::uuid, $11, $12, $13)")
                .bind(operation.as_slice()).bind(&intent_binding).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(event.as_slice()).bind(transaction.as_slice()).bind(SERVER_BUILD_ID).bind(occurred_at).bind(decision.as_slice()).bind(source_revision).bind(issued_at).bind(expires_at).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterAuthorityRecord { account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?, character_id: CharacterId::from_bytes(character).map_err(|_| DurabilityError::Unavailable)?, world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?, revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?, event_id: event, transaction_id: transaction, payload }))
        })).await?
    }

    /// Reconcile an uncertain bootstrap outcome by its operation identity alone,
    /// without re-authorizing: the committed result, or `None` if nothing
    /// committed. An expired intent can therefore still be reconciled.
    pub async fn reconcile_character_bootstrap(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        operation_id: [u8; 16],
    ) -> Result<Option<CharacterAuthorityRecord>> {
        if !uuid_v7_shape(&operation_id) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT o.account_id::text, o.world_id::text, o.character_id::text, o.event_id::text, o.transaction_id::text FROM game_character_operation_receipts o WHERE o.operation_id = encode($1, 'hex')::uuid")
                .bind(operation_id.as_slice()).fetch_optional(&mut *tx).await?;
            let record = match row {
                Some(row) => Some(read_record_row(&row, uuid_text(row.try_get("account_id")?)?, uuid_text(row.try_get("world_id")?)?)?),
                None => None,
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(record))
        })).await?
    }

    pub async fn read_current_character(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<CharacterAuthorityRecord> {
        let character = *character_id.as_bytes();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text FROM game_character_roots r JOIN game_character_operation_receipts o USING (character_id) WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1").bind(character.as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { return Ok(Err(CharacterAuthorityError::Rejected)); };
            let record = decode_current_row(&row)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(record))
        })).await?
    }

    /// Issue the authority capability for a sealed recovery fence: the database's
    /// latest admission must equal the fence and the full integrity check must pass.
    pub async fn open_character_authority<'f, 's>(
        &self,
        seal: &'f SealedCharacterRecoveryFence<'s>,
    ) -> Result<ReconciledCharacterAuthority<'f, 's>> {
        let record = seal.record().clone();
        let checked = record.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &checked).await?;
                    verify_character_integrity(&mut tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok::<(), CharacterAuthorityError>(()))
                })
            })
            .await??;
        Ok(ReconciledCharacterAuthority {
            record,
            root_identity: self.root_identity(),
            root_liveness: self.root_liveness(),
            _seal: seal,
        })
    }

    /// Admit a fresh store only after Operations has explicitly created generation one.
    pub async fn admit_fresh_character_recovery(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
    ) -> Result<()> {
        if recovery.record().recovery_generation != 1
            || recovery.record().predecessor_generation != 0
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let record = recovery.record().clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            // The definer admits generation one only into an empty Character
            // store; any surviving Character row is evidence of prior state,
            // except exactly this admission re-run after a lost acknowledgement
            // (migration 0007).
            let admitted: bool = sqlx::query_scalar("SELECT game_character_admit_fresh_recovery($1, encode($2,'hex')::uuid, $3::numeric, $4)")
                .bind(&record.authority_scope_id)
                .bind(record.recovery_event_id.as_slice())
                .bind(record.issued_at.to_string())
                .bind(&record.issuer_identity)
                .fetch_one(&mut *tx).await?;
            if !admitted {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Control-plane: configure the current Character interpretation. It is
    /// refused until the fresh generation-one recovery store is admitted,
    /// because that admission requires empty Character tables (OPS-NODE-BOOT-01
    /// D2). Returns the current interpretation revision.
    pub async fn configure_character_interpretation(
        &self,
        profile: &str,
        ruleset: &str,
        content: &str,
        starter: &str,
    ) -> Result<i64> {
        let values = [profile, ruleset, content, starter].map(str::to_owned);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let admitted: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_recovery_admissions)",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    if !admitted {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    }
                    let revision: i64 = sqlx::query_scalar(
                        "SELECT game_character_configure_interpretation($1, $2, $3, $4)",
                    )
                    .bind(&values[0])
                    .bind(&values[1])
                    .bind(&values[2])
                    .bind(&values[3])
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(revision))
                })
            })
            .await?
    }

    /// Reconcile a strict external successor while the exclusive generation guard is held.
    pub async fn reconcile_character_recovery(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
    ) -> Result<()> {
        let record = recovery.record().clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let current: Option<String> = sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?;
            let current = current.ok_or(DurabilityError::Unavailable)?.parse::<u64>().map_err(|_| DurabilityError::Unavailable)?;
            if current == record.recovery_generation {
                assert_recovery_fence(&mut tx, &record).await?;
            } else if current == record.predecessor_generation {
                assert_predecessor_admission(&mut tx, &record).await?;
                insert_recovery_admission(&mut tx, &record).await?;
            } else {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            // This is the explicit reconciliation point. Domain-specific integrity checks grow
            // here; restored receipts/audit/outbox never replace the external predecessor proof.
            verify_character_integrity(&mut tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Committed audit events not yet acknowledged, oldest first. Publication is
    /// at-least-once: an event stays pending until its exact acknowledgement.
    pub async fn pending_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        limit: u16,
    ) -> Result<Vec<CharacterAuditDelivery>> {
        if limit == 0 || limit > MAX_AUDIT_BATCH {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let rows = sqlx::query("SELECT event_id::text, transaction_id::text, occurred_at, server_build_id, payload FROM game_character_audit_outbox WHERE publication_state = 1 ORDER BY occurred_at, event_id LIMIT $1")
                .bind(i64::from(limit)).fetch_all(&mut *tx).await?;
            let mut deliveries = Vec::with_capacity(rows.len());
            for row in &rows {
                deliveries.push(CharacterAuditDelivery {
                    event_id: uuid_text(row.try_get("event_id")?)?,
                    transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                    occurred_at: row.try_get("occurred_at")?,
                    server_build_id: row.try_get("server_build_id")?,
                    payload: row.try_get("payload")?,
                });
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(deliveries))
        })).await?
    }

    /// Record the exact published event. Replaying an acknowledgement is a no-op.
    pub async fn acknowledge_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        event_id: [u8; 16],
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let state: Option<i16> = sqlx::query_scalar("SELECT publication_state FROM game_character_audit_outbox WHERE event_id = encode($1,'hex')::uuid FOR UPDATE")
                .bind(event_id.as_slice()).fetch_optional(&mut *tx).await?;
            match state {
                None => return Ok(Err(CharacterAuthorityError::Rejected)),
                Some(1) => {
                    sqlx::query("UPDATE game_character_audit_outbox SET publication_state = 2, published_at = greatest(occurred_at, floor(extract(epoch FROM statement_timestamp())*1000)::bigint) WHERE event_id = encode($1,'hex')::uuid")
                        .bind(event_id.as_slice()).execute(&mut *tx).await?;
                }
                Some(_) => {}
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Ordinary expiry: delete unheld events (envelope and payload) past the
    /// registered P90D ceiling. No pseudonymous copy is retained.
    pub async fn expire_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        batch: u16,
    ) -> Result<u64> {
        if batch == 0 || batch > MAX_AUDIT_BATCH {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    // The definer boundary serializes with holds on the retention lock
                    // and deletes only unheld records past expiry (migration 0006).
                    let deleted: i64 = sqlx::query_scalar("SELECT game_character_expire_audit($1)")
                        .bind(i32::from(batch))
                        .fetch_one(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(
                        u64::try_from(deleted).map_err(|_| DurabilityError::InvalidStoredState)?
                    ))
                })
            })
            .await?
    }
}

async fn insert_recovery_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let predecessor_digest = predecessor_evidence(record);
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, predecessor_digest, issued_at, issuer_identity, reconciled_at) VALUES ($1, $2::numeric, encode($3,'hex')::uuid, $4::numeric, $7, $5::numeric, $6, floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
        .bind(&record.authority_scope_id)
        .bind(record.recovery_generation.to_string())
        .bind(record.recovery_event_id.as_slice())
        .bind(record.predecessor_generation.to_string())
        .bind(record.issued_at.to_string())
        .bind(&record.issuer_identity)
        .bind(predecessor_digest)
        .execute(&mut **tx).await?;
    Ok(())
}

/// Compare the latest admission with `record`, holding it FOR SHARE.
async fn assert_recovery_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let sql = "SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, predecessor_digest, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE";
    let row = sqlx::query(sql)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let generation = row
        .try_get::<String, _>("recovery_generation")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    let predecessor = row
        .try_get::<String, _>("predecessor_generation")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    let issued_at = row
        .try_get::<String, _>("issued_at")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    if row.try_get::<String, _>("authority_scope_id")? != record.authority_scope_id
        || generation != record.recovery_generation
        || uuid_text(row.try_get("recovery_event_id")?)? != record.recovery_event_id
        || predecessor != record.predecessor_generation
        || issued_at != record.issued_at
        || row.try_get::<String, _>("issuer_identity")? != record.issuer_identity
        || stored_predecessor_evidence(&row)? != predecessor_evidence(record)
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}

/// Predecessor evidence as stored: absent for generation one.
fn predecessor_evidence(record: &CharacterRecoveryFenceV1) -> Option<Vec<u8>> {
    (record.predecessor_generation != 0).then(|| record.predecessor_digest.to_vec())
}

fn stored_predecessor_evidence(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
    Ok(row.try_get("predecessor_digest")?)
}

fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err(DurabilityError::Unavailable);
    }
    let mut out = [0; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| DurabilityError::Unavailable)?;
    }
    Ok(out)
}
/// In-transaction current read for a composition that already holds the
/// reconciled capability: asserts the sealed recovery fence in the caller's
/// transaction, then returns the active Character (lifecycle 1), if any.
pub(super) async fn load_current_character(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    character_id: [u8; 16],
) -> std::result::Result<Option<CharacterAuthorityRecord>, DurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text FROM game_character_roots r JOIN game_character_operation_receipts o USING (character_id) WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1 FOR SHARE OF r")
        .bind(character_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(decode_current_row).transpose()
}

fn decode_current_row(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CharacterAuthorityRecord, DurabilityError> {
    let account = uuid_text(row.try_get("account_id")?)?;
    let character = uuid_text(row.try_get("character_id")?)?;
    let world = uuid_text(row.try_get("world_id")?)?;
    let revision: String = row.try_get("character_revision")?;
    let revision = revision
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(CharacterAuthorityRecord {
        account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?,
        character_id: CharacterId::from_bytes(character)
            .map_err(|_| DurabilityError::Unavailable)?,
        world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?,
        revision: CharacterRevision::new(revision).map_err(|_| DurabilityError::Unavailable)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        payload: encode_bootstrap(&account, &character, &world),
    })
}
fn read_record_row(
    row: &sqlx::postgres::PgRow,
    account: [u8; 16],
    world: [u8; 16],
) -> std::result::Result<CharacterAuthorityRecord, DurabilityError> {
    let character = uuid_text(row.try_get("character_id")?)?;
    Ok(CharacterAuthorityRecord {
        account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?,
        character_id: CharacterId::from_bytes(character)
            .map_err(|_| DurabilityError::Unavailable)?,
        world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?,
        revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        payload: encode_bootstrap(&account, &character, &world),
    })
}

/// Current (highest-revision) Game-owned Character interpretation, if any.
async fn current_interpretation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<Option<(i64, CharacterInterpretationV1)>, DurabilityError> {
    let Some(row) = sqlx::query(
        "SELECT interpretation_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision \
           FROM game_character_interpretations ORDER BY interpretation_revision DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let text = |column: &str| -> std::result::Result<String, DurabilityError> {
        row.try_get(column)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let interpretation = CharacterInterpretationV1::new(
        &text("profile_revision")?,
        &text("ruleset_revision")?,
        &text("content_revision")?,
        &text("starter_template_revision")?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision: i64 = row
        .try_get("interpretation_revision")
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Some((revision, interpretation)))
}

fn uuid_string(value: &[u8; 16]) -> String {
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

/// Current allowed S1/S2 Platform account-security evidence for this exact
/// AccountId. The S2 floor retains the exact authenticated S1 body, which is
/// re-decoded against the registered source authority and the exact request
/// binding; it must allow the account inside the existing fresh window at
/// trusted Game time. The floor is share-locked, so a concurrent S2 advance
/// serializes with this commit. Absent, other-purpose or stale evidence is
/// not current evidence and never a fallback authorization.
async fn current_account_security_allows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &[u8; 16],
    now: i64,
) -> std::result::Result<bool, DurabilityError> {
    let account_id = uuid_string(account);
    let Some(registered) = sqlx::query_scalar::<_, String>(
        "SELECT source_authority FROM game_durability_native_source_registration WHERE registration_id = 1 FOR SHARE",
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(false);
    };
    let Some(row) = sqlx::query(
        "SELECT operation, source_revision::text, decision_identity, observed_at, semantic_facts \
           FROM game_durability_native_source_floors \
          WHERE registration_id = 1 AND source_authority = $1 AND floor_subject = $2 FOR SHARE",
    )
    .bind(&registered)
    .bind(format!("account:{account_id}"))
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(false);
    };
    let invalid = |_| DurabilityError::InvalidStoredState;
    if row.try_get::<String, _>(0).map_err(invalid)? != ACCOUNT_SECURITY_OPERATION {
        return Ok(false);
    }
    let revision: String = row.try_get(1).map_err(invalid)?;
    let decision: String = row.try_get(2).map_err(invalid)?;
    let observed_at: i64 = row.try_get(3).map_err(invalid)?;
    let body: Vec<u8> = row.try_get(4).map_err(invalid)?;
    let request = Request::Account {
        recovery: false,
        account_id: &account_id,
        purpose: ACCOUNT_SECURITY_PURPOSE,
        scope: ACCOUNT_SECURITY_SCOPE,
    };
    let Ok(Response::Observed(observation)) = decode_response(&request, &registered, &body) else {
        return Ok(false);
    };
    if observation.source_revision.to_string() != revision
        || observation.decision_identity.as_str() != decision
        || observation.source_observed_at != observed_at
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let Facts::Account { allowed, .. } = observation.facts else {
        return Ok(false);
    };
    let deadline = i64::try_from(observation.clock_uncertainty_seconds)
        .ok()
        .and_then(|uncertainty| {
            observed_at
                .checked_add(ACCOUNT_SECURITY_FRESH_SECONDS)?
                .checked_sub(uncertainty)
        });
    Ok(allowed && observed_at <= now && deadline.is_some_and(|deadline| now <= deadline))
}

/// Complete first-slice integrity: each root has its exact receipt, including the
/// reconstructed intent binding; every retained audit event binds to that
/// receipt and root with an intact payload hash. Expired events may be absent.
/// The retained intent source high-water exists exactly when a receipt exists
/// and equals the newest receipt's decision, so a restore can neither drop nor
/// regress it (absence is never initialized to zero).
async fn verify_character_integrity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_operation_receipts o USING (character_id) \
           LEFT JOIN game_character_audit_outbox a ON a.event_id = o.event_id \
          WHERE o.character_id IS NULL OR o.account_id <> r.account_id OR o.world_id <> r.world_id \
             OR o.character_revision <> r.character_revision \
             OR o.command_binding <> decode('01', 'hex') || uuid_send(o.issuer_decision_id) \
                || int8send(o.intent_source_revision) || uuid_send(o.operation_id) \
                || uuid_send(r.account_id) || uuid_send(r.world_id) \
                || int8send(o.issued_at_source) || int8send(o.expires_at_source) \
                || int2send(octet_length(r.profile_revision)::int2) || convert_to(r.profile_revision, 'UTF8') \
                || int2send(octet_length(r.ruleset_revision)::int2) || convert_to(r.ruleset_revision, 'UTF8') \
                || int2send(octet_length(r.content_revision)::int2) || convert_to(r.content_revision, 'UTF8') \
                || int2send(octet_length(r.starter_template_revision)::int2) || convert_to(r.starter_template_revision, 'UTF8') \
             OR (a.event_id IS NOT NULL AND (a.character_id <> r.character_id \
                 OR a.transaction_id <> o.transaction_id OR a.server_build_id <> o.server_build_id \
                 OR a.occurred_at <> o.occurred_at \
                 OR a.payload_sha256 <> sha256(a.payload))) \
         UNION ALL \
         SELECT 1 FROM game_character_audit_outbox a \
           LEFT JOIN game_character_operation_receipts o ON o.event_id = a.event_id \
          WHERE o.event_id IS NULL OR o.character_id <> a.character_id \
         UNION ALL \
         SELECT 1 FROM game_character_operation_receipts o \
           LEFT JOIN game_character_roots r USING (character_id) \
          WHERE r.character_id IS NULL \
         UNION ALL \
         SELECT 1 FROM game_character_audit_legal_holds h \
           LEFT JOIN game_character_audit_outbox a USING (event_id) \
          WHERE h.released_at IS NULL AND a.event_id IS NULL \
         UNION ALL \
         SELECT 1 WHERE (SELECT count(*) FROM game_character_bootstrap_intent_floors) \
                     <> (SELECT count(*) FROM (SELECT 1 FROM game_character_operation_receipts LIMIT 1) x) \
         UNION ALL \
         SELECT 1 FROM game_character_bootstrap_intent_floors f \
          WHERE f.source_revision <> (SELECT max(intent_source_revision) FROM game_character_operation_receipts) \
             OR NOT EXISTS (SELECT 1 FROM game_character_operation_receipts o \
                             WHERE o.intent_source_revision = f.source_revision \
                               AND o.issuer_decision_id = f.issuer_decision_id \
                               AND o.command_binding = f.intent_binding) \
         LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .map_or(Ok(()), |_| Err(DurabilityError::Unavailable))?;
    // A self-consistent hash is not semantic evidence: every retained payload
    // must be exactly the registered encoding of its root's identities.
    let mut after = String::from("00000000-0000-0000-0000-000000000000");
    loop {
        let rows = sqlx::query(
            "SELECT a.event_id::text, a.payload, r.account_id::text, r.character_id::text, r.world_id::text \
               FROM game_character_audit_outbox a JOIN game_character_roots r USING (character_id) \
              WHERE a.event_id > $1::uuid ORDER BY a.event_id LIMIT 256",
        )
        .bind(&after)
        .fetch_all(&mut **tx)
        .await?;
        let Some(last) = rows.last() else {
            return Ok(());
        };
        after = last.try_get("event_id")?;
        for row in &rows {
            // Typed identities: UUIDv7 version and RFC variant, not raw bytes.
            let account = AccountId::from_bytes(uuid_text(row.try_get("account_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let character = CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let world = WorldId::from_bytes(uuid_text(row.try_get("world_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let expected =
                encode_bootstrap(account.as_bytes(), character.as_bytes(), world.as_bytes());
            if row.try_get::<Vec<u8>, _>("payload")? != expected {
                return Err(DurabilityError::Unavailable);
            }
        }
    }
}

/// The database's current admission must be exactly the predecessor the external
/// successor retains: the digest covers every field, including the admission's
/// own predecessor digest, so the whole chain is bound.
async fn assert_predecessor_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let row = sqlx::query("SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, predecessor_digest, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1")
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let number = |name: &str| -> std::result::Result<u64, DurabilityError> {
        row.try_get::<String, _>(name)?
            .parse::<u64>()
            .map_err(|_| DurabilityError::Unavailable)
    };
    let digest: Option<Vec<u8>> = row.try_get("predecessor_digest")?;
    let predecessor = CharacterRecoveryFenceV1 {
        authority_scope_id: row.try_get("authority_scope_id")?,
        recovery_generation: number("recovery_generation")?,
        recovery_event_id: uuid_text(row.try_get("recovery_event_id")?)?,
        predecessor_generation: number("predecessor_generation")?,
        predecessor_digest: match digest {
            None => [0; 32],
            Some(bytes) => bytes.try_into().map_err(|_| DurabilityError::Unavailable)?,
        },
        issued_at: number("issued_at")?,
        issuer_identity: row.try_get("issuer_identity")?,
    };
    if predecessor.recovery_generation != record.predecessor_generation
        || predecessor.authority_scope_id != record.authority_scope_id
        || predecessor.issuer_identity != record.issuer_identity
        || recovery_record_digest(&predecessor).map_err(|_| DurabilityError::Unavailable)?
            != record.predecessor_digest
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}
