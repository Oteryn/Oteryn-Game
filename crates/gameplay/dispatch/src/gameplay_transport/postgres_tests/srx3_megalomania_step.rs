//! Native corpse MINTs and the real Player/Creature position owners exercise
//! original33949/33984 cohorts. No Item fixture grant or invented Creature Player.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::creature_source_items::SourceGroundStep;
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::sync::Arc;

#[test]
fn srx3_megalomania_native_corpses_player_and_creature_steps_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-megalomania-ground-step", || {
        run(async |admin| {
            use crate::gameplay_transport::capabilities::{
                BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
            };
            use oteryn_protocol_oteryn::boss_encounter as wire;
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsMegalomania,
                MovementLocalPosition {
                    x: 33676,
                    y: 31634,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_megalomania_step", false, false)
                    .await?;
            // Imported prior access, as in the existing actual Megalomania entry
            // fixture. Every new entry, corpse, STEP and counter uses its owner.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,31,extract(epoch from clock_timestamp())::bigint,false,false)")
                .bind(id(41).as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(
                &mut owner,
                &scene,
                PinnedActivitySourceRoom::GoshnarsMegalomania,
            )?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[
                    4,
                    6,
                    7,
                    10,
                    11,
                    15,
                    18,
                    23,
                    92,
                    wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                ],
            )
            .ok_or("actual boss capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 217).await?;
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_megalomania",
                true,
                &mut Vec::new(),
            )
            .await?;
            assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance Player")?;
            assert_eq!(fence.runtime_scope, instance);
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual ready Instance")?;
            let mut view = owner.runtime_view(&registered);
            view.drain_monster_melee().await;
            let player = admitted.runtime_actor.ok_or("actual Player actor")?;
            let (target, creature_origin, direction, reverse, creature_direction) = {
                let runtime = view.runtime.lock().await;
                let at = runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position();
                [
                    (
                        1,
                        0,
                        StepDirection::East,
                        StepDirection::West,
                        crate::movement::CardinalStep::West,
                    ),
                    (
                        -1,
                        0,
                        StepDirection::West,
                        StepDirection::East,
                        crate::movement::CardinalStep::East,
                    ),
                    (
                        0,
                        1,
                        StepDirection::South,
                        StepDirection::North,
                        crate::movement::CardinalStep::North,
                    ),
                    (
                        0,
                        -1,
                        StepDirection::North,
                        StepDirection::South,
                        crate::movement::CardinalStep::South,
                    ),
                ]
                .into_iter()
                .find_map(|(dx, dy, direction, reverse, creature_direction)| {
                    let target = MovementLocalPosition {
                        x: at.x + dx,
                        y: at.y + dy,
                        ..at
                    };
                    let origin = MovementLocalPosition {
                        x: at.x + 2 * dx,
                        y: at.y + 2 * dy,
                        ..at
                    };
                    (free_tile(&view, &runtime, target) && free_tile(&view, &runtime, origin))
                        .then_some((target, origin, direction, reverse, creature_direction))
                })
                .ok_or("actual adjacent three-cell native movement lane")?
            };
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.necromantic_focus",
                66001,
            )
            .await?;
            set_counter(&view, &admitted, 24).await?;
            let read = original_tile(&view, &admitted, target).await?;
            let original = read
                .items()
                .iter()
                .find(|item| item.definition.production_key == "oteryn:item.tibia.i33984")
                .ok_or("actual native Focus corpse33984")?
                .item_instance_id;
            paced(&mut view, &admitted, direction).await?;
            assert_counter(&view, &admitted, 19).await?;
            let first_input = {
                let runtime = view.runtime.lock().await;
                let native = view
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .ok_or("native source")?;
                SourceGroundStep::from_current_player_step(
                    &runtime,
                    view.effective_rooms()
                        .gate_field_step_ingress()
                        .ok_or("current source room")?,
                    native,
                    player,
                    admitted.game_session_id,
                    runtime.read_actor_position(player).map_err(debug)?,
                    view.reset_epoch,
                    &read,
                    original,
                )
                .ok_or("same original33984 committed STEP")?
            };
            paced(&mut view, &admitted, reverse).await?;
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.necromantic_focus",
                66002,
            )
            .await?;
            let later = original_tile(&view, &admitted, target).await?;
            let later_ids: Vec<_> = later
                .items()
                .iter()
                .map(|item| item.item_instance_id)
                .collect();
            assert!(!later_ids.contains(&original));
            let permit = view.spell_lane_permit().await.ok_or("real replay lane")?;
            let replay = h
                .root
                .commit_source_ground_step(&permit, view.character, view.holder, &first_input)
                .await
                .map_err(debug)?;
            assert!(replay.receipt.already_committed && replay.removed);
            drop(permit);
            assert_counter(&view, &admitted, 19).await?;
            assert_eq!(
                original_tile(&view, &admitted, target)
                    .await?
                    .items()
                    .iter()
                    .map(|item| item.item_instance_id)
                    .collect::<Vec<_>>(),
                later_ids
            );

            // A successful Weeping callback returns true, so the later
            // original Dead Aspect and Remains callbacks still execute.
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.aspect_of_power",
                66003,
            )
            .await?;
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.weeping_soul",
                66004,
            )
            .await?;
            set_counter(&view, &admitted, 24).await?;
            paced(&mut view, &admitted, direction).await?;
            assert_counter(&view, &admitted, 0).await?;
            {
                let runtime = view.runtime.lock().await;
                let states = view.states.spell_states.lock().await;
                assert!(
                    states
                        .get(&runtime, player, admitted.game_session_id)
                        .ok_or("current mixed Player")?
                        .conditions
                        .active_at(
                            crate::domain::condition::ConflictKey::Outfit,
                            view.owner_now().get()
                        )
                        .is_some()
                );
                runtime
                    .assert_actor_spell_unreserved(player)
                    .map_err(debug)?;
            }
            paced(&mut view, &admitted, reverse).await?;

            // With an already active Outfit, the first Weeping returns nil.
            // The actual MoveEvents dispatcher stops this original vector,
            // leaving the later Dead Aspect and the counter untouched.
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.aspect_of_power",
                66005,
            )
            .await?;
            actual_corpse(
                &view,
                &admitted,
                target,
                "oteryn:creature.weeping_soul",
                66006,
            )
            .await?;
            set_counter(&view, &admitted, 24).await?;
            let blocked = original_tile(&view, &admitted, target).await?;
            let blocked_ids: Vec<_> = blocked
                .items()
                .iter()
                .map(|item| item.item_instance_id)
                .collect();
            assert_eq!(
                blocked
                    .items()
                    .first()
                    .map(|item| item.definition.production_key.as_str()),
                Some("oteryn:item.tibia.i33876"),
                "actual original top-down callback order"
            );
            paced(&mut view, &admitted, direction).await?;
            assert_counter(&view, &admitted, 24).await?;
            assert_eq!(
                original_tile(&view, &admitted, target)
                    .await?
                    .items()
                    .iter()
                    .map(|item| item.item_instance_id)
                    .collect::<Vec<_>>(),
                blocked_ids
            );
            paced(&mut view, &admitted, reverse).await?;

            // Both known nonplayer nil branches stop before a later33949.
            // The original refusal completes synchronously without Item SQL
            // or a fabricated pending movement continuation.
            for remains in [false, true] {
                if remains {
                    actual_corpse(
                        &view,
                        &admitted,
                        target,
                        "oteryn:creature.necromantic_focus",
                        66007,
                    )
                    .await?;
                }
                let before = original_tile(&view, &admitted, target).await?;
                assert_eq!(
                    before
                        .items()
                        .first()
                        .map(|item| item.definition.production_key.as_str()),
                    Some(if remains {
                        "oteryn:item.tibia.i33984"
                    } else {
                        "oteryn:item.tibia.i33876"
                    })
                );
                let blocked_actor = {
                    let mut runtime = view.runtime.lock().await;
                    runtime
                        .spawn_arena_opponent(
                            "oteryn:creature.rat",
                            creature_origin,
                            |runtime, at| Some(free_tile(&view, runtime, at)),
                        )
                        .map_err(debug)?
                };
                assert!(
                    actual_creature_step(&h, &view, blocked_actor, creature_direction, target)
                        .await?
                        .is_none()
                );
                assert_eq!(
                    original_tile(&view, &admitted, target)
                        .await?
                        .items()
                        .iter()
                        .map(|item| item.item_instance_id)
                        .collect::<Vec<_>>(),
                    before
                        .items()
                        .iter()
                        .map(|item| item.item_instance_id)
                        .collect::<Vec<_>>()
                );
                view.runtime
                    .lock()
                    .await
                    .remove_encounter_actor(blocked_actor)
                    .map_err(debug)?;
            }

            // Same sealed physical primitive, but an actual Creature movement
            // has no session, character, lease, spell or fabricated command.
            for (ordinal, key, should_remove) in [
                (66009, "oteryn:creature.greater_splinter_of_madness", true),
                (66011, "oteryn:creature.rat", false),
            ] {
                actual_corpse(
                    &view,
                    &admitted,
                    target,
                    "oteryn:creature.aspect_of_power",
                    ordinal,
                )
                .await?;
                let creature = {
                    let mut runtime = view.runtime.lock().await;
                    runtime
                        .spawn_arena_opponent(key, creature_origin, |runtime, at| {
                            Some(free_tile(&view, runtime, at))
                        })
                        .map_err(debug)?
                };
                let retained =
                    actual_creature_step(&h, &view, creature, creature_direction, target)
                        .await?
                        .ok_or("actual33949 prefix before the original nil barrier")?;
                let permit = view
                    .spell_lane_permit()
                    .await
                    .ok_or("actual Creature Item lane")?;
                view.drain_source_creature_ground_steps(&permit, view.owner_now().get())
                    .await
                    .map_err(|_| "actual Creature STEP ACK")?;
                drop(permit);
                {
                    let runtime = view.runtime.lock().await;
                    assert_eq!(runtime.contains_live_creature(creature), !should_remove);
                    if !should_remove {
                        runtime
                            .assert_actor_spell_unreserved(creature)
                            .map_err(debug)?;
                    }
                }
                if !should_remove {
                    // Walk the surviving ordinary Creature back to its known
                    // empty original cell before a second native death there.
                    let mut runtime = view.runtime.lock().await;
                    let at = runtime.read_actor_position(creature).map_err(debug)?;
                    runtime
                        .borrow_movement_position()
                        .commit_cardinal(at, creature_origin)
                        .map_err(debug)?;
                }
                // A later canonical corpse keeps its UUID when the old immutable
                // physical result is reconciled, including after Splinter removal.
                actual_corpse(
                    &view,
                    &admitted,
                    target,
                    "oteryn:creature.necromantic_focus",
                    ordinal + 1,
                )
                .await?;
                let before = original_tile(&view, &admitted, target)
                    .await?
                    .items()
                    .iter()
                    .map(|item| item.item_instance_id)
                    .collect::<Vec<_>>();
                let permit = view
                    .spell_lane_permit()
                    .await
                    .ok_or("actual Creature replay lane")?;
                assert!(
                    h.root
                        .commit_source_ground_step(&permit, view.character, view.holder, &retained)
                        .await
                        .map_err(debug)?
                        .receipt
                        .already_committed
                );
                drop(permit);
                assert_eq!(
                    original_tile(&view, &admitted, target)
                        .await?
                        .items()
                        .iter()
                        .map(|item| item.item_instance_id)
                        .collect::<Vec<_>>(),
                    before
                );
                if !should_remove {
                    view.runtime
                        .lock()
                        .await
                        .remove_encounter_actor(creature)
                        .map_err(debug)?;
                }
            }
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i33950'").fetch_one(&h.pool).await?,0,
                "transform on the removed original33949 cannot mint33950");
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='source_step'").fetch_one(&h.pool).await?,0,
                "these source counter/Item callbacks acquire no Quest flag");
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=6 AND convert_from(intent,'UTF8')::jsonb#>>'{source_step,actor_kind}'='creature'").fetch_one(&h.pool).await?,2);
            eprintln!("SRX3_MEGALOMANIA_NATIVE_CORPSE_PLAYER_CREATURE_STEP_REPLAY_REACHED");
            Ok(())
        })
    })
}

