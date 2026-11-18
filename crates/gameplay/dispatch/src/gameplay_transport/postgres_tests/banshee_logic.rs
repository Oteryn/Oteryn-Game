//! Exercise the shared reversible USE when its campfire is already in the other state.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NativeEntryStart, channel_content_pin::ChannelContentPin};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn banshee_logic_use_flips_lever_without_overwriting_a_discordant_campfire() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-logic", || run(exercise))
}

async fn exercise(admin: String) -> TestResult {
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
    let world = super::quest_callbacks::source_world_spatial(
        (lever.x, lever.y, lever.floor),
        &[
            (lever.x, lever.y, lever.floor, 30018, 2772),
            (campfire.x, campfire.y, campfire.floor, 0, 1996),
            (32311, 31975, -13, 0, 1998),
        ],
    )?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let start = NativeEntryStart {
        x: 32307,
        y: 31975,
        floor: 13,
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
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
