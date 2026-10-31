use super::*;
use crate::durability::item_loop_move::carried_weight;
use crate::durability::item_mint::uuid_text;
use crate::durability::item_mint_audit::{self as mint, OneItemStateV1, Type2Transaction};
use crate::durability::item_transfer::{
    ContainerEntryPosition, definition_message, entry_message, load_entries, load_slot,
    read_derived_transfer_capacity_centi_oz, stack_maximum,
};
use crate::durability::item_transfer_audit::{OneItemCommandRefV1, OneItemInventoryV1};
use crate::durability::reward_claim_mint_audit::{
    self as audit, BossRewardClaimSourceV1, OneItemRewardClaimCauseV1, OneItemRewardClaimMintV1,
    RewardClaimMintEventScope, ScopedRewardClaimMintEventIdentity,
};
use sqlx::Row;

#[derive(Clone, Copy, Debug)]
pub(super) struct CapacityLine {
    pub quantity: u32,
    pub maximum: u32,
    pub weight: Option<u32>,
    pub container: bool,
}

/// Selects the exact sequential prefix, stopping at the first item that cannot be placed.
/// It never skips an overweight entry to grant a later reward, matching Crystal collectAll.
pub(super) fn capacity_prefix(
    lines: &[CapacityLine],
    slots: usize,
    carried: u64,
    capacity: u64,
) -> Result<usize, BossRewardCollectionStatus> {
    let mut used = carried;
    let mut count = 0;
    for line in lines.iter().take(BOSS_REWARD_COLLECTION_MAX) {
        let refusal = if count == slots {
            Some(BossRewardCollectionStatus::NoRoom)
        } else if line.quantity == 0
            || line.quantity > line.maximum
            || (line.container && line.quantity != 1)
            || line.weight.is_none()
        {
            Some(BossRewardCollectionStatus::ContentUnavailable)
        } else {
            match used.checked_add(u64::from(line.weight.unwrap_or(0)) * u64::from(line.quantity)) {
                Some(next) if next <= capacity => {
                    used = next;
                    None
                }
                _ => Some(BossRewardCollectionStatus::NoRoom),
            }
        };
        if let Some(refusal) = refusal {
            return if count == 0 { Err(refusal) } else { Ok(count) };
        }
        count += 1;
    }
    Ok(count)
}

pub(super) async fn deliver<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &BossRewardCollectRequest,
    content: &C,
    rows: Vec<store::Entitlement>,
    now: i64,
) -> Result<BossRewardCollection, DurabilityError> {
    let remaining = store::remaining(tx, fence, request.selection, now).await?;
    let refuse = |status| BossRewardCollection {
        status,
        moved: 0,
        remaining,
    };
    let Some(backpack) = load_slot(tx, fence.character_id).await? else {
        return Ok(refuse(BossRewardCollectionStatus::NoRoom));
    };
    let Some(capacity) = content
        .loop_facts(&backpack.definition)
        .and_then(|f| f.container_capacity)
    else {
        return Ok(refuse(BossRewardCollectionStatus::ContentUnavailable));
    };
    let entries = load_entries(tx, backpack.item_instance_id).await?;
    let tree_count: i64 = sqlx::query_scalar("WITH RECURSIVE tree(item_instance_id,depth) AS (SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded")
        .bind(backpack.item_instance_id.as_slice()).fetch_one(&mut ***tx).await?;
    let slots = (capacity.min(20) as usize)
        .saturating_sub(entries.len())
        .min(500_usize.saturating_sub(
            usize::try_from(tree_count).map_err(|_| DurabilityError::InvalidStoredState)?,
        ));
    let carried = match carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        content,
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return Ok(refuse(BossRewardCollectionStatus::ContentUnavailable)),
    };
    let limit = read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await?;
    let lines = rows
        .iter()
        .map(|row| {
            let facts = content.loop_facts(&row.item.definition);
            CapacityLine {
                quantity: row.item.quantity,
                maximum: facts
                    .as_ref()
                    .and_then(|f| stack_maximum(f.stack).ok())
                    .map_or(0, |(maximum, _)| maximum),
                weight: facts.as_ref().and_then(|f| f.weight),
                container: facts.is_some_and(|f| f.container_capacity.is_some()),
            }
        })
        .collect::<Vec<_>>();
    let count = match capacity_prefix(&lines, slots, carried, limit) {
        Ok(count) => count,
        Err(status) => return Ok(refuse(status)),
    };
    let now_u64 = u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut selected = Vec::with_capacity(count);
    // Reuse the personal container's real collect_all/expiry consumer for each qualified group.
    for group in rows[..count].chunk_by(|left, right| left.death == right.death) {
        let mut chest = store::chest(&group[0], *fence.character_id.as_bytes(), group.to_vec())?;
        chest
            .collect_prefix(now_u64, &mut selected, BOSS_REWARD_COLLECTION_MAX, 0)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    let mut ordinal = entries
        .iter()
        .map(|entry| entry.placement_ordinal)
        .max()
        .unwrap_or(0);
    let mut touched = std::collections::BTreeSet::new();
    for row in &selected {
        ordinal = ordinal
            .checked_add(1)
            .ok_or(DurabilityError::InvalidStoredState)?;
        mint_one(
            tx,
            fence,
            request,
            row,
            ContainerEntryPosition {
                parent_item_instance_id: backpack.item_instance_id,
                placement_ordinal: ordinal,
            },
            now,
        )
        .await?;
        touched.insert(row.death);
    }
    for death in touched {
        store::refresh_group(tx, fence, death).await?;
    }
    Ok(BossRewardCollection {
        status: BossRewardCollectionStatus::Collected,
        moved: u16::try_from(selected.len()).map_err(|_| DurabilityError::InvalidStoredState)?,
        remaining: store::remaining(tx, fence, request.selection, now).await?,
    })
}