fn free_tile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &crate::foundation::ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    runtime.native_summon_cell_free(at)
        && owner
            .effective_rooms()
            .gate_monster_ai()
            .is_some_and(|room| {
                room.movement_cells().index().lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) == Ok(crate::content::CollisionClass::Walkable)
            })
}
async fn paced(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(500))
        .ok_or("source pace")?;
    let (moved, _) = owner
        .ordinary_paced_step(
            admitted.runtime_actor.ok_or("actual Player")?,
            admitted.game_session_id,
            direction,
            None,
            None,
        )
        .await;
    assert_eq!(moved.disposition, StepDisposition::Moved);
    Ok(())
}
async fn set_counter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    count: u8,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actual Player")?;
    let mut states = owner.states.spell_states.lock().await;
    let before = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("canonical Player state")?;
    let after = before
        .stage_soul_war_torment_counter(count)
        .ok_or("canonical counter successor")?;
    if after.revision() != before.revision() {
        assert!(states.commit(&runtime, actor, admitted.game_session_id, after));
    }
    Ok(())
}
async fn assert_counter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    count: u8,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let current = states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("actual Player")?,
            admitted.game_session_id,
        )
        .ok_or("current Player")?;
    assert_eq!(current.soul_war_torment_count(), count);
    assert_eq!(
        current.soul_war_torment_icon_count(),
        (count != 0).then_some(u16::from(count))
    );
    Ok(())
}
async fn original_tile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    position: MovementLocalPosition,
) -> TestResult<crate::durability::spell_item_transaction::StandingTileRead> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current Player fence")?
        .ok_or("Player fence")?
        .item_fence();
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native source")?;
    let target = {
        let runtime = owner.runtime.lock().await;
        crate::durability::spell_items_abi::SpellGroundTarget::for_native_tile_read(
            owner
                .effective_rooms()
                .gate_field_step_ingress()
                .ok_or("source room")?,
            &runtime,
            position,
        )
        .map_err(debug)?
    };
    let permit = owner.spell_lane_permit().await.ok_or("actual tile lane")?;
    owner
        .root
        .read_standing_player_tile(
            &permit,
            owner.character,
            owner.holder,
            fence,
            native.source_digest(),
            target,
        )
        .await
        .map_err(|error| debug(error).into())
}

