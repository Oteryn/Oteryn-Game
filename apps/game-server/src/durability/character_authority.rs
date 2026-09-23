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
use oteryn_game_server::domain::{AccountId, CharacterId, CharacterRevision, WorldId};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterAuthorityError>;
const MAX_CONTEXT_BYTES: usize = 128;
const MAX_OPERATION_BINDING_BYTES: usize = 1024;
const MAX_HOLD_REASON_BYTES: usize = 512;
const MAX_AUDIT_BATCH: u16 = 64;

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
    fn record_for(&self, root: &DurabilityRoot) -> Result<CharacterRecoveryFenceV1> {
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
pub struct BootstrapCommand {
    pub operation_id: [u8; 16],
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub profile_revision: String,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub starter_template_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuthorityRecord {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub revision: CharacterRevision,
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    /// Registered audit payload; `None` once the event reached ordinary expiry.
    pub payload: Option<Vec<u8>>,
}

/// One committed audit event awaiting at-least-once publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuditDelivery {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at: i64,
    pub payload: Vec<u8>,
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CONTEXT_BYTES
}

fn binding(command: &BootstrapCommand) -> Vec<u8> {
    let mut value = Vec::with_capacity(MAX_OPERATION_BINDING_BYTES);
    for part in [
        command.account_id.as_bytes().as_slice(),
        command.world_id.as_bytes().as_slice(),
    ] {
        value.extend_from_slice(part);
    }
    for part in [
        &command.profile_revision,
        &command.ruleset_revision,
        &command.content_revision,
        &command.starter_template_revision,
    ] {
        value.extend_from_slice(&(part.len() as u16).to_be_bytes());
        value.extend_from_slice(part.as_bytes());
    }
    value
}

impl DurabilityRoot {
    /// Qualification/bootstrap authority: the process must prove its current
    /// operator-authorized incarnation in the same transaction as the write.
    pub async fn bootstrap_character(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        command: BootstrapCommand,
    ) -> Result<CharacterAuthorityRecord> {
        if !uuid_v7_shape(&command.operation_id)
            || ![
                &command.profile_revision,
                &command.ruleset_revision,
                &command.content_revision,
                &command.starter_template_revision,
            ]
            .into_iter()
            .all(|v| bounded(v))
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let proof = proof.clone();
        let operation = command.operation_id;
        let command_binding = binding(&command);
        if command_binding.len() > MAX_OPERATION_BINDING_BYTES {
            return Err(CharacterAuthorityError::Rejected);
        }
        let account = *command.account_id.as_bytes();
        let world = *command.world_id.as_bytes();
        let profile = command.profile_revision;
        let ruleset = command.ruleset_revision;
        let content = command.content_revision;
        let starter = command.starter_template_revision;
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            // The write must come from the currently registered process incarnation.
            if !prove_current_incarnation(&mut tx, &proof).await? {
                return Err(DurabilityError::Unavailable);
            }
            sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES (encode($1, 'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(account.as_slice()).execute(&mut *tx).await?;
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id = encode($1, 'hex')::uuid FOR UPDATE")
                .bind(account.as_slice()).fetch_one(&mut *tx).await?;
            if let Some(row) = sqlx::query("SELECT o.command_binding, o.character_id::text, o.event_id::text, o.transaction_id::text, a.payload FROM game_character_operation_receipts o LEFT JOIN game_character_audit_outbox a ON a.event_id = o.event_id AND a.character_id = o.character_id WHERE o.operation_id = encode($1, 'hex')::uuid")
                .bind(operation.as_slice()).fetch_optional(&mut *tx).await? {
                let old: Vec<u8> = row.try_get("command_binding")?;
                if old != command_binding { return Ok(Err(CharacterAuthorityError::Conflict)); }
                let record = read_record_row(&row, account, world)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(record));
            }
            let ids = sqlx::query("SELECT game_character_uuid_v7()::text AS character_id, game_character_uuid_v7()::text AS event_id, game_character_uuid_v7()::text AS transaction_id")
                .fetch_one(&mut *tx).await?;
            let character = uuid_text(ids.try_get("character_id")?)?;
            let event = uuid_text(ids.try_get("event_id")?)?;
            let transaction = uuid_text(ids.try_get("transaction_id")?)?;
            let payload = encode_bootstrap(&account, &character, &world);
            sqlx::query("INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, 1, $4, $5, $6, $7)")
                .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).bind(profile).bind(ruleset).bind(content).bind(starter).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_audit_outbox(event_id, transaction_id, transaction_ordinal, transaction_count, event_type_id, schema_revision, retention_profile_id, character_id, occurred_at, expires_at, payload, payload_sha256, publication_state) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, encode($6,'hex')::uuid, floor(extract(epoch FROM statement_timestamp())*1000)::bigint, floor(extract(epoch FROM statement_timestamp())*1000)::bigint + 7776000000, $7, sha256($7), 1)")
                .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED)).bind(i64::from(EVENT_SCHEMA_REVISION)).bind(RETENTION_PROFILE).bind(character.as_slice()).bind(&payload).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id) VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, 1, encode($6,'hex')::uuid, encode($7,'hex')::uuid)")
                .bind(operation.as_slice()).bind(command_binding).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(event.as_slice()).bind(transaction.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterAuthorityRecord { account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?, character_id: CharacterId::from_bytes(character).map_err(|_| DurabilityError::Unavailable)?, world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?, revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?, event_id: event, transaction_id: transaction, payload: Some(payload) }))
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
            let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text, a.payload FROM game_character_roots r JOIN game_character_operation_receipts o USING (character_id) LEFT JOIN game_character_audit_outbox a ON a.event_id = o.event_id AND a.character_id = r.character_id WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1").bind(character.as_slice()).fetch_optional(&mut *tx).await?;
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
        let record = seal.record.clone();
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
        if recovery.record.recovery_generation != 1 || recovery.record.predecessor_generation != 0 {
            return Err(CharacterAuthorityError::Rejected);
        }
        let record = recovery.record.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            // Any surviving Character row is evidence of prior state: never "fresh".
            let existing: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_recovery_admissions) + (SELECT count(*) FROM game_character_account_guards) + (SELECT count(*) FROM game_character_roots) + (SELECT count(*) FROM game_character_operation_receipts) + (SELECT count(*) FROM game_character_audit_outbox) + (SELECT count(*) FROM game_character_audit_legal_holds)")
                .fetch_one(&mut *tx).await?;
            if existing != 0 { return Ok(Err(CharacterAuthorityError::Conflict)); }
            insert_recovery_admission(&mut tx, &record).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Reconcile a strict external successor while the exclusive generation guard is held.
    pub async fn reconcile_character_recovery(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
    ) -> Result<()> {
        let record = recovery.record.clone();
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
            let rows = sqlx::query("SELECT event_id::text, transaction_id::text, occurred_at, payload FROM game_character_audit_outbox WHERE publication_state = 1 ORDER BY occurred_at, event_id LIMIT $1")
                .bind(i64::from(limit)).fetch_all(&mut *tx).await?;
            let mut deliveries = Vec::with_capacity(rows.len());
            for row in &rows {
                deliveries.push(CharacterAuditDelivery {
                    event_id: uuid_text(row.try_get("event_id")?)?,
                    transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                    occurred_at: row.try_get("occurred_at")?,
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

    /// Place an explicit legal hold on one retained event; it blocks ordinary expiry.
    pub async fn place_character_audit_legal_hold(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        event_id: [u8; 16],
        reason: &str,
        authorizing_actor: &str,
    ) -> Result<[u8; 16]> {
        if reason.is_empty() || reason.len() > MAX_HOLD_REASON_BYTES || !bounded(authorizing_actor)
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let reason = reason.to_owned();
        let actor = authorizing_actor.to_owned();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let retained: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_audit_outbox WHERE event_id = encode($1,'hex')::uuid FOR UPDATE")
                .bind(event_id.as_slice()).fetch_optional(&mut *tx).await?;
            if retained.is_none() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let active: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_audit_legal_holds WHERE event_id = encode($1,'hex')::uuid AND released_at IS NULL")
                .bind(event_id.as_slice()).fetch_optional(&mut *tx).await?;
            if active.is_some() {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            let hold: String = sqlx::query_scalar("INSERT INTO game_character_audit_legal_holds(hold_id, event_id, reason, authorizing_actor, started_at) VALUES (game_character_uuid_v7(), encode($1,'hex')::uuid, $2, $3, floor(extract(epoch FROM statement_timestamp())*1000)::bigint) RETURNING hold_id::text")
                .bind(event_id.as_slice()).bind(reason).bind(actor).fetch_one(&mut *tx).await?;
            let hold = uuid_text(&hold)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(hold))
        })).await?
    }

    /// Release an active hold; the event returns to its ordinary expiry.
    pub async fn release_character_audit_legal_hold(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        hold_id: [u8; 16],
        releasing_actor: &str,
    ) -> Result<()> {
        if !bounded(releasing_actor) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let actor = releasing_actor.to_owned();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let released = sqlx::query("UPDATE game_character_audit_legal_holds SET released_at = greatest(started_at, floor(extract(epoch FROM statement_timestamp())*1000)::bigint), released_by = $2 WHERE hold_id = encode($1,'hex')::uuid AND released_at IS NULL")
                .bind(hold_id.as_slice()).bind(actor).execute(&mut *tx).await?;
            if released.rows_affected() != 1 {
                return Ok(Err(CharacterAuthorityError::Conflict));
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
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let deleted = sqlx::query("DELETE FROM game_character_audit_outbox WHERE event_id IN (SELECT a.event_id FROM game_character_audit_outbox a WHERE a.expires_at <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint AND NOT EXISTS (SELECT 1 FROM game_character_audit_legal_holds h WHERE h.event_id = a.event_id AND h.released_at IS NULL) ORDER BY a.expires_at, a.event_id LIMIT $1 FOR UPDATE)")
                .bind(i64::from(batch)).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(deleted.rows_affected()))
        })).await?
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

