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
    if callback.effects.outfit_reward.is_some() {
        return wardrobe(owner, admitted, h, callback, trigger).await;
    }
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
    let result = (|| -> TestResult {
        std::thread::Builder::new()
            .name(task.clone())
            .stack_size(256 << 20)
            .spawn(move || {
                super::quest_world_callbacks::scenario(&task, trigger, false)
                    .map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "quest access scenario panicked")??;
        Ok(())
    })();
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
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

#[test]
fn qengine_l1_world_warmaster_base_wardrobe() -> TestResult {
    scenario(
        "l1-warmaster-wardrobe",
        Position {
            x: 33053,
            y: 31020,
            floor: -7,
        },
    )
}

#[test]
fn qengine_l1_world_wayfarer_base_wardrobe() -> TestResult {
    scenario(
        "l1-wayfarer-wardrobe",
        Position {
            x: 33073,
            y: 31173,
            floor: -8,
        },
    )
}

// Wardrobe USE borrows the same grant participant as NPC claims, and then the
// ordinary appearance command proves the resulting account fact is selectable.
async fn wardrobe(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
    use oteryn_protocol_oteryn::appearance::{Change, ChangeResult, Selection};
    let reward = callback
        .effects
        .outfit_reward
        .clone()
        .ok_or("wardrobe outfit")?;
    let seed = RewardCallback {
        world_event: None,
        repeatable: false,
        key: callback.key.clone(),
        quest: callback.quest.clone(),
        npc: "oteryn:npc.fixture".into(),
        keywords: vec!["fixture".into()],
        topic: 0,
        next_topic: 0,
        storage: callback.storage.clone(),
        take: vec![],
        present: vec![],
        give: None,
        outfit: Some(reward.clone()),
        achievement: None,
        reply: vec![],
        sources: vec![],
    };
    super::qengine_rewards::seed(owner, admitted, &seed, 9000).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("appearance capability")?;
    assert!(admitted.continuity.selected_capabilities.contains(25));
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
            .fetch_one(&h.pool)
            .await?;
    let context = {
        let runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        crate::durability::quest_world_callback::QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(
                owner.bundle_map().ok_or("map")?.overlay.base(),
            ),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .revision(),
        }
    };
    let session = admitted.game_session_id;
    let request = |ordinal| -> TestResult<_> {
        Ok(crate::durability::quest_state::QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: crate::durability::quest_state::QuestCause::Use(
                crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(ordinal).map_err(debug)?,
                ),
            ),
        })
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    {
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request(9001)?,
                owner.quest_catalogue.clone().ok_or("catalogue")?,
                &permit,
                context.clone(),
                None,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                result,
                crate::durability::quest_state::QuestTransitionOutcome::Refused(_)
            ),
            "a fresh wardrobe grant requires the admitted cosmetics participant"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_appearance_unlock_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        before,
        "refused fresh admission has no value effect"
    );
    for state in &callback.storage {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, owner.character, &state.track).await?,
            state.minimum
        );
    }
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        use_lever(owner, admitted, command, trigger, 0).await?,
        UseDisposition::Committed
    );
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after, before + 1);
    // A historical replay reads its immutable receipt even without a new grant
    // participant, as when admission is reconciling after a generation change.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    {
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request(command)?,
                owner.quest_catalogue.clone().ok_or("catalogue")?,
                &permit,
                context,
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            result,
            crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
        ));
    }

    for state in &callback.storage {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, owner.character, &state.track).await?,
            state.set
        );
    }
    assert_eq!(
        use_lever(owner, admitted, command, trigger, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_appearance_unlock_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        after
    );
    let next = admitted.continuity.next_command_id;
    assert_eq!(
        use_lever(owner, admitted, next, trigger, 0).await?,
        UseDisposition::StaleState
    );
    let next = admitted.continuity.next_command_id;
    assert_eq!(
        use_lever(owner, admitted, next, trigger, 1).await?,
        UseDisposition::Rejected
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_appearance_unlock_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        after
    );
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("wardrobe catalogue")?;
    let option = observed
        .catalogue
        .outfits
        .iter()
        .find(|o| o.key == reward.outfit)
        .ok_or("wardrobe outfit is missing")?;
    assert_eq!(option.permitted_addons, 0);
    assert_eq!(
        option.look_type,
        if reward.outfit.ends_with("warmaster") {
            336
        } else {
            366
        }
    );
    let change = Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: Selection {
            outfit_key: reward.outfit,
            colours: [0; 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let next = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        next,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        ChangeResult::decode(&result).map_err(debug)?,
        ChangeResult::Committed(_)
    ));
    Ok(())
}

