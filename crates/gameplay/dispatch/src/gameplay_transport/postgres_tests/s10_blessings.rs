//! Source-bound bank population through NPC_TALK and the fenced bank writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn s10_blessings_postgres_nonpremium_and_phoenix_stages() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(2, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 blessings PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 blessings duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn s10_blessings_enforced_pvp_postgres_refuses_purchase() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(3, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 enforced PvP PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 blessings_enforced_pvp duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn s10_twist_postgres_accepts_amulet_carried_in_backpack() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(2, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 amulet PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 twist_carried_amulet duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(world_type: i16, amulet: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "s10_blessings", true, false).await?;
        prepare_current_session(&h).await?;
        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,20,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'s10-blessing-fixture')")
            .bind(id(42).as_slice()).bind(world_type).execute(&h.pool).await?;

        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let mut deps = Dependencies::with_content(
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
        )?;
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            5,
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            ),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        deps.admitted.runtime_actor = Some(deps.actor);
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        let (_, clock, _, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Arc::default(),
                Some(clock),
                None,
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), deps.admitted.game_session_id);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
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
            let rows = ["kawill", "norf", "pydar", "quentin"]
                .iter()
                .enumerate()
                .map(|(i, key)| NpcPlacements {
                    key: format!("oteryn:npc.{key}"),
                    placements: vec![NpcPlacement {
                        x: (at.x + 1 + (i % 2) as i32) as u16,
                        y: (at.y + (i / 2) as i32) as u16,
                        floor: -7,
                        direction: MovementFacing::South,
                    }],
                })
                .collect();
            runtime
                .place_npcs(&NpcPlacementTable { npcs: rows }, |_| true, true)
                .map_err(debug)?
                .actors
        };
        let mut items = vec![("oteryn:item.tibia.i3043", 3)];
        if amulet {
            items.push(("oteryn:item.tibia.i3057", 1));
        }
        super::item_look::seed_main_backpack(&h, &items).await?;
        let mut command = 1;
        // Twist requires another blessing or a carried amulet; Pydar requires Kawill.
        for (index, topic, expected_paid, expected_stage) in [
            (3, "blessing", i64::from(amulet), 0),
            (2, "blessing", i64::from(amulet), 0),
            (1, "blessing", 1 + i64::from(amulet), 0),
            (3, "blessing", 2, 0),
            (0, "blessing", 2, 1),
            (2, "blessing", 3, 1),
        ] {
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let actor = actors[index];
            let npc = EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            };
            for text in ["hi", topic, "yes", "bye"] {
                super::npc_paid_travel::talk(&owner, &deps, npc, command, text).await?;
                command += 1;
            }
            let paid: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_npc_purchase_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            let stage: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_npc_stage_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            if world_type == 3 && index == 1 {
                assert_eq!(paid, 0, "Enforced PvP cannot sell blessings");
                assert_eq!(stage, 0);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if world_type == 3 {
                continue;
            }
            assert_eq!(paid, expected_paid, "NPC index={index}");
            assert_eq!(stage, expected_stage, "NPC index={index}");
        }
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "replay fence")?
            .ok_or("current replay fence")?
            .item_fence();
        let replay = owner
            .resume_npc_bank(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: session,
                    command_id: 19,
                    item_fence: Some(fence),
                },
            )
            .await
            .ok_or("retained Kawill preparation")?;
        assert!(
            !replay.inventory_changed,
            "free replay must not move inventory"
        );
        let active: bool =
            sqlx::query_scalar("SELECT active FROM game_character_phoenix_prerequisites")
                .fetch_one(&h.pool)
                .await?;
        assert!(!active, "Pydar must consume Kawill's preparation");
        let held: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings")
            .fetch_one(&h.pool)
            .await?;
        let fees: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=3")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!((held, fees), (3, 3));
        // Crystal kawill.lua:49-54 checks only its preparation flag, not held Spark.
        // After Pydar consumes the flag, a fresh command may prepare the next visit.
        for expected_stages in [2_i64, 2_i64] {
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let kawill = EntityRef {
                identity: actors[0].placement_identity(),
                generation: actors[0].actor_local_generation(),
            };
            for text in ["hi", "blessing", "yes", "bye"] {
                let response =
                    super::npc_paid_travel::talk(&owner, &deps, kawill, command, text).await?;
                assert!(
                    !response.inventory_changed,
                    "free preparation cannot move inventory"
                );
                command += 1;
            }
            let actual: (bool, i64, i64) = sqlx::query_as("SELECT active,(SELECT count(*) FROM game_character_npc_stage_receipts),(SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=3) FROM game_character_phoenix_prerequisites")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                actual,
                (true, expected_stages, 3),
                "fresh Kawill preparation is legal while Spark is held; active preparation cannot duplicate"
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
