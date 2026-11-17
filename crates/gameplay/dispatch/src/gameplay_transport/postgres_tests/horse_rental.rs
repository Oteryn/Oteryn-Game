use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_optional_cosmetics;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::appearance as wire;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn horse_rental_postgres_npc_charges_grants_and_expires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("horse-rental", scenario)?;
    println!(
        "PASS horse_rental_postgres_npc_charges_grants_and_expires elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        // Compile content before opening the admission holder; cold Native setup can exceed
        // the holder's idle lifetime on a contended CI worker.
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut cosmetics: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/cosmetics/outfits/native-cosmetics.json"
            ))?)?;
        // Keep the real rental mount bindings under the admission fixture's revision.
        cosmetics["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&cosmetics)?));
        let h = Harness::create_with_admission_seed(admin, "horse_rental", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
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
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
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
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
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
            runtime
                .bind_attacker_lease(
                    dependencies.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.palomino".into(),
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
        // Keep the purse in a nested container to exercise carried coin custody.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{}{}{}",
            crate::durability::item_fee_burn::test_support::item(
                70,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                71,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                72,
                "oteryn:item.tibia.i3035",
                "definition-r1",
                5
            ),
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(71).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "current session fence")?
            .ok_or("active fence")?
            .item_fence();
        use sha2::{Digest, Sha256};
        let confirm = (3..100)
            .find(|command| {
                let occurrence = crate::gameplay_transport::npc_trade_command::derived(
                    b"oteryn:npc-horse-rental:v1:0",
                    session,
                    *command,
                );
                Sha256::new()
                    .chain_update(b"oteryn:horse-rental:v1")
                    .chain_update(occurrence)
                    .finalize()[1]
                    % 10
                    < 3
            })
            .ok_or("escape roll")?;
        // Greetings, prompt, and confirmation are all real NPC_TALK dispatches.
        for (command_id, text) in [(1, "hi"), (2, "rent"), (confirm, "yes")] {
            let outcome = owner
                .npc_talk(
                    dependencies.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id,
                        item_fence: Some(fence),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert!(!outcome.unknown, "{text}: unknown outcome");
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
            if text == "yes" {
                assert!(
                    outcome.inventory_changed,
                    "disposition={:?}",
                    outcome.disposition
                );
            }
        }
        let replay = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: confirm,
                    item_fence: Some(fence),
                },
            )
            .await
            .ok_or("committed rental replay")?;
        assert!(replay.inventory_changed);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_horse_rental_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(receipts, 1);
        let (price, duration): (i64, i64) = sqlx::query_as(
            "SELECT f.fee_gold_units::bigint, r.expires_at_ms-r.rented_at_ms
             FROM game_horse_rental_receipts r JOIN game_item_fee_burns f ON f.transaction_id=r.fee_transaction_id",
        ).fetch_one(&h.pool).await?;
        assert_eq!((price, duration), (500, 86_400_000));
        let loose_duration: i64 = sqlx::query_scalar(
            "SELECT s.loose_until_ms-r.rented_at_ms FROM game_horse_stations s
             JOIN game_horse_rental_receipts r USING(world_id)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(loose_duration, 4 * 86_400_000);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("appearance fence")?;
        let appearance = h
            .root
            .read_current_character_appearance_state(&authority, &h.node, fence, Vec::new())
            .await
            .map_err(debug)?;
        let mount = appearance.rental_mount.ok_or("rental entitlement")?;
        let (context, cosmetics) = owner.appearance_context().ok_or("native cosmetics")?;
        let (look, _, _) = cosmetics
            .mount_metadata(mount)
            .ok_or("rental mount binding")?;
        let starter = cosmetics.fallback(appearance.sex);
        let mounted = owner
            .appearance_change(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 101,
                    item_fence: Some(fence.item_fence()),
                },
                wire::Change {
                    context,
                    expected_root_revision: fence.expected_character_revision.get(),
                    prior: None,
                    selection: wire::Selection {
                        outfit_key: starter.outfit_key,
                        colours: starter.colours,
                        addons: starter.addons,
                        mount_key: Some(mount.into()),
                    },
                    content_revision: cosmetics.content_revision().into(),
                },
            )
            .await;
        assert!(
            matches!(mounted, Some(wire::ChangeResult::Committed(_))),
            "{mounted:?}"
        );
        assert_eq!(
            owner.current_mount(dependencies.actor, session).await,
            Some(look)
        );
        sqlx::query("UPDATE game_horse_rental_receipts SET rented_at_ms=1,expires_at_ms=86400001")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            owner.current_mount(dependencies.actor, session).await,
            Some(0)
        );
        Ok(())
    })
}
