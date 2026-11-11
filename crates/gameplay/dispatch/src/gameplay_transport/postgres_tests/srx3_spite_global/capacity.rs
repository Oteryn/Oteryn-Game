//! Known nil creation is terminal for its occurrence, including a lost result.
use super::super as pg;
use super::*;
use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::SemanticTimeMicros;

async fn fill_cell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    scene: &SourceScene,
    position: MovementLocalPosition,
    first_actor: u32,
) -> TestResult<(GroundPlacement, Vec<[u8; 16]>)> {
    let (scope, generation, ground) = {
        let runtime = owner.runtime.lock().await;
        let target = SpellGroundTarget::for_native_tile_read(&scene.room, &runtime, position)
            .map_err(debug)?;
        (
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation(),
            target.placement(
                [
                    crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                    pg::fixture::id(3).as_slice(),
                ]
                .concat(),
            ),
        )
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("ordinary Ground MINT lane")?;
    let mut items = Vec::new();
    for ordinal in 0..64 {
        // Ordinary native MINT fixture causes retain all production custody,
        // audit, capacity and scope checks. No Ground rows are synthesized.
        let mut candidate = owner
            .root
            .freeze_item_mint(
                owner.character,
                owner.holder,
                ItemMintRequest {
                    cause: ItemMintCause::for_test_scope(
                        scope,
                        generation,
                        first_actor + ordinal,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:loot.spite-full-ground".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:purpose.spite-full-ground".into(),
                        0,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                    ground: ground.clone(),
                    content_revision: ground.content_revision.clone(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        let ItemMintOutcome::Committed(receipt) = owner
            .root
            .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
            .await
            .map_err(debug)?
        else {
            return Err("every capacity root must be a fresh ordinary MINT".into());
        };
        items.push(receipt.item_instance_id);
    }
    assert_eq!(cell_count(h, &ground).await?, 64);
    owner.states.loop_ground.invalidate();
    Ok((ground, items))
}

async fn cell_count(h: &Harness, ground: &GroundPlacement) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.spatial_position=$1 AND g.map_revision=$2 AND g.content_revision=$3 AND g.native_room_placement_context=$4 AND i.lifecycle=1",
    ).bind(&ground.spatial_position).bind(&ground.map_revision).bind(&ground.content_revision)
        .bind(&ground.native_room_placement_context).fetch_one(&h.pool).await?)
}

#[test]
fn srx3_spite_full_ground_nil_survives_result_loss_and_freed_capacity_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    pg::kill_reward_e2e::scenario_thread::run("srx3-spite-capacity-replay", || {
        run(async |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33743,
                    y: 31629,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_spite_capacity_replay",
                false,
                false,
            )
            .await?;
            let evidence = pg::house_transfer::RecoverySource::start(&h).await?;
            pg::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            pg::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            pg::item_look::seed_main_backpack(&h, &[]).await?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("native Spite content")?;
            let proof = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("registered source event")?;
            let (callback, prepared) = {
                let mut runtime = owner.runtime.lock().await;
                let callback = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &proof,
                    1,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
                (callback, prepared)
            };
            let (ground, items) =
                fill_cell(&owner, &h, &scene, callback.position(), 30_000).await?;
            {
                let permit = owner
                    .spell_lane_permit()
                    .await
                    .ok_or("Spite physical lane")?;
                // Lose the return value before dispatch can acknowledge this
                // callback. Its own retained phase must carry the nil outcome.
                drop(
                    owner
                        .root
                        .commit_spite_global_fire(
                            &permit,
                            owner.character,
                            owner.holder,
                            &callback,
                            &prepared,
                        )
                        .await
                        .map_err(debug)?,
                );
            }
            assert_eq!(cell_count(&h, &ground).await?, 64);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4",
                )
                .fetch_one(&h.pool)
                .await?,
                0,
                "nil creation invents no physical receipt"
            );
            let picked = *items.last().ok_or("last real MINT")?;
            {
                let runtime = owner.runtime.lock().await;
                let at = runtime
                    .read_actor_position(admitted.runtime_actor.ok_or("actual Player")?)
                    .map_err(debug)?
                    .position();
                let field = callback.position();
                assert!(oteryn_gameplay_items::reach::within_reach(
                    (at.x, at.y, i32::from(at.floor)),
                    (field.x, field.y, i32::from(field.floor)),
                ));
            }
            let permit = owner.spell_lane_permit().await.ok_or("Spite replay lane")?;
            // These ordinary MINTs carry the source's native address, not a
            // map-overlay drop address. Free one through the real fenced Item
            // custody writer, without projecting the test roots as wire handles.
            let fence = admitted.item_fence.ok_or("current Item fence")?;
            let moved = owner
                .root
                .commit_item_loop_move(
                    &permit,
                    owner.character,
                    owner.holder,
                    &fence,
                    CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(700).map_err(debug)?,
                    ),
                    &NativeItemWeights(native),
                    LoopMoveRequest {
                        item: picked,
                        count: None,
                        pickup_cell: Some(ground.spatial_position.clone()),
                        destination: LoopDestination::MainBackpack,
                    },
                )
                .await
                .map_err(debug)?
                .map_err(debug)?;
            assert!(!moved.replayed);
            assert_eq!(cell_count(&h, &ground).await?, 63);
            assert!(
                owner
                    .root
                    .read_character_backpack(owner.character, fence.character_id)
                    .await?
                    .ok_or("actual native backpack")?
                    .entries
                    .iter()
                    .any(|entry| entry.item.item_instance_id == picked),
                "the same ordinary MINT entered current Character custody"
            );
            assert!(
                matches!(
                    owner
                        .root
                        .commit_spite_global_fire(
                            &permit,
                            owner.character,
                            owner.holder,
                            &callback,
                            &prepared,
                        )
                        .await
                        .map_err(debug)?,
                    SpiteGlobalFireOutcome::CreateNoSpace
                ),
                "losing the result cannot reinterpret the original nil after room is freed"
            );
            assert_eq!(cell_count(&h, &ground).await?, 63);
            let (later, later_prepared) = {
                let mut runtime = owner.runtime.lock().await;
                assert!(callback.current_for(&runtime) && prepared.current_for(&runtime));
                assert!(
                    callback
                        .prepare_current_expiry(&mut runtime, &scene.room, callback.due_us())
                        .is_err(),
                    "a failed creation never arms its five-second expiry"
                );
                let later = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &proof,
                    2,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let prepared = later.prepare_current_write(&runtime).map_err(debug)?;
                (later, prepared)
            };
            assert!(
                owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &callback,
                        &later_prepared,
                    )
                    .await
                    .is_err(),
                "the retained nil does not bypass the exact prepared callback binding"
            );
            let SpiteGlobalFireOutcome::Committed(created) = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &later,
                    &later_prepared,
                )
                .await
                .map_err(debug)?
            else {
                return Err("the next occurrence must create in the freed room".into());
            };
            assert!(!created.removed);
            assert_eq!(cell_count(&h, &ground).await?, 64);
            let SpiteGlobalFireOutcome::Committed(replayed) = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &later,
                    &later_prepared,
                )
                .await
                .map_err(debug)?
            else {
                return Err(
                    "receipt reconciliation must precede the now-full capacity check".into(),
                );
            };
            assert!(replayed.receipt.already_committed);
            assert_eq!(
                created.receipt.transaction_id,
                replayed.receipt.transaction_id
            );
            let counts: (i64, i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='fixture:purpose.spite-full-ground'), (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4), (SELECT count(*) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.cause_kind=4), (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire')",
            ).fetch_one(&h.pool).await?;
            assert_eq!(counts, (64, 1, 1, 0));
            Ok(())
        })
    })
}

