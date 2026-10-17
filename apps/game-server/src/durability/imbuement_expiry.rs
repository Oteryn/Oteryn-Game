//! Private continuation's native in-transaction Expire compositor.
//! No Root/transport entrypoint, timer, Content admission, migration or runtime grant is
//! installed. The caller cannot turn supplied zero DATA into authority: the complete row is
//! re-read under real current recovery/session/node/World/Channel/equipment fences.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::{
    EquipmentError, assert_current_equipment_in_transaction, read_equipment_in_transaction,
};
use super::imbuement_expiry_audit::{
    ImbuementExpiryEventIdentity, OneItemImbuementExpiryV1, OneItemImbuementSlotV1,
    check_expired_slot, encode_imbuement_expiry_event,
};
use super::item_mint_audit::{
    self as audit, OneItemStateV1, OneItemTypedDefinitionRevisionV1, SelectedType2Tuple,
    Type2Transaction,
};
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::{Acquire, Postgres, Row, Transaction};

/// Proposed installation follows the checkpoint schema, only in the native test database.
/// The shared migration number and production admission are deliberately unresolved.
pub const IMBUEMENT_EXPIRY_SCHEMA_PROPOSAL: &str =
    include_str!("imbuement_expiry_schema_proposal.sql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbuementExpiryRequest {
    pub item_instance_id: [u8; 16],
    pub before: OneItemImbuementSlotV1,
    /// A fixed non-caller Expire occurrence, distinct from the retained applying occurrence.
    pub expiry_occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImbuementExpiryOutcome {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug)]
pub enum ImbuementExpiryError {
    InvalidInput,
    AuthorityRejected,
    NotEquipped,
    Stale,
    ConflictingOccurrence,
    CapacityExceeded,
    Durability(DurabilityError),
    Database(sqlx::Error),
}
impl From<sqlx::Error> for ImbuementExpiryError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}
impl From<EquipmentError> for ImbuementExpiryError {
    fn from(value: EquipmentError) -> Self {
        match value {
            EquipmentError::Database(error) => Self::Database(error),
            EquipmentError::Durability(error) => Self::Durability(error),
            EquipmentError::Rejected(_) => Self::AuthorityRejected,
        }
    }
}
type Result<T> = std::result::Result<T, ImbuementExpiryError>;

fn request_binding(request: &ImbuementExpiryRequest) -> Result<[u8; 32]> {
    check_expired_slot(&request.before).map_err(|_| ImbuementExpiryError::InvalidInput)?;
    for id in [
        &request.item_instance_id,
        &request.expiry_occurrence,
        &request.transaction_id,
        &request.event_id,
    ] {
        audit::check_uuid_v7(id).map_err(|_| ImbuementExpiryError::InvalidInput)?;
    }
    audit::check_technical_text(&request.server_build_id)
        .map_err(|_| ImbuementExpiryError::InvalidInput)?;
    if request.event_id == request.transaction_id
        || request.expiry_occurrence.as_slice() == request.before.applying_occurrence.as_slice()
        || request.occurred_at_unix_ms <= 0
        || request.occurred_at_unix_ms > i64::MAX - audit::AUDIT_RETENTION_P90D_MS
    {
        return Err(ImbuementExpiryError::InvalidInput);
    }
    // Closed, length-delimited semantic replay identity. It excludes current authority:
    // immutable historical receipts cannot authorize a new mutation after fence loss.
    let mut hash = Sha256::new();
    hash.update(b"oteryn:imbuement-expiry-request:v1");
    for id in [
        &request.item_instance_id,
        &request.expiry_occurrence,
        &request.transaction_id,
        &request.event_id,
    ] {
        hash.update(id);
    }
    hash.update(request.occurred_at_unix_ms.to_be_bytes());
    hash.update((request.server_build_id.len() as u64).to_be_bytes());
    hash.update(request.server_build_id.as_bytes());
    hash.update(request.before.encode_to_vec());
    Ok(hash.finalize().into())
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    request: &ImbuementExpiryRequest,
    binding: &[u8; 32],
) -> Result<Option<ImbuementExpiryOutcome>> {
    let stored = sqlx::query(
        "SELECT request_binding,uuid_send(character_id) AS character FROM game_imbuement_expiry_receipts \
         WHERE (character_id=encode($1,'hex')::uuid AND expiry_occurrence=encode($2,'hex')::uuid) \
            OR transaction_id=encode($3,'hex')::uuid OR event_id=encode($4,'hex')::uuid \
            OR (item_instance_id=encode($5,'hex')::uuid AND slot=$6 \
                AND applying_occurrence=encode($7,'hex')::uuid) LIMIT 2",
    )
    .bind(character.as_slice()).bind(request.expiry_occurrence.as_slice())
    .bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice())
    .bind(request.item_instance_id.as_slice()).bind(request.before.slot as i16)
    .bind(&request.before.applying_occurrence)
    .fetch_all(&mut **tx).await?;
    match stored.as_slice() {
        [] => Ok(None),
        [row]
            if row.try_get::<Vec<u8>, _>("request_binding")?.as_slice() == binding.as_slice()
                && row.try_get::<Vec<u8>, _>("character")?.as_slice() == character.as_slice() =>
        {
            Ok(Some(ImbuementExpiryOutcome::AlreadyCommitted))
        }
        _ => Err(ImbuementExpiryError::ConflictingOccurrence),
    }
}

