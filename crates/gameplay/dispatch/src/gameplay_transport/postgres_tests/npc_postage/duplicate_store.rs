//! Preserve unsold duplicates in an ordinary carried backpack through the real item-loop owner.
use super::*;
pub(super) async fn apply(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    definition: &str,
) -> TestResult {
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$3 ORDER BY e.placement_ordinal LIMIT 1")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(definition).fetch_one(&h.pool).await?;
    let bag: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2854'")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    let request = LoopMove {
        item: ItemKey::Instance(item.into_bytes()),
        count: Some(1),
        target: LoopTarget::Container(ItemKey::Instance(bag.into_bytes())),
    };
    assert_eq!(owner.move_loop_item(actor, command, request).await,
        LoopMoveResult::Committed, "retain duplicate: {definition}");
    assert_eq!(exact_shops::quantity(h, definition).await?, 1);
    let preserved: i64 = sqlx::query_scalar("SELECT i.quantity::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE i.item_instance_id=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=$3")
        .bind(item).bind(id(41).as_slice()).bind(bag).fetch_one(&h.pool).await?;
    assert_eq!(preserved, 1, "the unsold duplicate stays live in its carried bag");
    Ok(())
}
