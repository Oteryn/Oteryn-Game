//! Native Benevola conversation -> Quest receipt -> carried debit + shared counter, one PG tx.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::world_event::changes::counters::{
    CounterKind, CounterState,
    benevola::{BenevolaHandover, BenevolaSource, NPC, WOLF},
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable,
    NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn worldchg_benevola_native_yes_debits_one_and_credits_ten_atomically() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-benevola", scenario)
}
async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    let wolves: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(WOLF).fetch_one(&h.pool).await?;
    let recovery: i64 = sqlx::query_scalar("SELECT (state->'counts'->>1)::bigint FROM game_world_change_counters WHERE change_key='overhunting'").fetch_one(&h.pool).await?;
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_change_credit_receipts WHERE source_kind=3",
    )
    .fetch_one(&h.pool)
    .await?;
    let consumed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-transition/worldchange/benevola/handover'").fetch_one(&h.pool).await?;
    Ok((wolves, recovery, receipts, consumed))
}
fn handover_for(
    proof: &serde_json::Value,
    session: crate::foundation::GameSessionId,
    id: u64,
) -> TestResult<BenevolaHandover> {
    Ok(BenevolaHandover {
        command: crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(id).map_err(debug)?,
        ),
        source: serde_json::from_value(proof["native_npc"]["source"].clone())?,
    })
}
fn scenario() -> TestResult {
    run(async |admin| {
        let start = std::time::Instant::now();
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "worldchg_benevola", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("native generation")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (32595, 31745, 7),
        );
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.reset_epoch = 1;
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        assert!(book.get(NPC).is_some(), "production native Benevola");
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Nature helper", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        deps.initialize(&owner).await;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "gameplay fence")?
            .ok_or("gameplay fence")?;
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                floor: -7,
                                x: 32596,
                                y: 31745,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            runtime
                .visible_entities()
                .npcs
                .iter()
                .find(|n| runtime.npc_key(n.actor) == Some(NPC))
                .map(|n| EntityRef {
                    identity: n.actor.placement_identity(),
                    generation: n.generation,
                })
                .ok_or("source Benevola actor")?
        };
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
            .bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
            .bind(world.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        h.root.world_changes(world, 1).await?.ok_or("shared day")?;
        let mut state = CounterState::new(CounterKind::Overhunting, 1, false);
        state.stage = 2;
        sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='overhunting'")
            .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(state)?).execute(&h.pool).await?;
        // Migration-owned fixture input only. Observation/debit uses real carried custody.
        super::item_look::seed_main_backpack(&h, &[(WOLF, 1), (WOLF, 1)]).await?;
        let command = |id| UseCommand {
            game_session_id: session,
            command_id: id,
            item_fence: Some(fence.item_fence()),
        };
        for (id, text) in [(1, "hi"), (2, "mission")] {
            let reply = owner
                .npc_talk(
                    deps.actor,
                    command(id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "native {text}");
        }
        let before = snapshot(&h).await?;
        assert_eq!(before, (2, 0, 0, 0));
        let reply = owner
            .npc_talk(
                deps.actor,
                command(3),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .await;
        assert_eq!(reply.disposition, NpcIntentDisposition::Ok);
        assert!(reply.inventory_changed);
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        let proof:serde_json::Value=sqlx::query_scalar("SELECT convert_from(proof,'UTF8')::jsonb FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-transition/worldchange/benevola/handover'").fetch_one(&h.pool).await?;
        let source: BenevolaSource = serde_json::from_value(proof["native_npc"]["source"].clone())?;
        let handover = BenevolaHandover {
            command: crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(3).map_err(debug)?,
            ),
            source,
        };
        // Simulate a committed handover whose acknowledgement was lost, followed by
        // fresh snapshot conversation retirement and an advanced native owner epoch.
        let engaged = owner
            .states
            .chat
            .engaged(session)
            .ok_or("original conversation")?;
        assert!(owner.states.chat.end_engagement(session, engaged));
        owner.reset_epoch = 2;
        owner
            .states
            .npc_conversations
            .retain_benevola_for_test(fence.item_fence(), handover.clone());
        let recovered = owner
            .npc_talk(
                deps.actor,
                command(30),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "hi".into(),
                },
            )
            .await;
        assert_eq!(recovered.disposition, NpcIntentDisposition::Ok);
        assert!(
            recovered.inventory_changed,
            "historical receipt refreshes authoritative inventory"
        );
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        // Same command with a substituted frozen source must conflict in the reader too.
        let mut changed_source = handover.clone();
        changed_source.source.npc_identity[0] ^= 1;
        assert!(
            h.root
                .reconcile_benevola_handover(&authority, fence.character_id, changed_source)
                .await
                .is_err()
        );
        let mut absent = handover.clone();
        absent.command = crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(31).map_err(debug)?,
        );
        owner
            .states
            .npc_conversations
            .retain_benevola_for_test(fence.item_fence(), absent);
        let expired = owner
            .npc_talk(
                deps.actor,
                command(32),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "hi".into(),
                },
            )
            .await;
        assert_eq!(expired.disposition, NpcIntentDisposition::Rejected);
        assert_eq!(
            snapshot(&h).await?,
            (1, 10, 1, 1),
            "missing history never re-executes stale context"
        );
        owner.reset_epoch = 1;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        for (id, text) in [(33, "hi"), (34, "mission")] {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let restarted = owner
                .npc_talk(
                    deps.actor,
                    command(id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(
                restarted.disposition,
                NpcIntentDisposition::Ok,
                "expired retained request must not wedge fresh {text}"
            );
        }
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        let catalogue = BenevolaHandover::catalogue("content-1").map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        // Historical Quest receipt replays before the stale pre-write Character revision.
        assert!(matches!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    fence,
                    &permit,
                    handover.clone(),
                    Arc::clone(&catalogue)
                )
                .await
                .map_err(debug)?,
            crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        let mut conflict = handover.clone();
        conflict.source.npc_identity[0] ^= 1;
        assert!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    fence,
                    &permit,
                    conflict,
                    Arc::clone(&catalogue)
                )
                .await
                .is_err()
        );
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        let mut wrong = handover.clone();
        wrong.source.position[0] += 1;
        assert!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    fence,
                    &permit,
                    wrong,
                    Arc::clone(&catalogue)
                )
                .await
                .is_err()
        );
        let current = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "current fence")?
            .ok_or("current fence")?;
        let mut stale = current;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(current.connection_generation.get() + 1)
                .map_err(debug)?;
        let mut fresh = handover.clone();
        fresh.command = crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(4).map_err(debug)?,
        );
        assert!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    stale,
                    &permit,
                    fresh,
                    Arc::clone(&catalogue)
                )
                .await
                .is_err()
        );
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        // Independently current epoch is checked before a new counter/item write.
        let mut stale_day = handover;
        stale_day.command = crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(5).map_err(debug)?,
        );
        stale_day.source.world.reset_epoch = 2;
        assert!(matches!(
            h.root
                .commit_benevola_handover(
                    &authority, &h.node, current, &permit, stale_day, catalogue
                )
                .await
                .map_err(debug)?,
            crate::durability::quest_state::QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(&h).await?, (1, 10, 1, 1));
        // A second real handover consumes the second item; the next handover has no item.
        // Its speculative shared credit MUST roll back with the custody refusal.
        let mut second = handover_for(&proof, session, 6)?;
        assert!(matches!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    current,
                    &permit,
                    second.clone(),
                    BenevolaHandover::catalogue("content-1").map_err(debug)?
                )
                .await
                .map_err(debug)?,
            crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
        ));
        assert_eq!(snapshot(&h).await?, (0, 20, 2, 2));
        let latest = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "latest fence")?
            .ok_or("latest fence")?;
        second.command = crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(7).map_err(debug)?,
        );
        assert!(matches!(
            h.root
                .commit_benevola_handover(
                    &authority,
                    &h.node,
                    latest,
                    &permit,
                    second,
                    BenevolaHandover::catalogue("content-1").map_err(debug)?
                )
                .await
                .map_err(debug)?,
            crate::durability::quest_state::QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(snapshot(&h).await?, (0, 20, 2, 2));
        let tracks:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE quest_key='oteryn:quest/worldchange/benevola'").fetch_one(&h.pool).await?;
        assert_eq!(tracks, 0);
        assert!(start.elapsed().as_secs_f64() > 1.0);
        println!(
            "WORLDCHG-Q PG EXECUTED: native Benevola yes, one captured wolf debit, ten shared recovery, replay, retained reconnect/epoch history, stale no-receipt recovery, wrong source, stale Character/epoch, no personal progression; elapsed={:?}",
            start.elapsed()
        );
        drop(permit);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
