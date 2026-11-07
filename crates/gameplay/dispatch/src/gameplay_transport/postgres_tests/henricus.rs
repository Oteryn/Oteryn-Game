//! BANK-3 through real NPC_TALK dispatch and runtime-role PostgreSQL.
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
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn henricus_postgres_dispatch_purchase_refusals_and_replay() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Henricus PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario(partial: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "henricus", true, false).await?;
        eprintln!("POSTGRES_STARTED BANK-3 henricus");
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
        let policy = active
            .native_gameplay()
            .and_then(|native| native.progression())
            .ok_or("production progression")?
            .policy_for("profile-1", "ruleset-1", "content-1")
            .map_err(debug)?;
        let progression =
            crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
        let initial_experience = progression.policy.thresholds[119].minimum_experience.get() + 10;

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
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.henricus".into(),
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
        // This scenario adds the purchase's level and purse to the existing admission fixture.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_progression_state SET level=120,total_experience=$2,simulation_revision=$3,evidence_revision=$4,declaration_revision=$5,policy_revision=$6,reward_revision=$7 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(initial_experience).bind(&progression.context.simulation).bind(&progression.context.evidence).bind(&progression.context.declaration).bind(&progression.policy_revision).bind(&progression.reward_revision).execute(&mut *tx).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{}{}",
            crate::durability::item_fee_burn::test_support::item(
                70,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                72,
                "oteryn:item.tibia.i3043",
                "definition-r1",
                5
            ),
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "current session fence")?
            .ok_or("active fence")?
            .item_fence();
        // A lapsed/unactivated Premium entitlement cannot gate the public Inquisition offer.
        consumer.release(id(40));
        for (command_id, text) in [(1, "hi"), (2, "blessing"), (3, "yes")] {
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
            assert!(!outcome.unknown, "{text}");
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
            assert!(!outcome.inventory_changed);
            if command_id == 3 {
                assert_eq!(
                    outcome.conversation.ok_or("quest refusal")?.lines,
                    ["You must complete The Inquisition Quest first."]
                );
            }
        }
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline','oteryn:quest.the_inquisition_quest',25,2)")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        if partial {
            sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'oteryn:blessing.the_spiritual_shielding','fixture')")
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        if partial {
            tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
            for (command_id, text) in [(4, "bless"), (5, "yes")] {
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
                assert!(!outcome.unknown);
                assert!(!outcome.inventory_changed);
                if command_id == 5 {
                    assert_eq!(
                        outcome
                            .conversation
                            .ok_or("partial ownership refusal")?
                            .lines,
                        [
                            "You already have a regular blessing. I can only provide the complete set."
                        ]
                    );
                }
            }
            let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_npc_purchase_receipts),(SELECT count(*) FROM game_item_fee_burns),(SELECT count(*) FROM game_character_blessings)").fetch_one(&h.pool).await?;
            assert_eq!(counts, (0, 0, 1));
            let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(72).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 5);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        // A confirmed insufficient-funds quote must leave both fee and grant absent.
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (command_id, text) in [(4, "bless"), (5, "yes")] {
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
            assert!(!outcome.unknown);
            assert!(!outcome.inventory_changed);
            if command_id == 5 {
                assert_eq!(
                    outcome.conversation.ok_or("funds refusal")?.lines,
                    ["You do not have enough gold for this service."]
                );
            }
        }
        let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_npc_purchase_receipts),(SELECT count(*) FROM game_item_fee_burns),(SELECT count(*) FROM game_character_blessings)")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (0, 0, i64::from(partial)));
        // Only fixture funding bypasses guards. Every purchase below uses the real writer.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET quantity=20 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(72).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (command_id, text) in [(6, "blessing"), (7, "yes")] {
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
            assert!(!outcome.unknown, "{text}: {:?}", outcome.disposition);
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
            assert_eq!(outcome.inventory_changed, command_id == 7);
        }
        let paid: (Vec<String>, i64) = sqlx::query_as(
            "SELECT blessing_keys,fee_gold_units FROM game_character_npc_purchase_receipts",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(paid.0.len(), 5);
        assert_eq!(paid.1, 110_000);
        let held: Vec<String> = sqlx::query_scalar(
            "SELECT blessing_key FROM game_character_blessings ORDER BY blessing_key",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(held.len(), 5);
        assert!(
            !held
                .iter()
                .any(|key| key.contains("mountain") || key.contains("twist"))
        );
        let replay = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 7,
                    item_fence: Some(fence),
                },
            )
            .await
            .ok_or("original command replay")?;
        assert!(replay.inventory_changed);
        // A fresh command refuses an already complete held set, including after replay.
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (command_id, text) in [(8, "blessing"), (9, "yes")] {
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
            assert!(!outcome.unknown);
            assert!(!outcome.inventory_changed);
            if command_id == 9 {
                assert_eq!(
                    outcome.conversation.ok_or("held refusal")?.lines,
                    ["You already have all five regular blessings."]
                );
            }
        }
        let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_npc_purchase_receipts),(SELECT count(*) FROM game_item_fee_burns),(SELECT count(*) FROM game_character_blessings)")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1, 5));
        let paid_coins: i64 = sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(paid_coins, paid.1);
        let remaining: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 200_000 - paid.1);
        let cell = dependencies
            .runtime
            .lock()
            .await
            .read_actor_position(dependencies.actor)
            .map_err(debug)?
            .position();
        let death = crate::gameplay_transport::actor_spell::PlayerDeath {
            occurrence: crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(
                220,
            ))
            .map_err(debug)?,
            cell,
        };
        let (settled, _) = owner.commit_death(session, &progression, death).await;
        assert_eq!(settled, crate::gameplay_transport::DeathCommit::Settled);
        let after_death: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(after_death, 0);
        // Retry returns the original result after the effects have legitimately been consumed.
        let replay = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 7,
                    item_fence: Some(fence),
                },
            )
            .await
            .ok_or("replay after death")?;
        assert!(replay.inventory_changed);
        let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_npc_purchase_receipts),(SELECT count(*) FROM game_item_fee_burns),(SELECT count(*) FROM game_character_blessings)").fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1, 0));
        // Reopen authority to execute production recovery verification against the new receipt.
        drop(owner);
        drop(authority);
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        drop(reopened);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn henricus_postgres_dispatch_partial_ownership_refuses_without_charge() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Henricus partial ownership PostgreSQL thread panicked")??;
    Ok(())
}
