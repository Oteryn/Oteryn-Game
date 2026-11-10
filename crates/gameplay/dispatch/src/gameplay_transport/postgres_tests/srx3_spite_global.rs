//! Actual GlobalEvent and Player STEP through the existing physical Item/Quest owners.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::creature_source_items::SpiteGlobalFireCallback;
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_weeping_actual_death_step_outfit_and_later_item_replay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-weeping-source-step", || {
        run(async |admin| {
            use crate::domain::condition::ConflictKey;
            use crate::durability::creature_source_items::SourceGroundStep;
            use crate::durability::spell_items_abi::SpellGroundTarget;
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
            use oteryn_simulation_determinism::{
                DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
            };
            reset();
            let h = Harness::create_with_admission_seed(admin, "srx3_weeping_step", false, false)
                .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let origin = MovementLocalPosition {
                x: 33743,
                y: 31629,
                floor: 14,
            };
            let (scene, activation_pin) =
                SourceScene::new_with_activation(PinnedActivitySourceRoom::GoshnarsSpite, origin)?;
            assert_ne!(
                scene.pin.frame_binding_digest(),
                activation_pin.frame_binding_digest()
            );
            // The scene needs slots for its Player, Spite and three actual Weeping actors.
            let deps = super::Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                5,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            // Actual public runtime uses BundleWorld, while content activation
            // owns ENTRY; exercise the production compatibility proof too.
            super::boss_activity::issue_content_and_reset(&h, &activation_pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let actor = admitted.runtime_actor.ok_or("actual source Player")?;
            let native = scene
                .controller
                .active()
                .and_then(|g| g.native_gameplay())
                .ok_or("actual native source")?;
            let (target_position, direction, reverse, boss) = {
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let (at, direction, reverse) = [
                    (1, 0, StepDirection::East, StepDirection::West),
                    (-1, 0, StepDirection::West, StepDirection::East),
                    (0, 1, StepDirection::South, StepDirection::North),
                ]
                .into_iter()
                .map(|(x, y, d, r)| {
                    (
                        MovementLocalPosition {
                            x: origin.x + x,
                            y: origin.y + y,
                            floor: origin.floor,
                        },
                        d,
                        r,
                    )
                })
                .find(|(at, _, _)| native_free_tile(&scene, &runtime, *at))
                .ok_or("actual adjacent non-fire source tile")?;
                let boss_at = [(-2, 0), (2, 0), (0, 2)]
                    .into_iter()
                    .map(|(x, y)| MovementLocalPosition {
                        x: origin.x + x,
                        y: origin.y + y,
                        floor: origin.floor,
                    })
                    .find(|at| native_free_tile(&scene, &runtime, *at))
                    .ok_or("actual Spite source tile")?;
                let boss = runtime
                    .spawn_arena_opponent(
                        "oteryn:creature.goshnar_s_spite",
                        boss_at,
                        |current, position| Some(native_free_tile(&scene, current, position)),
                    )
                    .map_err(debug)?;
                runtime.set_test_creature_health(boss, 100).map_err(debug)?;
                (at, direction, reverse, boss)
            };
            // Two real native deaths yield two original plain33876 Items on the
            // same destination. The first callback applies an outfit; the next
            // original callback must then early-return without another burn.
            for command in [65020, 65021] {
                weeping_corpse_from_actual_death(
                    &owner,
                    &admitted,
                    &scene,
                    target_position,
                    command,
                )
                .await?;
            }
            assert_eq!(sqlx::query_scalar::<_,i64>(
                "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'"
            ).fetch_one(&h.pool).await?,2);
            let permit = owner.spell_lane_permit().await.ok_or("actual Item lane")?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "current source fence")?
                .ok_or("live source Player")?
                .item_fence();
            let target = {
                let runtime = owner.runtime.lock().await;
                SpellGroundTarget::for_native_tile_read(&scene.room, &runtime, target_position)
                    .map_err(debug)?
            };
            let read = owner
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
                .map_err(debug)?;
            assert_eq!(
                read.items()
                    .iter()
                    .filter(|item| item.definition.production_key == "oteryn:item.tibia.i33876")
                    .count(),
                2
            );
            drop(permit);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(500))
                .ok_or("source movement pace")?;
            let (moved, _) = owner
                .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                .await;
            assert_eq!(moved.disposition, StepDisposition::Moved);
            let consumed:String = sqlx::query_scalar(
                "SELECT l.item_instance_id::text FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.cause_kind=6"
            ).fetch_one(&h.pool).await?;
            let consumed = sqlx::types::Uuid::parse_str(&consumed)?.into_bytes();
            let (input, outfit, hp_after) = {
                let runtime = owner.runtime.lock().await;
                let movement = runtime.read_actor_position(actor).map_err(debug)?;
                assert_eq!(movement.position(), target_position);
                let input = SourceGroundStep::from_current_player_step(
                    &runtime,
                    &scene.room,
                    native,
                    actor,
                    admitted.game_session_id,
                    movement,
                    owner.reset_epoch,
                    &read,
                    consumed,
                )
                .ok_or("same original committed movement and SQL Item")?;
                let mut occurrence = [0; 16];
                occurrence.copy_from_slice(&input.binding()[..16]);
                let draw = deterministic_decision_u64(
                    &GameplayDecisionRoot::from_bytes(native.source_digest()),
                    DecisionOccurrenceId::from_bytes(occurrence),
                    "spite_weeping_heal",
                    0,
                )
                .map_err(debug)?;
                let draw = crate::spell::uniform_draw(draw, 1, 100);
                let snapshot = runtime.companion_snapshot(boss).map_err(debug)?;
                assert_eq!(
                    snapshot.health,
                    if draw <= 10 {
                        (100 + snapshot.maximum_health / 10).min(snapshot.maximum_health)
                    } else {
                        100
                    },
                    "one exact source10% heal draw"
                );
                let states = owner.states.spell_states.lock().await;
                let player = states
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("current source Player")?;
                let outfit = player
                    .conditions
                    .active_at(ConflictKey::Outfit, owner.owner_now().get())
                    .ok_or("real Weeping outfit")?
                    .clone();
                assert_eq!(
                    outfit.definition().values(),
                    crate::foundation::ConditionValues::Outfit {
                        look_type: 286,
                        duration_ms: 14_000,
                    }
                );
                assert!(
                    player
                        .conditions
                        .active_at(ConflictKey::Outfit, outfit.started_at() + 14_000_000)
                        .is_none(),
                    "the canonical condition expires at the exact source14s"
                );
                (input, outfit, snapshot.health)
            };
            let scope = owner.runtime.lock().await.binding().runtime_scope();
            let ground = owner.root.read_item_loop_ground_for_scope(scope).await?;
            let remaining: Vec<_> = ground
                .iter()
                .filter(|item| item.item.definition.production_key == "oteryn:item.tibia.i33876")
                .collect();
            assert_eq!(
                remaining.len(),
                1,
                "the other original callback sees CONDITION_OUTFIT"
            );
            assert!(remaining[0].source_plain_corpse);
            assert_ne!(remaining[0].item.item_instance_id, consumed);
            // A second actual STEP during that same canonical outfit is also a
            // refusal. It neither removes the remaining corpse nor refreshes
            // the condition or applies a second source heal.
            for step in [reverse, direction] {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_millis(500))
                    .ok_or("source refusal pace")?;
                let (moved, _) = owner
                    .ordinary_paced_step(actor, admitted.game_session_id, step, None, None)
                    .await;
                assert_eq!(moved.disposition, StepDisposition::Moved);
            }
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states
                        .get(&runtime, actor, admitted.game_session_id)
                        .ok_or("current refused Player")?
                        .conditions
                        .active_at(ConflictKey::Outfit, owner.owner_now().get()),
                    Some(&outfit)
                );
                assert_eq!(
                    runtime.companion_snapshot(boss).map_err(debug)?.health,
                    hp_after
                );
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=6"
                )
                .fetch_one(&h.pool)
                .await?,
                1,
                "outfit refusal creates no Item receipt"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='source_step'")
                .fetch_one(&h.pool).await?,0,"Weeping has no Quest transition");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(500))
                .ok_or("source replay pace")?;
            let (moved, _) = owner
                .ordinary_paced_step(actor, admitted.game_session_id, reverse, None, None)
                .await;
            assert_eq!(moved.disposition, StepDisposition::Moved);
            weeping_corpse_from_actual_death(&owner, &admitted, &scene, target_position, 65022)
                .await?;
            let before_ground = owner.root.read_item_loop_ground_for_scope(scope).await?;
            let mut before_ids: Vec<_> = before_ground
                .iter()
                .filter(|item| item.item.definition.production_key == "oteryn:item.tibia.i33876")
                .map(|item| item.item.item_instance_id)
                .collect();
            before_ids.sort();
            assert_eq!(
                before_ids.len(),
                2,
                "original unconsumed and later newly minted UUID"
            );
            let before_state = {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("current replay Player")?
                    .clone()
            };
            let permit = owner.spell_lane_permit().await.ok_or("real replay lane")?;
            let replay = owner
                .root
                .commit_source_ground_step(&permit, owner.character, owner.holder, &input)
                .await
                .map_err(debug)?;
            assert!(replay.receipt.already_committed && replay.removed);
            drop(permit);
            let mut after_ids: Vec<_> = owner
                .root
                .read_item_loop_ground_for_scope(scope)
                .await?
                .iter()
                .filter(|item| item.item.definition.production_key == "oteryn:item.tibia.i33876")
                .map(|item| item.item.item_instance_id)
                .collect();
            after_ids.sort();
            assert_eq!(
                after_ids, before_ids,
                "receipt replay never selects a later matching UUID"
            );
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states.get(&runtime, actor, admitted.game_session_id),
                    Some(&before_state)
                );
                assert_eq!(
                    runtime.companion_snapshot(boss).map_err(debug)?.health,
                    hp_after
                );
                runtime
                    .assert_actor_spell_unreserved(actor)
                    .map_err(debug)?;
                runtime.assert_actor_spell_unreserved(boss).map_err(debug)?;
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=6"
                )
                .fetch_one(&h.pool)
                .await?,
                1
            );
            eprintln!("SRX3_ACTUAL_WEEPING_DEATH_STEP_OUTFIT_REPLAY_REACHED");
            Ok(())
        })
    })
}