#[test]
fn srx3_spite_full_ground_owner_advances_next_occurrence_and_native_ai_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    pg::kill_reward_e2e::scenario_thread::run("srx3-spite-capacity-owner", || {
        run(async |admin| {
            reset();
            let scene = pg::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33743,
                y: 31629,
                floor: 14,
            })?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_spite_capacity_owner",
                false,
                false,
            )
            .await?;
            let evidence = pg::house_transfer::RecoverySource::start(&h).await?;
            pg::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            pg::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let active = scene.controller.active().ok_or("actual active World")?;
            let native = active.native_gameplay().ok_or("actual native World")?;
            let world = active
                .qualify_published_world_context(&scene.room, &scene.pin)
                .ok_or("actual published World")?;
            let proof = native
                .qualify_spite_global_event_in_published_world(&scene.room, &scene.pin, &world)
                .ok_or("actual registered World source")?;
            let rat = {
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let key = "oteryn:creature.rat";
                let health = runtime.companion_policy(key).map_err(debug)?.maximum_health;
                let rat = runtime
                    .admit_pinned_monster_lab_creature(
                        MovementLocalPosition {
                            x: 33742,
                            y: 31629,
                            floor: 14,
                        },
                        key,
                        health,
                    )
                    .map_err(debug)?;
                runtime.install_creature_policy(rat, key).map_err(debug)?;
                rat
            };
            let mut grounds = Vec::new();
            for (index, [x, y, floor]) in proof.positions().into_iter().enumerate() {
                let (ground, _) = fill_cell(
                    &owner,
                    &h,
                    &scene,
                    MovementLocalPosition {
                        x,
                        y,
                        floor: i16::try_from(floor)?,
                    },
                    40_000 + u32::try_from(index)? * 64,
                )
                .await?;
                grounds.push(ground);
            }
            let registered = owner.owner_now().get();
            assert!(
                owner
                    .register_world_spite_global(registered)
                    .await
                    .map_err(debug)?
            );
            let mut prior_sequence = {
                let states = owner.states.spell_states.lock().await;
                assert!(states.monster_melee.creature_ai.state(rat).is_none());
                states.monster_ai_sequence
            };
            for due in [registered + 14_000_000, registered + 28_000_000] {
                owner
                    .drain_monster_melee_at(SemanticTimeMicros::from_micros(due))
                    .await;
                let states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states.spite_global.next_due_us(),
                    Some(due + 14_000_000),
                    "known nil creation advances once and never arms a five-second expiry"
                );
                assert!(
                    states.monster_melee.creature_ai.state(rat).is_some(),
                    "the same owner pass reaches real native Creature AI"
                );
                assert!(
                    states.monster_ai_sequence > prior_sequence,
                    "a full literal fire anchor cannot stall the Channel AI cycle"
                );
                prior_sequence = states.monster_ai_sequence;
            }
            for ground in grounds {
                assert_eq!(cell_count(&h, &ground).await?, 64);
            }
            let counts: (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='fixture:purpose.spite-full-ground'), (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4), (SELECT count(*) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.cause_kind=4)",
            ).fetch_one(&h.pool).await?;
            assert_eq!(
                counts,
                (256, 0, 0),
                "ordinary Ground roots remain intact and nil creates no physical write"
            );
            Ok(())
        })
    })
}
