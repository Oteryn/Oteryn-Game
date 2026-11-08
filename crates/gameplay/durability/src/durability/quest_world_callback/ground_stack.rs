//! M02: read-only portable-item prerequisites share the existing Ground custody lock.
//! Nomads Land authored recipe s4 derives from TibiaWiki spoiler page 22842,
//! revision 1002600; arrangement is a prerequisite, not an offering to consume.
//! See docs/reference/content-prep/SRVWLD-M02.md: TibiaWiki BR Arito's Task,
//! lines115-117 specify ordered crate/amphora piles. Crystal 201189f
//! data/items/items.xml:5020-5024 and6245-6247 identify the two source Items.
use super::{DurabilityError, QuestWorldContext, SpellLanePermit};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement;
use sqlx::{Postgres, Row, Transaction};

pub(crate) async fn matches(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
    requirements: &[WorldRequirement],
) -> Result<bool, DurabilityError> {
    if requirements.iter().all(|r| r.ground_stack.is_empty()) {
        return Ok(true);
    }
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    // Lock Item roots in global identity order BEFORE key 33, like the
    // existing text/move owners. Acquiring Item locks after the Ground owner
    // could deadlock with a move which already retained its source Item.
    permit.check_channel(world_id, channel_id)?;
    let mut required = requirements
        .iter()
        .filter(|r| !r.ground_stack.is_empty())
        .collect::<Vec<_>>();
    required.sort_by_key(|r| r.position);
    let positions = required
        .iter()
        .map(|r| spatial(r.position))
        .collect::<Vec<_>>();
    let protected: Vec<String> = sqlx::query_scalar(
        "SELECT i.item_instance_id::text FROM game_item_ground_locations g \
         JOIN game_item_instances i USING(item_instance_id,world_id) \
         WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
         AND g.instance_id IS NULL AND g.spatial_position=ANY($3::bytea[]) \
         AND g.map_revision=$4 AND g.content_revision=$5 \
         AND g.native_room_placement_context=$6 \
         AND g.runtime_scope_ownership_generation=$7::text::numeric \
         AND i.lifecycle=1 AND i.definition_family='Item' \
         ORDER BY i.item_instance_id LIMIT 65 FOR SHARE OF i",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(positions)
    .bind(&context.map_revision)
    .bind(&context.content_revision)
    .bind(&context.placement_context)
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_all(&mut **tx)
    .await?;
    if protected.len() > 64 {
        return Ok(false);
    }
    // Lane-derived exact scope proof precedes key 33, the existing Ground
    // owner lock used by every location mutation. It prevents new phantoms.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    for requirement in required {
        let spatial = spatial(requirement.position);
        // Separate statement after the owner lock: observe committed custody,
        // not a pre-wait snapshot. Item share locks retain quantities until the
        // enclosing Quest receipt commits. No Ground row or Item is mutated.
        let rows = sqlx::query(
            "SELECT i.item_instance_id::text AS item,i.definition_production_key,i.quantity,g.stack_ordinal \
             FROM game_item_ground_locations g JOIN game_item_instances i \
             USING(item_instance_id,world_id) \
             WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
             AND g.instance_id IS NULL AND g.spatial_position=$3 \
             AND g.map_revision=$4 AND g.content_revision=$5 \
             AND g.native_room_placement_context=$6 \
             AND g.runtime_scope_ownership_generation=$7::text::numeric \
             AND i.lifecycle=1 AND i.definition_family='Item' \
             ORDER BY g.stack_ordinal,i.item_instance_id LIMIT 9",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(spatial)
        .bind(&context.map_revision)
        .bind(&context.content_revision)
        .bind(&context.placement_context)
        .bind(fence.scope_ownership_generation.get().to_string())
        .fetch_all(&mut **tx)
        .await?;
        if rows.len() != requirement.ground_stack.len() {
            return Ok(false);
        }
        let mut previous = 0;
        for (row, expected) in rows.iter().zip(&requirement.ground_stack) {
            // A root arriving between the preliminary read and the Ground lock
            // was not quantity-locked. Refuse instead of locking out of order.
            if !protected
                .iter()
                .any(|id| row.try_get::<&str, _>("item").ok() == Some(id.as_str()))
            {
                return Ok(false);
            }
            let ordinal: Option<i64> = row.try_get("stack_ordinal")?;
            let Some(ordinal) = ordinal.filter(|n| *n > previous) else {
                return Ok(false);
            };
            if row.try_get::<&str, _>("definition_production_key")? != expected.item
                || row.try_get::<i64, _>("quantity")? != i64::from(expected.count)
            {
                return Ok(false);
            }
            previous = ordinal;
        }
    }
    Ok(true)
}

fn spatial(position: oteryn_gameplay_quests::quest::world_callbacks::Position) -> Vec<u8> {
    [
        i32::from(position.x).to_be_bytes().as_slice(),
        i32::from(position.y).to_be_bytes().as_slice(),
        i16::from(position.floor).to_be_bytes().as_slice(),
    ]
    .concat()
}

#[cfg(test)]
mod tests;
