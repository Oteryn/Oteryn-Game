use super::engenc2_cults_fixtures::kill;
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CarrierError, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    OwnerDamageCommand,
};
use oteryn_gameplay_entities::carrier::NativeSummonAdmissionSpec;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

const EXIT: Position = Position {
    x: 103,
    y: 100,
    floor: -7,
};
const CAPTAIN: Position = Position {
    x: 116,
    y: 112,
    floor: -7,
};
const LANDING: MovementLocalPosition = MovementLocalPosition {
    x: 116,
    y: 117,
    floor: 7,
};
const LION: &str = "oteryn:creature.lion_commander";
const USURPER: &str = "oteryn:creature.usurper_commander";
const DRUME: &str = "oteryn:creature.drume";
const KESAR: &str = "oteryn:creature.kesar";
const TIMEOUT_MS: u32 = 1_200_000;
// Crystal action_drume.lua:12,39-41,130 and items.xml:58504-58507.
// The B3 region z06-x126-y126 retains the bare rope at (32456,32508,-6),
// ordinal 1, one tile west of the first staging position.
pub(super) const ROPE: Position = Position {
    x: 99,
    y: 100,
    floor: -7,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExitCase {
    Timeout,
    Defeat,
    ObstructedDefeat,
    ScheduledTimeout,
}

pub(super) fn bind(callback: &mut Callback, exit: ExitCase) -> TestResult {
    let mut json = serde_json::to_value(&*callback)?;
    if exit == ExitCase::ScheduledTimeout {
        json["selector"] = serde_json::json!({
            "native": {"position": ROPE, "ordinal": 1, "appearance": 31366},
        });
        json["lever"] = serde_json::Value::Null;
    }
    json["effects"]["battlefield"] = serde_json::json!({
        "exit": EXIT,
        "captain": CAPTAIN,
        "boss": {"x":116,"y":113,"floor":-7},
        "timeout_ms":TIMEOUT_MS,
    });
    json["effects"]["encounter"]["phase"] = "drume_skirmish".into();
    let encounter = json["effects"]["encounter"]["monsters"]
        .as_array_mut()
        .ok_or("fixture monsters")?;
    encounter.truncate(6);
    for (index, monster) in encounter.iter_mut().enumerate() {
        *monster = serde_json::json!({
            "creature": if index < 3 { LION } else { USURPER },
            "position": {"x": 110 + index, "y": 114, "floor": -7},
        });
    }
    json["sources"]
        .as_array_mut()
        .ok_or("fixture sources")?
        .push("Crystal 201189f8 the_order_of_lion_quest/action_drume.lua:1-18".into());
    *callback = serde_json::from_value(json)?;
    Ok(())
}

pub(super) async fn admit_second(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    leader: &AdmittedSession,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let member = admit_fixture_player(
        owner,
        harness,
        [60, 63, 71],
        MovementLocalPosition {
            x: 100,
            y: 101,
            floor: 7,
        },
    )
    .await?;
    assert_ne!(member.1, leader.game_session_id);
    Ok(member)
}

async fn admit_fixture_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    [character, account, session]: [u8; 3],
    at: MovementLocalPosition,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let member = crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
        &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
            root: harness.root.clone(),
            node: harness.node.clone(),
            recovery: &harness.recovery,
        },
        character,
        account,
        session,
        now,
    )
    .await?;
    let session = member.current_game_session_id();
    let mut runtime = owner.runtime.lock().await;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, at)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(actor, session, member.current_character_lease())
        .map_err(debug)?;
    Ok((actor, session))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    leader: &AdmittedSession,
    (second, second_session): (ExactActorRef, GameSessionId),
    exit_case: ExitCase,
) -> TestResult {
    let first = leader.runtime_actor.ok_or("first actor")?;
    let actors = [first, second];
    let after_deadline = owner.owner_now().get() + u64::from(TIMEOUT_MS) * 1000 + 1;
    assert_positions(owner, actors, LANDING).await?;
    // Keep the summon master outside the skirmish so master teardown cannot
    // accidentally satisfy the timeout assertion.
    let (master, summoned) = summon(
        owner,
        MovementLocalPosition {
            x: 102,
            y: 102,
            floor: 7,
        },
        MovementLocalPosition { x: 117, ..LANDING },
    )
    .await?;
    if exit_case == ExitCase::ScheduledTimeout {
        // Crystal action_drume.lua:23-34,119-122 schedules this clear at
        // admission, independently of the commander-death callbacks.
        assert_eq!(list(owner, LION).await?.len(), 3);
        assert_eq!(list(owner, USURPER).await?.len(), 3);
        let due = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(after_deadline);
        Box::pin(owner.drain_monster_melee_at(due)).await;
        assert_positions(
            owner,
            actors,
            super::quest_world_callbacks_fixtures::local(EXIT),
        )
        .await?;
        for session in [leader.game_session_id, second_session] {
            assert_feedback(owner, session, "Your time is over.");
        }
        for species in [LION, USURPER, DRUME, KESAR] {
            assert!(list(owner, species).await?.is_empty());
        }
        let runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(summoned));
        assert!(runtime.contains_live_creature(master));
        return Ok(());
    }
    let spawn_boss = exit_case == ExitCase::Timeout;
    if spawn_boss {
        set_captain_tile_passable(owner, 0, false)?;
    }
    let obstruction = if exit_case == ExitCase::ObstructedDefeat {
        let exit = super::quest_world_callbacks_fixtures::local(EXIT);
        Some(
            owner
                .runtime
                .lock()
                .await
                .admit_encounter_actor("oteryn:creature.rat", 20, exit)
                .map_err(debug)?,
        )
    } else {
        None
    };
    let species = if spawn_boss { USURPER } else { LION };
    for command in 100..103 {
        let target = list(owner, species)
            .await?
            .into_iter()
            .next()
            .ok_or("squad member")?;
        kill(owner, leader, target, species, command).await?;
        if command < 102 {
            assert!(list(owner, DRUME).await?.is_empty());
            assert!(list(owner, KESAR).await?.is_empty());
            assert_positions(owner, actors, LANDING).await?;
        }
    }
    assert!(list(owner, species).await?.is_empty());
    if let Some(obstruction) = obstruction {
        // Crystal commander_kills.lua:8-25 and action_drume.lua:23-32
        // dispatch different messages and apply different summon rules.
        assert_positions(owner, actors, LANDING).await?;
        assert_eq!(list(owner, USURPER).await?.len(), 3);
        for session in [leader.game_session_id, second_session] {
            assert!(owner.states.chat.drain(session).is_empty());
        }
        let mut runtime = owner.runtime.lock().await;
        assert!(runtime.contains_live_creature(summoned));
        runtime.remove_encounter_actor(obstruction).map_err(debug)?;
        let states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, after_deadline);
    }
    if spawn_boss {
        assert!(list(owner, DRUME).await?.is_empty());
        assert!(list(owner, KESAR).await?.is_empty());
        {
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let now = owner.owner_now().get();
            crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, now);
            set_captain_tile_passable(owner, 1, true)?;
            crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, now + 1);
            crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, now + 2);
        }
        assert_eq!(list(owner, DRUME).await?.len(), 1);
        let captains = list(owner, KESAR).await?;
        assert_eq!(captains.len(), 1);
        {
            let mut runtime = owner.runtime.lock().await;
            let captain = captains[0];
            let health = runtime.companion_snapshot(captain).map_err(debug)?.health;
            let now_ms = owner.owner_now().get() / 1_000;
            for (actor, session) in [(first, leader.game_session_id), (second, second_session)] {
                let damage = runtime
                    .borrow_exact_actor_commit_at(now_ms)
                    .commit_damage_for_bound_attacker(
                        captain,
                        actor,
                        CommandRef::new(session, CommandId::new(104).map_err(debug)?),
                        0,
                        OwnerDamageCommand {
                            target: KESAR.as_bytes(),
                            occurrence: b"drume-battlefield",
                            binding: b"kesar-immortal",
                            damage: health,
                        },
                    );
                assert_eq!(damage, Err(CarrierError::PlanConflict));
                assert_eq!(
                    runtime.companion_snapshot(captain).map_err(debug)?.health,
                    health
                );
            }
        }
        assert_positions(owner, actors, LANDING).await?;
        let target = list(owner, DRUME)
            .await?
            .into_iter()
            .next()
            .ok_or("Drume")?;
        kill(owner, leader, target, DRUME, 103).await?;
        exercise_busy_timeout(
            owner,
            harness,
            [(first, leader.game_session_id), (second, second_session)],
        )
        .await?;
        for session in [leader.game_session_id, second_session] {
            assert_feedback(owner, session, "Your time is over.");
        }
        assert!(list(owner, LION).await?.is_empty());
        assert!(list(owner, KESAR).await?.is_empty());
        assert_positions(
            owner,
            actors,
            super::quest_world_callbacks_fixtures::local(EXIT),
        )
        .await?;
        assert!(!owner.runtime.lock().await.contains_live_creature(summoned));
    } else {
        for session in [leader.game_session_id, second_session] {
            assert_feedback(owner, session, "You lost the skirmish.");
        }
        assert!(list(owner, USURPER).await?.is_empty());
        assert!(list(owner, DRUME).await?.is_empty());
        assert_positions(
            owner,
            actors,
            super::quest_world_callbacks_fixtures::local(EXIT),
        )
        .await?;
        {
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let tick = crate::gameplay_transport::encounter_phases::tick;
            if exit_case == ExitCase::Defeat {
                tick(&mut runtime, &states, owner.owner_now().get());
            }
            assert!(runtime.contains_live_creature(summoned));
            // Crystal action_drume.lua:23-34,119-122 retains the scheduled
            // clear after commander_kills.lua:14-23 ejects the players.
            tick(&mut runtime, &states, after_deadline + 1);
            assert!(!runtime.contains_live_creature(summoned));
            let ambient = runtime
                .admit_encounter_actor("oteryn:creature.rat", 20, LANDING)
                .map_err(debug)?;
            tick(&mut runtime, &states, after_deadline + 2);
            assert!(runtime.contains_live_creature(ambient));
        }
        for session in [leader.game_session_id, second_session] {
            assert!(owner.states.chat.drain(session).is_empty());
        }
        assert_positions(
            owner,
            actors,
            super::quest_world_callbacks_fixtures::local(EXIT),
        )
        .await?;
    }
    assert!(owner.runtime.lock().await.contains_live_creature(master));
    let exit_effects = owner
        .bundle_map()
        .ok_or("battlefield map")?
        .facts
        .movement_effects_since(0)
        .into_iter()
        .filter(|(_, at, effect)| {
            (at.x, at.y, at.floor) == (EXIT.x, EXIT.y, EXIT.floor) && *effect == 11
        })
        .count();
    assert_eq!(exit_effects, usize::from(!spawn_boss));
    Ok(())
}

