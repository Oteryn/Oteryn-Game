//! Literal USE against retained palace B3 tiles and the actual authored bindings.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};

fn stage(name: &str) -> TestResult<&str> {
    name.rsplit('-').next().ok_or_else(|| "Asura stage".into())
}
fn bindings() -> TestResult<Bindings> {
    let b = Bindings::parse(include_bytes!(
        "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    ))?;
    b.verify_embedded_recipe_evidence()?;
    Ok(b)
}
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let key = format!("oteryn:quest-callback/engenc/asura/{}/entry", stage(name)?);
    bindings()?
        .world_callbacks()
        .find(|c| c.key == key)
        .cloned()
        .ok_or_else(|| "Asura entry".into())
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    Ok(callback(name)?
        .effects
        .encounter
        .as_ref()
        .ok_or("encounter")?
        .players[0]
        .from)
}
pub(super) fn catalogue(name: &str) -> TestResult<QuestStateCatalogue> {
    let mut b = bindings()?;
    // Isolate each real caller at its authored predecessor, with no fabricated receipts.
    let predecessor = match stage(name)? {
        "s2" => "oteryn:quest-progress/authored/asura_palace_quest/s1",
        "s3" => "oteryn:quest-progress/engenc/asura/s2/opened",
        "s4" => "oteryn:quest-progress/engenc/asura/s3/opened",
        _ => return Err("Asura stage".into()),
    };
    b.quests[0]
        .tracks
        .iter_mut()
        .find(|t| t.key == predecessor)
        .ok_or("predecessor")?
        .initial = 1;
    Ok(b.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}
pub(super) fn world(
    native: &NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let cb = callback(name)?;
    let at = cb.selector.position.ok_or("entry position")?;
    let staging = start(name)?;
    let (bytes, region, bounds, ids) = match stage(name)? {
        "s2" => (
            include_bytes!("../../../../../../content/world/placements/region-z03-x128-y127.b3")
                .as_slice(),
            (3, 128, 127),
            (32958, 32674, 32961, 32677),
            vec![923, 21973],
        ),
        "s3" => (
            include_bytes!("../../../../../../content/world/placements/region-z02-x128-y127.b3")
                .as_slice(),
            (2, 128, 127),
            (32952, 32676, 32962, 32683),
            vec![923],
        ),
        "s4" => (
            include_bytes!("../../../../../../content/world/placements/region-z02-x128-y127.b3")
                .as_slice(),
            (2, 128, 127),
            (32942, 32675, 32950, 32683),
            vec![923],
        ),
        _ => return Err("Asura stage".into()),
    };
    let mut crops = vec![NativeCrop {
        bytes,
        region,
        bounds,
        start: (staging.x, staging.y),
        expected: vec![ExpectedTile {
            position: (at.x, at.y),
            ids,
            counted_entry: None,
        }],
    }];
    if stage(name)? == "s2" {
        crops.push(NativeCrop {
            bytes: include_bytes!(
                "../../../../../../content/world/placements/region-z04-x128-y127.b3"
            ),
            region: (4, 128, 127),
            bounds: (32957, 32673, 32965, 32680),
            start: (staging.x, staging.y),
            expected: vec![ExpectedTile {
                position: (32963, 32677),
                ids: vec![936, 5736],
                counted_entry: None,
            }],
        });
    }
    let world = native_scene_with_appearances(native, crops, &[], &[1675, 5737])?;
    for defender in &cb.effects.encounter.as_ref().ok_or("encounter")?.monsters {
        assert!(
            world.enterable(crate::map::overlay::TilePos {
                x: defender.position.x,
                y: defender.position.y,
                floor: defender.position.floor,
            }),
            "{} defender tile must retain walkable native geometry",
            defender.creature
        );
    }
    Ok(world)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let cb = callback(name)?;
    let encounter = cb.effects.encounter.as_ref().ok_or("encounter")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    owner
        .runtime
        .lock()
        .await
        .install_companion_policies(native.policies().clone())
        .map_err(debug)?;
    let selector = cb.selector.native.as_ref().ok_or("native selector")?;
    let key = oteryn_world_bundle::bundle::placement_key(
        selector.position.floor,
        selector.position.x,
        selector.position.y,
        selector.ordinal,
    )
    .ok_or("placement")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
        &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        },
    )
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let reply =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await;
    if reply.is_err() {
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(&cb.key)
        .fetch_one(&h.pool)
        .await?;
        eprintln!(
            "M30_ASURA_DISPATCH_UNRESOLVED receipts={receipts} creatures={}",
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .len()
        );
    }
    let reply = reply?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let roster = {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(encounter.players[0].to)
        );
        let creatures = runtime.visible_entities().creatures;
        assert_eq!(creatures.len(), 2);
        for creature in &creatures {
            let snapshot = runtime.companion_snapshot(creature.actor).map_err(debug)?;
            assert_eq!(
                snapshot.state.policy.definition_key,
                encounter.monsters[0].creature
            );
        }
        creatures.iter().map(|c| c.actor).collect::<Vec<_>>()
    };
    let reply = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .iter()
            .map(|c| c.actor)
            .collect::<Vec<_>>(),
        roster
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&cb.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    let door_key = format!("oteryn:quest-callback/engenc/asura/{}/door", stage(name)?);
    let door = bindings()?
        .world_callbacks()
        .find(|callback| callback.key == door_key)
        .cloned()
        .ok_or("chamber door")?;
    let selector = door.selector.native.as_ref().ok_or("native door")?;
    let mut next_command = walk(
        owner,
        admitted,
        name,
        &[(selector.position.x - 1, selector.position.y)],
        command + 1,
    )
    .await?;
    let key = oteryn_world_bundle::bundle::placement_key(
        selector.position.floor,
        selector.position.x,
        selector.position.y,
        selector.ordinal,
    )
    .ok_or("door placement")?;
    let map = owner.bundle_map().ok_or("map")?;
    let revision = map.facts.object_revision(key);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
        &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: revision,
        },
    )
    .map_err(debug)?;
    let reply =
        super::quest_world_callbacks::wire(owner, admitted, next_command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Rejected,
        "the admitted participant must defeat both defenders before opening the door"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        revision
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&door_key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 0);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        2
    );
    eprintln!("M30_ASURA_DISPATCH_EARLY_DOOR_REFUSED {}", stage(name)?);
    next_command += 1;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            &[6, 17],
        )
        .ok_or("combat capabilities")?;

    // High-level player fixture bounds test runtime; native defender health,
    // armor, resistances, and all death/quest owners remain unchanged.
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get_mut(&runtime, actor, session)
            .ok_or("combat player")?
            .facts
            .level = 10_000;
    }
    for (index, defender) in encounter.monsters.iter().enumerate() {
        next_command = walk(
            owner,
            admitted,
            name,
            &[
                (defender.position.x - 1, defender.position.y),
                (defender.position.x + 1, defender.position.y),
                (defender.position.x, defender.position.y - 1),
                (defender.position.x, defender.position.y + 1),
            ],
            next_command,
        )
        .await?;
        let target = {
            let runtime = owner.runtime.lock().await;
            let row = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|row| {
                    roster.contains(&row.actor)
                        && runtime.read_actor_position(row.actor).is_ok_and(|at| {
                            at.position() == super::quest_world_callbacks::local(defender.position)
                        })
                })
                .ok_or("admitted defender")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: row.actor.placement_identity(),
                generation: row.generation,
            }
        };
        let payload = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
            .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, next_command, 11, payload).await?;
        assert_eq!(
            reply,
            oteryn_protocol_oteryn::attack::encode_attack_intent_result(
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            )
        );
        next_command += 1;
        let count_track = format!("oteryn:quest-progress/engenc/asura/{}/count", stage(name)?);
        let mut credited = false;
        for _ in 0..128 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            owner.drain_kill_rewards(session).await;
            let count: i64 = sqlx::query_scalar(
                "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)"
            ).bind(fence.character_id.as_bytes().as_slice()).bind(&count_track).fetch_one(&h.pool).await?;
            if count == i64::try_from(index + 1)? {
                credited = true;
                break;
            }
            owner.clock_origin -= std::time::Duration::from_secs(3);
        }
        assert!(
            credited,
            "ordinary combat must credit admitted defender {index}"
        );
        next_command = walk(
            owner,
            admitted,
            name,
            &[(selector.position.x - 1, selector.position.y)],
            next_command,
        )
        .await?;
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, next_command, 2, payload).await?;
        next_command += 1;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            if index == 0 {
                oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
            } else {
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            }
        );
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&door_key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    assert_ne!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        revision
    );
    eprintln!("M30_ASURA_DISPATCH_COMBAT_DOOR_EXECUTED {}", stage(name)?);

    eprintln!("M30_ASURA_DISPATCH_ADMISSION_EXECUTED {}", stage(name)?);
    Ok(())
}

