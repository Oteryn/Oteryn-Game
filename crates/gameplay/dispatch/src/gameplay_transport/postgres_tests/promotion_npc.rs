//! PREM-2b+PREM-5 through command 7's real composed dispatch and runtime-role PostgreSQL.
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
fn promotion_npc_postgres_dispatch_purchase_replay_bank_login_and_death() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "promotion PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "promotion_npc", true, false).await?;
        eprintln!("POSTGRES_STARTED PREM-2b+PREM-5 promotion_npc version=17.6");
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
        let (consumer, clock, activation, proof) = purchase_fixture::current(id(40));
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
            .states.chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = dependencies.admitted.game_session_id;
        owner.states.chat.admit(session, "Fixture Hero", owner.owner_now());
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
                            key: "oteryn:npc.eremo".into(),
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
        // Greetings, prompt, and confirmation are all real NPC_TALK dispatches.
        for (command_id, text) in [(1, "hi"), (2, "promotion"), (3, "yes")] {
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
        let promoted = owner
            .root
            .read_promotion_for_session(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        assert!(promoted);
        let replay = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 3,
                    item_fence: Some(fence),
                },
            )
            .await
            .ok_or("retained purchase")?;
        assert!(replay.inventory_changed);
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_npc_purchase_receipts),(SELECT count(*) FROM game_item_fee_burns)").fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1));
        let coins:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(coins, 3);
        // Before delivery the promotion exception is available, while blessing stays closed.
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (command_id, text) in [(4, "blessing"), (5, "yes")] {
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
        }
        // Activation is the production latch owner; the second coordinator reads the same row.
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let current = SpellPremiumCoordinator::new(
            consumer.clone(),
            Some(purchase_fixture::current(id(40)).1),
            Some(activation),
            Some(h.root.clone()),
        );
        owner.premium_coordinator = Some(&current);
        owner.admit_promotion(session).await;
        assert_eq!(
            owner
                .states.premium_sessions
                .lock()
                .map_err(debug)?
                .1
                .get(&session),
            Some(&true)
        );
        // Display was admitted once; losing entitlement cannot erase permanent promotion.
        consumer.release(id(40));
        assert!(
            owner
                .root
                .read_promotion_for_session(&authority, &h.node, fence)
                .await
                .map_err(debug)?
        );
        consumer.prove_for_test(proof);
        // Fund the Bank through its existing production deposit writer before unequipping.
        let (gold, platinum, crystal) = {
            let runtime = dependencies.runtime.lock().await;
            crate::gameplay_transport::npc_command::bank_coin_definitions(&owner, &runtime)
                .ok_or("coin definitions")?
        };
        let backpack = owner
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
            active.native_gameplay().ok_or("native")?,
            &backpack.backpack.definition,
        )
        .map_err(debug)?;
        let request = crate::gameplay_transport::npc_command::bank_request(
            &crate::gameplay_transport::bank_npc::BankNpcOperation::Deposit(30_000),
            Some(crate::durability::bank::BankCoinFacts {
                gold,
                platinum,
                crystal,
                backpack: facts,
            }),
            session,
            10,
        )
        .ok_or("Bank request")?;
        let crate::durability::bank::BankRequest::Deposit(request) = request else {
            return Err("deposit request".into());
        };
        owner
            .root
            .bank_deposit(&authority, &h.node, fence, request)
            .await
            .map_err(debug)?;
        // No backpack is a supported bank-only fee. Its absence is transaction-bound.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
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
            assert!(!outcome.unknown, "disposition={:?}", outcome.disposition);
            if text == "yes" {
                assert!(
                    outcome.inventory_changed,
                    "disposition={:?}",
                    outcome.disposition
                );
            }
        }
        let paid:(String,i64)=sqlx::query_as("SELECT blessing_key,fee_gold_units FROM game_character_npc_purchase_receipts WHERE cause_kind=3").fetch_one(&h.pool).await?;
        assert_eq!(
            paid,
            ("oteryn:blessing.the_wisdom_of_solitude".into(), 20_000)
        );
        let bank: i64 = sqlx::query_scalar(
            "SELECT bank_debit_gold_units FROM game_item_fee_burns WHERE cause_kind=3",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(bank, 20_000);
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
            assert!(!outcome.inventory_changed);
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_npc_purchase_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 2);
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
        let (held,lost):(Vec<String>,i64)=sqlx::query_as("SELECT blessings_before,experience_lost FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(held, vec!["oteryn:blessing.the_wisdom_of_solitude"]);
        assert_eq!(
            u128::try_from(lost)?,
            crate::domain::death::death_experience_loss(120, 1, true).map_err(debug)?
        );
        let held_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(held_count, 0);

        // The after-delivery promotion purchase gate fails closed even for an existing buyer.
        consumer.release(id(40));
        // Historical death replay uses its committed held set after consumption and Premium lapse.
        let (replayed, _) = owner.commit_death(session, &progression, death).await;
        assert_eq!(replayed, crate::gameplay_transport::DeathCommit::Settled);
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (command_id, text) in [(10, "promotion"), (11, "yes")] {
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
            if command_id == 11 {
                assert_eq!(
                    outcome.conversation.ok_or("Premium refusal reply")?.lines,
                    ["You need current Premium for this service."]
                );
            }
        }
        assert_eq!(
            owner
                .states.premium_sessions
                .lock()
                .map_err(debug)?
                .1
                .get(&session),
            Some(&true)
        );

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