#[test]
fn wevents_postgres_spring_native_nest_use() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("spring-native-use", || {
        super::quest_world_callbacks::scenario(
            "spring-nest-1",
            Position {
                x: 32344,
                y: 31604,
                floor: -6,
            },
            false,
        )
    });
    eprintln!(
        "WEVENTS PG EXECUTED: spring native USE duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn spring_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    super::qengine_rewards::spring_fixture(owner, admitted, h, false).await?;
    assert_eq!(
        use_lever(owner, admitted, 1, trigger, 0).await?,
        UseDisposition::Committed
    );
    let credited: i64 = sqlx::query_scalar("SELECT credited FROM game_world_timed_participants WHERE world_id=encode($1,'hex')::uuid AND event_key='spring_into_life'")
        .bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(credited, 1);
    Ok(())
}

#[test]
fn wevents_postgres_insect_native_water_use() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("insect-native-use", || {
        for (name, x, y) in [
            ("insect-restoration-water", 32206, 32057),
            ("insect-restoration-1", 32205, 32059),
            ("insect-restoration-2", 32206, 32060),
            ("insect-restoration-3", 32206, 32061),
        ] {
            super::quest_world_callbacks::scenario(name, Position { x, y, floor: -7 }, false)?;
        }
        Ok(())
    });
    eprintln!(
        "WEVENTS PG EXECUTED: insect native USE duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn insect_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    let at = 1_934_078_400_000_i64;
    let epoch = (1..=4095)
        .find(|epoch| {
            crate::durability::world_event::changes::roll(owner.world_id, *epoch, at)
                .is_ok_and(|d| d.timed.insectoid)
        })
        .ok_or("insect selected day")?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'insect-native-use',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4::double precision/1000),'2031-04')")
        .bind(owner.world_id.as_bytes().as_slice()).bind(i64::try_from(epoch)?).bind([7u8;32].as_slice()).bind(at).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2)")
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(i64::try_from(epoch)?)
        .execute(&h.pool)
        .await?;
    owner.reset_epoch = epoch;
    owner
        .root
        .set_world_event_clock_for_test(Some(at))
        .map_err(debug)?;
    let fire = match (trigger.x, trigger.y) {
        (32205, 32059) => Some(1),
        (32206, 32060) => Some(2),
        (32206, 32061) => Some(3),
        _ => None,
    };
    if fire.is_some() {
        use crate::durability::world_event::changes::timed::{self, EventKind, Input};
        let mut tx = h.pool.begin().await?;
        let mut state = timed::poll(
            &mut tx,
            owner.world_id,
            epoch,
            EventKind::InsectoidInvasion,
            at,
        )
        .await?
        .ok_or("insect fixture state")?;
        for _ in 0..3 {
            state
                .advance(EventKind::InsectoidInvasion, at, Input::Fill(0), 0)
                .map_err(debug)?;
        }
        sqlx::query("UPDATE game_world_timed_events SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND event_key='insectoid_invasion'")
            .bind(owner.world_id.as_bytes().as_slice()).bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
        tx.commit().await?;
        assert_eq!(
            use_lever(owner, admitted, 1, trigger, 0).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            use_lever(owner, admitted, 2, trigger, 1).await?,
            UseDisposition::Rejected
        );
    } else {
        for sequence in 1..=3 {
            assert_eq!(
                use_lever(owner, admitted, sequence, trigger, sequence - 1).await?,
                UseDisposition::Committed
            );
        }
        assert_eq!(
            use_lever(owner, admitted, 4, trigger, 3).await?,
            UseDisposition::Rejected
        );
    }
    let state = owner
        .root
        .timed_world_event(
            owner.world_id,
            epoch,
            crate::durability::world_event::changes::timed::EventKind::InsectoidInvasion,
        )
        .await?
        .ok_or("insect state")?;
    assert_eq!(
        state.lifecycle.objects.get(&0).ok_or("bucket")?.units(),
        if fire.is_some() { 2 } else { 3 }
    );
    if let Some(member) = fire {
        assert!(!state.lifecycle.objects.get(&member).ok_or("fire")?.lit());
    }
    Ok(())
}

