//! Sandomo bookkeeping and quantity exchange through production NPC_TALK.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::npc_rules::sandomo::NPC;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn sandomo_postgres_dispatch_records_contributions_and_mints_requested_quantity() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Sandomo PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "sandomo", true, false).await?;
        eprintln!("POSTGRES_STARTED sandomo");
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
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let dependencies = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, _activation, _proof) = purchase_fixture::current(id(40));
        let coordinator =
            SpellPremiumCoordinator::new(consumer.clone(), Some(clock), None, Some(h.root.clone()));
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        let quest_state =
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?;
        let quest_catalogue = quest_state.catalogue();
        owner.quest_catalogue = Some(Arc::new(quest_catalogue.clone()));
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
                &model,
                &[NPC],
            )));
        let session = dependencies.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        dependencies
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                dependencies.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let quest = "oteryn:quest.roshamuul_quest";
        let character = fence.character_id.as_bytes();
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        // Historical quest writes follow character creation at revision 1.
        for advance in [
            "UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
            "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
        ] {
            let advanced = sqlx::query(advance)
                .bind(character.as_slice())
                .execute(&mut *tx)
                .await?;
            assert_eq!(advanced.rows_affected(), 1);
        }
        let revision = "2";
        // Seed historical repairs, with one contribution still left after two purchases.
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric)")
            .bind(character.as_slice()).bind(quest).bind(quest_catalogue.content_revision())
            .bind(quest_catalogue.definition_hash(quest).ok_or("quest hash")?.as_slice())
            .bind(revision).execute(&mut *tx).await?;
        for (key, value) in [
            ("oteryn:quest-progress/roshamuul/mortar", 2_i64),
            ("oteryn:quest-progress/roshamuul/silencers", 1),
        ] {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric)")
                .bind(character.as_slice()).bind(quest).bind(key).bind(value).bind(revision)
                .execute(&mut *tx).await?;
        }
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                238,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1,
            ),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(character.as_slice()).bind(id(238).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        owner.refresh_quest_session(session, true).await;
        for (command_id, message, expected) in [
            (1, "hi", ""),
            (2, "gratitude", "repairs"),
            (3, "repairs", "201"),
            (4, "books", "201"),
            (5, "yes", "201"),
            (6, "trade", "100"),
            (7, "3", "2 clusters"),
            (8, "2.0", "1 inquisition gold"),
        ] {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let current = owner
                .current_quest_fence(session)
                .await
                .map_err(debug)?
                .ok_or("fence")?;
            let outcome = owner
                .npc_talk(
                    dependencies.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id,
                        item_fence: Some(current.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: message.into(),
                    },
                )
                .await;
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{message}");
            assert_eq!(outcome.inventory_changed, command_id == 8, "{message}");
            assert!(
                outcome
                    .conversation
                    .is_some_and(|reply| reply.lines.iter().any(|line| line.contains(expected))),
                "{message}"
            );
        }
        let balance: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/roshamuul/gold-record'")
            .bind(character.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(balance, 1);
        let outstanding: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(value),0)::bigint FROM game_character_quest_tracks              WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2              AND track_key IN ('oteryn:quest-progress/roshamuul/mortar','oteryn:quest-progress/roshamuul/silencers')",
        )
        .bind(character.as_slice())
        .bind(quest)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(outstanding, 0);
        let minted: i64 = sqlx::query_scalar("SELECT coalesce(sum(m.quantity),0)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE m.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i20062'")
            .bind(character.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(minted, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
