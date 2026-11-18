use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[path = "l6_world_pickups/world.rs"]
mod world;

const LEVER: TilePos = TilePos {
    x: 33270,
    y: 31476,
    floor: -14,
};
#[test]
fn spawnsum_ferumbras_lever_admits_staged_players_and_opening_actors() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                super::reset();
                let h =
                    Harness::create_with_admission_seed(admin, "spawnsum_ferumbras", false, false)
                        .await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Ferumbras PG test panicked")?;
    eprintln!(
        "FERUMBRAS_POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    result.map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    super::prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native item profiles")?;
    let world = world::native_scene_with_appearances(
        native,
        [(33269, 31476, 33271, 31481), (33385, 31466, 33398, 31480)]
            .into_iter()
            .map(|bounds| world::NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x130-y122.b3"
                )),
                region: (14, 130, 122),
                bounds,
                start: (33270, 31477),
                expected: vec![],
            })
            .collect(),
        &[],
        &[8911, 8912, 14955, 14961],
    )?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let start = MovementLocalPosition {
        x: 33270,
        y: 31477,
        floor: 14,
    };
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
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
        16,
        content_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let first = deps.admitted.game_session_id;
    let second = GameSessionId::decode(&id(51)).map_err(debug)?;
    let mut admit = |session, position| -> TestResult<_> {
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, position)
            .map_err(debug)?;
        Ok(actor)
    };
    deps.actor = admit(first, start)?;
    let second_actor = admit(second, MovementLocalPosition { x: 33271, ..start })?;
    for (actor, session, character) in [(deps.actor, first, 41), (second_actor, second, 44)] {
        let lease = CharacterLease::new(CharacterId::decode(&id(character)).map_err(debug)?, 2)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
    }
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    deps.initialize_player(&owner, second_actor, second).await;

    let map = owner.bundle_map().ok_or("bundle")?;
    let lever = placement_key(LEVER, 2).ok_or("lever")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&lever.to_be_bytes());
    let use_lever = |actor, session, command_id| {
        owner.use_world_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: None,
            },
            WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: map.facts.object_revision(lever),
            },
        )
    };
    assert_eq!(
        use_lever(deps.actor, first, 1)
            .await
            .map_err(debug)?
            .disposition,
        UseDisposition::Committed
    );
    let runtime = owner.runtime.lock().await;
    for actor in [deps.actor, second_actor] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 33392,
                y: 31479,
                floor: 14
            }
        );
    }
    let census = runtime.positioned_melee_census().map_err(debug)?;
    let count = |key: &str| {
        census
            .iter()
            .filter(|(actor, _, session)| {
                session.is_none() && runtime.matches_live_creature_identity(*actor, key.as_bytes())
            })
            .count()
    };
    assert_eq!(count("oteryn:creature.ascending_ferumbras"), 1);
    assert_eq!(count("oteryn:creature.enthralled_demon"), 4);
    Ok(())
}