/// Composes only in a pre-opened native Type2 transaction. Every new write obtains fresh
/// opaque native authority with command=None, then compares all retained slot fields.
/// A savepoint removes receipt, deletion witness, row change and audit together on refusal,
/// including when a caller subsequently commits the surrounding transaction.
pub async fn expire_imbuement_in_transaction(
    tx: &mut Type2Transaction<'_>,
    root: &DurabilityRoot,
    recovered: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
    request: &ImbuementExpiryRequest,
) -> Result<ImbuementExpiryOutcome> {
    let binding = request_binding(request)?;
    let recovery = recovered
        .record_for(root)
        .map_err(|_| ImbuementExpiryError::AuthorityRejected)?;
    assert_recovery_fence(tx, &recovery)
        .await
        .map_err(ImbuementExpiryError::Durability)?;
    // Read-only replay does not borrow an old receipt as current authority.
    if let Some(outcome) = replay(tx, fence.character_id.as_bytes(), request, &binding).await? {
        return Ok(outcome);
    }
    let tuple = tx.tuple();
    let mut savepoint = tx.begin().await?;
    let result = async {
        let authority = assert_current_equipment_in_transaction(
            &mut savepoint, &recovery, node, fence, None, content_digest,
        ).await?;
        let equipment = read_equipment_in_transaction(&mut savepoint, &authority).await?;
        // The native Character root lock serializes concurrent attempts. Reconcile once
        // more after acquiring it before interpreting an absent row as a stale snapshot.
        if let Some(outcome) = replay(&mut savepoint, &equipment.character, request, &binding).await? {
            return Ok(outcome);
        }
        let occupied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_imbuement_expiry_receipts WHERE created_xact_id=pg_current_xact_id())",
        ).fetch_one(&mut *savepoint).await?;
        if occupied {
            return Err(ImbuementExpiryError::CapacityExceeded);
        }
        let item = equipment.items.iter()
            .find(|item| item.item_instance_id == request.item_instance_id)
            .ok_or(ImbuementExpiryError::NotEquipped)?;
        let current = sqlx::query(
            "SELECT slot,imbuement_key,content_revision,category,tier,remaining_ms,\
                uuid_send(applying_occurrence) AS occurrence,uuid_send(transaction_id) AS applying_tx \
             FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid AND slot=$2 \
                AND octet_length(imbuement_key) BETWEEN 1 AND 512 \
                AND octet_length(content_revision) BETWEEN 1 AND 512 \
                AND octet_length(category) BETWEEN 1 AND 512 FOR UPDATE",
        ).bind(request.item_instance_id.as_slice()).bind(request.before.slot as i16)
            .fetch_optional(&mut *savepoint).await?
            .ok_or(ImbuementExpiryError::Stale)?;
        let actual = OneItemImbuementSlotV1 {
            slot: u32::try_from(current.try_get::<i16, _>("slot")?).map_err(|_| ImbuementExpiryError::Stale)?,
            imbuement_key: current.try_get("imbuement_key")?,
            content_revision: current.try_get("content_revision")?,
            category: current.try_get("category")?,
            tier: u32::try_from(current.try_get::<i16, _>("tier")?).map_err(|_| ImbuementExpiryError::Stale)?,
            remaining_ms: u64::try_from(current.try_get::<i64, _>("remaining_ms")?).map_err(|_| ImbuementExpiryError::Stale)?,
            applying_occurrence: current.try_get("occurrence")?,
            applying_transaction_id: current.try_get("applying_tx")?,
        };
        if actual != request.before {
            return Err(ImbuementExpiryError::Stale);
        }
        let (world, channel) = scope_of(fence).map_err(|_| ImbuementExpiryError::AuthorityRejected)?;
        let key_number: i32 = sqlx::query_scalar("SELECT COALESCE((SELECT key_number FROM game_item_keys WHERE item_instance_id=encode($1,'hex')::uuid),0)")
            .bind(request.item_instance_id.as_slice()).fetch_one(&mut *savepoint).await?;
        let event = OneItemImbuementExpiryV1 {
            item: Some(OneItemStateV1 {
                item_instance_id: item.item_instance_id.to_vec(), world_id: world.as_bytes().to_vec(),
                definition: Some(OneItemTypedDefinitionRevisionV1 {
                    family: item.definition.family.clone(), production_key: item.definition.production_key.clone(),
                    revision_ref: item.definition.revision_ref.clone(),
                }),
                quantity: item.quantity, lifecycle: audit::ITEM_LIFECYCLE_LIVE,
                key_number: u32::try_from(key_number).map_err(|_| ImbuementExpiryError::Stale)?,
            }),
            row_before: Some(actual), character_id: equipment.character.to_vec(),
            world_id: world.as_bytes().to_vec(), channel_id: channel.as_bytes().to_vec(),
            scope_ownership_generation: fence.scope_ownership_generation.get(),
            expiry_occurrence: request.expiry_occurrence.to_vec(),
            character_revision: equipment.character_revision, equipment_revision: equipment.revision,
            equipment_slot: u32::from(item.slot),
        };
        let envelope = encode_imbuement_expiry_event(
            ImbuementExpiryEventIdentity {
                event_id: request.event_id, transaction_id: request.transaction_id,
                occurred_at_unix_ms: request.occurred_at_unix_ms, server_build_id: &request.server_build_id,
            }, event, tuple.tuple(),
        ).map_err(|_| ImbuementExpiryError::InvalidInput)?;
        write_expiry(&mut savepoint, request, &binding, &equipment, fence, &envelope, tuple).await
    }.await;
    match result {
        Ok(value) => {
            savepoint.commit().await?;
            Ok(value)
        }
        Err(error) => {
            savepoint.rollback().await?;
            Err(error)
        }
    }
}

