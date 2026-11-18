//! Crystal 201189f8, the_order_of_lion_quest/creaturescripts_commander_kills.lua:8-50
//! and action_drume.lua:1-18. The fixture supplies a translated battlefield.
use super::engenc2_cults::{encounter_scenario, kill};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

const EXIT: Position = Position {
    x: 103,
    y: 100,
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

#[test]
fn drume_final_usurper_death_spawns_boss_and_kesar_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(true))
}

#[test]
fn drume_final_lion_death_clears_squads_and_ejects_both_players() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(false))
}

#[test]
fn drume_lever_replaces_vacated_battlefield_before_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(true, Some(true))
}

pub(super) fn bind(callback: &mut Callback) -> TestResult {
    let mut json = serde_json::to_value(&*callback)?;
    json["effects"]["battlefield"] = serde_json::json!({
        "exit": EXIT,
        "captain": {"x":116,"y":112,"floor":-7},
        "boss": {"x":116,"y":113,"floor":-7},
        "timeout_ms":1_200_000,
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
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let member =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(harness, 61, 63, 71, now)
            .await?;
    let session = member.current_game_session_id();
    assert_ne!(session, leader.game_session_id);
    let mut runtime = owner.runtime.lock().await;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: 100,
                y: 101,
                floor: 7,
            },
        )
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(actor, session, member.current_character_lease())
        .map_err(debug)?;
    Ok((actor, session))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &AdmittedSession,
    second: ExactActorRef,
    spawn_boss: bool,
) -> TestResult {
    let first = leader.runtime_actor.ok_or("first actor")?;
    let actors = [first, second];
    assert_positions(owner, actors, LANDING).await?;
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
    if spawn_boss {
        assert_eq!(list(owner, DRUME).await?.len(), 1);
        assert_eq!(list(owner, KESAR).await?.len(), 1);
        assert_positions(owner, actors, LANDING).await?;
        let target = list(owner, DRUME)
            .await?
            .into_iter()
            .next()
            .ok_or("Drume")?;
        kill(owner, leader, target, DRUME, 103).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::encounter_phases::tick(
                &mut runtime,
                &states,
                owner.owner_now().get() + 1_200_000_000,
            );
        }
        assert!(list(owner, LION).await?.is_empty());
        assert!(list(owner, KESAR).await?.is_empty());
        assert_positions(owner, actors, super::quest_world_callbacks::local(EXIT)).await?;
    } else {
        assert!(list(owner, USURPER).await?.is_empty());
        assert!(list(owner, DRUME).await?.is_empty());
        assert_positions(owner, actors, super::quest_world_callbacks::local(EXIT)).await?;
    }
    Ok(())
}

pub(super) async fn vacate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: [(ExactActorRef, GameSessionId); 2],
) -> TestResult<Vec<ExactActorRef>> {
    assert_positions(owner, players.map(|(actor, _)| actor), LANDING).await?;
    let mut original = list(owner, LION).await?;
    assert_eq!(original.len(), 3);
    original.extend(list(owner, USURPER).await?);
    assert_eq!(original.len(), 6);
    {
        let map = owner.bundle_map().ok_or("battlefield map")?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut runtime = owner.runtime.lock().await;
        let mut proofs = Vec::new();
        for ((actor, session), y) in players.into_iter().zip([100, 101]) {
            proofs.push(
                crate::map::floor::prepare_quest_encounter_ejection(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    runtime.read_actor_position(actor).map_err(debug)?,
                    MovementLocalPosition {
                        x: 100,
                        y,
                        floor: 7,
                    },
                    &closed,
                )
                .map_err(debug)?,
            );
        }
        runtime
            .commit_quest_relocation(proofs, || true)
            .map_err(debug)?;
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
