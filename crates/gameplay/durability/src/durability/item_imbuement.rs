//! PARITY-IMBUE-FORGE-WIRE-1 shrine apply and clear (IMBUE-FORGE0 §4.1, migration 0196).
//!
//! One fenced Type 2 transaction per accepted command: the CommandRef record, the consumed
//! backpack materials, the item-only fee (cause 7) and the one `game_item_imbuements` row the
//! record's trigger writes or removes. A refusal writes nothing and is not replayed; a replay of
//! the same `(session, command)` with the same binding answers OK, another binding
//! `REVISION_CHANGED`.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence, uuid_text};
use super::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_fee_burn::{FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts};
use super::item_fee_burn_audit::imbuement_cause::{
    IMBUEMENT_ACTION_APPLY, IMBUEMENT_ACTION_CLEAR, IMBUEMENT_MATERIALS_MAX,
};
use super::item_fee_burn_audit::{OneItemImbuementMaterialV1, OneItemImbuementV1};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{
    ITEM_LIFECYCLE_LIVE, OneItemStateV1, Type2Transaction, check_uuid_v7,
};
use super::item_transfer::definition_message;
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::imbuement::ImbuementResult;
use sqlx::Row;

/// One recipe material: `count` units from one plain direct backpack stack of the definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbuementMaterialNeed {
    pub production_key: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImbuementWrite {
    Apply {
        slot: u8,
        /// The stored category, the wire family minus one.
        category: u8,
        imbuement_key: String,
        imbuement_revision: String,
        tier: u8,
        gold: u64,
        materials: Vec<ImbuementMaterialNeed>,
    },
    Clear {
        slot: u8,
        gold: u64,
    },
}

#[derive(Debug, Clone)]
pub struct ImbuementCommand {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub command_id: u64,
    pub binding: [u8; 32],
    pub content_digest: [u8; 32],
    pub item: [u8; 16],
    pub write: ImbuementWrite,
    pub coins: FeeChangeFacts,
    pub occurred_at_unix_ms: i64,
}

#[derive(Debug)]
pub enum ImbuementWriteError {
    InvalidInput,
    /// The holder's current gameplay fence does not hold; nothing was written.
    AuthorityRejected,
    Unavailable(DurabilityError),
}

/// A filled-scroll or etcher mutation. The source is consumed by the same receipt-triggered
/// transaction which applies or clears the target; the writer re-resolves both custody rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImbuementItemWrite {
    ApplyScroll {
        slot: u8,
        category: u8,
        imbuement_key: String,
        imbuement_revision: String,
        tier: u8,
    },
    Etch { slots: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct ImbuementItemCommand {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub command_id: u64,
    pub binding: [u8; 32],
    pub content_digest: [u8; 32],
    pub source: [u8; 16],
    pub source_production_key: String,
    pub target: [u8; 16],
    pub write: ImbuementItemWrite,
    pub occurred_at_unix_ms: i64,
}

impl From<DurabilityError> for ImbuementWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for ImbuementWriteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

type Result<T> = std::result::Result<T, ImbuementWriteError>;

async fn replay(
    connection: &mut sqlx::PgConnection,
    session: GameSessionId,
    command_id: u64,
    binding: [u8; 32],
) -> Result<Option<ImbuementResult>> {
    let stored: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT request_binding FROM game_item_imbuement_records \
          WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
    )
    .bind(session.as_bytes().as_slice())
    .bind(command_id.to_string())
    .fetch_optional(connection)
    .await?;
    Ok(stored.map(|stored| {
        if stored == binding {
            ImbuementResult::Ok
        } else {
            ImbuementResult::RevisionChanged
        }
    }))
}

/// The canonical hyphenated text of a UUID, as PostgreSQL prints it.
fn uuid_string(value: &[u8]) -> String {
    let mut text = String::with_capacity(36);
    for (index, byte) in value.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn state(
    item: &[u8],
    world: &[u8],
    definition: &TypedDefinitionRef,
    quantity: u32,
    lifecycle: u32,
) -> OneItemStateV1 {
    OneItemStateV1 {
        item_instance_id: item.to_vec(),
        world_id: world.to_vec(),
        definition: Some(definition_message(definition)),
        quantity,
        lifecycle,
        key_number: 0,
    }
}

fn definition(row: &sqlx::postgres::PgRow) -> Result<TypedDefinitionRef> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}