fn native_free_tile(
    scene: &SourceScene,
    runtime: &crate::foundation::ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    runtime.native_summon_cell_free(at)
        && scene.room.movement_cells().index().lookup(
            scene.room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) == Ok(crate::content::CollisionClass::Walkable)
}

async fn weeping_corpse_from_actual_death(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    scene: &SourceScene,
    position: MovementLocalPosition,
    ordinal: u64,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    let actor = admitted.runtime_actor.ok_or("actual native Player")?;
    let fence = admitted.item_fence.ok_or("current Item fence")?;
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
        let child = runtime
            .spawn_arena_opponent("oteryn:creature.weeping_soul", position, |current, at| {
                Some(native_free_tile(scene, current, at))
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
                id: format!("srx3-native-weeping:{ordinal}"),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: format!("srx3-native-weeping:{ordinal}").into_bytes(),
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
                .any(|effect| effect.health.is_some_and(|hp| hp.health_after == 0))
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
                    .and_then(|g| g.native_gameplay())
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

#[test]
fn srx3_spite_private_registration_uses_actual_ready_scope_before_entry_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-spite-private-global", || {
        run(async |admin| {
            use crate::foundation::RuntimeScopeRefV1;
            use oteryn_simulation_determinism::SemanticTimeMicros;
            reset();
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_spite_private_global",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33774,
                    y: 31634,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let scope = super::activity_runtime::allocate_ready_instance(&h, 218).await?;
            let started = owner.owner_now().get();
            // Exercise the actual ready-runtime registration owner without
            // manufacturing a callback, accepting a lever or opening the gate.
            let ready = owner
                .ensure_pinned_source_runtime(
                    PinnedActivitySourceRoom::GoshnarsSpite,
                    Some((
                        "oteryn:creature.goshnar_s_spite",
                        MovementLocalPosition {
                            x: 33743,
                            y: 31632,
                            floor: 14,
                        },
                    )),
                    1024,
                )
                .await
                .map_err(debug)?;
            assert_eq!(ready.scope, scope);
            assert!(matches!(scope, RuntimeScopeRefV1::Instance { .. }));
            let view = owner.runtime_view(&ready);
            let first_due = {
                let runtime = view.runtime.lock().await;
                assert!(
                    runtime
                        .activity_participant_sessions()
                        .map_err(debug)?
                        .is_empty()
                );
                assert!(
                    runtime.spawn_point_creatures().is_empty(),
                    "prewarming cannot start the lever's XML respawn clock"
                );
                let states = view.states.spell_states.lock().await;
                assert!(
                    states.soul_war_callbacks.requires_committed_entry(),
                    "Spite's exact XML plan waits for the actual handoff even without a boss-specific Think owner"
                );
                states
                    .spite_global
                    .next_due_us()
                    .ok_or("production private Global registration")?
            };
            assert!(first_due >= started + 14_000_000);
            assert!(first_due <= owner.owner_now().get() + 14_000_000);
            let again = owner
                .ensure_pinned_source_runtime(
                    PinnedActivitySourceRoom::GoshnarsSpite,
                    Some((
                        "oteryn:creature.goshnar_s_spite",
                        MovementLocalPosition {
                            x: 33743,
                            y: 31632,
                            floor: 14,
                        },
                    )),
                    1024,
                )
                .await
                .map_err(debug)?;
            assert_eq!(again.scope, ready.scope);
            assert_eq!(
                again
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spite_global
                    .next_due_us(),
                Some(first_due),
                "rediscovering the same ready owner cannot restart its independent Global clock"
            );

            // The existing test port supplies exact semantic owner turns; every
            // Item write below is still the real fenced native/SQL producer.
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due - 1))
                .await;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4"
                )
                .fetch_one(&h.pool)
                .await?,
                0,
                "no Global MINT before its original14s"
            );
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due))
                .await;
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create' AND operation_count=1"
            ).fetch_one(&h.pool).await?, 1, "one real private-scope MINT at the registered deadline");
            let expiry_due = view
                .states
                .spell_states
                .lock()
                .await
                .spite_global
                .next_due_us()
                .ok_or("original private expiry")?;
            assert_eq!(expiry_due, first_due + 5_000_000);
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due))
                .await;
            assert_eq!(
                view.states
                    .spell_states
                    .lock()
                    .await
                    .spite_global
                    .next_due_us(),
                Some(expiry_due)
            );
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(expiry_due))
                .await;
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire' AND operation_count=1"
            ).fetch_one(&h.pool).await?, 1, "existing private owner expires the same current field once");
            assert!(
                view.runtime.lock().await.spawn_point_creatures().is_empty(),
                "Global ticks never substitute for committed XML activation"
            );
            Ok(())
        })
    })
}

