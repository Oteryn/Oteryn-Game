//! CREATURE-MOVE-1 through the real composed Channel dispatch and existing PostgreSQL harness.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;

#[test]
fn creature_movement_dispatch_steps_between_thinks_on_postgres() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "creature_move_dispatch", false, false)
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
                    key: "canary:creature/rat".into(),
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
            4,
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
        owner.movement_cells = &cells;
        owner.clock_origin = std::time::Instant::now();
        deps.initialize(&owner).await;
        owner.clock_origin -= Duration::from_secs(10);
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
