use oteryn_gameplay_world::map::view::MapFacts;
use super::fixture::{Harness, debug, id};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Callback;

use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

pub(super) async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn use_marker(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    callback: &Callback,
    command: u64,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use_marker_at_depth(owner, admitted, callback, command, 1).await
}

pub(super) async fn use_marker_at_depth(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    callback: &Callback,
    command: u64,
    ordinal: u8,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use oteryn_protocol_oteryn::world_object::{
        WorldObjectTarget, decode_use_result, encode_use_intent,
    };
    let p = callback.selector.position.ok_or("marker")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
        .ok_or("placement")?;
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
        expected_revision: owner.bundle_map().ok_or("map")?.facts.object_revision(key),
    })
    .map_err(debug)?;
    decode_use_result(
        &super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload).await?,
    )
    .map_err(debug)
}

pub(super) fn scene(
    entry: &Callback,
    marker: &Callback,
) -> TestResult<crate::map::boot::BundleWorld> {
    if entry.key.contains("/fury_gates/") {
        return super::engworldchg_fury::scene(entry, marker);
    }
    if entry.key.contains("/hive_born/") {
        use oteryn_gameplay_quests::quest::{hive_access, world_callbacks::Position};

        let east = entry.selector.position.ok_or("eastern checkpoint")?;
        let west = marker.selector.position.ok_or("western checkpoint")?;
        let gate = |x, y| {
            hive_access::GATES
                .iter()
                .find(|gate| gate.position == (Position { x, y, floor: -1 }))
                .ok_or("Hive gate")
        };
        let east_gate = gate(33598, 31228)?;
        let west_gate = gate(33495, 31197)?;
        let objects = [
            (east_gate.position, east_gate.appearance),
            (west_gate.position, west_gate.appearance),
            (west, marker.selector.appearance.ok_or("tower marker")?),
        ];
        let cells: Vec<_> = (-2i16..=3)
            .flat_map(|offset| {
                (-1i16..=1).map(move |row| Position {
                    x: west.x.saturating_add_signed(offset),
                    y: west.y.saturating_add_signed(row),
                    ..west
                })
            })
            .collect();
        return super::quest_world_callbacks_fixtures::world_cells(
            entry,
            east,
            entry.selector.appearance.ok_or("eastern marker")?,
            1,
            &objects,
            &[],
            None,
            &cells,
        );
    }
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let at = entry.selector.position.ok_or("entry")?;
    let target = marker.selector.position.ok_or("marker")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for callback in [entry, marker] {
        let appearance = callback.selector.appearance.ok_or("appearance")?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let mut sectors: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let max_x = at.x
        + if entry.key.contains("/insectoid_invasion/") {
            14
        } else {
            3
        };
    let max_y = entry
        .effects
        .encounter
        .as_ref()
        .map_or(at.y + 2, |encounter| (at.y + 2).max(encounter.room_max.y));
    for y in at.y - 2..=max_y {
        for x in at.x - 2..=max_x {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (x, y) == (at.x, at.y) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: Default::default(),
                });
            }
            if (x, y) == (target.x, target.y) && (x, y) != (at.x, at.y) {
                items.push(sector::Item {
                    palette: 2,
                    depth: 0,
                    attrs: Default::default(),
                });
            }
            sectors
                .entry((at.floor, x / 32, y / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items,
                });
        }
    }
    data.manifest.world = bundle::Extent {
        min_x: at.x - 2,
        min_y: at.y - 2,
        max_x: max_x + 1,
        max_y: max_y + 1,
        floors: vec![at.floor],
    };
    let mut sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|t| (t.y, t.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) async fn live_creatures(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
) -> TestResult<Vec<crate::foundation::ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter_map(|(actor, _, session)| {
            (session.is_none()
                && runtime
                    .companion_snapshot(actor)
                    .is_ok_and(|s| s.state.policy.definition_key == creature))
            .then_some(actor)
        })
        .collect())
}

pub(super) async fn kill_actor(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    target: crate::foundation::ExactActorRef,
    command: u64,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        let definition = runtime
            .companion_snapshot(target)
            .map_err(debug)?
            .state
            .policy
            .definition_key
            .clone();
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: definition.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"m31-native-island-death",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
    }
    // Returning a semantic pass schedules its holder asynchronously. Service
    // ready demand on a bounded retry, as the production owner loop does.
    let mut drains = 0;
    while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
        drains += 1;
        assert!(drains < 8, "the native journal death settles");
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    Ok(())
}

pub(super) async fn step(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    // Each helper call reconnects the transport, retaining the real movement cadence.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    Ok(())
}
