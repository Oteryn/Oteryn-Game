//! Native two-floor winches through the real USE transport and Channel owners.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{goblin_merchant as elevator, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn qm50_native_elevator_call_ride_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qm50-elevator", || {
        super::quest_world_callbacks::scenario(
            "goblin-merchant-elevator-upper",
            Position {
                x: 33052,
                ..elevator::UPPER
            },
            false,
        )
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let crop = |floor, bytes, expected| NativeCrop {
        bytes,
        region: (floor, 129, 125),
        bounds: (33051, 32098, 33053, 32101),
        start: (33051, 32098),
        expected,
    };
    native_scene_with_appearances(
        native,
        vec![
            crop(
                6,
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z06-x129-y125.b3"
                )),
                vec![ExpectedTile {
                    position: (33051, 32099),
                    ids: vec![17970, 17947, 17942],
                    counted_entry: None,
                }],
            ),
            crop(
                7,
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y125.b3"
                )),
                vec![ExpectedTile {
                    position: (33051, 32099),
                    ids: vec![16282, 17943, 17938],
                    counted_entry: None,
                }],
            ),
        ],
        &[],
        &[17938, 17939, 17942, 17943],
    )
}

async fn use_winch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    upper: bool,
) -> TestResult<UseDisposition> {
    if id >= admitted.continuity.next_command_id {
        let actor = admitted.runtime_actor.ok_or("actor")?;
        // Wait on the actual source action deadline; exact replays bypass the gate.
        loop {
            let delay = owner
                .states
                .world_gate
                .lock()
                .map_err(|_| "world gate")?
                .equip_delay(actor, 0, owner.clock_origin.elapsed());
            let Some(delay) = delay else {
                break;
            };
            tokio::time::sleep(delay).await;
        }
    }
    let map = owner.bundle_map().ok_or("map")?;
    let pos = if upper {
        elevator::UPPER
    } else {
        elevator::LOWER
    };
    let ordinal = if upper { 3 } else { 2 };
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x + 1, pos.y, ordinal)
        .ok_or("winch")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let trip = elevator::trip(elevator::UPPER_KEY, false, false)?;
    let position = |p: Position| crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let transforms = trip
        .transforms
        .map(|t| (position(t.position), t.before, t.after));
    let batch = map
        .facts
        .prepare_quest_platform_batch(
            map.overlay.base(),
            position(trip.from),
            position(trip.to),
            &trip.items,
            &transforms,
        )
        .ok_or("native elevator batch qualification")?;
    assert!(
        map.facts
            .quest_platform_batch_landing(map.overlay.base(), &batch, position(trip.to)),
        "the source frame transformation must qualify the upper landing"
    );
    let to = super::quest_world_callbacks::local(trip.evacuate);
    assert_eq!(
        owner
            .movement_cells
            .index()
            .lookup(
                owner.movement_cells.scope(),
                crate::content::LogicalCell {
                    x: to.x,
                    y: to.y,
                    z: i32::from(to.floor)
                }
            )
            .map_err(debug)?,
        crate::content::CollisionClass::Walkable,
        "source-qualified evacuation landing"
    );
    let creatures = {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native creature policies")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        runtime
            .commit_quest_creature_spawn(&[
                (
                    "oteryn:creature.rat",
                    health,
                    super::quest_world_callbacks::local(elevator::LOWER),
                ),
                (
                    "oteryn:creature.rat",
                    health,
                    super::quest_world_callbacks::local(elevator::LOWER),
                ),
            ])
            .map_err(debug)?
    };
    assert_eq!(
        use_winch(owner, admitted, 1, true).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    let upper = crate::map::overlay::TilePos {
        x: 33051,
        y: 32099,
        floor: -6,
    };
    assert!(
        map.facts
            .movement_items(upper)
            .iter()
            .any(|e| e.facts.appearance_id == 17938)
    );
    for creature in &creatures {
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(*creature)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(Position {
                y: 32100,
                ..elevator::LOWER
            }),
            "every stacked departure occupant is evacuated"
        );
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        origin,
        "calling the elevator leaves the caller beside the winch"
    );
    assert_eq!(
        use_winch(owner, admitted, 1, true).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_winch(owner, admitted, 2, true).await?,
        UseDisposition::Rejected,
        "a rider must step onto the platform"
    );
    let step = super::quest_world_callbacks::wire(
        owner,
        admitted,
        3,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::South,
        ),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&step).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    assert_eq!(
        use_winch(owner, admitted, 4, true).await?,
        UseDisposition::Committed
    );
    let landed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        landed.position(),
        super::quest_world_callbacks::local(elevator::LOWER)
    );
    assert_eq!(
        use_winch(owner, admitted, 4, true).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        landed
    );
    assert_eq!(
        use_winch(owner, admitted, 5, false).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(elevator::UPPER)
    );
    Ok(())
}
