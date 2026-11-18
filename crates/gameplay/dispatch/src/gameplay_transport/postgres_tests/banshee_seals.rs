//! PostgreSQL dispatch regressions for the Banshee seal callbacks.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NativeEntryStart, channel_content_pin::ChannelContentPin};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn banshee_logic_use_flips_lever_without_overwriting_a_discordant_campfire() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-logic", || {
        run(async |admin| exercise(admin, false).await)
    })
}

#[test]
fn banshee_sacrifice_step_refuses_without_a_blood_splash() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-sacrifice", || {
        run(async |admin| exercise(admin, true).await)
    })
}

async fn exercise(admin: String, sacrifice: bool) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "banshee_logic", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let lever = Position {
        x: 32308,
        y: 31975,
        floor: -13,
    };
    let campfire = Position {
        x: 32309,
        y: 31975,
        floor: -13,
    };
    let flame = (32250, 31892, -14);
    let (source, objects) = if sacrifice {
        (
            flame,
            vec![
                (32250, 31892, -14, 35016, 1959),
                (32261, 31849, -15, 0, 1959),
            ],
        )
    } else {
        (
            (lever.x, lever.y, lever.floor),
            vec![
                (lever.x, lever.y, lever.floor, 30018, 2772),
                (campfire.x, campfire.y, campfire.floor, 0, 1996),
                (32311, 31975, -13, 0, 1998),
            ],
        )
    };
    let world = super::quest_callbacks::source_world_spatial(source, &objects)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let start = NativeEntryStart {
        x: i32::from(source.0) - 1,
        y: i32::from(source.1),
        floor: -i16::from(source.2),
    };
    let room = activated
        .qualified_room()
        .bundle_view(cells.clone(), frame, world.map_revision_digest(), start)
        .ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .map_err(debug)?;
    let deps = Dependencies {
        actor,
        runtime: tokio::sync::Mutex::new(runtime),
        ..deps
    };
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    deps.initialize(&owner).await;
    if sacrifice {
        exercise_sacrifice(
            &mut owner,
            &h,
            super::AdmittedSession {
                runtime_actor: Some(actor),
                ..deps.admitted
            },
            frame,
            start,
        )
        .await?;
    } else {
        exercise_logic(&owner, &world, actor, fence, lever, campfire).await?;
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn exercise_logic(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    world: &crate::map::boot::BundleWorld,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    lever: Position,
    campfire: Position,
) -> TestResult {
    let session = fence.game_session_id;
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/banshees-logic-lever-30018",
        "quest": "oteryn:quest.the_queen_of_the_banshees_quest",
        "trigger": "use",
        "selector": { "position": lever, "unique": 30018 },
        "lever": [2772, 2773],
        "reversible": true,
        "storage": [],
        "offerings": [],
        "passages": [{
            "position": campfire,
            "appearance": 1998,
            "replacement": 1996,
            "if_present": true
        }],
        "return_on_failure": false,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-6-sixth_seal_lever.lua:3"]
    }))?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .is_ok()
    );
    let placement_key = |p: Position| {
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("placement")
    };
    let campfire_key = placement_key(campfire)?;
    let lever_key = placement_key(lever)?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&lever_key.to_be_bytes());
    let map = owner.bundle_map().ok_or("bundle map")?;
    let initial = map.facts.object_revision(campfire_key);
    for (command_id, changed) in [(1, false), (2, true)] {
        let outcome = owner
            .use_object(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence.item_fence()),
                },
                WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: map.facts.object_revision(lever_key),
                },
            )
            .await;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert_eq!(map.facts.object_revision(campfire_key) != initial, changed);
    }
    Ok(())
}

async fn exercise_sacrifice(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: super::AdmittedSession,
    frame: [u8; 32],
    start: NativeEntryStart,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
    const TRACK: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal";
    const DOOR: &str =
        "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal_door";
    const KEY: &str = "oteryn:quest-callback/banshees-fourth-seal";
    const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-4-fourth_seal_flame.lua:3";
    let generation = owner.active_generation.ok_or("active generation")?;
    let bindings: Bindings = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(SOURCE)),
            "sources": [SOURCE],
            "tracks": [TRACK, DOOR].map(|key| serde_json::json!({
                "key": key, "initial": -1, "minimum": -1, "maximum": 1
            })),
            "stages": [{
                "key": "sacrifice-flame",
                "completes": false,
                "trigger": { "kind": "step_in", "callback": {
                    "key": KEY,
                    "quest": QUEST,
                    "trigger": "step_in",
                    "selector": { "position": {"x": 32250, "y": 31892, "floor": -14}, "unique": 35016 },
                    "teleport": {"x": 32261, "y": 31849, "floor": -15},
                    "reversible": false,
                    "return_on_failure": true,
                    "requires_world_admission": true,
                    "offerings": [],
                    "passages": [],
                    "storage": [TRACK, DOOR].map(|track| serde_json::json!({
                        "track": track, "minimum": -1, "maximum": 0, "set": 1
                    })),
                    "effects": {
                        "splash_offering": { "position": {"x": 32243, "y": 31892, "floor": -14}, "subtype": 5 }
                    },
                    "sources": [SOURCE]
                }}
            }]
        }]
    }))?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(std::sync::Arc::new(bindings.attach(&base)?));
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (outcome, _) = owner
        .paced_step(actor, admitted.game_session_id, StepDirection::East)
        .await;
    assert_eq!(
        outcome.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after.position(),
        MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor
        }
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 0);
    Ok(())
}
