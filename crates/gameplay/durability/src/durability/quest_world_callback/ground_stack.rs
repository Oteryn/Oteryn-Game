//! A bounded complete portable Ground read, protected through the quest commit.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::{GroundStackFact, ground_stack_matches};
const MAX_STACK_ITEMS: usize = 64;

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldContext,
) -> Result<bool, DurabilityError> {
    let mut requirements = callback
        .world_requirements
        .iter()
        .filter(|r| r.ground_stack.is_some())
        .collect::<Vec<_>>();
    if requirements.is_empty() {
        return Ok(true);
    }
    if !callback.supports_ground_stack_guard()
        || requirements.len() > oteryn_gameplay_quests::quest::world_callbacks::MAX_EFFECTS
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    // Existing Ground mutations take this exact namespace lock. A try-lock and
    // NOWAIT Item locks preserve the Item-before-Ground owner lock order: busy
    // custody refuses instead of waiting on a pickup holding an Item row.
    let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if !available {
        return Ok(false);
    }
    requirements.sort_by_key(|requirement| requirement.position);
    for requirement in requirements {
        if !(-15..=0).contains(&requirement.position.floor) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let Position { x, y, floor } = requirement.position;
        let mut spatial = i32::from(x).to_be_bytes().to_vec();
        spatial.extend_from_slice(&i32::from(y).to_be_bytes());
        spatial.extend_from_slice(&i16::from(floor).to_be_bytes());
        // Ordinary portable Ground uses the ten-byte native map address. Native
        // corpse/spell-field projections have their own ABI and are not portable
        // operands. Do not filter quantity/definition/order here: extra or
        // unknown entries must fail the complete-stack predicate, not disappear.
        let rows = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,g.stack_ordinal,g.map_revision,g.content_revision,g.native_room_placement_context,g.runtime_scope_ownership_generation::text AS generation FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.scope_kind=1 AND g.channel_id=encode($2,'hex')::uuid AND g.instance_id IS NULL AND g.spatial_position=$3 AND i.lifecycle=1 ORDER BY g.stack_ordinal,i.item_instance_id LIMIT $4 FOR UPDATE OF i,g NOWAIT")
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(spatial)
            .bind(i64::try_from(MAX_STACK_ITEMS + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .fetch_all(&mut **tx).await?;
        if rows.len() > MAX_STACK_ITEMS {
            return Ok(false);
        }
        let mut entries: Vec<([u8; 16], String, u32)> = Vec::with_capacity(rows.len());
        let mut previous_ordinal = 0;
        for row in rows {
            if row.try_get::<String, _>("definition_family")? != "Item"
                || row.try_get::<String, _>("map_revision")? != context.map_revision
                || row.try_get::<String, _>("content_revision")? != context.content_revision
                || row.try_get::<Vec<u8>, _>("native_room_placement_context")?
                    != context.placement_context
                || row.try_get::<String, _>("generation")?
                    != fence.scope_ownership_generation.get().to_string()
            {
                return Ok(false);
            }
            let ordinal = row
                .try_get::<Option<i64>, _>("stack_ordinal")?
                .and_then(|n| u64::try_from(n).ok());
            let Some(ordinal) = ordinal else {
                return Ok(false);
            };
            if ordinal <= previous_ordinal {
                return Ok(false);
            }
            previous_ordinal = ordinal;
            entries.push((
                row.try_get::<Vec<u8>, _>("item")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                row.try_get("definition_production_key")?,
                u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ));
        }
        let Some(expected) = requirement.ground_stack.as_ref() else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if !ground_stack_matches(
            expected,
            entries.iter().map(|(instance, item, count)| {
                Some(GroundStackFact {
                    instance: *instance,
                    item,
                    count: *count,
                })
            }),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}
