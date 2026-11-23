//! CREATURE-MOVE-1 through the real composed Channel dispatch and existing PostgreSQL harness.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::Duration;

#[test]
fn creature_movement_dispatch_steps_between_thinks_on_postgres() -> TestResult {
    scenario(false)
}

#[test]
fn creatures_monster_swings_at_adjacent_target_with_65_players_in_view() -> TestResult {
    scenario(true)
}

fn scenario(crowded: bool) -> TestResult {
    run(async move |admin| {
        let harness = Harness::create_with_admission_seed(
            admin,
            if crowded {
                "creatures_crowded_melee"
            } else {
                "creature_move_dispatch"
            },
            false,
            false,
        )
        .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        deps.room = activated.qualified_room().clone();
        use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
        let world =
            crate::map::spell_tiles::tests::movement_arena::world().with_spawns_for_test(Table {
                creatures: vec![Creature {
                    key: if crowded {
                        "oteryn:creature.rat"
                    } else {
                        "canary:creature/rat"
                    }
                    .into(),
                    period: Period::All,
                }],
                sources: vec![Source {
                    key: "oteryn:spawn/creature-move-test".into(),
                    floor: -7,
                    x: 104,
                    y: 100,
                    points: vec![Point {
                        creature: 0,
                        x: 104,
                        y: 100,
                        direction: Direction::South,
                        respawn_ms: 60_000,
                        boss_policy: None,
                    }],
                }],
            });
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let room = deps
            .room
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .ok_or("bundle combat view")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (100, 100, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            if crowded { 66 } else { 4 },
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let sources = world
            .activation_spawns(|key| {
                let policy = runtime.companion_policy(key).ok()?;
                Some((u64::try_from(policy.maximum_health).ok()?, false))
            })
            .ok_or("qualified spawn policies")?;
        runtime
            .begin_activation_spawn_sources(sources)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .realize_activation_spawn_window(0)
                .map_err(debug)?
                .realized,
            1
        );
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut bystanders = Vec::new();
        if crowded {
            for tag in 80..144 {
                let session = crate::foundation::GameSessionId::decode(&id(tag)).map_err(debug)?;
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                bystanders.push((actor, session));
            }
        }
        let start = MovementLocalPosition {
            x: 104,
            y: 100,
            floor: 7,
        };
        let rat = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("native rat")?
            .actor;
        assert_eq!(runtime.creature_spawn_cell(rat), Some(start));
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.combat_room = Some(room);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.clock_origin = std::time::Instant::now();
        deps.initialize(&owner).await;
        owner.clock_origin -= Duration::from_secs(10);
        if crowded {
            // A due native attack out of reach leaves an extra melee pending.
            // Keep the Rat out of reach while its source attack timer advances.
            let mut pending = None;
            for _ in 0..4 {
                {
                    let mut runtime = deps.runtime.lock().await;
                    let mut from = runtime.read_actor_position(rat).map_err(debug)?;
                    assert_eq!(from.position().y, start.y);
                    while from.position().x < start.x {
                        let to = MovementLocalPosition {
                            x: from.position().x + 1,
                            ..start
                        };
                        from = runtime
                            .borrow_movement_position()
                            .commit_cardinal(from, to)
                            .map_err(debug)?;
                    }
                }
                owner.drain_monster_melee().await;
                let mut runtime = deps.runtime.lock().await;
                let origin = runtime.read_actor_position(rat).map_err(debug)?.position();
                let from = runtime.read_actor_position(deps.actor).map_err(debug)?;
                let next = MovementLocalPosition {
                    x: from.position().x + 1,
                    ..from.position()
                };
                let step = runtime
                    .borrow_movement_position()
                    .commit_cardinal(from, next)
                    .map_err(debug)?;
                let states = owner.states.spell_states.lock().await;
                if states
                    .monster_melee
                    .creature_ai
                    .has_movement_melee_retry(&runtime)
                {
                    pending = Some(origin);
                    break;
                }
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(step, from.position())
                    .map_err(debug)?;
                owner.clock_origin -= Duration::from_secs(2);
            }
            let origin = pending.ok_or("due out-of-reach Rat melee")?;
            let now = owner.owner_now();
            let time = SemanticTimeMicros::from_micros(now.get());
            {
                let states = owner.states.spell_states.lock().await;
                let melee = &states.monster_melee;
                let think = melee.creature_ai.state(rat).ok_or("native Rat Think")?;
                assert_eq!(think.target(), Some(deps.actor));
                assert!(
                    melee
                        .creature_ai
                        .due(rat)
                        .is_some_and(|due| now.get() < due)
                );
                let attack = owner.states.attack.lock().await;
                assert!(
                    !attack
                        .combat_state(deps.actor, deps.admitted.game_session_id, time)
                        .in_fight
                );
            }
            for (actor, session) in bystanders {
                deps.initialize_player(&owner, actor, session).await;
            }
            {
                let mut runtime = deps.runtime.lock().await;
                let mut from = runtime.read_actor_position(deps.actor).map_err(debug)?;
                assert_eq!(from.position().y, origin.y);
                while from.position().x < origin.x - 1 {
                    let to = MovementLocalPosition {
                        x: from.position().x + 1,
                        ..origin
                    };
                    from = runtime
                        .borrow_movement_position()
                        .commit_cardinal(from, to)
                        .map_err(debug)?;
                }
                assert_eq!(origin.x - from.position().x, 1);
                let states = owner.states.spell_states.lock().await;
                assert!(
                    states
                        .monster_melee
                        .creature_ai
                        .has_movement_melee_retry(&runtime)
                );
                let attack = owner.states.attack.lock().await;
                let census = runtime.positioned_melee_census().map_err(debug)?;
                let room = owner.rooms().gate_monster_ai().ok_or("combat room")?;
                let targets = crate::gameplay_transport::monster_ai_cycle::melee_targets(
                    room,
                    &runtime,
                    &states,
                    &attack,
                    &census,
                    now.get(),
                );
                assert_eq!(
                    targets
                        .iter()
                        .filter(|(_, position, _, invisible, protected)| {
                            !invisible
                                && !protected
                                && position.floor == origin.floor
                                && crate::ai_think::targeting::sees(*position, origin)
                        })
                        .count(),
                    65,
                );
            }
            owner.drain_monster_melee_at(now).await;
            let states = owner.states.spell_states.lock().await;
            let melee = &states.monster_melee;
            let selected = melee
                .creature_ai
                .native_melee_selection(rat, 0)
                .ok_or("native movement melee selection")?;
            assert_eq!(selected.target, Some(deps.actor));
            eprintln!("CREATURES_CROWDED_MELEE_POSTGRES_CASE_EXECUTED");
            let attack = owner.states.attack.lock().await;
            assert!(
                attack
                    .combat_state(deps.actor, deps.admitted.game_session_id, time)
                    .in_fight,
                "the selected attack started the target's in-fight deadline"
            );
            drop(attack);
            drop(states);
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }
        owner.drain_monster_melee().await;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(rat)
                .map_err(debug)?
                .position(),
            start,
            "the think schedules a deadline without moving"
        );
        let (next, sequence) = {
            let states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .monster_melee
                    .creature_ai
                    .state(rat)
                    .and_then(|state| state.target())
                    .is_some(),
                "the actual think acquired the player after re-entry protection"
            );
            (states.next_monster_ai_pass_us, states.monster_ai_sequence)
        };
        assert!(next > owner.owner_now().get());
        assert!(
            next < owner.owner_now().get() + 900_000,
            "step is due before the next think"
        );
        let until = next.saturating_sub(owner.owner_now().get());
        owner.clock_origin -= Duration::from_micros(until + 100);
        owner.drain_monster_melee().await;
        let after = deps
            .runtime
            .lock()
            .await
            .read_actor_position(rat)
            .map_err(debug)?;
        assert_ne!(
            after.position(),
            start,
            "real dispatch committed the creature's step"
        );
        assert_eq!(after.position().floor, start.floor);
        let states = owner.states.spell_states.lock().await;
        assert_eq!(states.monster_ai_sequence, sequence + 1);
        assert!(
            states
                .monster_melee
                .creature_ai
                .due(rat)
                .is_some_and(|due| due > next),
            "step firing leaves the original think deadline pending"
        );
        drop(states);
        let displaced = {
            let mut runtime = deps.runtime.lock().await;
            crate::movement::relocate::relocate_creature(
                &mut runtime,
                rat,
                &cells,
                &std::collections::BTreeSet::new(),
                MovementLocalPosition { x: 53, ..start },
            )
            .map_err(debug)?
        };
        owner.clock_origin -= Duration::from_secs(1);
        owner.drain_monster_melee().await;
        let returned = deps
            .runtime
            .lock()
            .await
            .read_actor_position(rat)
            .map_err(debug)?;
        assert_eq!(
            returned.position(),
            start,
            "the real think leashes to the authoritative spawn"
        );
        assert_eq!(returned.revision(), displaced.revision() + 1);
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .due(rat),
            None
        );
        Ok(())
    })
}
