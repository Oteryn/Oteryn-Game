//! Crystal 201189f data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:1-126.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::GameSessionId;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::time::Duration;

pub(super) const NAME: &str = "feaster-brain-head";
const BOSS: &str = "oteryn:creature.brain_head";
const ADD: &str = "oteryn:creature.cerebellum";
const DOOR: Position = at(31937, 32325);
const ENTRY: Position = at(31963, 32324);
pub(super) const EXIT: Position = at(31971, 32325);

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -10 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let monsters: Vec<_> = [(BOSS, at(31954, 32325))]
        .into_iter()
        .chain([(31953, 32324), (31955, 32324), (31953, 32326), (31955, 32326),
                (31960, 32320), (31960, 32330), (31947, 32320), (31947, 32330)]
            .map(|(x, y)| (ADD, at(x, y))))
        .map(|(creature, position)| serde_json::json!({"creature": creature,"position":position}))
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/solo-room/brain-head",
        "quest":"oteryn:quest.feaster_of_souls_quest",
        "trigger":"step_in",
        "selector":{"position":DOOR,"ordinal":0},
        "requires_world_admission":true,"return_on_failure":true,"failure_teleport":EXIT,
        "reversible":false,"passages":[],"offerings":[],"storage":[],
        "effects":{"encounter":{
            "minimum_level":250,"players":[{"from":DOOR,"to":ENTRY}],
            "monsters":monsters,
            "room_min":at(31941,32313),"room_max":at(31966,32337),
            "solo_expiry":{"exit":EXIT,"time_limit_ms":300000,"cooldown_seconds":36000,
                "shared":{"capacity":5,"join_window_ms":30000}}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:1-126"]
    }))?)
}

#[test]
fn brain_head_step_admits_into_an_occupied_room_then_ejects_at_the_stage_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, DOOR, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    // An occupied shared room can admit another player. The three source
    // stages must not be projected as a solo occupancy gate.
    {
        let mut runtime = owner.runtime.lock().await;
        let session = GameSessionId::decode(&id(112)).map_err(debug)?;
        let lease = runtime.reserve_fresh_session(session).map_err(debug)?;
        let spectator = runtime.commit_fresh_session(lease).map_err(debug)?;
        runtime.initialize_pinned_test_position(spectator, local(ENTRY)).map_err(debug)?;
    }
    let result = wire(owner, admitted, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East)).await?;
    assert_eq!(decode_step_result(&result).map_err(debug)?, StepDisposition::Moved);
    let position = || async {
        Ok::<_, Box<dyn std::error::Error>>(owner.runtime.lock().await
            .read_actor_position(actor).map_err(debug)?.position())
    };
    assert_eq!(position().await?, local(ENTRY));
    let live = super::engworldchg_journals::live_creatures;
    assert_eq!(live(owner, BOSS).await?.len(), 1);
    assert_eq!(live(owner, ADD).await?.len(), 8);
    let remaining: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         FROM game_character_boss_cooldowns
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice()).bind(BOSS)
    .fetch_one(&harness.pool).await?;
    assert!((35_990_000..=36_000_000).contains(&remaining));
    let due = owner.states.quest_callbacks.next_encounter_due_us().ok_or("room deadline")?;
    let advance = Duration::from_micros(due.saturating_sub(owner.owner_now().get()));
    owner.clock_origin = owner.clock_origin.checked_sub(advance).ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(position().await?, local(EXIT));
    assert!(live(owner, BOSS).await?.is_empty());
    assert!(live(owner, ADD).await?.is_empty());
    Ok(())
}
