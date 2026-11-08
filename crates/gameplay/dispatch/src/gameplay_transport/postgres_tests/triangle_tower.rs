//! Source-literal Triangle Tower lever, including its different closing wall.
use super::TestResult;
use super::quest_world_callbacks::wire;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn triangle_tower_source_wall_cycle_over_use_wire() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "triangle-tower-wall",
                Position {
                    x: 32573,
                    y: 32121,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Triangle Tower wire panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED triangle_tower_wall duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    world: &crate::map::boot::BundleWorld,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let wall = crate::map::overlay::TilePos {
        x: 32566,
        y: 32119,
        floor: -7,
    };
    let lever_key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: trigger.x,
            y: trigger.y,
            floor: trigger.floor,
        },
        1,
    )
    .ok_or("lever key")?;
    let wall_key = crate::map::view::placement_key(wall, 1).ok_or("wall key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&lever_key.to_be_bytes());
    let wall_id = *world
        .base()
        .tile(wall.x, wall.y, wall.floor)
        .and_then(|t| t.ids().get(1))
        .ok_or("wall id")?;
    let original = world.facts().base_entry(wall, 1, wall_id).ok_or("wall")?;
    assert_eq!(original.appearance_id, 1270);
    assert_eq!(
        world
            .base()
            .tile(wall.x, wall.y, wall.floor)
            .ok_or("wall tile")?
            .ids()
            .len(),
        2,
        "one ground and one source wall"
    );
    assert!(!world.enterable(wall));
    let cycle = callback
        .effects
        .item_cycle
        .as_ref()
        .ok_or("canonical wall cycle")?;
    assert_eq!((cycle.remove, cycle.create), (1270, 1025));
    assert_eq!(
        (cycle.position.x, cycle.position.y, cycle.position.floor),
        (wall.x, wall.y, wall.floor)
    );
    for (command, revision) in [(1, 0), (1, 0), (2, 1), (3, 2), (4, 3)] {
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(super::fixture::debug)?;
        let reply = wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply)
                .map_err(super::fixture::debug)?,
            UseDisposition::Committed
        );
        assert_eq!(world.facts().object_revision(lever_key), command);
        if command == 1 {
            assert!(world.facts().base_hidden(wall_key));
            assert!(world.enterable(wall));
        } else {
            assert!(
                world.facts().base_hidden(wall_key),
                "1270 is never restored"
            );
            assert!(!world.enterable(wall));
        }
        let extra = world.facts().movement_items(wall);
        assert_eq!(extra.len(), usize::try_from(command.saturating_sub(1))?);
        assert!(extra.iter().all(|entry| entry.facts.appearance_id == 1025));
    }
    let lever = world
        .facts()
        .base_entry(
            crate::map::overlay::TilePos {
                x: trigger.x,
                y: trigger.y,
                floor: trigger.floor,
            },
            1,
            1,
        )
        .ok_or("lever")?;
    assert_eq!(
        lever.appearance_id, 2772,
        "source cannot reopen the created1025 wall"
    );
    // Return through the actual native basement portal after the wall has closed.
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let expected = runtime
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?;
        let closed = crate::map::floor::BundleDoors(world.facts().doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
            &runtime,
            &owner.movement_cells,
            actor,
            admitted.game_session_id,
            expected,
            crate::foundation::MovementLocalPosition {
                x: 32565,
                y: 32120,
                floor: 8,
            },
            false,
            &closed,
            None,
        )
        .map_err(super::fixture::debug)?;
        runtime
            .commit_source_step(proof)
            .map_err(super::fixture::debug)?;
    }
    let reply = wire(
        owner,
        admitted,
        5,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        ),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply)
            .map_err(super::fixture::debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?
            .position(),
        crate::foundation::MovementLocalPosition {
            x: 32569,
            y: 32110,
            floor: 7
        }
    );
    Ok(())
}
