use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CharacterId, CharacterLease, MovementLocalPosition, WorldId};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;

#[test]
fn familiar_defense_advances_with_the_postgres_holder_checked_out() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("familiar-defense-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "familiar_defense_holder", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = WorldId::decode(&super::id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    world,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native gameplay")?;
            let room = pin.qualified_room().clone();
            let (channel_pin, _, _, _) = pin.into_channel_parts();
            let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.clock_origin -= Duration::from_secs(3);
            let session = deps.admitted.game_session_id;
            let familiar = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        session,
                        CharacterLease::new(CharacterId::decode(&super::id(41)).map_err(debug)?, 2)
                            .map_err(debug)?,
                    )
                    .map_err(debug)?;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let position = runtime
                    .borrow_movement_position()
                    .read(deps.actor)
                    .map_err(debug)?
                    .position();
                let mut spawn = runtime
                    .prepare_companion_spawn(
                        deps.actor,
                        session,
                        "Knight Familiar",
                        MovementLocalPosition {
                            x: position.x + 1,
                            ..position
                        },
                        None,
                        None,
                        1,
                        0,
                        false,
                    )
                    .map_err(debug)?;
                spawn.bind_semantic_creation(0).map_err(debug)?;
                runtime.install_companion_spawn(spawn).map_err(debug)?
            };
            deps.initialize(&owner).await;
            let held = h.root.try_issue_semantic_pass()?;
            owner.tick_familiar_defenses(deps.actor, session);
            let runtime = deps.runtime.lock().await;
            let snapshot = runtime.companion_snapshot(familiar).map_err(debug)?;
            assert_eq!(
                snapshot.state.familiar_defense.map(|clock| clock.ordinal),
                Some(1)
            );
            drop(runtime);
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