async fn exercise_busy_timeout(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    players: [(ExactActorRef, GameSessionId); 2],
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use crate::gameplay_transport::encounter_phases::tick;

    // The caster stays outside the battlefield, so only retirement, not
    // either player's relocation proof, can encounter the reservation.
    let (caster, session) = admit_fixture_player(
        owner,
        harness,
        [61, 64, 72],
        MovementLocalPosition {
            x: 104,
            y: 100,
            floor: 7,
        },
    )
    .await?;
    let lions = list(owner, LION).await?;
    let target = *lions.last().ok_or("surviving lion commander")?;
    let now = owner.owner_now();
    let deadline = now.get() + u64::from(TIMEOUT_MS) * 1000;
    let staged = {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(caster, session)
            .map_err(debug)?;
        let batch = OwnerCombatBatch {
            caster,
            attacker: lease.character_id(),
            current_lease_generation: lease.generation(),
            command: CommandRef::new(session, CommandId::new(105).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "drume-timeout-pending-target".into(),
                revisions: std::array::from_fn(|_| "fixture:1".into()),
            },
            binding: b"{\"drume_timeout_target\":true}".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: 1,
                },
            }],
            deferred: None,
        };
        let mut staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        runtime.reserve_spell_batch(&mut staged).map_err(debug)?;
        assert!(runtime.actor_spell_reserved(target));
        let states = owner.states.spell_states.lock().await;
        tick(&mut runtime, &states, deadline);
        staged
    };
    assert_positions(owner, players.map(|(actor, _)| actor), LANDING).await?;
    assert_eq!(list(owner, LION).await?, lions);
    assert_eq!(list(owner, KESAR).await?.len(), 1);
    for (_, session) in players {
        assert!(owner.states.chat.drain(session).is_empty());
    }

    let mut runtime = owner.runtime.lock().await;
    runtime
        .release_definitely_uncommitted_spell_batch(&staged)
        .map_err(debug)?;
    let states = owner.states.spell_states.lock().await;
    tick(&mut runtime, &states, deadline + 1);
    tick(&mut runtime, &states, deadline + 2);
    Ok(())
}