#[test]
fn srx3_spite_actual_player_step_consumes_once_with_fifty_six_second_clock_postgres() -> TestResult
{
    spite_source_step_case(false)
}

#[test]
fn srx3_spite_cancelled_field_cleanup_retains_original_translation_postgres() -> TestResult {
    spite_source_step_case(true)
}

fn spite_source_step_case(cancel_during_cleanup: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-spite-source-step", move || {
        run(async move |admin| {
            use crate::durability::character_progression::CharacterProgressionError;
            use crate::durability::quest_state::{
                QuestTransitionOutcome, soul_war_source::SoulWarSourceAction,
            };
            use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
            use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "srx3_spite_step", false, false).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33743,
                    y: 31629,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            h.root.set_test_quest_clock(1_800_000_000).map_err(debug)?;
            let actor = admitted.runtime_actor.ok_or("actual source Player")?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual source generation")?;
            let registration = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("real registered Spite source")?;
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("physical Item lane")?;
            let (callback, prepared) = {
                let mut runtime = owner.runtime.lock().await;
                let callback = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &registration,
                    1,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
                (callback, prepared)
            };
            let created = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &prepared,
                )
                .await
                .map_err(debug)?;
            let first:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1")
                .bind(created.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            drop(permit);

            // A known unavailable precondition parks the real committed movement
            // before its Quest call. Resuming must use that retained selection,
            // without deriving a fresh event from a later standing observation.
            assert!(owner.quest_catalogue.is_none());
            if cancel_during_cleanup {
                use crate::gameplay_transport::spite_step::SPITE_STEP_CAPTURED_TEST_BARRIER;
                use std::future::{Future, poll_fn};
                use std::task::Poll;
                let captured = std::sync::Arc::new(tokio::sync::Notify::new());
                {
                    let mut movement = std::pin::pin!(SPITE_STEP_CAPTURED_TEST_BARRIER.scope(
                        std::sync::Arc::clone(&captured),
                        owner.ordinary_paced_step(
                            actor,
                            admitted.game_session_id,
                            StepDirection::North,
                            None,
                            None,
                        ),
                    ));
                    let mut entered = std::pin::pin!(captured.notified());
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        poll_fn(|cx| {
                            if entered.as_mut().poll(cx).is_ready() {
                                return Poll::Ready(Ok(()));
                            }
                            if let Poll::Ready(result) = movement.as_mut().poll(cx) {
                                return Poll::Ready(Err(format!(
                                    "movement ended before retained cleanup barrier: {result:?}"
                                )));
                            }
                            Poll::Pending
                        }),
                    )
                    .await??;
                    // Dropping the real future cancels field transaction cleanup,
                    // after movement but before any Source STEP SQL/read began.
                }
                let original = owner
                    .retained_spite_movement_for_test()
                    .await
                    .ok_or("physical movement retained before field cleanup")?;
                assert_eq!(
                    original.position(),
                    MovementLocalPosition {
                        x: 33743,
                        y: 31628,
                        floor: 14
                    }
                );
                assert_eq!(original.revision(), original.last_translation_revision());
                {
                    let runtime = owner.runtime.lock().await;
                    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?, original);
                    assert!(runtime.assert_actor_spell_unreserved(actor).is_err());
                }
                assert!(owner.retained_spite_step_for_test().await.is_none());
                // Observe the cancelled SQL holder's real return; no fixed sleep
                // or synthetic receipt substitutes for the retained source read.
                tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    loop {
                        assert!(owner.spell_lane_permit().await.is_none());
                        if owner.retained_spite_step_for_test().await.is_some() {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let (input, _) = owner
                    .retained_spite_step_for_test()
                    .await
                    .ok_or("same source after cancelled field cleanup")?;
                assert_eq!(input.context().movement_revision, original.revision());
            } else {
                let (moved, _) = owner
                    .ordinary_paced_step(
                        actor,
                        admitted.game_session_id,
                        StepDirection::North,
                        None,
                        None,
                    )
                    .await;
                assert_eq!(moved.disposition, StepDisposition::Moved);
            }
            let (input, fence) = owner
                .retained_spite_step_for_test()
                .await
                .ok_or("actual retained Source STEP")?;
            assert_eq!(
                input.item_instance_id(),
                sqlx::types::Uuid::parse_str(&first)?.into_bytes()
            );
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .assert_actor_spell_unreserved(actor)
                    .is_err()
            );
            let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&first).fetch_one(&h.pool).await?;
            assert!(live, "retention alone never removes the field");
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let catalogue = std::sync::Arc::new(lowered.catalogue().clone());
            // Change one context invariant while retaining the actual source,
            // session, Item, catalogue and movement. None is a fresh authority.
            let direct_permit = owner
                .spell_lane
                .acquire()
                .await
                .map_err(|_| "unresolved physical lane")?;
            for mutation in 0..4 {
                let mut wrong = input.context();
                match mutation {
                    0 => {
                        wrong.reset_epoch = wrong.reset_epoch.checked_add(1).ok_or("reset epoch")?
                    }
                    1 => {
                        wrong.movement_revision = wrong
                            .movement_revision
                            .checked_add(1)
                            .ok_or("movement revision")?
                    }
                    2 => wrong.actor_placement[0] ^= 1,
                    _ => {
                        *wrong
                            .placement_context
                            .first_mut()
                            .ok_or("native frame context")? ^= 1
                    }
                }
                let result = owner
                    .root
                    .commit_soul_war_source_action(
                        owner.character,
                        owner.holder,
                        fence,
                        SoulWarSourceAction::Step(input.clone()),
                        std::sync::Arc::clone(&catalogue),
                        Some((&direct_permit, wrong)),
                    )
                    .await;
                assert!(
                    matches!(result, Err(CharacterProgressionError::InvalidInput)),
                    "a changed source context cannot consume its current Item"
                );
            }
            drop(direct_permit);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("retained Source STEP resolves through sole writer")?;
            assert!(owner.retained_spite_step_for_test().await.is_none());
            owner
                .runtime
                .lock()
                .await
                .assert_actor_spell_unreserved(actor)
                .map_err(debug)?;
            let row:(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step')")
                .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
            assert_eq!(row, (1_800_000_000, 1));
            let retired:(i16,i64,bool)=sqlx::query_as("SELECT lifecycle,quantity,EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=$1::uuid")
                .bind(&first).fetch_one(&h.pool).await?;
            assert_eq!(retired, (2, 0, false));

            let (later, later_write) = {
                let mut runtime = owner.runtime.lock().await;
                let later = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &registration,
                    2,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let write = later.prepare_current_write(&runtime).map_err(debug)?;
                (later, write)
            };
            let later_receipt = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &later,
                    &later_write,
                )
                .await
                .map_err(debug)?;
            let later_item:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1")
                .bind(later_receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            let replay = owner
                .root
                .commit_soul_war_source_action(
                    owner.character,
                    owner.holder,
                    fence,
                    SoulWarSourceAction::Step(input.clone()),
                    std::sync::Arc::clone(&catalogue),
                    Some((&permit, input.context())),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
            let later_live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&later_item).fetch_one(&h.pool).await?;
            assert!(
                later_live,
                "immutable replay never selects a later current field"
            );
            let bare = owner
                .root
                .commit_character_quest_transition(
                    owner.character,
                    owner.holder,
                    fence,
                    SoulWarSourceAction::Step(input.clone())
                        .request(&fence)
                        .map_err(debug)?,
                    std::sync::Arc::clone(&catalogue),
                )
                .await;
            assert!(
                matches!(bare, Err(CharacterProgressionError::InvalidInput)),
                "a bare source cause is not the sealed Item participant"
            );

            let shape:bool=sqlx::query_scalar("SELECT game_spite_source_step_receipt_shape(r) FROM game_character_quest_receipts r WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert!(shape);
            for patch in [
                serde_json::json!({"cause_ordinal":1}),
                serde_json::json!({"cause_id":sqlx::types::Uuid::from_bytes(*fence.game_session_id.as_bytes()).to_string()}),
                serde_json::json!({"completes":true}),
                serde_json::json!({"track_keys":[rule::MAW]}),
                serde_json::json!({"values_before":[1_799_999_945]}),
                serde_json::json!({"committed_at":1_800_000_001}),
            ] {
                let accepted:bool=sqlx::query_scalar("SELECT game_spite_source_step_receipt_shape(jsonb_populate_record(NULL::game_character_quest_receipts,to_jsonb(r)||$2::jsonb)) FROM game_character_quest_receipts r WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(patch.clone()).fetch_one(&h.pool).await?;
                assert!(!accepted, "one invalid source receipt invariant: {patch}");
            }
            drop(permit);
            for (seconds, expected) in [(55, 1_i64), (1, 2_i64)] {
                for direction in [StepDirection::South, StepDirection::North] {
                    if direction == StepDirection::North {
                        h.root.advance_test_quest_clock(seconds).map_err(debug)?;
                    }
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_millis(500))
                        .ok_or("monotonic step pace")?;
                    let (moved, _) = owner
                        .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                        .await;
                    assert_eq!(moved.disposition, StepDisposition::Moved);
                }
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(count, expected);
                let consumed:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(
                    consumed, expected,
                    "a cooldown refusal leaves no consumption row"
                );
                let clock:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
                assert_eq!(
                    clock,
                    if seconds == 55 {
                        1_800_000_000
                    } else {
                        1_800_000_056
                    }
                );
                let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&later_item).fetch_one(&h.pool).await?;
                assert_eq!(
                    live,
                    seconds == 55,
                    "cooldown refusal preserves the same field; exactly 56 seconds admits it"
                );
            }
            let last:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
            assert_eq!(last, 1_800_000_056);
            Ok(())
        })
    })
}

