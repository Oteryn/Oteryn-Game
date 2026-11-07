//! Keep each unbought duplicate through the real item-loop owner, making room for later payouts.
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
    let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2 ORDER BY e.placement_ordinal LIMIT 1")
        .bind(id(41).as_slice()).bind(definition).fetch_one(&h.pool).await?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let target = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: -at.floor,
    };
    let request = LoopMove {
        item: ItemKey::Instance(item.into_bytes()),
        count: Some(1),
        target: LoopTarget::Ground(target),
    };
    let result = owner.move_loop_item(actor, command, request).await;
    assert_eq!(
        result,
        LoopMoveResult::Committed,
        "retain duplicate: {definition}"
    );
    assert_eq!(exact_shops::quantity(h, definition).await?, 0);
    let preserved: i64 = sqlx::query_scalar("SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=$1 AND lifecycle=1")
        .bind(item).fetch_one(&h.pool).await?;
    assert_eq!(preserved, 1, "the unsold duplicate remains live");
    Ok(())
}