async fn actual_creature_step(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    direction: crate::movement::CardinalStep,
    target: MovementLocalPosition,
) -> TestResult<Option<SourceGroundStep>> {
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("real Creature map lane")?;
    let queue = Arc::clone(&owner.states.spell_states.lock().await.spite_steps.creatures);
    let mut runtime = owner.runtime.lock().await;
    runtime
        .note_rotten_movement_time(owner.owner_now().get())
        .map_err(debug)?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let objects = owner.door.lock().await;
    let mut tx = h.pool.begin().await?;
    let scope = runtime.binding().runtime_scope();
    let authority =
        crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            scope,
            runtime.binding().scope_generation().get(),
        )
        .await
        .map_err(debug)?;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("current Creature room")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native source")?;
    let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
        &mut tx, &authority, &runtime, room, &objects, native,
    );
    map.bind_source_ground_steps(Arc::clone(&queue), owner.reset_epoch)
        .map_err(|_| "actual queue binding")?;
    assert_eq!(map.read(&runtime, target).await, Some(true));
    assert!(map.validate_transaction(&runtime).await);
    let moved = map
        .view()
        .step(
            &mut runtime,
            room.movement_cells(),
            actor,
            before,
            direction,
        )
        .map_err(debug)?;
    assert_eq!(moved.position(), target);
    let head = queue.front().map_err(debug)?;
    let input = head.as_ref().and_then(|pending| pending.input()).cloned();
    assert_eq!(
        runtime.assert_actor_spell_unreserved(actor).is_err(),
        input.is_some()
    );
    if let Some(head) = &head {
        assert!(queue.retains_head(&runtime, head).map_err(debug)?);
        assert_eq!(
            queue.next_due_us(&runtime).map_err(debug)?,
            runtime
                .committed_movement_occurrence(actor)
                .map_err(debug)?
                .map(|movement| movement.observed_us())
        );
        assert_eq!(
            map.view()
                .step(&mut runtime, room.movement_cells(), actor, moved, direction),
            Err(crate::movement::MovementError::Actor(
                crate::foundation::CarrierError::PlanConflict
            ))
        );
    }
    drop(map);
    tx.rollback().await?;
    assert_eq!(
        queue.is_pending(),
        input.is_some(),
        "dropping/rolling back the read future retains the committed move and original cohort"
    );
    Ok(input)
}