#[test]
fn wevents_devovorga_native_essence_admission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    assert!(
        oteryn_gameplay_durability::GAME_MIGRATOR
            .iter()
            .any(|migration| migration.version == 1127),
        "native selector rewards require migration 1127"
    );
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("essence-native-admission", || {
        super::quest_world_callbacks::scenario(
            "devovorga-essence-0",
            Position {
                x: 32969,
                y: 31572,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "WEVENTS PG EXECUTED: Essence native admission duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn essence_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    trigger: Position,
    actor: ExactActorRef,
) -> TestResult {
    use super::quest_world_callbacks::local;
    let at = 1_934_078_400_000_i64;
    let epoch = owner.reset_epoch;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'essence-native',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4::double precision/1000),'2031-04')")
        .bind(owner.world_id.as_bytes().as_slice()).bind(i64::try_from(epoch)?).bind([7u8;32].as_slice()).bind(at).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2)")
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(i64::try_from(epoch)?)
        .execute(&h.pool)
        .await?;
    owner
        .root
        .set_world_event_clock_for_test(Some(at))
        .map_err(debug)?;
    assert_eq!(
        use_lever(owner, admitted, 1, trigger, 0).await?,
        UseDisposition::Rejected
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Essence fence")?
        .ok_or("Essence active fence")?;
    let offering = oteryn_gameplay_quests::quest::world_callbacks::Offering {
        position: trigger,
        item: "oteryn:item.tibia.i11666".into(),
        count: 1,
    };
    for serial in [880, 881] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &offering,
            serial,
            true,
        )
        .await?;
    }
    let amount = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11666' AND lifecycle=1",
        )
    };
    assert_eq!(amount().fetch_one(&h.pool).await?, 2);
    let room = callback.effects.encounter.as_ref().ok_or("Essence room")?;
    let map = owner.bundle_map().ok_or("map")?;
    let lease = map
        .facts
        .reserve_encounter_room(map.overlay.base(), tile(room.room_min), tile(room.room_max))
        .ok_or("room lease")?;
    assert!(matches!(
        use_lever(owner, admitted, 2, trigger, 0).await?,
        UseDisposition::Occupied | UseDisposition::NotPossible
    ));
    assert_eq!(amount().fetch_one(&h.pool).await?, 2);
    lease.release();
    let admission = use_lever(owner, admitted, 3, trigger, 0).await?;
    let remaining = amount().fetch_one(&h.pool).await?;
    eprintln!(
        "Essence native paid use={admission:?} remaining_fee={remaining} pending={}",
        owner.has_pending_quest_callback(admitted.game_session_id)
    );
    assert_eq!(admission, UseDisposition::Committed);
    assert_eq!(remaining, 0);
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(room.players[0].to)
    );
    assert!(
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .iter()
            .any(|(actor, at, session)| session.is_none()
                && at.position() == local(room.monsters[0].position)
                && runtime
                    .companion_snapshot(*actor)
                    .is_ok_and(|snapshot| snapshot.maximum_health == 7000))
    );
    drop(runtime);
    let map = owner.bundle_map().ok_or("map")?;
    assert!(
        !map.facts
            .quest_callback_blocked(map.overlay.base(), tile(room.room_min), false)
    );
    Ok(())
}

