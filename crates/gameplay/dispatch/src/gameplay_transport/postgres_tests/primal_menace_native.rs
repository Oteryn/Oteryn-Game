//! The real source profile and native physical owner, not a synthetic wave receipt.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::primal_menace::{spawn_creature, spawn_fungosaurus_from_pod, tick};
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think as rules;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn primal_menace_postgres_native_cadence_spawns_and_retires_actual_beasts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("primal-native-cadence", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "primal_native_cadence", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let world = super::fixture::fence(1)?.runtime_scope.world_id();
            let source = serde_json::to_vec(&serde_json::json!({
                "schema": "OTERYN_NATIVE_SOURCE_REGION_BUNDLE/v1",
                "channel": include_str!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"),
                "primal": include_str!("../../../../../../tools/content-schema/native-gameplay/crystal-primal-region.json"),
            }))?;
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world,
                    Some(&source),
                );
            let native = controller
                .active()
                .and_then(|a| a.native_gameplay())
                .ok_or("active native source")?;
            let qualified_room = pin.qualified_room().clone();
            let channel_pin = pin.into_channel_pin();
            let deps = Dependencies::with_content_and_capacity(
                channel_pin,
                qualified_room,
                native.spell_book().clone(),
                64,
            )?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = controller.active();
            // The generic fixture closes combat by default. This is the actual
            // qualified Source room retained from the consumed activation pin,
            // following the existing boss-activity Source composition.
            assert!(matches!(
                deps.room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Source(_)
            ));
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&deps.room), Some(&deps.room));
            deps.initialize(&owner).await;
            let room = owner
                .rooms
                .gate_monster_ai()
                .ok_or("current native combat room")?;
            let content = owner
                .active_generation
                .and_then(|a| a.native_gameplay())
                .ok_or("native gameplay")?;
            assert!(
                crate::content::bosses::primal_closure::qualify(content).is_some(),
                "source closure must precede a native encounter"
            );
            let mut runtime = owner.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let center = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let candidate = crate::content::bosses::activity_template::captured_boss_levers()
                .map_err(debug)?
                .into_iter()
                .find(|c| c.creature_key.as_deref() == Some(rules::PRIMAL_MENACE_KEY))
                .ok_or("pinned Primal lever source")?;
            let template = controller
                .qualify_activity_template(&deps.room, &deps.room, "content-1", &candidate)
                .map_err(|error| format!("source Primal room qualification: {error:?}"))?;
            let cell = template.boss_position();
            let position = MovementLocalPosition {
                x: cell.x,
                y: cell.y,
                floor: i16::try_from(cell.z)?,
            };
            let boss = spawn_creature(&owner, &mut runtime, rules::PRIMAL_MENACE_KEY, position)
                .ok_or("source Primal admission")?;
            tick(&owner, &mut runtime, room, content, 0);
            tick(&owner, &mut runtime, room, content, 10_001);
            assert_eq!(
                runtime
                    .positioned_melee_census()
                    .map_err(debug)?
                    .iter()
                    .filter(|(actor, _, _)| runtime
                        .companion_snapshot(*actor)
                        .is_ok_and(|s| rules::PRIMAL_BEAST_POOL
                            .contains(&s.state.policy.definition_key.as_str())))
                    .count(),
                0,
                "the source three-second warmup is physical"
            );
            tick(&owner, &mut runtime, room, content, 13_001);
            let beasts: Vec<_> = runtime
                .positioned_melee_census()
                .map_err(debug)?
                .into_iter()
                .filter_map(|(actor, _, _)| {
                    runtime
                        .companion_snapshot(actor)
                        .ok()
                        .filter(|s| {
                            rules::PRIMAL_BEAST_POOL
                                .contains(&s.state.policy.definition_key.as_str())
                        })
                        .map(|_| actor)
                })
                .collect();
            assert!(
                !beasts.is_empty(),
                "scheduled source beasts must really occupy native slots"
            );
            let count = runtime.positioned_melee_census().map_err(debug)?.len();
            tick(&owner, &mut runtime, room, content, 13_001);
            assert_eq!(
                runtime.positioned_melee_census().map_err(debug)?.len(),
                count,
                "a repeated owner turn cannot duplicate spawns"
            );
            tick(&owner, &mut runtime, room, content, 33_002);
            for beast in beasts {
                assert!(
                    runtime.live_creature_health(beast).is_none(),
                    "the exact expired generation retires"
                );
            }
            assert!(
                runtime
                    .positioned_melee_census()
                    .map_err(debug)?
                    .iter()
                    .any(
                        |(actor, _, _)| runtime.companion_snapshot(*actor).is_ok_and(|s| s
                            .state
                            .policy
                            .definition_key
                            == rules::FUNGOSAURUS_KEY)
                    ),
                "expired source beasts become real Fungosaurus actors"
            );
            assert!(runtime.live_creature_health(boss).is_some());
            assert!(!runtime.native_summon_cell_free(center));
            let forced = spawn_fungosaurus_from_pod(&owner, &mut runtime, center)
                .ok_or("source forced pod spawn on occupied tile")?;
            assert_eq!(
                runtime
                    .read_actor_position(forced)
                    .map_err(debug)?
                    .position(),
                center
            );
            assert!(
                spawn_fungosaurus_from_pod(
                    &owner,
                    &mut runtime,
                    MovementLocalPosition {
                        x: i32::MAX,
                        y: i32::MAX,
                        floor: center.floor
                    }
                )
                .is_none(),
                "force cannot invent an unknown source tile"
            );
            Ok(())
        })
    })
}
