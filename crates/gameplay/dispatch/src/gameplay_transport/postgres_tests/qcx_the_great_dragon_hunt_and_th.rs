//! Lost Brother dialogue, remains, reward, and trade through live dispatch.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, UseCommand}};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

const STAGE: &str = "oteryn:quest-progress/quest/u10_80/the_lost_brother_quest";

#[test]
fn lost_brother_accepts_reports_and_unlocks_trade_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx63-lost-brother", scenario);
    eprintln!("POSTGRES_CASE_COMPLETED qcx63-lost-brother duration={:?}", started.elapsed());
    result
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
        .bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?)
}

async fn rewards(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid), (SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3039')")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}

async fn place_npc(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner
        .states
        .chat
        .admit(session, "Lost Brother Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let mut runtime = owner.runtime.lock().await;
    let spot = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.tarun".into(),
                    placements: vec![NpcPlacement {
                        floor: -i8::try_from(spot.floor)?,
                        x: u16::try_from(spot.x)?,
                        y: u16::try_from(spot.y + 1)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let placed = runtime
        .visible_entities()
        .npcs
        .first()
        .copied()
        .ok_or("Tarun placement")?;
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: placed.actor.placement_identity(),
        generation: placed.generation,
    })
}

async fn trade_window(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
) -> TestResult<bool> {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC clock")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest fence")?;
    let outcome = owner.npc_talk(
        admitted.runtime_actor.ok_or("actor")?,
        UseCommand { game_session_id: admitted.game_session_id, command_id, item_fence: Some(fence.item_fence()) },
        NpcTalkIntent { npc_actor: npc, text: "trade".into() },
    ).await;
    assert!(!outcome.unknown);
    Ok(outcome.window.is_some_and(|window| !window.offers.is_empty()))
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let trigger = Position { x: 32959, y: 32674, floor: -4 };
        let callback = world_callbacks::load()?.into_iter()
            .find(|row| row.key == "oteryn:quest-callback/the-lost-brother-remains")
            .ok_or("source remains callback")?;
        let world = super::quest_world_callbacks::world(&callback, trigger)?;
        let h = Harness::create_quest_reward(admin, "qcx63_lost_brother", true).await?;
        prepare_current_session(&h).await?;
        let mut deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("production generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 32958,
                    y: 32674,
                    floor: 4,
                },
            )
            .ok_or("Lost Brother bundle view")?;
        let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (32958, 32674, 4),
        );
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 32958,
                    y: 32674,
                    floor: 4,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.room = room.clone();
        let premium = super::qengine_endgame_stories::current_premium(&h).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&premium);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "quest fence")?.ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        // The replacement qualified carrier must carry the same native Character
        // lease as the committed fresh session before any NPC or USE preflight.
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let session = admitted.game_session_id;
        let npc = place_npc(&owner, deps.actor, session).await?;
        let (initial_xp, initial_gems) = rewards(&h).await?;
        let talk = super::qengine_rewards::talk;
        talk(&mut owner, &mut admitted, npc, 20, "hi").await?;
        assert!(!trade_window(&mut owner, &admitted, npc, 21).await?);
        for (number, text) in [(22, "mission"), (23, "no")] {
            talk(&mut owner, &mut admitted, npc, number, text).await?;
        }
        assert_eq!(stage(&h).await?, 0);
        for (number, text) in [(24, "mission"), (25, "yes")] {
            assert_eq!(talk(&mut owner, &mut admitted, npc, number, text).await?, NpcIntentDisposition::Ok);
        }
        assert_eq!(stage(&h).await?, 1, "Tarun must record the accepted mission");
        super::quest_world_callbacks::wire(&owner, &mut admitted, 1, 1, encode_step_intent(StepDirection::East)).await?;
        assert_eq!(stage(&h).await?, 2, "the source remains tile must report the discovery");
        for (number, text) in [(26, "hi"), (27, "mission"), (28, "mission")] {
            talk(&mut owner, &mut admitted, npc, number, text).await?;
        }
        assert_eq!(stage(&h).await?, 3);
        assert_eq!(rewards(&h).await?, (initial_xp + 3000, initial_gems + 1));
        assert!(trade_window(&mut owner, &admitted, npc, 29).await?);
        h.cleanup().await
    })
}