#[test]
fn wevents_devovorga_native_season_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    assert!(
        oteryn_gameplay_durability::GAME_MIGRATOR
            .iter()
            .any(|migration| migration.version == 1127),
        "native selector rewards require migration 1127"
    );
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("devovorga-native-reward", || {
        super::quest_world_callbacks::scenario(
            "devovorga-season-reward",
            Position {
                x: 32970,
                y: 31580,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "WEVENTS PG EXECUTED: devovorga native reward duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn devovorga_reward(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    use crate::durability::world_event::changes::timed::{self, EventKind, Input, Phase};
    let at: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM '2031-09-02 06:00 Europe/Berlin'::timestamptz)*1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'devovorga-reward',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000),'2031-09')")
        .bind(owner.world_id.as_bytes().as_slice()).bind([7u8;32].as_slice()).bind(at-86_400_000).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    owner.reset_epoch = 1;
    owner
        .root
        .set_world_event_clock_for_test(Some(at))
        .map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "reward fence")?
        .ok_or("reward session")?;
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await?
        .ok_or("reward progression")?;
    let context = before.context.clone();
    let baseline = before.total_experience.get();
    let thresholds: [LevelThreshold; 160] = (0_u32..160)
        .map(|offset| LevelThreshold {
            level: before.level + offset,
            minimum_experience: ExactI64::new(baseline + i64::from(offset) * 100),
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "fixture threshold count")?;
    let request = ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(228)).map_err(debug)?,
        amount: ExactI64::new(i64::from(160 - before.level) * 100),
        context: context.clone(),
        policy_revision: before.policy_revision.clone(),
        reward_revision: before.reward_revision.clone(),
        policy: FiniteProgressionPolicy {
            context: context.clone(),
            policy_revision: before.policy_revision,
            reward_revision: before.reward_revision,
            death_policy_revision: "death-1".into(),
            declared_difference_revision: context.declaration,
            thresholds,
            terminal_exclusive_experience: ExactI64::new(baseline + 16_000),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    assert!(matches!(
        slot.commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            None
        )
        .await?,
        ExperienceCommitOutcome::Committed(_)
    ));
    drop(slot);

    let mut tx = h.pool.begin().await?;
    let mut state = timed::poll(&mut tx, owner.world_id, 1, EventKind::RiseOfDevovorga, at)
        .await?
        .ok_or("reward season")?;
    for member in [0, 1, 2, 3, 4, 6, 7, 0, 1, 2, 3, 4, 8, 5] {
        state
            .advance(EventKind::RiseOfDevovorga, at, Input::Defeat(member), 0)
            .map_err(debug)?;
    }
    assert_eq!(state.phase, Phase::Victory);
    sqlx::query("UPDATE game_world_timed_events SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND event_key='rise_of_devovorga'").bind(owner.world_id.as_bytes().as_slice()).bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        use_lever(owner, admitted, 1, trigger, 0).await?,
        UseDisposition::Rejected
    );
    sqlx::query("INSERT INTO game_world_timed_participants(world_id,event_key,cycle_id,character_id,credited,reward_ready) SELECT world_id,event_key,cycle_id,encode($2,'hex')::uuid,1,false FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key='rise_of_devovorga'")
        .bind(owner.world_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    assert_eq!(
        use_lever(owner, admitted, 2, trigger, 0).await?,
        UseDisposition::Rejected
    );
    sqlx::query("UPDATE game_world_timed_participants SET reward_ready=true WHERE event_key='rise_of_devovorga'").execute(&h.pool).await?;
    assert_eq!(
        use_lever(owner, admitted, 3, trigger, 0).await?,
        UseDisposition::Committed
    );
    let bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9601' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(bags, 1);
    let achievement:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/i_did_my_part'").fetch_one(&h.pool).await?;
    assert_eq!(achievement, 1);
    assert_eq!(
        use_lever(owner, admitted, 4, trigger, 1).await?,
        UseDisposition::Rejected
    );
    let bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9601' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(bags, 1);
    Ok(())
}
