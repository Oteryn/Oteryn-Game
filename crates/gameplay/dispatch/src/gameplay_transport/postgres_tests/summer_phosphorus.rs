//! W16: the admitted source curse through the actual scheduled monster dispatcher.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn summer_phosphorus_curse_postgres_real_dispatch() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "summer_phosphorus", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 11, -7),
        );
        let mut deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.combat_room = Some(deps.room.clone());
        {
            let mut runtime = deps.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            // Test-only placement of the exact admitted final form exercises its spell;
            // it supplies no production encounter admission or normal-to-final transition.
            runtime
                .admit_source_pinned_lab_creature(
                    MovementLocalPosition {
                        x: 2,
                        y: 12,
                        floor: -7,
                    },
                    "oteryn:creature.phosphorus_final",
                    90_000,
                )
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells();
        owner.clock_origin -= std::time::Duration::from_secs(12);
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .try_lock()
                .map_err(debug)?
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        max_health: 100_000,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("initialize current player")?;
        }
        owner.drain_monster_melee().await;
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.try_lock().map_err(debug)?;
        let state = states
            .get(&runtime, deps.actor, deps.admitted.game_session_id)
            .ok_or("current player after scheduled spell")?;
        let curse = state
            .owned_conditions()
            .instances()
            .iter()
            .find(|condition| {
                condition.definition().key()
                    == "oteryn:effect.spell.phosphorus_curse.effect-condition-1"
            })
            .ok_or("scheduled dispatcher installed the exact source curse")?;
        let Some(crate::foundation::condition::DamageSchedule::Fixed { delayed, segments }) =
            curse.definition().source_damage_schedule()
        else {
            return Err("source damage schedule".into());
        };
        assert!(*delayed);
        let expected = oteryn_gameplay_encounters::bosses::summer_2026::PHOSPHORUS_CURSE_TICKS;
        assert_eq!(segments.len(), expected.len());
        for (segment, amount) in segments.iter().zip(expected) {
            assert_eq!(segment.count, 1);
            assert_eq!(segment.interval_ms, 2_000);
            assert_eq!(segment.amount, amount);
        }
        eprintln!("W16-PHOSPHORUS-PG: real scheduled dispatcher installed all 21 source ticks");
        Ok(())
    })
}