async fn assert_recovery_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let row = sqlx::query("SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, predecessor_digest, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE")
        .fetch_optional(&mut **tx).await?
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
        payload: row.try_get("payload")?,
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
        payload: row.try_get("payload")?,
    })
}

/// Complete first-slice integrity: each root has its exact receipt, including the
/// reconstructed command binding; every retained audit event binds to that
/// receipt and root with an intact payload hash. Expired events may be absent.
async fn verify_character_integrity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_operation_receipts o USING (character_id) \
           LEFT JOIN game_character_audit_outbox a ON a.event_id = o.event_id \
          WHERE o.character_id IS NULL OR o.account_id <> r.account_id OR o.world_id <> r.world_id \
             OR o.character_revision <> r.character_revision \
             OR o.command_binding <> uuid_send(r.account_id) || uuid_send(r.world_id) \
                || int2send(octet_length(r.profile_revision)::int2) || convert_to(r.profile_revision, 'UTF8') \
                || int2send(octet_length(r.ruleset_revision)::int2) || convert_to(r.ruleset_revision, 'UTF8') \
                || int2send(octet_length(r.content_revision)::int2) || convert_to(r.content_revision, 'UTF8') \
                || int2send(octet_length(r.starter_template_revision)::int2) || convert_to(r.starter_template_revision, 'UTF8') \
             OR (a.event_id IS NOT NULL AND (a.character_id <> r.character_id \
                 OR a.transaction_id <> o.transaction_id OR a.payload_sha256 <> sha256(a.payload))) \
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
