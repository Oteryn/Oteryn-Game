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
use std::sync::atomic::Ordering;

#[path = "l6_world_pickups/world.rs"]
mod world;

const LEVER: TilePos = TilePos {
    x: 33270,
    y: 31476,
    floor: -14,
};
#[test]
fn spawnsum_ferumbras_lever_admits_two_players_and_ejects_on_timeout() -> TestResult {
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
        [
            world::NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y122.b3"
                )),
                region: (14, 129, 122),
                bounds: (33269, 31476, 33272, 31482),
                start: (33270, 31477),
                expected: vec![],
            },
            world::NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x130-y122.b3"
                )),
                region: (14, 130, 122),
                bounds: (33385, 31461, 33399, 31481),
                start: (33392, 31479),
                expected: vec![],
            },
        ]
        .into_iter()
        .chain(std::iter::once(world::NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x130-y126.b3"
            )),
            region: (13, 130, 126),
            bounds: (33319, 32318, 33320, 32319),
            start: (33319, 32318),
            expected: vec![],
        }))
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
    let crystal = TilePos {
        x: 33390,
        y: 31468,
        floor: -14,
    };
    let crystal_key = placement_key(crystal, 1).ok_or("crystal placement")?;
    assert!(map.facts.transform_movement_item(crystal_key, 14961));
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
    owner.states.test_owner_time.store(100, Ordering::Relaxed);
    assert_eq!(
        use_lever(deps.actor, first, 1).await.disposition,
        UseDisposition::Committed
    );
    assert_eq!(
        crate::map::view::compose_tile(&map.overlay, &map.facts, crystal)
            .map_err(debug)?
            .ok_or("reset crystal tile")?
            .entries[1]
            .facts
            .appearance_id,
        14955
    );
    let mut runtime = owner.runtime.lock().await;
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
    let mut states = owner.states.spell_states.lock().await;
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &mut states, 1_200_000_099);
    assert_eq!(
        runtime
            .read_actor_position(second_actor)
            .map_err(debug)?
            .position()
            .floor,
        14
    );
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &mut states, 1_200_000_100);
    for actor in [deps.actor, second_actor] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 33319,
                y: 32318,
                floor: 13
            }
        );
    }
    Ok(())
}
