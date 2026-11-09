//! Eligibility reasons follow the donor's carried-item then Premium branch order.
use super::*;
use oteryn_gameplay_quests::quest::QuestRefusal;

pub(in crate::durability) async fn eligibility(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
    premium: bool,
) -> Result<Option<QuestRefusal>, DurabilityError> {
    if !super::context_is_current(tx, permit, fence, context).await? {
        return Ok(Some(QuestRefusal::StageMismatch));
    }
    let world = fence.runtime_scope.world_id();
    let complete: bool = sqlx::query_scalar(
        "SELECT game_carried_step_census_complete(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !complete {
        return Ok(Some(QuestRefusal::StageMismatch));
    }
    let present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3595' AND i.definition_revision_ref='definition-r1' AND i.quantity>=1)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .fetch_one(&mut **tx).await?;
    Ok(if !present {
        Some(QuestRefusal::SourceOfferingMissing)
    } else if !premium {
        Some(QuestRefusal::SourceOfferingPremiumRequired)
    } else {
        None
    })
}
