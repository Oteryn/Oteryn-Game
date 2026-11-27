use super::TestResult;

use super::fixture::{Harness, debug};

use crate::durability::DurabilityRoot;
use crate::durability::item_decay_retire::CorpseDecayFence;

use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};

use crate::gameplay_transport::{
    ComposedFreshAdmission, source_item_cycle::SOURCE_OWNER_SWEEP_COMPLETE,
};

use std::{sync::Arc, time::Duration};

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    children: i64,
    reconcile: bool,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("the committed kill has a visible corpse")?;
    let generation = owner
        .runtime
        .lock()
        .await
        .owner_fence()
        .map_err(debug)?
        .generation();
    // An unrelated ordinary item on a neighboring tile in the same scope, through MINT.
    let ground = {
        let mut candidate = h
            .root
            .freeze_item_mint(
                owner.character,
                &h.node,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        generation,
                        1_000,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:loot.corpse-decay-control".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:purpose.corpse-decay-control".into(),
                        0,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                corpse
                                    .position
                                    .x
                                    .checked_add(1)
                                    .ok_or("Ground x")?
                                    .try_into()?,
                                corpse.position.y.try_into()?,
                                corpse.position.floor.try_into()?,
                            ),
                        corpse_ref: vec![3; 16],
                        map_revision: "map-1".into(),
                        content_revision: "content-1".into(),
                        native_room_placement_context: vec![6; 16],
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(|error| format!("ordinary Ground MINT freeze: {error:?}"))?;
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        match h
            .root
            .commit_item_mint(&permit, owner.character, &h.node, &mut candidate)
            .await
            .map_err(|error| format!("ordinary Ground MINT commit: {error:?}"))?
        {
            ItemMintOutcome::Committed(item) => item.item_instance_id,
            other => return Err(debug(other)),
        }
    };
    owner.tick_timed_items().await;
    assert_eq!(
        retire_count(h).await?,
        0,
        "a young corpse never retires early"
    );
    assert_eq!(
        owner.runtime.lock().await.visible_corpses(),
        vec![corpse.clone()]
    );

    // Let the real Item deadline pass finish the transforms before testing
    // terminal retirement. Rebase the durable clock without changing durations.
    let item = corpse.item.item_instance_id;
    let lifetime: i64 = sqlx::query_scalar(
        "SELECT game_corpse_retirement_deadline(item_instance_id) - materialized_at
         FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    set_corpse_age(h, item, lifetime - 30_000).await?;
    sweep_until(owner, async || {
        Ok(current_definition(h, item).await? == "oteryn:item.tibia.i3996")
    })
    .await?;
    set_corpse_age(h, item, lifetime).await?;
    // Ordinary Ground has no materialization deadline. Age its creation
    // receipt by the same amount to keep it as the retirement control.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_item_mint_receipts
         SET occurred_at = floor(extract(epoch FROM clock_timestamp())*1000)::bigint - $2
         WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(ground.as_slice())
    .bind(lifetime)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    if reconcile {
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        h.root
            .retire_decayed_corpse(
                &permit,
                owner.character,
                &h.node,
                CorpseDecayFence {
                    // W02 keeps this pinned Channel fixture in its exact tagged scope.
                    runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                        owner.world_id,
                        owner.channel_id,
                    ),
                    scope_ownership_generation: generation,
                },
                corpse.item.item_instance_id,
            )
            .await
            .map_err(debug)?;
        assert_eq!(owner.runtime.lock().await.visible_corpses().len(), 1);
    }
    for _ in 0..MAX_SWEEPS {
        owner_sweep(owner).await?;
        if retire_count(h).await? == children + 1
            && owner.runtime.lock().await.visible_corpses().is_empty()
        {
            break;
        }
    }
    assert_eq!(retire_count(h).await?, children + 1);
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 0, "no loot is spilled to Ground");
    let grounds: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(grounds, 1, "only the ordinary Ground item remains");
    let retired: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE lifecycle = 2 AND quantity = 0",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retired, children + 1);
    let ordinary: (i16, i64) = sqlx::query_as("SELECT i.lifecycle, i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id = encode($1,'hex')::uuid")
        .bind(ground.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        ordinary,
        (1, 1),
        "ordinary Ground survives the same elapsed time"
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .bound_corpse(corpse.identity)
            .is_none()
    );
    // A subsequent owner sweep cannot emit another retirement.
    owner_sweep(owner).await?;
    assert_eq!(retire_count(h).await?, children + 1);
    Ok(())
}

