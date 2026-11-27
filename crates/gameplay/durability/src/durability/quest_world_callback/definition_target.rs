//! Ground custody stays stable through the tool debit and quest receipt.
use super::*;

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let target = context
        .selected_tool
        .as_ref()
        .and_then(|tool| tool.target.as_ref());
    let Some(definition) = &callback.selector.target_definition else {
        return Ok(target.is_none());
    };
    let Some(target) = target else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    permit.check_channel(world_id, channel_id)?;
    let live: Option<bool> = sqlx::query_scalar(
        "SELECT true FROM game_item_instances \
         WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
         AND lifecycle=1 AND definition_family='Item' AND definition_production_key=$3 FOR SHARE",
    )
    .bind(target.item.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(definition)
    .fetch_optional(&mut **tx)
    .await?;
    if live.is_none() {
        return Ok(false);
    }
    // Item custody writers take the Item lock before the Ground owner lock.
    // game_item_ground_owner_lock serializes custody changes. Read Ground
    // after taking the same lock, using only the runtime role’s SELECT grant.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    let position: Option<(Vec<u8>, bool)> = sqlx::query_as(
        "SELECT spatial_position,map_revision<>$5 FROM game_item_ground_locations \
         WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
         AND scope_kind=1 AND channel_id=encode($3,'hex')::uuid \
         AND runtime_scope_ownership_generation=$4::text::numeric \
         AND (map_revision=$5 OR map_revision='sha256:'||encode(sha256(convert_to($5,'UTF8')),'hex')) \
         AND content_revision=$6 AND native_room_placement_context=$7 \
         AND ($8<>'oteryn:item.tibia.i7535' OR game_quest_fresh_corpse(item_instance_id,$8))",
    )
    .bind(target.item.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(&context.map_revision)
    .bind(&context.content_revision)
    .bind(&context.placement_context)
    .bind(definition)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((position, native_axes)) = position else {
        return Ok(false);
    };
    let cell = super::super::spell_items_abi::decode_ground_cell(&position)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let [x, y, floor] = target.actor_position;
    let target_floor = if native_axes { cell.z } else { -cell.z };
    Ok(target.target_position == [cell.x, cell.y, target_floor]
        && x.abs_diff(cell.x) <= u32::from(callback.use_range)
        && y.abs_diff(cell.y) <= u32::from(callback.use_range)
        && floor == target_floor)
}

impl crate::durability::DurabilityRoot {
    /// A handle comes from the wire view. Resolve only its current Ground row;
    /// the committing owner independently locks and proves its custody.
    pub async fn read_world_callback_ground_target(
        &self,
        scope: crate::foundation::RuntimeScopeRefV1,
        item: [u8; 16],
    ) -> Result<Option<(String, String, Vec<u8>)>, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = scope
        else {
            return Ok(None);
        };
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            let target = sqlx::query_as(
                "SELECT i.definition_production_key, g.map_revision, g.spatial_position
                 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id)
                 WHERE g.item_instance_id=encode($1,'hex')::uuid
                   AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid
                   AND g.scope_kind=1 AND i.lifecycle=1 AND i.definition_family='Item'",
            )
            .bind(item.as_slice())
            .bind(world_id.as_bytes().as_slice())
            .bind(channel_id.as_bytes().as_slice())
            .fetch_optional(&mut *tx).await?;
            super::super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(target)
        })).await
    }
}

impl crate::durability::DurabilityRoot {
    /// Direct encounter USE borrows the same fresh-mint predicate while the
    /// caller holds the Channel item lane against decay and custody writers.
    pub async fn koshei_corpse_remaining(
        &self,
        permit: &SpellLanePermit,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        item: [u8; 16],
    ) -> Result<Option<std::time::Duration>, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(None);
        };
        permit.check_channel(world_id, channel_id)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    let millis: Option<i64> = sqlx::query_scalar(
                        "SELECT s.expires_at_unix_ms - ceil(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id) JOIN game_spell_field_temporal_schedules s ON s.item_instance_id=i.item_instance_id AND s.source_transaction_id=i.minted_transaction_id AND s.source_stage=1 AND s.source_ordinal=0
                 WHERE g.item_instance_id=encode($1,'hex')::uuid
                   AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid
                   AND g.scope_kind=1 AND g.runtime_scope_ownership_generation=$4::text::numeric
                   AND game_quest_fresh_corpse(g.item_instance_id,'oteryn:item.tibia.i7535')",
                    )
                    .bind(item.as_slice())
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .bind(fence.scope_ownership_generation.get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(millis.and_then(|ms| u64::try_from(ms).ok()).map(std::time::Duration::from_millis))
                })
            })
            .await
    }
}
