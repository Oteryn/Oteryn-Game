//! QENGINE-1 source access through the literal production command dispatcher.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::foundation::{
    CharacterId, CharacterLease, ExactActorRef, GameSessionId, MovementFacing,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

async fn use_lever(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    trigger: Position,
    revision: u64,
) -> TestResult<UseDisposition> {
    use oteryn_protocol_oteryn::world_object::*;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let key = crate::map::view::placement_key(tile(trigger), 1).ok_or("key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: revision,
    };
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    use super::quest_world_callbacks::local;
    if callback.effects.encounter.is_some() {
        use crate::gameplay_transport::capabilities::SelectedCapabilities;
        let mut supported = admitted
            .continuity
            .selected_capabilities
            .as_slice()
            .to_vec();
        supported.extend([4, 6, 11, 15, 18]);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &supported)
                .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(11));
    }
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let key = crate::map::view::placement_key(tile(trigger), 1).ok_or("key")?;
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    if callback.key.contains("waterfall-") {
        let (x, y, floor) = if callback.key.ends_with("cliff") {
            (32968, 32626, -5)
        } else if callback.key.ends_with("cave-entry") {
            (32968, 32631, -8)
        } else if callback.key.ends_with("cave-exit") {
            (32971, 32620, -8)
        } else {
            (32968, 32630, -7)
        };
        assert_eq!(
            callback.teleport.unwrap_or(trigger),
            Position { x, y, floor },
            "pinned source destination"
        );
        step(owner, admitted, 1, StepDirection::East).await?;
        let snapshot = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            snapshot.position(),
            local(callback.teleport.unwrap_or(trigger))
        );
        let expected_facing = if callback.key.ends_with("cave-entry") {
            MovementFacing::South
        } else {
            MovementFacing::East
        };
        assert_eq!(snapshot.facing(), Some(expected_facing));
        step(owner, admitted, 1, StepDirection::East).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?,
            snapshot,
            "transport replay must not move or repeat the splash"
        );
        let effects = owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0);
        if callback.key.ends_with("cliff") || callback.key.ends_with("splash") {
            let pos = callback.teleport.unwrap_or(trigger);
            assert_eq!(
                effects
                    .iter()
                    .filter(|(_, at, effect)| *at == tile(pos) && *effect == 54)
                    .count(),
                1
            );
        } else {
            assert!(effects.is_empty());
        }
    } else if callback.key.ends_with("thais-lighthouse-plate") {
        step(owner, admitted, 1, StepDirection::East).await?;
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(
            map.facts
                .base_entry(tile(trigger), 1, 1)
                .ok_or("plate")?
                .appearance_id,
            430
        );
        let stairs = tile(callback.passages[0].position);
        assert_eq!(
            map.facts
                .base_entry(stairs, 1, 2)
                .ok_or("stairs")?
                .appearance_id,
            7767
        );
        {
            let mut runtime = owner.runtime.lock().await;
            let session = GameSessionId::decode(&id(99)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let partner = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    partner,
                    local(Position {
                        y: stairs.y - 1,
                        x: stairs.x,
                        floor: stairs.floor,
                    }),
                )
                .map_err(debug)?;
            let expected = runtime.read_actor_position(partner).map_err(debug)?;
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let proof = crate::map::floor::prepare_bundle_step(
                &runtime,
                owner.movement_cells,
                partner,
                session,
                expected,
                crate::movement::CardinalStep::South,
                false,
                &closed,
            )
            .map_err(debug)?;
            let landed = runtime.commit_source_step(proof).map_err(debug)?;
            assert_eq!(
                landed.position(),
                local(Position {
                    x: stairs.x,
                    y: stairs.y,
                    floor: stairs.floor - 1
                }),
                "the opened stairs must actually descend"
            );
        }
        step(owner, admitted, 2, StepDirection::East).await?;
        assert_eq!(
            map.facts
                .base_entry(tile(trigger), 1, 1)
                .ok_or("plate")?
                .appearance_id,
            431
        );
        assert_eq!(
            map.facts
                .base_entry(stairs, 1, 2)
                .ok_or("stairs")?
                .appearance_id,
            429
        );
        assert_eq!(
            map.facts.object_revision(key),
            4,
            "native plate animation plus quest passage callback on each ingress/egress"
        );
    } else if let Some(cycle) = &callback.effects.item_cycle {
        assert_eq!((cycle.remove, cycle.create), (1270, 1025));
        assert_eq!(
            cycle.position,
            Position {
                x: 32566,
                y: 32119,
                floor: -7
            }
        );
        assert_eq!(
            use_lever(owner, admitted, 1, trigger, 0).await?,
            UseDisposition::Committed
        );
        let map = owner.bundle_map().ok_or("map")?;
        let original = crate::map::view::placement_key(tile(cycle.position), 1).ok_or("wall")?;
        assert!(map.facts.base_hidden(original));
        assert!(
            !map.facts
                .quest_callback_blocked(map.overlay.base(), tile(cycle.position), true)
        );
        assert!(map.facts.movement_items(tile(cycle.position)).is_empty());
        assert_eq!(
            use_lever(owner, admitted, 2, trigger, 1).await?,
            UseDisposition::Committed
        );
        let map = owner.bundle_map().ok_or("map")?;
        assert!(
            map.facts.base_hidden(original),
            "1270 must never be restored"
        );
        let added = map.facts.movement_items(tile(cycle.position));
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].facts.appearance_id, 1025);
        assert!(
            map.facts
                .quest_callback_blocked(map.overlay.base(), tile(cycle.position), false)
        );
        assert_eq!(
            use_lever(owner, admitted, 2, trigger, 1).await?,
            UseDisposition::Committed,
            "duplicate transport replay"
        );
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .movement_items(tile(cycle.position))
                .len(),
            1
        );
    } else if let Some(pair) = &callback.effects.portals {
        assert_eq!(
            use_lever(owner, admitted, 1, trigger, 0).await?,
            UseDisposition::Committed
        );
        for portal in &pair.portals {
            let map = owner.bundle_map().ok_or("map")?;
            assert!(
                crate::map::floor::is_teleport(
                    owner.movement_cells,
                    crate::content::LogicalCell {
                        x: i32::from(portal.position.x),
                        y: i32::from(portal.position.y),
                        z: -i32::from(portal.position.floor)
                    }
                ),
                "native monsters must avoid a dynamic teleport"
            );
            assert_eq!(
                map.facts
                    .quest_portal_destination(tile(portal.position), owner.owner_now().get()),
                Some(tile(portal.destination))
            );
            assert_eq!(map.facts.movement_items(tile(portal.position)).len(), 1);
        }
        assert_eq!(
            use_lever(owner, admitted, 2, trigger, 0).await?,
            UseDisposition::StaleState
        );
        if pair.lifetime_us.is_some() {
            let map = owner.bundle_map().ok_or("map")?;
            let due = map
                .facts
                .next_movement_decay_time()
                .ok_or("portal deadline")?;
            assert_eq!(map.facts.expire_movement_items(due - 1), 0);
            assert_eq!(
                map.facts.expire_movement_items(due),
                1,
                "one atomic pair expires"
            );
            for portal in &pair.portals {
                assert!(
                    map.facts
                        .quest_portal_destination(tile(portal.position), due)
                        .is_none()
                );
                assert!(map.facts.movement_items(tile(portal.position)).is_empty());
            }
            assert_eq!(
                use_lever(owner, admitted, 3, trigger, 1).await?,
                UseDisposition::Committed,
                "expired pair's lever can reset"
            );
            return Ok(());
        }
        relocate(
            owner,
            actor,
            admitted.game_session_id,
            Position {
                x: 32231,
                y: 32276,
                floor: -9,
            },
        )
        .await?;
        step(owner, admitted, 3, StepDirection::East).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(pair.portals[0].destination)
        );
        step(owner, admitted, 4, StepDirection::South).await?;
        step(owner, admitted, 5, StepDirection::South).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(pair.portals[1].destination)
        );
        relocate(
            owner,
            actor,
            admitted.game_session_id,
            Position {
                x: trigger.x - 1,
                ..trigger
            },
        )
        .await?;
        assert_eq!(
            use_lever(owner, admitted, 6, trigger, 1).await?,
            UseDisposition::Committed
        );
        for portal in &pair.portals {
            let map = owner.bundle_map().ok_or("map")?;
            assert_eq!(
                map.facts
                    .quest_portal_destination(tile(portal.position), owner.owner_now().get()),
                None
            );
            assert!(map.facts.movement_items(tile(portal.position)).is_empty());
        }
    } else if callback.effects.encounter.is_some() {
        encounter(owner, admitted, actor, callback, trigger).await?;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        receipts_before,
        "volatile access must not mint quest progress"
    );
    Ok(())
}