fn quantity(row: &sqlx::postgres::PgRow) -> Result<u32> {
    u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| DurabilityError::InvalidStoredState.into())
}

/// The command's writes inside the fenced transaction; `Ok(refusal)` leaves it to roll back.
async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    command: &ImbuementCommand,
) -> Result<ImbuementResult> {
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let world = world.as_bytes().as_slice();
    let bag: Option<String> = sqlx::query_scalar(
        "SELECT i.item_instance_id::text FROM game_item_container_slots s \
           JOIN game_item_instances i USING(item_instance_id) \
          WHERE s.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i",
    )
    .bind(character)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some(bag) = bag else {
        return Ok(ImbuementResult::RevisionChanged);
    };
    let bag = uuid_text(&bag)?;
    let Some(target) = sqlx::query(
        "SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref, \
                i.quantity,e.placement_ordinal::text \
           FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) \
          WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid \
            AND e.character_id=encode($3,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i",
    )
    .bind(command.item.as_slice())
    .bind(bag.as_slice())
    .bind(character)
    .fetch_optional(&mut ***tx)
    .await?
    else {
        return Ok(ImbuementResult::RevisionChanged);
    };
    let target_definition = definition(&target)?;
    let target_ordinal = numeric_u64(&target, "placement_ordinal")?;
    let target_state = state(
        &command.item,
        world,
        &target_definition,
        quantity(&target)?,
        ITEM_LIFECYCLE_LIVE,
    );
    let rows = sqlx::query(
        "SELECT slot::bigint,category::bigint,imbuement_key,imbuement_revision,tier::bigint \
           FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY slot",
    )
    .bind(command.item.as_slice())
    .fetch_all(&mut ***tx)
    .await?;
    let mut stored = Vec::with_capacity(rows.len());
    for row in &rows {
        stored.push((
            row.try_get::<i64, _>(0)?,
            row.try_get::<i64, _>(1)?,
            row.try_get::<String, _>(2)?,
            row.try_get::<String, _>(3)?,
            row.try_get::<i64, _>(4)?,
        ));
    }
    let (action, slot, category, key, revision, tier, gold, needs) = match &command.write {
        ImbuementWrite::Apply {
            slot,
            category,
            imbuement_key,
            imbuement_revision,
            tier,
            gold,
            materials,
        } => {
            if stored.iter().any(|(stored_slot, stored_category, ..)| {
                *stored_slot == i64::from(*slot) || *stored_category == i64::from(*category)
            }) {
                return Ok(ImbuementResult::SlotOccupied);
            }
            (
                IMBUEMENT_ACTION_APPLY,
                *slot,
                *category,
                imbuement_key.clone(),
                imbuement_revision.clone(),
                *tier,
                *gold,
                materials.as_slice(),
            )
        }
        ImbuementWrite::Clear { slot, gold } => {
            let Some((_, category, key, revision, tier)) = stored
                .iter()
                .find(|(stored_slot, ..)| *stored_slot == i64::from(*slot))
            else {
                return Ok(ImbuementResult::NotAllowed);
            };
            (
                IMBUEMENT_ACTION_CLEAR,
                *slot,
                u8::try_from(*category).map_err(|_| DurabilityError::InvalidStoredState)?,
                key.clone(),
                revision.clone(),
                u8::try_from(*tier).map_err(|_| DurabilityError::InvalidStoredState)?,
                *gold,
                &[][..],
            )
        }
    };
    // Each material from the first plain direct stack holding the whole count, in display order.
    let mut materials = Vec::with_capacity(needs.len());
    let mut lines: Vec<OneItemImbuementMaterialV1> = Vec::with_capacity(needs.len());
    for need in needs {
        let taken: Vec<String> = lines
            .iter()
            .filter_map(|line| line.before.as_ref())
            .map(|before| uuid_string(&before.item_instance_id))
            .collect();
        let Some(row) = sqlx::query(
            "SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key, \
                    i.definition_revision_ref,i.quantity,e.placement_ordinal::text,to_jsonb(i)::text AS before \
               FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) \
              WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid \
                AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 \
                AND i.quantity>=$4 AND i.item_instance_id<>encode($5,'hex')::uuid \
                AND NOT (i.item_instance_id::text=ANY($6)) \
                AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) \
                AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) \
                AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) \
                AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id) \
                AND NOT EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) \
                AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=i.item_instance_id) \
              ORDER BY e.placement_ordinal DESC LIMIT 1 FOR UPDATE OF i",
        )
        .bind(bag.as_slice())
        .bind(character)
        .bind(&need.production_key)
        .bind(i64::from(need.count))
        .bind(command.item.as_slice())
        .bind(&taken)
        .fetch_optional(&mut ***tx)
        .await?
        else {
            return Ok(ImbuementResult::MaterialsMissing);
        };
        let item = uuid_text(&row.try_get::<String, _>("item_instance_id")?)?;
        let ordinal = numeric_u64(&row, "placement_ordinal")?;
        let before = quantity(&row)?;
        let after = before - need.count;
        let definition = definition(&row)?;
        let before_json: serde_json::Value = serde_json::from_str(&row.try_get::<String, _>("before")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        materials.push(serde_json::json!({
            "item": uuid_string(&item),
            "ordinal": ordinal.to_string(),
            "count": need.count,
            "before": before_json,
        }));
        lines.push(OneItemImbuementMaterialV1 {
            before: Some(state(&item, world, &definition, before, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(
                &item,
                world,
                &definition,
                after,
                if after == 0 {
                    ITEM_LIFECYCLE_RETIRED
                } else {
                    ITEM_LIFECYCLE_LIVE
                },
            )),
            placement_ordinal: ordinal,
        });
    }
    if lines.len() > IMBUEMENT_MATERIALS_MAX {
        return Err(ImbuementWriteError::InvalidInput);
    }
    sqlx::query(
        "INSERT INTO game_item_imbuement_records(transaction_id,event_id,game_session_id,command_id, \
           character_id,world_id,request_binding,action,item_instance_id,parent_item_instance_id, \
           item_ordinal,slot,category,imbuement_key,imbuement_revision,tier,fee_gold_units,materials,occurred_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric, \
           encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid, \
           $11::text::numeric,$12,$13,$14,$15,$16,$17,$18::text::jsonb,$19)",
    )
    .bind(command.transaction_id.as_slice())
    .bind(command.event_id.as_slice())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(command.command_id.to_string())
    .bind(character)
    .bind(world)
    .bind(command.binding.as_slice())
    .bind(i16::try_from(action).map_err(|_| ImbuementWriteError::InvalidInput)?)
    .bind(command.item.as_slice())
    .bind(bag.as_slice())
    .bind(target_ordinal.to_string())
    .bind(i16::from(slot))
    .bind(i16::from(category))
    .bind(&key)
    .bind(&revision)
    .bind(i16::from(tier))
    .bind(i64::try_from(gold).map_err(|_| ImbuementWriteError::InvalidInput)?)
    .bind(serde_json::Value::Array(materials).to_string())
    .bind(command.occurred_at_unix_ms)
    .execute(&mut ***tx)
    .await?;
    // Materials leave before the fee plans its coins, so the change follows the remaining entries.
    for line in &lines {
        let (Some(before), Some(after)) = (&line.before, &line.after) else {
            return Err(ImbuementWriteError::InvalidInput);
        };
        sqlx::query(
            "UPDATE game_item_instances SET quantity=$2,lifecycle=$3,last_transaction_id=encode($4,'hex')::uuid \
              WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=$5",
        )
        .bind(before.item_instance_id.as_slice())
        .bind(i64::from(after.quantity))
        .bind(i16::try_from(after.lifecycle).map_err(|_| ImbuementWriteError::InvalidInput)?)
        .bind(command.transaction_id.as_slice())
        .bind(i64::from(before.quantity))
        .execute(&mut ***tx)
        .await?;
        if after.quantity == 0 {
            sqlx::query(
                "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(before.item_instance_id.as_slice())
            .execute(&mut ***tx)
            .await?;
        }
    }
    let audit = OneItemImbuementV1 {
        occurrence: command.transaction_id.to_vec(),
        action,
        item: Some(target_state),
        placement_ordinal: target_ordinal,
        slot: u32::from(slot),
        category: u32::from(category),
        imbuement_key: key,
        imbuement_revision: revision,
        tier: u32::from(tier),
        materials: lines,
    };
    if audit.fee_gold_units() != Some(gold) {
        return Err(ImbuementWriteError::InvalidInput);
    }
    let fee = FeeBurnRequest {
        cause: FeeBurnCause::Imbuement {
            occurrence: command.transaction_id,
            imbuement: Box::new(audit),
        },
        fee_gold_units: gold,
        transaction_id: command.transaction_id,
        event_id: command.event_id,
        occurred_at_unix_ms: command.occurred_at_unix_ms,
        server_build_id: "oteryn-game-server".into(),
        change: command.coins.clone(),
    };
    match super::item_fee_burn::burn_fee_in_transaction(tx, fence, &fee).await {
        Ok(_) => Ok(ImbuementResult::Ok),
        Err(FeeBurnError::InsufficientFunds) => Ok(ImbuementResult::InsufficientFunds),
        Err(FeeBurnError::Unavailable(error)) => Err(error.into()),
        Err(_) => Err(ImbuementWriteError::InvalidInput),
    }
}

impl DurabilityRoot {
    /// Replay of an accepted command before any handle or content resolution.
    pub async fn read_imbuement_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        session: GameSessionId,
        command_id: u64,
        binding: [u8; 32],
    ) -> Result<Option<ImbuementResult>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ImbuementWriteError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    match replay(&mut tx, session, command_id, binding).await {
                        Ok(value) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(ImbuementWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Atomic shrine apply or clear under the recovery, session, lease and scope fence. Only an
    /// `Ok` commits; every refusal rolls back.
    pub async fn commit_imbuement_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: ImbuementCommand,
    ) -> Result<ImbuementResult> {
        if check_uuid_v7(&command.transaction_id).is_err()
            || check_uuid_v7(&command.event_id).is_err()
            || check_uuid_v7(&command.item).is_err()
            || command.transaction_id == command.event_id
            || command.command_id == 0
            || command.occurred_at_unix_ms <= 0
        {
            return Err(ImbuementWriteError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ImbuementWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||$2,196))",
                    )
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(command.command_id.to_string())
                    .execute(&mut **tx)
                    .await?;
                    match replay(&mut tx, fence.game_session_id, command.command_id, command.binding)
                        .await
                    {
                        Ok(Some(result)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(result));
                        }
                        Ok(None) => (),
                        Err(ImbuementWriteError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(Err(ImbuementWriteError::AuthorityRejected));
                    }
                    if super::character_equipment::assert_current_equipment_in_transaction(
                        &mut tx,
                        &recovery,
                        &node,
                        &fence.item_fence(),
                        None,
                        command.content_digest,
                    )
                    .await
                    .is_err()
                    {
                        return Ok(Err(ImbuementWriteError::AuthorityRejected));
                    }
                    match apply(&mut tx, &fence, &command).await {
                        Ok(ImbuementResult::Ok) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(ImbuementResult::Ok))
                        }
                        Ok(refusal) => Ok(Ok(refusal)),
                        Err(ImbuementWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn imbuement_item_replay(
    connection: &mut sqlx::PgConnection,
    session: GameSessionId,
    command_id: u64,
    binding: [u8; 32],
) -> Result<Option<ImbuementResult>> {
    let stored: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT request_binding FROM game_item_imbuement_item_records \
          WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
    )
    .bind(session.as_bytes().as_slice())
    .bind(command_id.to_string())
    .fetch_optional(connection)
    .await?;
    Ok(stored.map(|stored| {
        if stored == binding {
            ImbuementResult::Ok
        } else {
            ImbuementResult::RevisionChanged
        }
    }))
}

impl DurabilityRoot {
    /// Reconcile a USE-with retry before resolving either session-local item handle.
    pub async fn read_imbuement_item_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        session: GameSessionId,
        command_id: u64,
        binding: [u8; 32],
    ) -> Result<Option<ImbuementResult>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ImbuementWriteError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    match imbuement_item_replay(&mut tx, session, command_id, binding).await {
                        Ok(value) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(ImbuementWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// One fenced durable outcome for source consumption and target mutation.
    pub async fn commit_imbuement_item_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: ImbuementItemCommand,
    ) -> Result<ImbuementResult> {
        if check_uuid_v7(&command.transaction_id).is_err()
            || check_uuid_v7(&command.event_id).is_err()
            || check_uuid_v7(&command.source).is_err()
            || check_uuid_v7(&command.target).is_err()
            || command.transaction_id == command.event_id
            || command.source == command.target
            || command.command_id == 0
            || command.occurred_at_unix_ms <= 0
        {
            return Err(ImbuementWriteError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ImbuementWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||$2,229))")
                        .bind(fence.game_session_id.as_bytes().as_slice())
                        .bind(command.command_id.to_string())
                        .execute(&mut **tx)
                        .await?;
                    match imbuement_item_replay(
                        &mut tx,
                        fence.game_session_id,
                        command.command_id,
                        command.binding,
                    )
                    .await
                    {
                        Ok(Some(result)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(result));
                        }
                        Ok(None) => {}
                        Err(ImbuementWriteError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err()
                        || super::character_equipment::assert_current_equipment_in_transaction(
                            &mut tx,
                            &recovery,
                            &node,
                            &fence.item_fence(),
                            None,
                            command.content_digest,
                        )
                        .await
                        .is_err()
                    {
                        return Ok(Err(ImbuementWriteError::AuthorityRejected));
                    }
                    let character = fence.character_id.as_bytes().as_slice();
                    let bag: Option<String> = sqlx::query_scalar(
                        "SELECT i.item_instance_id::text FROM game_item_container_slots s \
                         JOIN game_item_instances i USING(item_instance_id) \
                         WHERE s.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i",
                    )
                    .bind(character)
                    .fetch_optional(&mut **tx)
                    .await?;
                    let Some(bag) = bag else { return Ok(Ok(ImbuementResult::RevisionChanged)); };
                    let bag = uuid_text(&bag)?;
                    let ordinals = sqlx::query(
                        "SELECT (SELECT placement_ordinal::text FROM game_item_container_entries \
                           WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid \
                             AND character_id=encode($4,'hex')::uuid) source_ordinal, \
                                (SELECT placement_ordinal::text FROM game_item_container_entries \
                           WHERE item_instance_id=encode($2,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid \
                             AND character_id=encode($4,'hex')::uuid) target_ordinal",
                    )
                    .bind(command.source.as_slice())
                    .bind(command.target.as_slice())
                    .bind(bag.as_slice())
                    .bind(character)
                    .fetch_one(&mut **tx)
                    .await?;
                    let (Some(source_ordinal), Some(target_ordinal)) = (
                        ordinals.try_get::<Option<String>, _>(0)?,
                        ordinals.try_get::<Option<String>, _>(1)?,
                    ) else { return Ok(Ok(ImbuementResult::RevisionChanged)); };
                    let (action, slot, category, key, revision, tier, slots) = match command.write {
                        ImbuementItemWrite::ApplyScroll { slot, category, imbuement_key, imbuement_revision, tier } =>
                            (1_i16, Some(i16::from(slot)), Some(i16::from(category)), Some(imbuement_key), Some(imbuement_revision), Some(i16::from(tier)), Vec::new()),
                        ImbuementItemWrite::Etch { slots } =>
                            (2_i16, None, None, None, None, None, slots.into_iter().map(i16::from).collect()),
                    };
                    let inserted = sqlx::query(
                        "INSERT INTO game_item_imbuement_item_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,request_binding,action,source_item_instance_id,source_production_key,target_item_instance_id,parent_item_instance_id,source_ordinal,target_ordinal,slot,category,imbuement_key,imbuement_revision,tier,cleared_slots,occurred_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,$10,encode($11,'hex')::uuid,encode($12,'hex')::uuid,$13::text::numeric,$14::text::numeric,$15,$16,$17,$18,$19,$20,$21)",
                    )
                    .bind(command.transaction_id.as_slice()).bind(command.event_id.as_slice())
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id.to_string())
                    .bind(character).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(command.binding.as_slice()).bind(action).bind(command.source.as_slice())
                    .bind(command.source_production_key).bind(command.target.as_slice()).bind(bag.as_slice())
                    .bind(source_ordinal).bind(target_ordinal).bind(slot).bind(category).bind(key)
                    .bind(revision).bind(tier).bind(slots).bind(command.occurred_at_unix_ms)
                    .execute(&mut **tx).await;
                    match inserted {
                        Ok(_) => { tx.commit(deadline).await?; Ok(Ok(ImbuementResult::Ok)) }
                        Err(error) if error.as_database_error().is_some_and(|e| e.code().as_deref()==Some("23514")) => Ok(Ok(ImbuementResult::RevisionChanged)),
                        Err(error) => Err(error.into()),
                    }
                })
            })
            .await?
    }
}

#[cfg(test)]
#[path = "item_imbuement_tests.rs"]
mod tests;