async fn actual_corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: MovementLocalPosition,
    key: &str,
    ordinal: u64,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    let actor = admitted.runtime_actor.ok_or("actual Player")?;
    let fence = admitted.item_fence.ok_or("actual Item fence")?;
    let now = owner.owner_now();
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(ordinal).map_err(debug)?,
    );
    super::srx3_rotten_callbacks::publish_native_fixture_player_source(
        owner,
        fence,
        actor,
        command,
        now.get(),
    )
    .await?;
    let child = {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or("native source")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let child = runtime
            .spawn_arena_opponent(key, at, |runtime, position| {
                Some(free_tile(owner, runtime, position))
            })
            .map_err(debug)?;
        let maximum = runtime
            .companion_snapshot(child)
            .map_err(debug)?
            .maximum_health;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command,
            occurrence: SpellOccurrenceBinding {
                id: format!("srx3-mego-corpse:{ordinal}"),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: format!("srx3-mego-corpse:{ordinal}").into_bytes(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: child,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(child).map_err(debug)?,
                    magnitude: maximum,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(
            receipt
                .effects
                .iter()
                .any(|effect| effect.health.is_some_and(|health| health.health_after == 0))
        );
        let attack = owner.states.attack.lock().await;
        crate::gameplay_transport::kill_reward::record_batch_kills(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: owner
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .ok_or("native rewards")?
                    .reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        child
    };
    super::srx3_native_source_cast::drain_source_death(owner, child, now).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    Ok(())
}
