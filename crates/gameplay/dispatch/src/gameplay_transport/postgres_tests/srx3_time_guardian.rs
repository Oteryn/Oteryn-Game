//! Donor actions_the_time_guardian.lua:7-9 supplies three distinct actual actors.
//! The spell swaps their positions; replacing one actor's profile is insufficient.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const ARENA: MovementLocalPosition = MovementLocalPosition {
    x: 32977,
    y: 31662,
    floor: 14,
};
const FIRE: MovementLocalPosition = MovementLocalPosition {
    x: 32980,
    y: 31664,
    floor: 13,
};
const ICE: MovementLocalPosition = MovementLocalPosition {
    x: 32975,
    y: 31664,
    floor: 13,
};

#[test]
fn srx3_time_guardian_actual_native_callback_swaps_distinct_source_actors_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-time-guardian", || {
        run(async |admin| {
            reset();
            let kind = PinnedActivitySourceRoom::CallbackTimeGuardian;
            let scene = SourceScene::new(
                kind,
                MovementLocalPosition {
                    x: 32977,
                    y: 31667,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual native generation")?;
            let h = Harness::create_with_admission_seed(admin, "srx3_time_guardian", false, false)
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
            install_scene(&mut owner, &scene, kind)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let (base, fire, ice) = {
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                // Source action's exact three literals. This is initial source-roster
                // setup, never a fake transformed actor, native proposal or HP receipt.
                let base = runtime
                    .spawn_arena_opponent("oteryn:creature.the_time_guardian", ARENA, |_, _| {
                        Some(true)
                    })
                    .map_err(debug)?;
                let fire = runtime
                    .spawn_arena_opponent(
                        "oteryn:creature.the_blazing_time_guardian",
                        FIRE,
                        |_, _| Some(true),
                    )
                    .map_err(debug)?;
                let ice = runtime
                    .spawn_arena_opponent(
                        "oteryn:creature.the_freezing_time_guardian",
                        ICE,
                        |_, _| Some(true),
                    )
                    .map_err(debug)?;
                assert_ne!(base, fire);
                assert_ne!(base, ice);
                assert_ne!(fire, ice);
                runtime
                    .set_test_creature_health(base, 12_345)
                    .map_err(debug)?;
                (base, fire, ice)
            };
            let mut selected = false;
            for _ in 0..128 {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime.contains_live_creature(base),
                    "source base remains a live distinct actor"
                );
                let states = owner.states.spell_states.lock().await;
                selected = states
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(base)
                    .is_some_and(|report| {
                        report.proposals.iter().any(|proposal| {
                            proposal.ability.key == "oteryn:ability.spell.time_guardian"
                        })
                    });
                if selected {
                    break;
                }
            }
            assert!(
                selected,
                "actual native defense schedule selects source time guardian"
            );
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .matches_live_creature_identity(base, b"oteryn:creature.the_time_guardian")
                );
                assert!(runtime.matches_live_creature_identity(
                    fire,
                    b"oteryn:creature.the_blazing_time_guardian"
                ));
                assert!(runtime.matches_live_creature_identity(
                    ice,
                    b"oteryn:creature.the_freezing_time_guardian"
                ));
                let parked = runtime.read_actor_position(base).map_err(debug)?.position();
                assert!(
                    parked == FIRE || parked == ICE,
                    "source callback moves the original base actor into its selected occupied parking cell"
                );
                let phase = if parked == FIRE { fire } else { ice };
                let callback_origin = owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .native_callbacks
                    .time_guardian_cast_position_for_test(base)
                    .ok_or("retained actual Lua monsterPos/oldpos argument")?;
                assert_eq!(
                    runtime
                        .read_actor_position(phase)
                        .map_err(debug)?
                        .position(),
                    callback_origin,
                    "the original phase actor enters the exact current monsterPos retained by the source callback"
                );
                assert_eq!(
                    runtime.live_creature_health(phase),
                    runtime.live_creature_health(base),
                    "time_guardian synchronizes current HP in both directions"
                );
                assert_eq!(
                    runtime.current_top_actor_on_cell(parked).map_err(debug)?,
                    Some(base)
                );
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(30))
                .ok_or("return clock")?;
            owner.drain_monster_melee().await;
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .matches_live_creature_identity(base, b"oteryn:creature.the_time_guardian")
                );
                assert_eq!(
                    runtime
                        .read_actor_position(base)
                        .map_err(debug)?
                        .position()
                        .floor,
                    14,
                    "retained thirty-second callback returns the original base actor"
                );
                assert_eq!(
                    runtime.read_actor_position(fire).map_err(debug)?.position(),
                    FIRE
                );
                assert_eq!(
                    runtime.read_actor_position(ice).map_err(debug)?.position(),
                    ICE
                );
            }
            Ok(())
        })
    })?;
    eprintln!("SRX3_TIME_GUARDIAN_NATIVE_EXECUTED {:?}", started.elapsed());
    Ok(())
}