#[test]
fn srx3_spite_registered_global_fire_uses_real_scope_without_creature_postgres() -> TestResult {
    registered_global_case(false, false)
}

#[test]
fn srx3_spite_registered_global_reserves_current_native_expiry_target_postgres() -> TestResult {
    registered_global_case(true, false)
}

#[test]
fn srx3_spite_registered_global_original_five_second_expiry_removes_current_fire_postgres()
-> TestResult {
    registered_global_case(false, true)
}

fn registered_global_case(reserve_target: bool, expire: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global", move || {
        run(async move |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33742,
                    y: 31639,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_spite_global", false, false)
                .await?;
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
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual native GlobalEvent generation")?;
            assert_eq!(
                scene.room.pinned_activity_source_room(),
                Some(PinnedActivitySourceRoom::GoshnarsSpite)
            );
            assert!(
                scene.room.source_world().is_none(),
                "production retained source recipe uses the actual Bundle collision frame"
            );
            assert!(native.qualifies_soul_war_callbacks(scene.pin.server_artifact_digest()));
            let proof = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("actual retained registered Spite GlobalEvent DATA")?;
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("actual physical lane")?;
            let mut runtime = owner.runtime.lock().await;
            assert!(
                runtime
                    .source_registered_creature_census()
                    .map_err(debug)?
                    .into_iter()
                    .all(|(actor, _)| {
                        runtime.companion_snapshot(actor).is_ok_and(|snapshot| {
                            snapshot.state.policy.definition_key
                                != "oteryn:creature.goshnar_s_spite"
                        })
                    }),
                "GlobalEvent executes without a Spite boss parent"
            );
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
            assert!(callback.current_for(&runtime));
            assert_eq!(
                callback.position(),
                MovementLocalPosition {
                    x: 33743,
                    y: 31628,
                    floor: 14
                }
            );
            let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
            let target = if reserve_target || expire {
                oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt::install_companion_policies(native, &mut runtime).map_err(debug)?;
                let key = "oteryn:creature.goshnar_s_spite";
                let policy = runtime.companion_policy(key).map_err(debug)?;
                let target = runtime
                    .admit_pinned_monster_lab_creature(
                        MovementLocalPosition {
                            x: 33742,
                            y: 31630,
                            floor: 14,
                        },
                        key,
                        policy.maximum_health,
                    )
                    .map_err(debug)?;
                runtime
                    .install_creature_policy(target, key)
                    .map_err(debug)?;
                assert!(
                    runtime
                        .source_creature_is_registered(target)
                        .map_err(debug)?
                );
                assert_eq!(
                    runtime.companion_snapshot(target).map_err(debug)?.health,
                    policy.maximum_health
                );
                assert!(prepared.current_for(&runtime));
                let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
                let defense = runtime
                    .effective_creature_defense(&snapshot)
                    .map_err(debug)?;
                if reserve_target {
                    eprintln!("SRX3_REGISTERED_GLOBAL_CURRENT_NATIVE_RESERVATION_REACHED");
                    let reservation = runtime.reserve_source_global_action(&prepared, &[target]);
                    assert!(
                        reservation.is_ok(),
                        "real actor-free Global proof must reserve current native target: {reservation:?}"
                    );
                    let reservation = reservation.map_err(debug)?;
                    assert!(runtime.assert_actor_spell_unreserved(target).is_err());
                    runtime
                        .validate_source_global_action(&reservation, &prepared)
                        .map_err(debug)?;
                    runtime
                        .complete_spite_global_action(
                            &reservation,
                            &prepared,
                            Some((&snapshot, defense)),
                            false,
                        )
                        .map_err(debug)?;
                    assert_eq!(
                        runtime
                            .effective_creature_defense(&snapshot)
                            .map_err(debug)?,
                        defense
                    );
                    runtime
                        .assert_actor_spell_unreserved(target)
                        .map_err(debug)?;
                    assert!(
                        runtime
                            .validate_source_global_action(&reservation, &prepared)
                            .is_err()
                    );
                    assert!(
                        runtime
                            .complete_spite_global_action(
                                &reservation,
                                &prepared,
                                Some((&snapshot, defense)),
                                true,
                            )
                            .is_err(),
                        "a released reservation cannot publish a later duplicate defense change"
                    );
                    return Ok(());
                }
                Some((snapshot, defense))
            } else {
                None
            };
            drop(runtime);
            let receipt = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &prepared,
                )
                .await
                .map_err(|error| {
                    format!("actual registered GlobalEvent physical boundary: {error:?}")
                })?;
            let replay_prepared = {
                let runtime = owner.runtime.lock().await;
                assert!(
                    prepared.current_for(&runtime),
                    "current scope/pins after known commit"
                );
                callback.prepare_current_write(&runtime).map_err(debug)?
            };
            let replay = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &replay_prepared,
                )
                .await
                .map_err(debug)?;
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    replay_prepared.current_for(&runtime),
                    "current scope/pins after receipt replay"
                );
            }
            assert!(!receipt.removed);
            assert!(!replay.removed);
            assert_eq!(
                receipt.receipt.transaction_id,
                replay.receipt.transaction_id
            );
            assert!(replay.receipt.already_committed);
            let row:(i16,Option<Vec<u8>>,Option<String>,i32)=sqlx::query_as(
                "SELECT cause_kind,creature_placement,creature_definition_key,operation_count FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid")
                .bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(row, (4, None, None, 1));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1 AND definition_production_key='oteryn:item.tibia.i33877'")
                .bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(count, 1);
            if expire {
                // A later real MINT at the same anchor becomes the current
                // top-down Thing. The old callback retains a position, not its UUID.
                let (later, later_prepared) = {
                    let mut runtime = owner.runtime.lock().await;
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
                let later_receipt = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &later,
                        &later_prepared,
                    )
                    .await
                    .map_err(debug)?;
                let original_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1",
                ).bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                let later_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1",
                ).bind(later_receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                assert_ne!(original_item, later_item);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let expiry = {
                    let mut runtime = owner.runtime.lock().await;
                    assert!(
                        owner.owner_now().get() >= callback.due_us(),
                        "real source execution clock crossed original5s deadline"
                    );
                    callback
                        .prepare_current_expiry(&mut runtime, &scene.room, owner.owner_now().get())
                        .map_err(debug)?
                };
                let (prepared, reservation) = {
                    let mut runtime = owner.runtime.lock().await;
                    let prepared = expiry.prepare_current_write(&runtime).map_err(debug)?;
                    let (snapshot, _) = target.as_ref().ok_or("actual expiry target")?;
                    let reservation = runtime
                        .reserve_source_global_action(&prepared, &[snapshot.actor])
                        .map_err(debug)?;
                    (prepared, reservation)
                };
                eprintln!("SRX3_REGISTERED_GLOBAL_ORIGINAL_FIVE_SECOND_EXPIRY_REACHED");
                let expired = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?;
                assert!(expired.removed);
                {
                    let mut runtime = owner.runtime.lock().await;
                    let (snapshot, defense) = target.as_ref().ok_or("actual expiry target")?;
                    runtime
                        .complete_spite_global_action(
                            &reservation,
                            &prepared,
                            Some((snapshot, *defense)),
                            expired.removed,
                        )
                        .map_err(debug)?;
                    assert_eq!(
                        runtime
                            .effective_creature_defense(snapshot)
                            .map_err(debug)?,
                        defense + 10
                    );
                    assert_eq!(
                        runtime
                            .companion_snapshot(snapshot.actor)
                            .map_err(debug)?
                            .health,
                        snapshot.health
                    );
                }
                let removed: i32 = sqlx::query_scalar("SELECT operation_count FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire'")
                    .fetch_one(&h.pool).await?;
                assert_eq!(
                    removed, 1,
                    "actual same-tile current33877 is removed by the existing Item writer"
                );
                let removed_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=2",
                ).bind(expired.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(
                    removed_item, later_item,
                    "expiry removes the current matching Thing, not the originally minted UUID"
                );
                let replay = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?;
                assert!(replay.receipt.already_committed);
                assert!(
                    replay.removed,
                    "the immutable receipt retains the original current-field outcome"
                );
                let original_still_current: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)",
                ).bind(&original_item).fetch_one(&h.pool).await?;
                assert!(
                    original_still_current,
                    "receipt replay cannot remove the next current field at the retained position"
                );
                {
                    let mut runtime = owner.runtime.lock().await;
                    let (snapshot, defense) = target.as_ref().ok_or("actual expiry target")?;
                    assert!(
                        runtime
                            .complete_spite_global_action(
                                &reservation,
                                &prepared,
                                Some((snapshot, *defense)),
                                replay.removed,
                            )
                            .is_err()
                    );
                    assert_eq!(
                        runtime
                            .effective_creature_defense(snapshot)
                            .map_err(debug)?,
                        defense + 10
                    );
                }
            }
            Ok(())
        })
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_ACTUAL_POSTGRES elapsed={:?}",
        started.elapsed()
    );
    result
}

