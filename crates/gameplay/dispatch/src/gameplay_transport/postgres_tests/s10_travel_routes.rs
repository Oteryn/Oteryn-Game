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
fn s10_declared_routes_stay_held_without_charging() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            scenario(
                "s10_declared_routes_stay_held_without_charging",
                &["chemar", "petros"],
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 travel PostgreSQL thread panicked")??;
    eprintln!(
        "s10npc-marker: s10_declared_routes_stay_held_without_charging duration={:?}",
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
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
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

            super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i3035", 2)])
                .await?;
            let before = deps
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            for (stem, keyword, command) in [("chemar", "edron", 1), ("petros", "venore", 5)] {
                owner.clock_origin -= std::time::Duration::from_secs(1);
                let npc = npc_ref(stem)?;
                talk(&owner, &deps, npc, command, "hi").await?;
                let route = talk(&owner, &deps, npc, command + 1, keyword).await?;
                let reply = route.conversation.ok_or("held route reply")?;
                assert_eq!(reply.lines, ["I'm sorry but I don't sail there."]);
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