async fn walk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    name: &str,
    goals: &[(u16, u16)],
    mut next_command: u64,
) -> TestResult<u64> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let cb = callback(name)?;
    let encounter = cb.effects.encounter.as_ref().ok_or("encounter")?;
    // Walk through the retained native floor, avoiding the two live defenders.
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    use std::collections::{BTreeSet, VecDeque};
    let scene = world(native, name)?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let origin = Position {
        x: u16::try_from(at.x)?,
        y: u16::try_from(at.y)?,
        floor: i8::try_from(-at.floor)?,
    };
    let mut queue = VecDeque::from([((origin.x, origin.y), Vec::new())]);
    let mut seen = BTreeSet::new();
    let (route, goal) = loop {
        let (at, path) = queue.pop_front().ok_or("native route to chamber door")?;
        if goals.contains(&at) {
            break (path, at);
        }
        if !seen.insert(at) {
            continue;
        }
        for (dx, dy, direction) in [
            (1, 0, StepDirection::East),
            (-1, 0, StepDirection::West),
            (0, 1, StepDirection::South),
            (0, -1, StepDirection::North),
        ] {
            let Some(x) = at.0.checked_add_signed(dx) else {
                continue;
            };
            let Some(y) = at.1.checked_add_signed(dy) else {
                continue;
            };
            let next = (x, y);
            if next.0.abs_diff(origin.x) > 12
                || next.1.abs_diff(origin.y) > 12
                || encounter
                    .monsters
                    .iter()
                    .any(|m| (m.position.x, m.position.y) == next)
                || !scene.enterable(crate::map::overlay::TilePos {
                    x: next.0,
                    y: next.1,
                    floor: origin.floor,
                })
            {
                continue;
            }
            let mut path = path.clone();
            path.push(direction);
            queue.push_back((next, path));
        }
    };

    for direction in route {
        super::quest_world_callbacks::wire(
            owner,
            admitted,
            next_command,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
        )
        .await?;
        next_command += 1;
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(Position {
            x: goal.0,
            y: goal.1,
            floor: origin.floor,
        })
    );
    Ok(next_command)
}

fn run(stage: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(stage, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Asura dispatch thread failed")?
        .map_err(Into::into)
}
#[test]
fn engenc_postgres_native_destroyer_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s2",
        Position {
            x: 32959,
            y: 32675,
            floor: -3,
        },
    )
}
#[test]
fn engenc_postgres_native_fire_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s3",
        Position {
            x: 32960,
            y: 32679,
            floor: -2,
        },
    )
}
#[test]
fn engenc_postgres_native_energy_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s4",
        Position {
            x: 32948,
            y: 32679,
            floor: -2,
        },
    )
}
