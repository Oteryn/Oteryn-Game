//! S10 banker bindings and held travel routes through the NPC talk owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn s10_banker_service_reachable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            scenario("s10_banker_service_reachable", &["adrian"]).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 bank PostgreSQL thread panicked")??;
    eprintln!(
        "s10npc-marker: s10_banker_service_reachable duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn s10_cheese_blocks_chemar_and_petros_without_charging() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            scenario(
                "s10_cheese_blocks_chemar_and_petros_without_charging",
                &["chemar", "petros"],
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 travel PostgreSQL thread panicked")??;
    eprintln!(
        "s10npc-marker: s10_cheese_blocks_chemar_and_petros_without_charging duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(test: &'static str, npcs: &'static [&'static str]) -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, test, true, false).await?;
        prepare_current_session(&h).await?;
        eprintln!("s10npc-marker: {test} db ready");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let deps = Dependencies::with_content_and_capacity(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            ),
            pin.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            1 + npcs.len(),
        )?;
        use crate::gameplay_transport::spell_premium_coordinator::{
            SpellPremiumCoordinator, purchase_fixture,
        };
        let premium = if npcs == ["adrian"] {
            None
        } else {
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ))
        };
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        if let Some(premium) = premium.as_ref() {
            owner.premium_coordinator = Some(premium);
        }
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = Arc::new(NpcTalkBook::from_model(&model));
        owner.states.chat.install_npc_talk(Arc::clone(&book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let actors = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let placed = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: npcs
                            .iter()
                            .enumerate()
                            .map(|(index, stem)| {
                                Ok(NpcPlacements {
                                    key: format!("oteryn:npc.{stem}"),
                                    placements: vec![NpcPlacement {
                                        x: u16::try_from(at.x + i32::try_from(index)? + 1)?,
                                        y: u16::try_from(at.y)?,
                                        floor: -i8::try_from(at.floor)?,
                                        direction: MovementFacing::South,
                                    }],
                                })
                            })
                            .collect::<TestResult<Vec<_>>>()?,
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            placed.actors
        };
        let npc_ref = |stem: &str| -> TestResult<EntityRef> {
            let index = npcs
                .iter()
                .position(|key| *key == stem)
                .ok_or("NPC fixture key")?;
            let actor = actors.get(index).ok_or("NPC fixture actor")?;
            Ok(EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            })
        };
        if npcs == ["adrian"] {
            assert!(
                book.get("oteryn:npc.adrian")
                    .is_some_and(|entry| entry.bank())
            );
            verify_bank(&h, &mut owner, &deps, npc_ref("adrian")?).await?;
        } else {
            use super::npc_paid_travel_fixtures::talk;
            use oteryn_protocol_oteryn::npc_service::NpcConversationState;

            super::item_look_fixtures::seed_main_backpack(
                &h,
                &[
                    ("oteryn:item.tibia.i3035", 2),
                    ("oteryn:item.tibia.i169", 1),
                ],
            )
            .await?;
            let before = deps
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            for (stem, keyword, command, refusal) in [
                (
                    "chemar",
                    "edron",
                    1,
                    "Ugh... What is that stench? Cough! Cough! Not scarab cheese! Get out of here!",
                ),
                (
                    "petros",
                    "venore",
                    5,
                    "That cheese of yours stinks! Please leave!",
                ),
            ] {
                owner.clock_origin -= std::time::Duration::from_secs(1);
                let npc = npc_ref(stem)?;
                talk(&owner, &deps, npc, command, "hi").await?;
                let route = talk(&owner, &deps, npc, command + 1, keyword).await?;
                let reply = route.conversation.ok_or("Scarab Cheese refusal")?;
                assert_eq!(reply.lines, [refusal]);
                assert_eq!(reply.open, Some((npc, NpcConversationState::Open)));
                let outcome = talk(&owner, &deps, npc, command + 2, "yes").await?;
                assert!(!outcome.inventory_changed);
                talk(&owner, &deps, npc, command + 3, "bye").await?;
            }
            let after = deps
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            assert_eq!(after, before);
            let receipts: i64 = sqlx::query_scalar(
                "SELECT (SELECT count(*) FROM game_newhaven_travel_receipts) + (SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=8)",
            ).fetch_one(&h.pool).await?;
            assert_eq!(receipts, 0);
            // Move the test cheese out of carried custody, then exercise both
            // route-specific progress checks through the same dialogue owner.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let changed = sqlx::query(
                "DELETE FROM game_item_container_entries
                 WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(72).as_slice())
            .execute(&mut *tx)
            .await?;
            assert_eq!(changed.rows_affected(), 1);
            tx.commit().await?;

            let chem = npc_ref("chemar")?;
            owner.clock_origin -= std::time::Duration::from_secs(1);
            talk(&owner, &deps, chem, 20, "hi").await?;
            for (command, keyword, fare) in [
                (21, "edron", 40),
                (22, "hills", 60),
                (23, "issavi", 100),
                (24, "kazor", 80),
                (25, "svargrond", 60),
            ] {
                owner.clock_origin -= std::time::Duration::from_secs(1);
                let quote = talk(&owner, &deps, chem, command, keyword).await?;
                let reply = quote.conversation.ok_or("carpet quote")?;
                assert_eq!(reply.open, Some((chem, NpcConversationState::Confirming)));
                assert!(
                    reply
                        .lines
                        .iter()
                        .any(|line| line.ends_with(&format!("for {fare} gold?")))
                );
            }

            for (index, stem, keyword, track, quest, value, fare) in [
                (
                    0,
                    "chemar",
                    "zao",
                    "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10_1",
                    "oteryn:quest.the_new_frontier_quest",
                    2_i64,
                    60,
                ),
                (
                    1,
                    "petros",
                    "yalahar",
                    "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/searoutes_around_yalahar/darashia",
                    "oteryn:quest.in_service_of_yalahar_quest",
                    1_i64,
                    210,
                ),
            ] {
                owner.clock_origin -= std::time::Duration::from_secs(1);
                let npc = npc_ref(stem)?;
                let command = 30 + 5 * index;
                talk(&owner, &deps, npc, command, "hi").await?;
                let denied = talk(&owner, &deps, npc, command + 1, keyword).await?;
                assert_eq!(
                    denied.conversation.ok_or("quest refusal")?.open,
                    Some((npc, NpcConversationState::Open))
                );
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "INSERT INTO game_character_quest_tracks
                        (character_id, track_key, quest_key, value, committed_character_revision)
                     VALUES (encode($1,'hex')::uuid, $2, $3, $4, 2)",
                )
                .bind(id(41).as_slice())
                .bind(track)
                .bind(quest)
                .bind(value)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                let quote = talk(&owner, &deps, npc, command + 2, keyword).await?;
                let reply = quote.conversation.ok_or("quest route quote")?;
                assert_eq!(reply.open, Some((npc, NpcConversationState::Confirming)));
                assert!(
                    reply
                        .lines
                        .iter()
                        .any(|line| line.ends_with(&format!("for {fare} gold?")))
                );
                talk(&owner, &deps, npc, command + 3, "bye").await?;
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn verify_bank(
    h: &Harness,
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    npc: EntityRef,
) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 100)]).await?;
    for (command, text) in [(1, "hi"), (2, "deposit 20 gold"), (3, "yes")] {
        super::npc_paid_travel_fixtures::talk(owner, deps, npc, command, text).await?;
    }
    let balance = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(sum(balance),0)::bigint FROM game_account_bank_balances",
        )
        .fetch_one(&h.pool)
        .await
    };
    assert_eq!(balance().await?, 20);
    owner.clock_origin -= std::time::Duration::from_secs(1);
    for (command, text) in [(4, "withdraw 5 gold"), (5, "yes")] {
        super::npc_paid_travel_fixtures::talk(owner, deps, npc, command, text).await?;
    }
    assert_eq!(balance().await?, 15);
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(entries, 2);
    Ok(())
}