#[test]
fn srx3_spite_registered_global_qualifies_actual_published_world_without_private_alias()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global-world", || {
        let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
            x: 33742,
            y: 31639,
            floor: 14,
        })?;
        let active = scene.controller.active().ok_or("actual World activation")?;
        let native = active
            .native_gameplay()
            .ok_or("actual native registered source closure")?;
        let world = active
            .qualify_published_world_context(&scene.room, &scene.pin)
            .ok_or("actual published World frame/pin proof")?;
        assert!(world.matches(&scene.room, &scene.pin));
        assert!(scene.room.pinned_activity_source_room().is_none());
        assert!(scene.room.source_world().is_none());
        assert!(native.qualifies_soul_war_callbacks(scene.pin.server_artifact_digest()));
        eprintln!("SRX3_REGISTERED_GLOBAL_ACTUAL_PUBLISHED_WORLD_QUALIFIER_REACHED");
        let proof = native.qualify_spite_global_event_in_published_world(&scene.room, &scene.pin, &world)
            .ok_or("registered source GlobalEvent must qualify real World without a private scope alias")?;
        assert_eq!(proof.interval_ms(), 14_000);
        assert_eq!(proof.expiry_ms(), 5_000);
        assert_eq!(proof.content(), scene.pin.server_artifact_digest());
        assert_ne!(proof.body_digest(), [0; 32]);
        Ok(())
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_PUBLISHED_WORLD elapsed={:?}",
        started.elapsed()
    );
    result
}