async fn encounter(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    use super::quest_world_callbacks::local;
    let encounter = callback.effects.encounter.as_ref().ok_or("encounter")?;
    assert_eq!(
        (
            encounter.minimum_level,
            encounter.players.len(),
            encounter.monsters.len()
        ),
        (100, 4, 6)
    );
    let key = crate::map::view::placement_key(tile(trigger), 1).ok_or("key")?;
    let mut members = vec![(actor, admitted.game_session_id)];
    set_levels(owner, &members, 100).await?;
    assert_eq!(
        use_lever(owner, admitted, 1, trigger, 0).await?,
        UseDisposition::Rejected,
        "missing party"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        for (n, player) in encounter.players.iter().enumerate().skip(1) {
            let session = GameSessionId::decode(&id(70 + u8::try_from(n)?)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let who = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(who, local(player.from))
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    who,
                    session,
                    CharacterLease::new(
                        CharacterId::decode(&id(80 + u8::try_from(n)?)).map_err(debug)?,
                        1,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            members.push((who, session));
        }
    }
    set_levels(owner, &members, 99).await?;
    assert_eq!(
        use_lever(owner, admitted, 2, trigger, 0).await?,
        UseDisposition::Rejected,
        "under-level member"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    set_levels(owner, &members, 100).await?;
    assert_eq!(
        use_lever(owner, admitted, 3, trigger, 0).await?,
        UseDisposition::Rejected,
        "missing native monster policy"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
    }
    let blocker = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .admit_encounter_actor(
                &encounter.monsters[0].creature,
                1,
                local(encounter.players[0].to),
            )
            .map_err(debug)?
    };
    assert_eq!(
        use_lever(owner, admitted, 4, trigger, 0).await?,
        UseDisposition::NotPossible,
        "occupied landing"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(blocker)
        .map_err(debug)?;
    let mut fillers = Vec::new();
    {
        let mut runtime = owner.runtime.lock().await;
        for _ in 0..7 {
            fillers.push(
                runtime
                    .admit_encounter_actor(
                        &encounter.monsters[0].creature,
                        1,
                        local(Position {
                            x: trigger.x + 1,
                            ..trigger
                        }),
                    )
                    .map_err(debug)?,
            );
        }
    }
    let before = owner.runtime.lock().await.encounter_fixture_snapshot();
    assert_eq!(
        use_lever(owner, admitted, 5, trigger, 0).await?,
        UseDisposition::Rejected,
        "sixth monster exceeds remaining slots"
    );
    assert_eq!(
        owner.runtime.lock().await.encounter_fixture_snapshot(),
        before,
        "failed spawn must roll back every native slot and player"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    {
        let mut runtime = owner.runtime.lock().await;
        for filler in fillers {
            runtime.remove_encounter_actor(filler).map_err(debug)?;
        }
    }
    assert_eq!(
        use_lever(owner, admitted, 6, trigger, 0).await?,
        UseDisposition::Committed
    );
    {
        let runtime = owner.runtime.lock().await;
        for ((who, _), player) in members.iter().zip(&encounter.players) {
            assert_eq!(
                runtime.read_actor_position(*who).map_err(debug)?.position(),
                local(player.to)
            );
        }
        let census = runtime.positioned_melee_census().map_err(debug)?;
        for monster in &encounter.monsters {
            let found = census
                .iter()
                .filter(|(_, pos, session)| {
                    session.is_none() && pos.position() == local(monster.position)
                })
                .collect::<Vec<_>>();
            assert_eq!(found.len(), 1);
            assert_eq!(
                runtime
                    .companion_policy(&monster.creature)
                    .map_err(debug)?
                    .definition_key,
                monster.creature
            );
        }
    }
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        1
    );
    let before = owner.runtime.lock().await.encounter_fixture_snapshot();
    assert_eq!(
        use_lever(owner, admitted, 6, trigger, 0).await?,
        UseDisposition::Committed,
        "encounter command replay"
    );
    assert_eq!(
        owner.runtime.lock().await.encounter_fixture_snapshot(),
        before
    );
    Ok(())
}

async fn set_levels(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    members: &[(ExactActorRef, GameSessionId)],
    last_level: u32,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    for (n, &(who, session)) in members.iter().enumerate() {
        states.actors.retain(|(a, _, _)| *a != who);
        let mut facts = crate::gameplay_transport::actor_spell::tests::FACTS;
        facts.level = if n + 1 == members.len() {
            last_level
        } else {
            100
        };
        assert!(
            states
                .initialize(&runtime, who, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
    }
    Ok(())
}

// Fixture setup between disconnected source floors; traversal under test always uses STEP.
async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    at: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells,
        actor,
        session,
        expected,
        super::quest_world_callbacks::local(at),
        false,
        &closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

fn scenario(name: &str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let task = name.to_owned();
    std::thread::Builder::new()
        .name(task.clone())
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(&task, trigger, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "quest access scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn qengine_thais_plate_step_out_restores_stairs() -> TestResult {
    scenario(
        "thais-lighthouse-plate",
        Position {
            x: 32225,
            y: 32268,
            floor: -9,
        },
    )
}

#[test]
fn qengine_thais_portal_pair_teleports_and_removes_atomically() -> TestResult {
    scenario(
        "thais-lighthouse-portals",
        Position {
            x: 32225,
            y: 32285,
            floor: -10,
        },
    )
}

#[test]
fn qengine_triangle_tower_asymmetric_item_cycle() -> TestResult {
    scenario(
        "triangle-tower-lever",
        Position {
            x: 32573,
            y: 32121,
            floor: -7,
        },
    )
}

#[test]
fn qengine_thais_portal_pair_expires_atomically() -> TestResult {
    scenario(
        "thais-lighthouse-portals-expiry",
        Position {
            x: 32225,
            y: 32285,
            floor: -10,
        },
    )
}

#[test]
fn qengine_waterfall_facing_and_splash() -> TestResult {
    for (name, x, y, floor) in [
        ("waterfall-cliff", 32966, 32626, -5),
        ("waterfall-splash", 32968, 32630, -7),
        ("waterfall-cave-entry", 32968, 32629, -7),
        ("waterfall-cave-exit", 32967, 32630, -8),
    ] {
        scenario(name, Position { x, y, floor })?;
    }
    Ok(())
}

#[test]
fn qengine_annihilator_party_encounter_fail_closed() -> TestResult {
    scenario(
        "annihilator-entry",
        Position {
            x: 33226,
            y: 31671,
            floor: -13,
        },
    )
}
