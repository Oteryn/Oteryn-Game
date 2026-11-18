//! Adventurers Guild NPC and placed-item rewards through live dispatch.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, UseCommand},
};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const EXPLORER: Position = Position {
    x: 33007,
    y: 32607,
    floor: -8,
};
const EXPLORER_APPROACH: Position = Position {
    x: 33006,
    ..EXPLORER
};
const TARUN: Position = Position {
    x: 32976,
    y: 32715,
    floor: -7,
};
const TARUN_APPROACH: Position = Position { x: 32975, ..TARUN };
const REMAINS_APPROACH: Position = Position {
    x: 32958,
    y: 32674,
    floor: -4,
};

const STAGE: &str = "oteryn:quest-progress/quest/u10_80/the_lost_brother_quest";

#[test]
fn lost_brother_accepts_reports_and_unlocks_trade_once() -> TestResult {
    postgres_case("qcx63-lost-brother", false)
}

#[test]
fn great_dragon_hunt_explorer_awards_one_scale() -> TestResult {
    postgres_case("qcx63-dragon-scale", true)
}

fn postgres_case(name: &'static str, explorer: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || scenario(explorer));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
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
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/world/npc-placements/placements-00000-00055.json"
    ))?;
    let placement = data["records"]
        .as_array()
        .ok_or("NPC placement shard")?
        .iter()
        .map(|row| &row["declaration"])
        .find(|row| row["npc"] == "oteryn:npc.tarun")
        .ok_or("production Tarun placement")?;
    assert_eq!(
        placement["cell"],
        serde_json::json!({"x": TARUN.x, "y": TARUN.y, "floor": -TARUN.floor})
    );
    assert_eq!(placement["direction"], "north");
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(TARUN_APPROACH)
    );
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.tarun".into(),
                    placements: vec![NpcPlacement {
                        floor: TARUN.floor,
                        x: TARUN.x,
                        y: TARUN.y,
                        direction: MovementFacing::North,
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
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "trade".into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    Ok(outcome
        .window
        .is_some_and(|window| !window.offers.is_empty()))
}

async fn teleport(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    destination: Position,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        before,
        super::quest_world_callbacks::local(destination),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
async fn claim_scale(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
    use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
    use oteryn_protocol_oteryn::world_object::{
        CAPABILITY_ITEM_USE_V1, CAPABILITY_WORLD_INTERACTION_V1, UseDisposition, WorldObjectTarget,
        decode_use_result, encode_use_intent,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_WORLD_INTERACTION_V1,
            CAPABILITY_ITEM_USE_V1,
            CAPABILITY_WORLD_MAP_VIEW_V1,
        ],
    )
    .ok_or("world interaction capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_WORLD_INTERACTION_V1)
    );
    let key = oteryn_world_bundle::bundle::placement_key(EXPLORER.floor, EXPLORER.x, EXPLORER.y, 1)
        .ok_or("Dead Explorer placement key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    for command_id in [1, 2] {
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command_id, 2, payload.clone())
                .await?;
        if command_id == 1 {
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
        } else {
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Rejected
            );
        }
        let count: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i5882'")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            count, 1,
            "the explorer must grant exactly one Red Dragon Scale"
        );
    }
    Ok(())
}

fn scenario(explorer: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let trigger = Position {
            x: 32959,
            y: 32674,
            floor: -4,
        };
        let mut callback = world_callbacks::load()?
            .into_iter()
            .find(|row| row.key == "oteryn:quest-callback/the-lost-brother-remains")
            .ok_or("source remains callback")?;
        let (world, start) = if explorer {
            callback.selector.position = Some(EXPLORER);
            callback.selector.appearance = Some(21412);
            callback.trigger = world_callbacks::Trigger::Use;
            (
                super::quest_world_callbacks::world(&callback, EXPLORER)?,
                EXPLORER_APPROACH,
            )
        } else {
            (
                super::quest_world_callbacks::world_with_cells(
                    &callback,
                    trigger,
                    &[TARUN, TARUN_APPROACH],
                )?,
                TARUN_APPROACH,
            )
        };
        let h = Harness::create_quest_reward(admin, "qcx63_lost_brother", true).await?;
        let result = async {
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
        let start = super::quest_world_callbacks::local(start);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
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
            (start.x, start.y, start.floor),
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
            .initialize_pinned_test_position(deps.actor, start)
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
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
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
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(|e| format!("callback admission equipment: {e:?}"))?;
        let native = generation.native_gameplay().ok_or("native")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(|e| format!("callback starter items: {e:?}"))?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        // Seed an incomplete Guild save that predates both new callback families.
        super::qengine_outfit_stories::seed_quest_prerequisites(
            &h,
            &owner,
            &admitted,
            &[("oteryn:quest-progress/quest/u9_80/adventurers_guild/quest_line".into(), 1)],
            110,
        ).await?;
        let mut prior_save = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *prior_save)
            .await?;
        let imported = sqlx::query(
            "INSERT INTO game_character_quest_states
                (character_id, quest_key, pinned_content_revision, definition_hash,
                 started_character_revision, committed_character_revision)
             SELECT character_id, 'oteryn:quest.adventurers_guild', 'content-1',
                    decode($2,'hex'), character_revision, character_revision
             FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .bind("ae3b30a75e15f42fef0701e744bb27e91596aec92d794c9b969244d4668e7eae")
        .execute(&mut *prior_save)
        .await?;
        assert_eq!(imported.rows_affected(), 1);
        prior_save.commit().await?;
        owner.admit_quest_session(&admitted).await;
        if explorer {
            return claim_scale(&owner, &mut admitted, &h).await;
        }
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
            assert_eq!(
                talk(&mut owner, &mut admitted, npc, number, text).await?,
                NpcIntentDisposition::Ok
            );
        }
        assert_eq!(
            stage(&h).await?,
            1,
            "Tarun must record the accepted mission"
        );
        teleport(&owner, &admitted, REMAINS_APPROACH).await?;
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            1,
            1,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            stage(&h).await?,
            2,
            "the source remains tile must report the discovery"
        );
        teleport(&owner, &admitted, TARUN_APPROACH).await?;
        for (number, text) in [(26, "hi"), (27, "mission"), (28, "mission")] {
            talk(&mut owner, &mut admitted, npc, number, text).await?;
        }
        assert_eq!(stage(&h).await?, 3);
        assert_eq!(rewards(&h).await?, (initial_xp + 3000, initial_gems + 1));
        assert!(trade_window(&mut owner, &admitted, npc, 29).await?);
        Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result.and(cleanup)
    })
}
