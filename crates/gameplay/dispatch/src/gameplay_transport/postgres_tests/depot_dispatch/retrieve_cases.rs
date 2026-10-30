//! DEPOT SEARCH E08: retrieve-all moves every depot item of the searched definition into the
//! main backpack through the item-loop batch owner, keeps what does not fit, and replays.
use super::*;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use oteryn_protocol_oteryn::depot_search::{Outcome, Reply, Request};
use oteryn_protocol_oteryn::item_view::ItemHandle;

/// Carried main-backpack entries, depot roots, and batch child receipts of every command.
async fn state(pool: &sqlx::PgPool) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1),\
                (SELECT count(*) FROM game_item_character_depot_locations),\
                (SELECT count(*) FROM game_item_loop_move_receipts WHERE quick_loot_ordinal>0)",
    )
    .fetch_one(pool)
    .await?)
}

fn reply(outcome: Outcome, retrieved: u32) -> Reply {
    Reply {
        outcome,
        retrieved,
        results: Vec::new(),
    }
}

/// Three carried stacks go to depot box 17 through the depot owner.
pub(super) async fn deposit(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    stacks: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    for (command_id, (handle, item)) in (1..).zip(stacks.iter().take(3)) {
        let deposit = DepotIntent {
            town: Some(0),
            box_number: 17,
            page: 0,
            action: DepotAction::Deposit(*handle),
        };
        let moved = owner
            .depot(actor, command(command_id), deposit, Some(ItemKey::Instance(*item)))
            .await
            .ok_or("deposit")?;
        assert_eq!(moved.reply.outcome, DepotOutcome::Moved);
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    stacks: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let retrieve = |command_id, all| {
        owner.depot_search_command(
            actor,
            admitted,
            command_id,
            Request::Retrieve {
                client_id: 3031,
                all,
            },
        )
    };
    assert_eq!(state(pool).await?, (19, 3, 0));
    // Retrieve-all moves what fits; the other two depot stacks stay.
    assert_eq!(retrieve(7, true).await, reply(Outcome::Retrieved, 100));
    assert_eq!(state(pool).await?, (20, 2, 1));
    // A replay answers the stored outcome and changes nothing.
    assert_eq!(retrieve(7, true).await, reply(Outcome::Retrieved, 100));
    assert_eq!(state(pool).await?, (20, 2, 1));
    // With the backpack full a new retrieval moves nothing and reports no room.
    assert_eq!(retrieve(8, true).await, reply(Outcome::NoRoom, 0));
    assert_eq!(state(pool).await?, (20, 2, 1));
    // Two carried stacks dropped to the Ground make room for both depot stacks.
    for (command_id, (_, item)) in (9..).zip(stacks.iter().skip(3).take(2)) {
        assert_eq!(
            owner
                .move_loop_item(
                    actor,
                    command(command_id),
                    LoopMove {
                        item: ItemKey::Instance(*item),
                        count: None,
                        target: LoopTarget::Ground(
                            oteryn_protocol_oteryn::item_view::WorldTilePosition {
                                x: 7,
                                y: 0,
                                floor: -7,
                            },
                        ),
                    },
                )
                .await,
            LoopMoveResult::Committed
        );
    }
    assert_eq!(retrieve(11, true).await, reply(Outcome::Retrieved, 200));
    assert_eq!(state(pool).await?, (20, 0, 3));
    let quantities: Vec<i64> = sqlx::query_scalar(
        "SELECT i.quantity::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3031'",
    )
    .fetch_all(pool)
    .await?;
    assert_eq!((quantities.len(), quantities.iter().sum::<i64>()), (20, 2000));
    assert_eq!(retrieve(12, true).await, reply(Outcome::NotFound, 0));
    let receipts: Vec<(bool, i64)> = sqlx::query_as(
        "SELECT found,retrieved FROM game_depot_search_retrieve_receipts ORDER BY command_id",
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(
        receipts,
        [(true, 100), (true, 0), (true, 200), (false, 0)]
    );
    Ok(())
}