#[test]
fn srx3_spite_registered_world_owner_emits_one_selected_field_without_a_boss_after_fourteen_seconds_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global-owner", || {
        run(async |admin| {
            reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33742,
                y: 31639,
                floor: 14,
            })?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_spite_global_owner", false, false)
                    .await?;
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
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let registered = owner.owner_now().get();
            assert!(
                owner
                    .register_world_spite_global(registered)
                    .await
                    .map_err(debug)?,
                "actual published World registered callback prerequisite"
            );
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .source_registered_creature_census()
                        .map_err(debug)?
                        .is_empty(),
                    "the Global owner requires no boss or fake Creature parent"
                );
                let states = owner.states.spell_states.try_lock().map_err(debug)?;
                assert_eq!(
                    states.spite_global.next_due_us(),
                    Some(registered + 14_000_000)
                );
            }
            owner.drain_monster_melee().await;
            let before: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                before, 0,
                "no first-Think or boss-birth shortcut before source14s"
            );
            tokio::time::sleep(std::time::Duration::from_secs(14)).await;
            assert!(owner.owner_now().get() >= registered + 14_000_000);
            eprintln!("SRX3_REGISTERED_WORLD_GLOBAL_ACTUAL_FOURTEEN_SECOND_OWNER_REACHED");
            owner.drain_monster_melee().await;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create' AND operation_count=1 AND creature_placement IS NULL").fetch_one(&h.pool).await?;
            assert_eq!(
                count, 1,
                "one actual source dispatch creates one field at its selected literal anchor without a boss"
            );
            owner.drain_monster_melee().await;
            let replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create'").fetch_one(&h.pool).await?;
            assert_eq!(replay, 1, "one dispatch, no reroll or catchup batch");
            let expiry_due = owner
                .states
                .spell_states
                .try_lock()
                .map_err(debug)?
                .spite_global
                .next_due_us()
                .ok_or("retained original expiry")?;
            assert!(expiry_due > registered + 14_000_000);
            assert!(expiry_due <= owner.owner_now().get() + 5_000_000);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            assert!(owner.owner_now().get() >= expiry_due);
            owner.drain_monster_melee().await;
            let expired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire' AND operation_count=1 AND creature_placement IS NULL")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                expired, 1,
                "the actual owner removes the current field after its retained five-second deadline without a boss"
            );
            owner.drain_monster_melee().await;
            let expired_replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire'")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                expired_replay, 1,
                "completed expiry is not repeated by another owner pass"
            );
            Ok(())
        })
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_REAL_WORLD_OWNER elapsed={:?}",
        started.elapsed()
    );
    result
}