fn set_captain_tile_passable(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    revision: u64,
    passable: bool,
) -> TestResult {
    use crate::map::overlay::TilePos;
    use crate::map::view::{EntrySource, compose_tile};

    let map = owner.bundle_map().ok_or("battlefield map")?;
    let at = TilePos {
        x: CAPTAIN.x,
        y: CAPTAIN.y,
        floor: CAPTAIN.floor,
    };
    let tile = compose_tile(&map.overlay, &map.facts, at)
        .map_err(debug)?
        .ok_or("captain tile")?;
    let ground = tile.entries.first().ok_or("captain tile ground")?;
    let EntrySource::Base { placement_key, .. } = ground.source else {
        return Err("captain tile must have base ground".into());
    };
    assert!(map.facts.apply_quest_callback(
        placement_key,
        revision,
        ground.facts,
        &[],
        &[(placement_key, ground.facts, passable)],
        false,
        None,
    ));
    assert_eq!(
        map.facts
            .quest_callback_blocked(map.overlay.base(), at, false),
        !passable,
    );
    Ok(())
}

pub(super) async fn summon(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    master_at: MovementLocalPosition,
    child_at: MovementLocalPosition,
) -> TestResult<(ExactActorRef, ExactActorRef)> {
    let mut runtime = owner.runtime.lock().await;
    let rat = "oteryn:creature.rat";
    let master = runtime
        .admit_encounter_actor(rat, 20, master_at)
        .map_err(debug)?;
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let spec = NativeSummonAdmissionSpec::qualified(
        rat,
        rat,
        20,
        0,
        1,
        1,
        runtime.content_pin().server_artifact_digest(),
    )
    .map_err(debug)?;
    let child = runtime
        .admit_current_native_summon(stamp, master, &spec, child_at)
        .map_err(debug)?;
    assert_eq!(runtime.native_summon_master(child), Some(master));
    Ok((master, child))
}

