//! S10: real framed shop commands and durable payment after Rashid wanders one tile.
use super::TestResult;

#[test]
fn s10_rashid_wandering_postgres_shop_payment_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::npc_shop::run_shop_scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 Rashid PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 rashid_wandering_shop duration={:?}",
        started.elapsed(),
    );
    Ok(())
}

#[test]
fn s10_weekday_rollover_moves_live_rashid_on_postgres() -> TestResult {
    use super::fixture::{Harness, debug, run};
    use super::{Dependencies, prepare_current_session};
    use crate::foundation::{MovementLocalPosition, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use crate::gameplay_transport::npc_voice::NpcVoiceCycle;
    use oteryn_gameplay_entities::rashid_schedule::{RASHID_NPC_KEY, destination_at};
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "s10_rashid_rollover", false, false)
                        .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                deps.initialize(&owner).await;
                let session = deps.admitted.game_session_id;
                owner.states.chat.install_npc_talk(std::sync::Arc::new(
                    NpcTalkBook::default().with(RASHID_NPC_KEY, NpcTalkEntry::new("Rashid", None)),
                ));
                owner
                    .states
                    .chat
                    .admit(session, "Fixture Hero", owner.owner_now());
                owner.states.chat.set_npc_service(session);
                let now = crate::gameplay_transport::npc_trade_command::unix_ms_now();
                let yesterday = destination_at(now - 86_400_000).placement;
                let expected = destination_at(now).placement;
                let mut cycle = {
                    let mut runtime = deps.runtime.lock().await;
                    runtime
                        .initialize_pinned_test_position(
                            deps.actor,
                            MovementLocalPosition {
                                x: i32::from(yesterday.x) + 1,
                                y: i32::from(yesterday.y),
                                floor: -i16::from(yesterday.floor),
                            },
                        )
                        .map_err(debug)?;
                    runtime
                        .place_npcs(
                            &NpcPlacementTable {
                                npcs: vec![NpcPlacements {
                                    key: RASHID_NPC_KEY.into(),
                                    placements: vec![yesterday],
                                }],
                            },
                            |_| true,
                            true,
                        )
                        .map_err(debug)?;
                    NpcVoiceCycle::new(&runtime, &NpcTalkBook::default())
                };
                let npc = owner.runtime.lock().await.visible_entities().npcs[0];
                super::npc_paid_travel::talk(
                    &owner,
                    &deps,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    },
                    1,
                    "hi",
                )
                .await?;
                assert_eq!(owner.states.chat.engaged(session), Some(npc.actor));
                owner.emit_npc_voices(&mut cycle, 1_000_000).await;
                assert_eq!(owner.states.chat.engaged(session), None);
                assert!(!owner.states.chat.take_walked_away(session).is_empty());
                {
                    let runtime = deps.runtime.lock().await;
                    let actual = runtime.visible_entities().npcs[0].position;
                    assert_eq!(
                        (actual.x, actual.y, actual.floor),
                        (
                            i32::from(expected.x),
                            i32::from(expected.y),
                            -i16::from(expected.floor)
                        )
                    );
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 weekday PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 weekday_rollover duration={:?}",
        started.elapsed()
    );
    Ok(())
}