#[allow(clippy::too_many_arguments)]
async fn mint_one(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &BossRewardCollectRequest,
    row: &store::Entitlement,
    destination: ContainerEntryPosition,
    now: i64,
) -> Result<(), DurabilityError> {
    let ids = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_id").fetch_one(&mut ***tx).await?;
    let transaction = uuid_text(ids.try_get("transaction_id")?)?;
    let event = uuid_text(ids.try_get("event_id")?)?;
    let item = uuid_text(ids.try_get("item_id")?)?;
    let scope = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => {
            RewardClaimMintEventScope::Channel(*channel_id.as_bytes())
        }
        RuntimeScopeRefV1::Instance { instance_id, .. } => {
            RewardClaimMintEventScope::Instance(instance_id)
        }
    };
    let envelope = audit::encode_scoped_reward_claim_mint_event(
        ScopedRewardClaimMintEventIdentity {
            event_id: event,
            transaction_id: transaction,
            occurred_at_unix_ms: now,
            scope,
            server_build_id: crate::durability::character_authority::SERVER_BUILD_ID,
        },
        OneItemRewardClaimMintV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: item.to_vec(),
                world_id: fence.runtime_scope.world_id().as_bytes().to_vec(),
                definition: Some(definition_message(&row.item.definition)),
                quantity: row.item.quantity,
                lifecycle: mint::ITEM_LIFECYCLE_LIVE,
                key_number: 0,
            }),
            destination: Some(OneItemInventoryV1 {
                character_id: fence.character_id.as_bytes().to_vec(),
                expected_session_generation: fence.connection_generation.get(),
                expected_game_session_id: fence.game_session_id.as_bytes().to_vec(),
                expected_character_lease_generation: fence.character_lease_generation,
                container_entry: Some(entry_message(destination)),
                equipment_container_slot: false,
            }),
            source: Some(OneItemRewardClaimCauseV1 {
                typed_cause: audit::REWARD_CLAIM_MINT_TYPED_CAUSE.into(),
                content_revision: request.content_revision.clone(),
                ruleset_revision: request.ruleset_revision.clone(),
                sim_revision: request.sim_revision.clone(),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: fence.game_session_id.as_bytes().to_vec(),
                    command_id: request.command.command_id().get(),
                }),
                reward_claim: None,
                boss_reward: Some(BossRewardClaimSourceV1 {
                    death: row.death.to_vec(),
                    item_ordinal: u32::from(row.item.ordinal),
                    boss_key: row.boss_key.clone(),
                    lease_generation: fence.character_lease_generation,
                    scope_generation: fence.scope_ownership_generation.get(),
                }),
            }),
            before_semantically_absent: true,
            inbox: None,
        },
        tx.tuple().tuple(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_boss_reward_item_collections(transaction_id,event_id,item_instance_id,game_session_id,command_id,character_id,world_id,death_id,item_ordinal,parent_item_instance_id,placement_ordinal,occurred_at,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid,$11::text::numeric,$12,sha256($13))")
        .bind(transaction.as_slice()).bind(event.as_slice()).bind(item.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(row.death.as_slice()).bind(i32::from(row.item.ordinal))
        .bind(destination.parent_item_instance_id.as_slice()).bind(destination.placement_ordinal.to_string()).bind(now).bind(&envelope).execute(&mut ***tx).await?;
    let world = fence.runtime_scope.world_id();
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid)")
        .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(&row.item.definition.family).bind(&row.item.definition.production_key).bind(&row.item.definition.revision_ref)
        .bind(i64::from(row.item.quantity)).bind(transaction.as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
        .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(destination.parent_item_instance_id.as_slice())
        .bind(destination.placement_ordinal.to_string()).bind(transaction.as_slice()).execute(&mut ***tx).await?;
    let tuple = tx.tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(item.as_slice())
        .bind(now).bind(mint::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut ***tx).await?;
    let updated = sqlx::query("UPDATE game_boss_reward_items SET collected_transaction_id=encode($5,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid AND item_ordinal=$4 AND collected_transaction_id IS NULL AND NOT expired")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(row.death.as_slice()).bind(i32::from(row.item.ordinal)).bind(transaction.as_slice()).execute(&mut ***tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