async fn write_expiry(
    tx: &mut Transaction<'_, Postgres>,
    request: &ImbuementExpiryRequest,
    binding: &[u8; 32],
    equipment: &super::character_equipment::EquipmentSnapshot,
    fence: &CurrentCharacterItemFence,
    envelope: &[u8],
    tuple: SelectedType2Tuple,
) -> Result<ImbuementExpiryOutcome> {
    let (world, channel) = scope_of(fence).map_err(|_| ImbuementExpiryError::AuthorityRejected)?;
    let item = equipment
        .items
        .iter()
        .find(|item| item.item_instance_id == request.item_instance_id)
        .ok_or(ImbuementExpiryError::NotEquipped)?;
    let changed = sqlx::query(
        "INSERT INTO game_imbuement_expiry_receipts(transaction_id,event_id,expiry_occurrence,\
            character_id,world_id,channel_id,scope_generation,character_revision,equipment_revision,\
            equipment_slot,item_instance_id,slot,applying_occurrence,slot_before,item_before,\
            request_binding,occurred_at,envelope_sha256,schema_revision,retention_profile_id) \
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
            encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,\
            $8::text::numeric,$9::text::numeric,$10,s.item_instance_id,s.slot,s.applying_occurrence,\
            to_jsonb(s),to_jsonb(i),$13,$14,sha256($15),$16,$17 \
         FROM game_item_imbuements s JOIN game_item_instances i USING(item_instance_id) \
         WHERE s.item_instance_id=encode($11,'hex')::uuid AND s.slot=$12 AND s.remaining_ms=0",
    ).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice())
        .bind(request.expiry_occurrence.as_slice()).bind(equipment.character.as_slice())
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(equipment.character_revision.to_string()).bind(equipment.revision.to_string())
        .bind(i16::from(item.slot)).bind(request.item_instance_id.as_slice()).bind(request.before.slot as i16)
        .bind(binding.as_slice()).bind(request.occurred_at_unix_ms).bind(envelope)
        .bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(ImbuementExpiryError::Stale);
    }
    let changed = sqlx::query(
        "DELETE FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid \
         AND slot=$2 AND imbuement_key=$3 AND content_revision=$4 AND category=$5 AND tier=$6 \
         AND remaining_ms=0 AND applying_occurrence=encode($7,'hex')::uuid \
         AND transaction_id=encode($8,'hex')::uuid",
    )
    .bind(request.item_instance_id.as_slice())
    .bind(request.before.slot as i16)
    .bind(&request.before.imbuement_key)
    .bind(&request.before.content_revision)
    .bind(&request.before.category)
    .bind(request.before.tier as i16)
    .bind(&request.before.applying_occurrence)
    .bind(&request.before.applying_transaction_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if changed != 1 {
        return Err(ImbuementExpiryError::Stale);
    }
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,\
            transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,\
            occurred_at,expires_at,envelope,envelope_sha256,publication_state) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,$3,$4,$5,encode($6,'hex')::uuid,\
            $7,$7+$8,$9,sha256($9),1)",
    )
    .bind(request.event_id.as_slice())
    .bind(request.transaction_id.as_slice())
    .bind(i64::from(audit::EVENT_TYPE_ID))
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(request.item_instance_id.as_slice())
    .bind(request.occurred_at_unix_ms)
    .bind(audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;
    // Resolve our own consistency now, before releasing the savepoint. The deferred trigger
    // remains active and rejects an enclosing compositor that later changes preserved rows.
    sqlx::query("SELECT game_assert_imbuement_expiry(encode($1,'hex')::uuid)")
        .bind(request.transaction_id.as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(ImbuementExpiryOutcome::Committed)
}
