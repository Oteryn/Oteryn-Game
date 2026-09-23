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
};
use oteryn_game_server::domain::{AccountId, CharacterId, CharacterRevision, WorldId};
use sqlx::Row;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, CharacterAuthorityError>;
const MAX_CONTEXT_BYTES: usize = 128;
const MAX_OPERATION_BINDING_BYTES: usize = 1024;
static UUID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

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
    pub payload: Vec<u8>,
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CONTEXT_BYTES
}

fn uuid_v7() -> std::result::Result<[u8; 16], CharacterAuthorityError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CharacterAuthorityError::Rejected)?
        .as_millis();
    let millis = u64::try_from(millis).map_err(|_| CharacterAuthorityError::Rejected)?;
    let sequence = UUID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0_u8; 16];
    bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
    bytes[6..8].copy_from_slice(&((sequence as u16) | 0x7000).to_be_bytes());
    bytes[8..].copy_from_slice(&(sequence.rotate_left(23) | (1_u64 << 63)).to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(bytes)
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
        recovery: &SealedCharacterRecoveryFence<'_>,
        proof: &NodeIncarnationProof,
        command: BootstrapCommand,
    ) -> Result<CharacterAuthorityRecord> {
        if command.operation_id.iter().all(|byte| *byte == 0)
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
        let recovery = recovery.record.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            prove_current_incarnation(&mut tx, &proof).await.map_err(|_| DurabilityError::Unavailable)?;
            sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES (encode($1, 'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(account.as_slice()).execute(&mut *tx).await?;
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id = encode($1, 'hex')::uuid FOR UPDATE")
                .bind(account.as_slice()).fetch_one(&mut *tx).await?;
            if let Some(row) = sqlx::query("SELECT command_binding, character_id::text, event_id::text, transaction_id::text, payload FROM game_character_operation_receipts WHERE operation_id = encode($1, 'hex')::uuid")
                .bind(operation.as_slice()).fetch_optional(&mut *tx).await? {
                let old: Vec<u8> = row.try_get("command_binding")?;
                if old != command_binding { return Ok(Err(CharacterAuthorityError::Conflict)); }
                let record = read_record_row(&row, account, world)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(record));
            }
            let character = uuid_v7().map_err(|_| DurabilityError::Unavailable)?;
            let event = uuid_v7().map_err(|_| DurabilityError::Unavailable)?;
            let transaction = uuid_v7().map_err(|_| DurabilityError::Unavailable)?;
            let payload = encode_bootstrap(&account, &character, &world);
            sqlx::query("INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, 1, $4, $5, $6, $7)")
                .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).bind(profile).bind(ruleset).bind(content).bind(starter).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_audit_outbox(event_id, transaction_id, transaction_ordinal, transaction_count, event_type_id, schema_revision, retention_profile_id, character_id, occurred_at, payload, payload_sha256, publication_state) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, encode($6,'hex')::uuid, floor(extract(epoch FROM statement_timestamp())*1000)::bigint, $7, sha256($7), 1)")
                .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED)).bind(i64::from(EVENT_SCHEMA_REVISION)).bind(RETENTION_PROFILE).bind(character.as_slice()).bind(&payload).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id, payload) VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, 1, encode($6,'hex')::uuid, encode($7,'hex')::uuid, $8)")
                .bind(operation.as_slice()).bind(command_binding).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(event.as_slice()).bind(transaction.as_slice()).bind(&payload).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterAuthorityRecord { account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?, character_id: CharacterId::from_bytes(character).map_err(|_| DurabilityError::Unavailable)?, world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?, revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?, event_id: event, transaction_id: transaction, payload }))
        })).await?
    }

    pub async fn read_current_character(
        &self,
        recovery: &SealedCharacterRecoveryFence<'_>,
        character_id: CharacterId,
    ) -> Result<CharacterAuthorityRecord> {
        let character = *character_id.as_bytes();
        let recovery = recovery.record.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text, o.payload FROM game_character_roots r JOIN game_character_operation_receipts o USING (character_id) WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1").bind(character.as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { return Ok(Err(CharacterAuthorityError::Rejected)); };
            let record = decode_current_row(&row)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(record))
        })).await?
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
            let existing: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_recovery_admissions) + (SELECT count(*) FROM game_character_roots) + (SELECT count(*) FROM game_character_operation_receipts) + (SELECT count(*) FROM game_character_audit_outbox)")
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
                insert_recovery_admission(&mut tx, &record).await?;
            } else {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            // This is the explicit reconciliation point. Domain-specific integrity checks grow
            // here; restored receipts/audit/outbox never replace the external predecessor proof.
            sqlx::query("SELECT 1 FROM game_character_roots r LEFT JOIN game_character_operation_receipts o USING (character_id) LEFT JOIN game_character_audit_outbox a ON a.event_id = o.event_id WHERE o.character_id IS NULL OR a.event_id IS NULL LIMIT 1")
                .fetch_optional(&mut *tx).await?
                .map_or(Ok(()), |_| Err(DurabilityError::Unavailable))?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }
}

async fn insert_recovery_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, issued_at, issuer_identity, reconciled_at) VALUES ($1, $2::numeric, encode($3,'hex')::uuid, $4::numeric, $5::numeric, $6, floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
        .bind(&record.authority_scope_id)
        .bind(record.recovery_generation.to_string())
        .bind(record.recovery_event_id.as_slice())
        .bind(record.predecessor_generation.to_string())
        .bind(record.issued_at.to_string())
        .bind(&record.issuer_identity)
        .execute(&mut **tx).await?;
    Ok(())
}

async fn assert_recovery_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let row = sqlx::query("SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE")
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
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
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