async fn retire_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

pub(super) async fn qualify_first_transform(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    command: crate::gameplay_transport::connection::UseCommand,
    floor_destination: crate::foundation::MovementLocalPosition,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("committed Rat corpse")?;
    let item = corpse.item.item_instance_id;
    let fresh = "oteryn:item.tibia.i5964";
    let target = "oteryn:item.tibia.i3994";
    assert_eq!(corpse.item.production_key, fresh);
    let schedule: (i64, i64, String, String) = sqlx::query_as(
        "SELECT p.duration_millis, p.expires_at_unix_ms - m.materialized_at,
                p.definition_production_key, p.target_production_key
         FROM game_item_mint_receipts m JOIN game_spell_field_temporal_schedules p
           ON p.item_instance_id = m.item_instance_id AND p.source_transaction_id = m.transaction_id
         WHERE m.item_instance_id = encode($1,'hex')::uuid AND m.draw_ordinal = 0
           AND p.source_ordinal = 0 AND p.source_stage = 1",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(schedule, (10_000, 10_000, fresh.into(), target.into()));

    // nb_runtime inherits this role on a node. Apply it to the existing root's
    // single physical holder; the admin pool only moves fixture timestamps
    // and observes committed rows.
    super::journal_inventory::runtime_role(owner.root).await?;
    let grants: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_column_privilege('oteryn_game_runtime', 'game_item_instances', 'definition_family', 'UPDATE'),
                has_column_privilege('oteryn_game_runtime', 'game_item_instances', 'definition_production_key', 'UPDATE'),
                has_column_privilege('oteryn_game_runtime', 'game_item_instances', 'definition_revision_ref', 'UPDATE')",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(grants, (false, true, true));

    // PostgreSQL is the deadline authority. Rebase only this fixture's timestamps
    // and let the actual background loop read and write under its normal guards.
    set_corpse_age(h, item, -30_000).await?;
    owner_sweep(owner).await?;
    assert_eq!(current_definition(h, item).await?, fresh);
    assert_eq!(transform_count(h, item).await?, 0);

    set_corpse_age(h, item, schedule.0).await?;
    sweep_until(owner, async || {
        Ok(current_definition(h, item).await? == target)
    })
    .await?;
    for _ in 0..MAX_SWEEPS {
        owner_sweep(owner).await?;
        if owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .iter()
            .any(|corpse| {
                corpse.item.item_instance_id == item && corpse.item.production_key == target
            })
        {
            break;
        }
    }
    assert!(
        owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .iter()
            .any(|corpse| corpse.item.item_instance_id == item
                && corpse.item.production_key == target),
        "the runtime projects the committed stage"
    );
    assert_eq!(transform_count(h, item).await?, 1);

    // The fixture pool has one holder. Verify it retained the restricted
    // role, so replacing the connection cannot turn this into an admin test.
    let role: String = owner
        .root
        .try_issue_semantic_pass()?
        .run(|holder, _| {
            Box::pin(async move {
                Ok(sqlx::query_scalar("SELECT current_user::text")
                    .fetch_one(&mut **holder)
                    .await?)
            })
        })
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    let proven: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM game_spell_field_temporal_receipts r
            JOIN game_spell_field_temporal_schedules p USING (item_instance_id, source_stage)
            JOIN game_spell_field_temporal_audit a USING (transaction_id, event_id)
            JOIN game_item_instances i USING (item_instance_id)
            JOIN game_item_ground_locations g USING (item_instance_id)
            WHERE r.item_instance_id = encode($1,'hex')::uuid AND r.source_stage = 1
              AND r.occurred_at_unix_ms >= p.expires_at_unix_ms
              AND i.last_transaction_id = r.transaction_id AND i.lifecycle = 1
              AND i.definition_production_key = p.target_production_key
              AND r.source_stack_ordinal = g.stack_ordinal
         )",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        proven,
        "the stage receipt and audit committed with the same Ground item"
    );
    use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
    use crate::gameplay_transport::item_view::{ItemKey, ItemViewError, SessionItemView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
        encode_item_move_intent_for,
    };

    let key = ItemKey::Instance(item);
    let mut view = SessionItemView::resume(Default::default()).with_map_view();
    let source = view
        .map_view(&[key], |table| table.handle(&key).ok_or(ItemViewError::Encode))
        .map_err(debug)?;
    let selection = ItemMoveSelection {
        world_interaction: true,
        ..Default::default()
    };
    let pickup = encode_item_move_intent_for(
        &ItemMoveIntent {
            source,
            destination: ItemMoveDestination::MainBackpack,
        },
        selection,
    )
    .map_err(debug)?;
    let refusal = item_move(owner, actor, command, &pickup, selection, false, &mut view).await;
    assert!(
        matches!(refusal, ItemMoveStep::Result(ItemMoveOutcome::NotSupported, _)),
        "Rat floor-only corpse inventory pickup: {refusal:?}"
    );
    let drag = encode_item_move_intent_for(
        &ItemMoveIntent {
            source,
            destination: ItemMoveDestination::Ground(WorldTilePosition {
                x: floor_destination.x,
                y: floor_destination.y,
                floor: floor_destination.floor,
            }),
        },
        selection,
    )
    .map_err(debug)?;
    let drag_command = crate::gameplay_transport::connection::UseCommand {
        command_id: command.command_id + 1,
        ..command
    };
    let moved = item_move(owner, actor, drag_command, &drag, selection, false, &mut view).await;
    assert!(
        matches!(moved, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
        "Rat floor-only corpse drag: {moved:?}"
    );
    let mut expected_cell = floor_destination.x.to_be_bytes().to_vec();
    expected_cell.extend_from_slice(&floor_destination.y.to_be_bytes());
    expected_cell.extend_from_slice(&(-floor_destination.floor).to_be_bytes());
    let moved_cell: Vec<u8> = sqlx::query_scalar(
        "SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(moved_cell, expected_cell);
    let handoffs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_decay_handoffs WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(handoffs, 1, "floor move retains the absolute decay lineage");
    eprintln!("RAT_CORPSE_ITEM_MOVE_REACHED inventory=NotSupported floor=Moved handoffs={handoffs}");
    eprintln!(
        "RAT_CORPSE_OWNER_TICK_REACHED first_stage_ms={}",
        schedule.0
    );
    Ok(())
}

const MAX_SWEEPS: usize = 8;

async fn sweep_until(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mut observed: impl AsyncFnMut() -> TestResult<bool>,
) -> TestResult {
    for _ in 0..MAX_SWEEPS {
        owner_sweep(owner).await?;
        if observed().await? {
            return Ok(());
        }
    }
    Err("no committed corpse transform after bounded owner sweeps".into())
}

async fn owner_sweep(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    let wake = owner.states.spell_states.lock().await.owner_wake.clone();
    let swept = Arc::new(tokio::sync::Notify::new());
    let stop = oteryn_foundation::CancellationToken::new();
    wake.notify_one();
    tokio::time::timeout(Duration::from_secs(15), async {
        let cycle = DurabilityRoot::with_background_holder(
            SOURCE_OWNER_SWEEP_COMPLETE
                .scope(Arc::clone(&swept), owner.run_source_owner_cycles(&stop)),
        );
        let observe = async {
            swept.notified().await;
            stop.cancel();
        };
        super::fixture::join_two(cycle, observe).await;
    })
    .await
    .map_err(|_| "Channel owner did not complete the Item deadline sweep")?;
    Ok(())
}

async fn set_corpse_age(h: &Harness, item: [u8; 16], age_ms: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let delta: i64 = sqlx::query_scalar(
        "SELECT materialized_at - floor(extract(epoch FROM clock_timestamp())*1000)::bigint + $2
         FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .bind(age_ms)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at = materialized_at - $2 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(delta).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms = expires_at_unix_ms - $2 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(delta).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn current_definition(h: &Harness, item: [u8; 16]) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?)
}

async fn transform_count(h: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_spell_field_temporal_receipts WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?)
}
