use sha2::Digest;
use super::TestResult;

use super::fixture::{Harness, debug, id};

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
    floor_destination: Option<crate::foundation::MovementLocalPosition>,
    successor_drag: bool,
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
    eprintln!("RAT_CORPSE_OWNER_TICK_REACHED first_stage_ms={}", schedule.0);
    let Some(floor_destination) = floor_destination else {
        return Ok(());
    };
    let protection = async || -> TestResult<(Option<String>, i64)> {
        Ok(sqlx::query_as(
            "SELECT corpse_top_damage_character_id::text, materialized_at
             FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(item.as_slice()).fetch_one(&h.pool).await?)
    };
    let before_protection = protection().await?;
    let contents = owner.root.read_corpse_contents(owner.character, item).await?.ok_or("Rat contents")?;
    qualify_rat_ground_move(owner, h, actor, command, item, floor_destination).await?;
    assert_eq!(
        owner.root.read_corpse_contents(owner.character, item).await?,
        Some(contents.clone()),
    );
    assert_eq!(protection().await?, before_protection);
    let projected = owner.runtime.lock().await.bound_corpse(corpse.identity).ok_or("bound Rat corpse")?;
    assert_eq!(projected.position, floor_destination);
    assert_ne!(projected.position, corpse.position);
    assert!(projected.revision > corpse.revision);
    let map = owner.bundle_map().ok_or("Rat map")?;
    let index = {
        let runtime = owner.runtime.lock().await;
        crate::gameplay_transport::item_ref_admission::pinned_index(&runtime, owner.active_generation)
    };
    let overlay = crate::gameplay_transport::loop_move::loop_ground(owner, map.overlay.base(), index)
        .await.ok_or("Rat Ground overlay")?;
    let tile = |position: crate::foundation::MovementLocalPosition| -> TestResult<crate::map::overlay::TilePos> {
        Ok(crate::map::overlay::TilePos {
            x: position.x.try_into()?,
            y: position.y.try_into()?,
            floor: (-position.floor).try_into()?,
        })
    };
    let (at, _) = overlay.ground_entry(&item).ok_or("indexed Rat corpse")?;
    assert_eq!(at, tile(floor_destination)?);
    assert!(!overlay.overlay().tile(tile(corpse.position)?).is_some_and(|old| {
        old.added().iter().any(|entry| {
            matches!(entry.item(), crate::map::overlay::AddedItem::Ground(item_row) if item_row.item_instance_id == item)
        })
    }));


    let session = command.game_session_id;
    let (decay_actor, decay_fence, terminal_destination) = if successor_drag {
        let (successor, fence) = restart_rat_session(owner, h, session).await?;
        super::journal_inventory::runtime_role(owner.root).await?;
        use crate::durability::item_loop_move::{QuickLootCommand, QuickLootInput};
        use crate::foundation::{CommandId, CommandRef};
        use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
        use oteryn_protocol_oteryn::quick_loot::{QuickLootAction, QuickLootRequest, decode_quick_loot_result};
        {
            let permit = owner.spell_lane_permit().await.ok_or("successor item lane")?;
            let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
                .ok_or("Rat fixture native content")?;
            let cell: Vec<u8> = sqlx::query_scalar(
                "SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(item.as_slice()).fetch_one(&h.pool).await?;
            let result = owner.root.commit_quick_loot(
                &permit, owner.character, owner.holder, fence, &NativeItemWeights(native),
                QuickLootCommand {
                    command: CommandRef::new(session, CommandId::new(command.command_id + 2).map_err(debug)?),
                    cell: cell.try_into().map_err(|_| "Rat loot cell")?,
                    premium: true,
                    input: QuickLootInput::Legacy(QuickLootRequest {
                        action: QuickLootAction::Click, corpse: Some(item), category: None,
                        recipient: None, recipient_id: None, fallback_to_main: false,
                        filter_mode: None, filter_definition_ref: None,
                    }),
                    selected_corpse: Some(item),
                },
            ).await?.map_err(debug)?;
            let result = decode_quick_loot_result(&result).map_err(debug)?;
            assert!(result.items.iter().any(|entry| entry.accepted), "successor must loot the moved Rat: {result:?}");
        }
        qualify_rat_ground_move(owner, h, successor, crate::gameplay_transport::connection::UseCommand {
            command_id: command.command_id + 3, item_fence: Some(fence), ..command
        }, item, corpse.position).await?;
        let move_generation: String = sqlx::query_scalar(
            "SELECT runtime_scope_ownership_generation::text FROM game_item_ground_locations
             WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(move_generation, fence.scope_ownership_generation.get().to_string());
        let (successor, fence) = restart_rat_session(owner, h, session).await?;
        super::journal_inventory::runtime_role(owner.root).await?;
        (successor, Some(fence), floor_destination)
    } else {
        (actor, command.item_fence, corpse.position)
    };
    let remaining = owner.root.read_corpse_contents(owner.character, item).await?
        .ok_or("remaining Rat contents")?.len();
    let before_retirement: i64 = sqlx::query_scalar(
        "SELECT game_corpse_retirement_deadline(encode($1,'hex')::uuid) - materialized_at
           FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    let terminal_age: i64 = sqlx::query_scalar(
        "SELECT max(p.expires_at_unix_ms - m.materialized_at)
         FROM game_spell_field_temporal_schedules p JOIN game_item_mint_receipts m USING(item_instance_id)
         WHERE p.item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    set_corpse_age(h, item, terminal_age).await?;
    sweep_until(owner, async || Ok(current_definition(h, item).await? == "oteryn:item.tibia.i3996")).await?;
    if successor_drag {
        let generations: Vec<String> = sqlx::query_scalar(
            "SELECT ownership_generation::text FROM game_spell_field_temporal_receipts
             WHERE item_instance_id=encode($1,'hex')::uuid AND source_stage IN(2,3)
             ORDER BY source_stage",
        ).bind(item.as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(generations, vec![decay_actor.scope_generation().get().to_string(); 2]);
    }
    let later_command = crate::gameplay_transport::connection::UseCommand {
        command_id: command.command_id + 5, item_fence: decay_fence, ..command
    };
    qualify_rat_ground_move(owner, h, decay_actor, later_command, item, terminal_destination).await?;
    let (later_actor, later_command) = if successor_drag {
        (decay_actor, later_command)
    } else {
        let (actor, fence) = restart_rat_session(owner, h, session).await?;
        super::journal_inventory::runtime_role(owner.root).await?;
        (actor, crate::gameplay_transport::connection::UseCommand { item_fence: Some(fence), ..later_command })
    };
    let map = owner.bundle_map().ok_or("Rat map")?;
    let index = {
        let runtime = owner.runtime.lock().await;
        crate::gameplay_transport::item_ref_admission::pinned_index(&runtime, owner.active_generation)
    };
    let base = map.overlay.base();
    let overlay = crate::gameplay_transport::loop_move::loop_ground(owner, base, index)
        .await.ok_or("Rat Ground overlay before retirement")?;
    assert!(overlay.ground_entry(&item).is_some());
    set_corpse_age(h, item, before_retirement).await?;
    let expected_retirements = i64::try_from(remaining)? + 1;
    sweep_until(owner, async || Ok(retire_count(h).await? == expected_retirements)).await?;
    owner_sweep(owner).await?;
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;
    let map = owner.bundle_map().ok_or("Rat map")?;
    let base = map.overlay.base();
    let index = {
        let runtime = owner.runtime.lock().await;
        assert!(owner.states.loop_ground.current_for(&runtime, base)
            .is_none_or(|cached| cached.ground_entry(&item).is_none()), "retired Rat remains cached");
        crate::gameplay_transport::item_ref_admission::pinned_index(&runtime, owner.active_generation)
    };
    let overlay = crate::gameplay_transport::loop_move::loop_ground(owner, base, index)
        .await.ok_or("Rat Ground overlay after retirement")?;
    assert!(overlay.ground_entry(&item).is_none(), "retired Rat remains visible");
    let outcome = crate::gameplay_transport::loop_move::move_loop_item(
        owner, later_actor, crate::gameplay_transport::connection::UseCommand {
            command_id: later_command.command_id + 2, ..later_command
        },
        LoopMove { item: ItemKey::Instance(item), count: None, slot: None,
            target: LoopTarget::Ground(WorldTilePosition {
                x: corpse.position.x, y: corpse.position.y, floor: -corpse.position.floor,
            }),
        },
    ).await;
    assert_eq!(outcome, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
    assert_eq!(transform_count(h, item).await?, 3);
    assert_eq!(retire_count(h).await?, expected_retirements);
    let handoffs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_decay_handoffs WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(handoffs, if successor_drag { 3 } else { 2 }, "each floor move preserves its stage lineage once");
    eprintln!("RAT_CORPSE_ITEM_MOVE_REACHED inventory=NotPickupable floor=Moved handoffs={handoffs}");
    Ok(())
}

async fn qualify_rat_ground_move(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    command: crate::gameplay_transport::connection::UseCommand,
    item: [u8; 16],
    floor_destination: crate::foundation::MovementLocalPosition,
) -> TestResult {
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
        matches!(refusal, ItemMoveStep::Refused(
            ItemMoveOutcome::NotPickupable,
            oteryn_protocol_oteryn::refusal_detail::RefusalDetail::CannotPickup,
        )),
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
    expected_cell.extend_from_slice(&floor_destination.floor.to_be_bytes());
    let moved_cell: Vec<u8> = sqlx::query_scalar(
        "SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(moved_cell, expected_cell);
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
pub(super) async fn restart_channel_owner(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
) -> TestResult<crate::foundation::ExactActorRef> {
    replace_channel_owner(owner, harness, crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?).await
}

pub(super) async fn restart_rat_session(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    session: crate::foundation::GameSessionId,
) -> TestResult<(crate::foundation::ExactActorRef, crate::durability::item_transfer::CurrentCharacterItemFence)> {
    owner.root.try_issue_semantic_pass()?.run(|holder, _| Box::pin(async move {
        sqlx::query("RESET ROLE").execute(&mut **holder).await?;
        Ok(())
    })).await?;
    let actor = replace_channel_owner(owner, harness, session).await?;
    let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET scope_ownership_generation=$2::text::numeric
        WHERE game_session_id=encode($1,'hex')::uuid AND scope_ownership_generation=$2::text::numeric-1")
        .bind(session.as_bytes().as_slice()).bind(actor.scope_generation().get().to_string())
        .execute(&harness.pool).await?;
    assert_eq!(updated.rows_affected(), 1);
    let (current, _) = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session).await?;
    Ok((actor, crate::gameplay_transport::item_fence_of(current).ok_or("successor item fence")?))
}

async fn replace_channel_owner(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    session: crate::foundation::GameSessionId,
) -> TestResult<crate::foundation::ExactActorRef> {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::foundation::admission_authority_publication::*;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id);
    let predecessor = owner
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("scope predecessor")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2) ON CONFLICT DO NOTHING").bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(owner.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(replacement) = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes(sha2::Sha256::digest(predecessor.ownership_generation.to_be_bytes()).into()),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: harness.node.fact(),
            },
        })
        .await
        .map_err(debug)?
    else {
        return Err("scope replacement refused".into());
    };
    let generation = replacement.assignment.ownership_generation;
    assert_eq!(generation, predecessor.ownership_generation + 1);
    let publication_revision = replacement
        .fenced_publication_revision
        .ok_or("replacement runtime fence")?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        owner.root.clone(),
    );
    let mut change = guards
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("fenced Runtime guard")?;
    assert_eq!(change.publication_revision, publication_revision);
    change.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: publication_revision,
    };
    change.publication_revision += 1;
    change.source.source_revision += 1;
    change.source.decision_identity = format!("corpse-fixture-runtime-ready-{generation}");
    change.source.source_observed_at = now;
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready,
        ..
    } = &mut change.state
    else {
        return Err("Runtime guard shape".into());
    };
    assert_eq!(*ownership_generation, generation);
    *ready = true;
    let mut publisher = super::fresh_fixture::fresh::Source::new(now)?;
    publisher.rows = vec![change];
    let publication = AdmissionAuthorityPublicationV1::prepare(&publisher, now).map_err(debug)?;
    assert_eq!(
        owner
            .root
            .publish_runtime_readiness(&harness.node, &publication)
            .await
            .map_err(debug)?,
        crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    );
    let fact = harness.node.fact();
    let mut runtime = owner.runtime.lock().await;
    let mut restarted = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
        owner.world_id,
        owner.channel_id,
        fact.node_id(),
        fact.registration_revision(),
        generation,
        replacement.assignment.source_revision,
        &replacement.assignment.decision_identity,
        2,
        runtime.content_pin().clone(),
    )
    .map_err(debug)?;
    let reserved = restarted.reserve_fresh_session(session).map_err(debug)?;
    let peer = restarted.commit_fresh_session(reserved).map_err(debug)?;
    restarted
        .initialize_first_entry_position(peer)
        .map_err(debug)?;
    *runtime = restarted;
    drop(runtime);
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[11, 30],
        scope,
        generation,
        "policy-1",
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(debug)?;
    Ok(peer)
}
