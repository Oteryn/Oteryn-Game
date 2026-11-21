//! Buddel’s quest guards, fare and arrival through the production NPC_TALK owner.
use super::fixture::{Harness, debug, id, run};
use super::npc_paid_travel_fixtures::{talk, travel_world};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NativeEntryStart, NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn buddel_postgres_dispatch_quest_guards_and_paid_helheim_arrival() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Buddel PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "buddel_travel", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let bundle = travel_world(world, &[(32462, 31174, -7)])?;
        let start = bundle.start();
        let base = pin.qualified_room().clone();
        let cells = bundle
            .movement_cells(base.movement_cells())
            .map_err(debug)?;
        let room = base
            .bundle_view(
                cells,
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: start.x.into(),
                    y: start.y.into(),
                    floor: 6,
                },
            )
            .ok_or("bundle room")?;
        let deps = Dependencies::with_content_and_capacity(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                (start.x.into(), start.y.into(), 6),
            ),
            base,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            2,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = room.movement_cells().into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let placed = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.buddel".into(),
                            placements: vec![NpcPlacement {
                                x: start.x + 1,
                                y: start.y,
                                floor: start.floor,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let actor = placed.actors.first().ok_or("Buddel actor")?;
            EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            }
        };
        super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)])
            .await?;
        assert!(owner.ensure_source_map_initialized().await);
        talk(&owner, &deps, npc, 1, "hi").await?;
        let mut command = 2;
        for (barbarian, ice_islands) in [(7, 30), (9, 30), (8, 29)] {
            seed_questlines(&h, barbarian, ice_islands).await?;
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let refusal = talk(&owner, &deps, npc, command, "helheim").await?;
            let lines = refusal
                .conversation
                .ok_or("Buddel refusal")?
                .lines
                .join(" ");
            assert!(
                lines.contains("don't sail there"),
                "Buddel guard ({barbarian}, {ice_islands}): {lines}"
            );
            assert!(
                !talk(&owner, &deps, npc, command + 1, "yes")
                    .await?
                    .inventory_changed
            );
            command += 2;
        }
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, 0);
        assert_eq!(carried_gold(&h).await?, 100);

        seed_questlines(&h, 8, 30).await?;
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let quote = talk(&owner, &deps, npc, command, "helheim").await?;
        let lines = quote.conversation.ok_or("Buddel quote")?.lines.join(" ");
        assert!(lines.contains("50 gold"), "Buddel quote: {lines}");
        let arrival = talk(&owner, &deps, npc, command + 1, "yes").await?;
        assert!(arrival.inventory_changed);
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!((position.x, position.y, position.floor), (32462, 31174, 7));
        let receipt: (i64, i64, bool) = sqlx::query_as(
            "SELECT fare_gold_units,base_fare_gold_units,arrived_at IS NOT NULL
             FROM game_newhaven_travel_receipts WHERE route='oteryn:service.travel.buddel/helheim'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipt, (50, 50, true));
        assert_eq!(carried_gold(&h).await?, 50);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED Buddel refused quest values (7,30), (9,30), (8,29); paid 50 gold and arrived at Helheim (32462,31174,7)"
        );
        Ok(())
    })
}

async fn seed_questlines(h: &Harness, barbarian: i32, ice_islands: i32) -> TestResult {
    use oteryn_gameplay_quests::quest::{barbarian_mead, ice_islands_seals};
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in [
        (barbarian_mead::TRACKS[0], barbarian),
        (ice_islands_seals::QUESTLINE, ice_islands),
    ] {
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
             VALUES(encode($1,'hex')::uuid,$2,$3,$4,4)
             ON CONFLICT (character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
        )
        .bind(id(41).as_slice())
        .bind(track)
        .bind(ice_islands_seals::QUEST)
        .bind(value)
        .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn carried_gold(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_container_entries e
         JOIN game_item_instances i USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3031'",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
