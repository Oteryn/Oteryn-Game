//! Crystal 201189f8, rotten_blood_quest/movements_blood_entrance.lua:7-44.
//! The ingress pad is adjacent to the requested Putrefactory landing in this fixture.
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "unpaid-vestibule-step-health-loss";
pub(super) const HEALTH_FLOOR: &str = "unpaid-vestibule-health-floor";
const QUEST: &str = "oteryn:quest.fixture.vestibule";
const ACCESS: &str = "oteryn:quest-progress/fixture/vestibule-access";
const UNPAID: &str = "You offerings to this sanguine master of this realm have been insufficient. You can not pass.";
const LANDING: Position = Position {
    x: 34101,
    y: 31679,
    floor: -13,
};
const INGRESS: Position = Position {
    x: 34102,
    ..LANDING
};
const EJECT: Position = Position {
    x: 32955,
    y: 32398,
    floor: -9,
};

#[test]
fn unpaid_vestibule_walk_in_relocates_and_applies_agony_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, INGRESS, false))
}

#[test]
fn unpaid_vestibule_walk_in_relocates_without_damage_at_health_floor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(HEALTH_FLOOR, || {
        scenario(HEALTH_FLOOR, INGRESS, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/fixture/vestibule-ejection",
        "quest":QUEST, "trigger":"step_in", "actor_kind":"player",
        "selector":{"position":INGRESS,"ordinal":0},
        "passages":[], "storage":[{"track":ACCESS,"minimum":0,"maximum":3,"set":0,"read_only":true}],
        "offerings":[], "return_on_failure":true, "reversible":false,
        "teleport":EJECT,
        "success_message":UNPAID,
        "sources":["Crystal 201189f8, rotten_blood_quest/movements_blood_entrance.lua:7-44"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    name: &str,
) -> TestResult {
    let mut callback = serde_json::to_value(callback()?)?;
    callback["effects"]["step_health_loss"] = json!({"percent":20,"health_floor":10});
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":["Synthetic ingress pad; penalty and ejection from Crystal 201189f8"],
            "tracks":[{"key":ACCESS,"initial":0,"minimum":0,"maximum":5}],
            "stages":[{"key":"unpaid-ingress","completes":false,
                "trigger":{"kind":"step_in","callback":callback}}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let session = admitted.game_session_id;
    owner.states.chat.admit(session, "Fixture Hero", owner.owner_now());
    if name == HEALTH_FLOOR {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        // Crystal suppresses the whole hit when 12 HP would fall to exactly 10.
        states.get_mut(&runtime, actor, session).ok_or("actor health")?.set_health_for_test(12);
    }
    let health = || async {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        states
            .get(&runtime, actor, session)
            .map(|state| state.vitals().health)
            .ok_or("actor health")
    };
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(LANDING)
    );
    let before = health().await?;
    let expected = if name == HEALTH_FLOOR {
        assert_eq!(before, 12);
        before
    } else {
        assert!(before > 13);
        before - (before / 5).max(1)
    };
    let step = encode_step_intent(StepDirection::East);
    let reply = wire(owner, admitted, 1, 1, step.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(EJECT)
    );
    assert_eq!(health().await?, expected);
    assert_eq!(owner.states.chat.drain(session), vec![oteryn_protocol_oteryn::chat::ChatLine::Private {
        speaker_name: "Game".into(),
        text: UNPAID.into(),
    }]);
    assert_eq!(wire(owner, admitted, 1, 1, step).await?, reply);
    assert_eq!(health().await?, expected);
    assert!(owner.states.chat.drain(session).is_empty());
    Ok(())
}
