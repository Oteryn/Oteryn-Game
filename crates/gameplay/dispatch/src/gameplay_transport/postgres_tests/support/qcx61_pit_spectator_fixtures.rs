use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_move::{LoopMove, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
) -> TestResult {
    let item: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances
         WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3043'",
    )
    .fetch_one(&h.pool)
    .await?;
    let (x, y) = crate::durability::arena_match::donor::CENTERS[0];
    let command = UseCommand {
        item_fence: admitted.item_fence,
        game_session_id: admitted.game_session_id,
        command_id: 20320,
    };
    let request = LoopMove {
        item: ItemKey::Instance(item.try_into().map_err(|_| "item UUID")?),
        count: Some(1),
        target: LoopTarget::Ground(WorldTilePosition { x, y, floor: -7 }),
    };
    place(
        owner,
        admitted,
        actor,
        MovementLocalPosition { x, y, floor: 6 },
    )
    .await?;
    assert_eq!(
        owner.move_loop_item(actor, command, request).await,
        LoopMoveResult::TooFar
    );
    assert_eq!(move_count(h).await?, 0);

    place(
        owner,
        admitted,
        actor,
        MovementLocalPosition { x, y, floor: 7 },
    )
    .await?;
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    command_id: 20321,
                    ..command
                },
                request
            )
            .await,
        LoopMoveResult::Committed
    );
    assert_eq!(move_count(h).await?, 1);
    Ok(())
}

async fn place(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    actor: ExactActorRef,
    destination: MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let origin = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .relocate_arena_player(actor, admitted.game_session_id, origin, destination)
        .map_err(debug)?;
    Ok(())
}

async fn move_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
