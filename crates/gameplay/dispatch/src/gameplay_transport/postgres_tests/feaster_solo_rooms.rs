//! Crystal 201189f data-global/scripts/quests/feaster_of_souls/actions_portal_minis_feaster.lua:1-115.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::GameSessionId;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, world_callbacks::{self, Callback, Position}};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::{sync::Arc, time::Duration};

pub(super) const IRGIX: &str = "feaster-solo-irgix";
pub(super) const UNAZ: &str = "feaster-solo-unaz";
pub(super) const VOK: &str = "feaster-solo-vok";

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

struct Room {
    name: &'static str,
    boss: &'static str,
    door: Position,
    entry: Position,
    spawn: Position,
    min: Position,
    max: Position,
    exit: Position,
}

const ROOMS: &[Room] = &[
    Room {
        name: IRGIX,
        boss: "oteryn:creature.irgix_the_flimsy",
        door: at(33491, 31398, -8),
        entry: at(33467, 31405, -8),
        spawn: at(33467, 31399, -8),
        min: at(33460, 31393, -8),
        max: at(33474, 31408, -8),
        exit: at(33493, 31400, -8),
    },
    Room {
        name: UNAZ,
        boss: "oteryn:creature.unaz_the_mean",
        door: at(33566, 31475, -8),
        entry: at(33576, 31494, -8),
        spawn: at(33565, 31496, -8),
        min: at(33558, 31487, -8),
        max: at(33582, 31499, -8),
        exit: at(33563, 31477, -8),
    },
    Room {
        name: VOK,
        boss: "oteryn:creature.vok_the_freakish",
        door: at(33509, 31450, -9),
        entry: at(33508, 31494, -9),
        spawn: at(33508, 31486, -9),
        min: at(33501, 31483, -9),
        max: at(33515, 31496, -9),
        exit: at(33509, 31451, -9),
    },
];

fn room(name: &str) -> TestResult<&'static Room> {
    ROOMS.iter().find(|room| room.name == name).ok_or_else(|| "Feaster room".into())
}

pub(super) fn binding(name: &str) -> TestResult<Callback> {
    let r = room(name)?;
    Ok(serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-callback/solo-room/{}", r.name),
        "quest":"oteryn:quest.feaster_of_souls_quest",
        "trigger":"step_in",
        "selector":{"position":r.door,"native":{"position":r.door,"ordinal":1,"appearance":25047}},
        "requires_world_admission":true,"return_on_failure":true,
        "reversible":false,"passages":[],"offerings":[],"storage":[],
        "effects":{"teleport_effect":11,"encounter":{
            "minimum_level":250,"players":[{"from":r.door,"to":r.entry}],
            "monsters":[{"creature":r.boss,"position":r.spawn}],
            "room_min":r.min,"room_max":r.max,
            "solo_expiry":{"exit":r.exit,"time_limit_ms":600000,"cooldown_seconds":72000}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_portal_minis_feaster.lua:1-115"]
    }))?)
}

pub(super) fn world(callback: &Callback, name: &str) -> TestResult<crate::map::boot::BundleWorld> {
    let r = room(name)?;
    world_cells(callback, r.door, 25047, 1, &[], &[], None, &[r.exit])
}

pub(super) fn catalogue(base: &QuestStateCatalogue, callback: &Callback) -> TestResult<Arc<QuestStateCatalogue>> {
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(base, &document)?))
}

#[test]
fn irgix_solo_room_rejects_an_occupant_and_sets_cooldown_before_timed_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(IRGIX, || scenario(IRGIX, room(IRGIX)?.door, false))
}

#[test]
fn unaz_solo_room_rejects_an_occupant_and_sets_cooldown_before_timed_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(UNAZ, || scenario(UNAZ, room(UNAZ)?.door, false))
}

#[test]
fn vok_solo_room_rejects_an_occupant_and_sets_cooldown_before_timed_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(VOK, || scenario(VOK, room(VOK)?.door, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    harness: &Harness, fence: CurrentCharacterGameplayFence, name: &str,
) -> TestResult {
    let r = room(name)?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let other_session = GameSessionId::decode(&id(112)).map_err(debug)?;
    let other = {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime.reserve_fresh_session(other_session).map_err(debug)?;
        let other = runtime.commit_fresh_session(lease).map_err(debug)?;
        runtime.initialize_pinned_test_position(other, local(r.entry)).map_err(debug)?;
        other
    };
    let live = super::engworldchg_journals::live_creatures;
    let cooldown = || sqlx::query_scalar::<_, i64>(
        "SELECT available_at FROM game_character_boss_cooldowns
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(r.boss).fetch_optional(&harness.pool);
    let origin = Position { x:r.door.x - 1, ..r.door };
    for (command, expected) in [(1, origin), (2, r.entry)] {
        let result = wire(owner, admitted, command, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East)).await?;
        assert_eq!(decode_step_result(&result).map_err(debug)?, StepDisposition::Moved);
        assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(expected));
        if command == 1 {
            assert!(live(owner, r.boss).await?.is_empty());
            assert!(cooldown().await?.is_none());
            let mut runtime = owner.runtime.lock().await;
            let map = owner.bundle_map().ok_or("map")?;
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime, owner.movement_cells(), other, other_session,
                runtime.read_actor_position(other).map_err(debug)?, local(r.exit),
                false, &crate::map::floor::BundleDoors(&map.facts),
            ).map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
    }
    assert_eq!(live(owner, r.boss).await?.len(), 1);
    let future: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint + 72000000"
    ).fetch_one(&harness.pool).await?;
    let booked = cooldown().await?.ok_or("solo cooldown")?;
    assert!((future - 10_000..=future).contains(&booked));
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(r.entry));
    let due = owner.states.quest_callbacks.next_encounter_due_us().ok_or("solo deadline")?;
    let advance = Duration::from_micros(due.saturating_sub(owner.owner_now().get()));
    owner.clock_origin = owner.clock_origin.checked_sub(advance).ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(r.exit));
    assert!(live(owner, r.boss).await?.is_empty());
    Ok(())
}
