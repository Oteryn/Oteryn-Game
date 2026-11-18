//! Crystal 201189f8 data-global/scripts/quests/the_inquisition_quest/actions_vampire_hunt.lua:10-26.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::Offering;

pub(super) async fn all_present(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldToolContext,
    altars: &[Offering],
) -> Result<bool, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::CommitRejected);
    };
    for altar in altars {
        let Position { x, y, floor } = altar.position;
        let mut spatial = i32::from(x).to_be_bytes().to_vec();
        spatial.extend_from_slice(&i32::from(y).to_be_bytes());
        spatial.extend_from_slice(&i16::from(floor).to_be_bytes());
        let present: bool = sqlx::query_scalar(
            "SELECT EXISTS(
               SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
               WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid
                 AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5
                 AND g.native_room_placement_context=$6 AND g.runtime_scope_ownership_generation=$7::text::numeric
                 AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$8
                 AND i.quantity >= $9 AND game_item_loop_drop_ref(g.corpse_ref))",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(spatial)
        .bind(&context.map_revision)
        .bind(&context.content_revision)
        .bind(&context.placement_context)
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(&altar.item)
        .bind(i64::from(altar.count))
        .fetch_one(&mut **tx)
        .await?;
        if !present {
            return Ok(false);
        }
    }
    Ok(true)
}
