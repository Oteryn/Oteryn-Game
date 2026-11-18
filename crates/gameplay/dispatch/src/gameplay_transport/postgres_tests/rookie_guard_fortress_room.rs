//! Crystal 201189f8 data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:513-590.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::GameSessionId;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const NAME: &str = "rookie-guard-fortress-room";
const ENTRY: Position = Position { x: 31954, y: 32174, floor: -10 };
const LANDING: Position = Position { x: 31944, y: 32174, floor: -10 };
const TIMEOUT: Position = Position { x: 31980, y: 32173, floor: -10 };
pub(super) const EXIT_TILE: Position = Position { x: 31945, y: 32174, floor: -10 };
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission12";
const ENTER: &str = "oteryn:quest-callback/rookie-guard-fortress-room";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:513-590";

#[test]
fn fortress_room_rejects_a_second_player_and_retires_exact_boss_on_exit_or_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    })
}

fn document() -> serde_json::Value {
    use serde_json::json;
    json!({"schema":1,"callbacks":[
        {"key":ENTER, "quest":QUEST, "trigger":"step_in",
         "selector":{"position":ENTRY,"appearance":100},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":true,
         "storage":[{"track":TRACK,"minimum":11,"maximum":12,"set":12}],
         "effects":{
             "encounter":{"minimum_level":1,"players":[{"from":ENTRY,"to":LANDING}],
                 "monsters":[{"creature":"oteryn:creature.kraknaknork",
                              "position":{"x":31930,"y":32170,"floor":-10}}],
                 "room_min":{"x":31929,"y":32165,"floor":-10},
                 "room_max":{"x":31945,"y":32175,"floor":-10}},
             "encounter_timeout":{"duration_ms":300000,"landing":TIMEOUT,"heal":true,
                 "message":"With his last energy, Kraknaknork pushes you out of his throne room. Hurry back and defeat him before he regains his power."}},
         "sources":[SOURCE]},
        {"key":"oteryn:quest-callback/rookie-guard-fortress-room-exit","quest":QUEST,
         "trigger":"step_in","selector":{"position":EXIT_TILE,"appearance":100},
         "storage":[],"passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "teleport":ENTRY,"effects":{"encounter_exit":ENTER},"sources":[SOURCE]}
    ]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack { key: TRACK.into(), quest: QUEST.into(), initial: 11, min: -1, max: 13 }],
        vec![],
    ).map_err(debug)?;
    let document = document();
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture, &document.to_string(),
    )?));
    owner.states.quest_callbacks.fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;

    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let player = admitted.runtime_actor.ok_or("actor")?;
    let player_session = admitted.game_session_id;
    let peer_session = GameSessionId::decode(&id(58)).map_err(debug)?;
    let (origin, peer) = {
        let mut runtime = owner.runtime.lock().await;
        runtime.install_companion_policies(native.policies().clone()).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
        let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(peer, local(LANDING)).map_err(debug)?;
        (runtime.read_actor_position(player).map_err(debug)?.position(), peer)
    };
    let east = encode_step_intent(StepDirection::East);
    wire(owner, admitted, 1, 1, east.clone()).await?;
    let peer_outside = local(Position { x: ENTRY.x + 1, ..ENTRY });
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(player).map_err(debug)?.position(), origin);
        assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
        let current = runtime.read_actor_position(peer).map_err(debug)?;
        let map = owner.bundle_map().ok_or("map")?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, owner.movement_cells(), peer, peer_session, current, peer_outside, false, &doors,
        ).map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    advance_clock(owner, 2)?;
    wire(owner, admitted, 2, 1, east.clone()).await?;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(player).map_err(debug)?.position(), local(LANDING));
        assert_eq!(runtime.current_live_creature_census().map_err(debug)?.len(), 1);
    }
    advance_clock(owner, 2)?;
    wire(owner, admitted, 3, 1, east.clone()).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let current = runtime.read_actor_position(player).map_err(debug)?;
        assert_eq!(current.position(), local(ENTRY));
        assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
        let map = owner.bundle_map().ok_or("map")?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, owner.movement_cells(), player, player_session, current, origin, false, &doors,
        ).map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    advance_clock(owner, 2)?;
    wire(owner, admitted, 4, 1, east).await?;
    advance_clock(owner, 301)?;
    owner.drain_quest_encounter_deadlines().await;
    let runtime = owner.runtime.lock().await;
    assert_eq!(runtime.read_actor_position(player).map_err(debug)?.position(), local(TIMEOUT));
    assert_eq!(runtime.read_actor_position(peer).map_err(debug)?.position(), peer_outside);
    assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
    drop(runtime);
    let state: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(state, 12);
    Ok(())
}

fn advance_clock(owner: &mut ComposedFreshAdmission<'_, '_, '_>, seconds: u64) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(seconds))
        .ok_or("clock underflow")?;
    Ok(())
}
