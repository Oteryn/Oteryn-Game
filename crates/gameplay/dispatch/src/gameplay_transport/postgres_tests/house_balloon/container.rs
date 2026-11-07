//! W18: real named-House cupboards use the existing container observer and item loop.
use super::*;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;

pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    let actor = admitted.runtime_actor.ok_or("House container actor")?;
    let fence = admitted.item_fence.ok_or("House container fence")?;
    let cupboard = ItemKey::Instance(id(71));
    let bag = ItemKey::Instance(id(72));
    let gold = ItemKey::Instance(id(73));
    let observe = |target| owner.observe_container(actor, Some(fence), target);
    let Some(ContainerObservation::Container {
        capacity, entries, ..
    }) = observe(cupboard).await
    else {
        return Err(
            "current empty House cupboard must open through the real container observer".into(),
        );
    };
    assert_eq!(capacity, 8);
    assert!(entries.is_empty());
    let lineage: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let move_item = |command_id, item, target| {
        owner.move_loop_item(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            LoopMove {
                item,
                count: None,
                target,
            },
        )
    };
    assert_eq!(
        move_item(100, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(cupboard).await else {
        return Err("full cupboard remains observable".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, bag);
    assert_eq!(
        move_item(101, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container {
        parent, entries, ..
    }) = observe(bag).await
    else {
        return Err("nested House bag must open from its real House graph".into());
    };
    assert_eq!(parent, Some(cupboard));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    assert_eq!(entries[0].count.get(), 10);
    let subjects: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(reclaim_subject_character_id) FROM game_house_nested_reclaim_provenance ORDER BY item_instance_id")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        subjects,
        vec![id(41).to_vec(), id(41).to_vec()],
        "cupboard owner cannot acquire the guest's inserted bag or gold"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        move_item(101, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    let replay_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts, replay_receipts,
        "same command replays without a second move"
    );
    assert_eq!(
        move_item(102, gold, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(103, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(cupboard).await else {
        return Err("emptied cupboard remains observable".into());
    };
    assert!(entries.is_empty());
    let after_lineage: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        lineage, after_lineage,
        "container moves never remint their existing contents"
    );
    let remaining_provenance: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_nested_reclaim_provenance")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        remaining_provenance, 0,
        "taking contents out ends their House reclaim provenance"
    );
    // A populated bag moves as its existing subtree, without new item identities.
    assert_eq!(
        move_item(104, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(105, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(106, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(bag).await else {
        return Err("populated bag returns to carried custody".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    let mut stale = fence;
    stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
        fence.scope_ownership_generation.get() + 1,
    )
    .map_err(debug)?;
    assert!(
        !matches!(
            owner.observe_container(actor, Some(stale), cupboard).await,
            Some(ContainerObservation::Container { .. })
        ),
        "stale House generation cannot observe contents"
    );
    assert_ne!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 107,
                    item_fence: Some(stale)
                },
                LoopMove {
                    item: gold,
                    count: None,
                    target: LoopTarget::Container(cupboard)
                }
            )
            .await,
        LoopMoveResult::Committed,
        "stale House generation cannot insert contents"
    );
    // Leave through the native transfer owner before the existing disposition
    // runner drains a historical overdue notice (same fixture seam as HOUSE-1b).
    assert_eq!(
        move_item(108, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    let mut exit = vec![8, 2, 18, u8::try_from(HOUSE.len())?];
    exit.extend_from_slice(HOUSE.as_bytes());
    let exit_context = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        (
            owner.movement_cells.scope().generation_digest
                == runtime.content_pin().server_artifact_digest(),
            (state.position.x, state.position.y, state.position.floor),
            (
                state.origin_position.x,
                state.origin_position.y,
                state.origin_position.floor,
            ),
        )
    };
    match owner.transfer_house(admitted, 109, &exit).await {
        Some((Some(_outside), _)) => {}
        outcome => {
            let kind = if outcome.is_none() {
                "uncertain"
            } else {
                "refused"
            };
            return Err(format!("actual House exit {kind}: same_content_generation={}, actor_position={:?}, origin_position={:?}, entrance={:?}",
                exit_context.0,exit_context.1,exit_context.2,
                crate::gameplay_transport::house_query::entrance(HOUSE).map(|p|(p.x,p.y,p.floor))).into());
        }
    }
    sqlx::query("INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at) SELECT world_id,house_key,owner_character_id,ownership_revision,1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400001,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1 FROM game_house_ownership WHERE house_key=$1")
        .bind(HOUSE).execute(&h.pool).await?;
    for _ in 0..3 {
        owner
            .drain_source_house_dispositions()
            .await
            .map_err(debug)?;
    }
    let delivered: Vec<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT uuid_send(item_instance_id),uuid_send(character_id) FROM game_item_character_inbox_locations WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) ORDER BY item_instance_id")
        .bind(id(71).as_slice()).bind(id(72).as_slice()).bind(id(73).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        delivered,
        vec![
            (id(71).to_vec(), id(101).to_vec()),
            (id(72).to_vec(), id(41).to_vec()),
            (id(73).to_vec(), id(41).to_vec())
        ],
        "normal disposition returns each inserted descendant to its own retained subject"
    );
    let nested_remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(id(72).as_slice()).bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        nested_remaining, 0,
        "disposition does not leave Inbox items in nested custody"
    );
    eprintln!(
        "W18_PG house_container_open_insert_nested_remove_replay_stale elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