pub(super) async fn vacate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    remaining: (ExactActorRef, GameSessionId),
) -> TestResult<Vec<ExactActorRef>> {
    let mut original = list(owner, LION).await?;
    assert_eq!(original.len(), 3);
    original.extend(list(owner, USURPER).await?);
    assert_eq!(original.len(), 6);
    return_to_staging(owner, remaining, 101).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::encounter_phases::tick(
            &mut runtime,
            &states,
            owner.owner_now().get(),
        );
        assert!(
            original
                .iter()
                .all(|actor| runtime.contains_live_creature(*actor))
        );
    }
    Ok(original)
}

pub(super) async fn return_to_staging(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    (actor, session): (ExactActorRef, GameSessionId),
    y: i32,
) -> TestResult {
    let map = owner.bundle_map().ok_or("battlefield map")?;
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    assert_eq!(expected.position(), LANDING);
    let proof = crate::map::floor::prepare_quest_encounter_ejection(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        expected,
        MovementLocalPosition {
            x: 100,
            y,
            floor: 7,
        },
        &closed,
    )
    .map_err(debug)?;
    runtime
        .commit_quest_relocation(vec![proof], || true)
        .map_err(debug)?;
    Ok(())
}

pub(super) async fn assert_replaced(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: [ExactActorRef; 2],
    original: &[ExactActorRef],
) -> TestResult {
    assert_positions(owner, players, LANDING).await?;
    assert_eq!(list(owner, LION).await?.len(), 3);
    assert_eq!(list(owner, USURPER).await?.len(), 3);
    let runtime = owner.runtime.lock().await;
    assert!(
        original
            .iter()
            .all(|actor| !runtime.contains_live_creature(*actor))
    );
    Ok(())
}

pub(super) fn assert_feedback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    message: &str,
) {
    assert_eq!(
        owner.states.chat.drain(session),
        vec![oteryn_protocol_oteryn::chat::ChatLine::Private {
            speaker_name: "Game".into(),
            text: message.into(),
        }]
    );
}

pub(super) fn assert_admission_effects(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    lever: Position,
    after: u64,
) -> TestResult {
    let observed: Vec<_> = owner
        .bundle_map()
        .ok_or("battlefield map")?
        .facts
        .movement_effects_since(after)
        .into_iter()
        .map(|(_, pos, effect)| (pos.x, pos.y, pos.floor, effect))
        .collect();
    assert_eq!(
        observed,
        vec![
            (
                LANDING.x.try_into()?,
                LANDING.y.try_into()?,
                -i8::try_from(LANDING.floor)?,
                11
            ),
            (lever.x, lever.y, lever.floor, 3),
        ]
    );
    Ok(())
}

async fn list(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
) -> TestResult<Vec<ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .filter(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes()))
        .collect())
}

async fn assert_positions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actors: [ExactActorRef; 2],
    expected: MovementLocalPosition,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    for actor in actors {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            expected
        );
    }
    Ok(())
}
