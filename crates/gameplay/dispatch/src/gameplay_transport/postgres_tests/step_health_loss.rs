//! Crystal 201189f8, rotten_blood_quest/movements_blood_entrance.lua:7-44.
//! The donor entrance and the fixture ingress beside the Putrefactory landing are separate selectors.
use super::fixture::{Harness, debug};
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
pub(super) const PAID: &str = "paid-vestibule-step-no-health-loss";
pub(super) const BELOW_LEVEL: &str = "vestibule-step-below-level";
pub(super) const DONOR_ENTRANCE: &str = "vestibule-donor-entrance";
const QUEST: &str = "oteryn:quest.fixture.vestibule";
const ACCESS: &str = "oteryn:quest-progress/fixture/vestibule-access";
const LEVEL_REFUSAL: &str = "You need at least level 250 to enter.";
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
// Crystal movements_blood_entrance.lua:2,42-44 and
// content/world/placements/region-z09-x128-y126.b3: item 25181 is the third top-level entry.
const SOURCE_ENTRANCE: Position = Position { x: 32953, y: 32398, floor: -9 };
const SOURCE_APPEARANCE: u16 = 25181;
const SOURCE_ORDINAL: u8 = 2;

const EJECT: Position = Position {
    x: 32955,
    y: 32398,
    floor: -9,
};
pub(super) const DESTINATION: Position = Position {
    x: 34070,
    y: 31975,
    floor: -14,
};
const TELEPORT_EFFECT: u16 = 11;

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

#[test]
fn paid_vestibule_walk_in_admits_without_agony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(PAID, || scenario(PAID, INGRESS, false))
}

#[test]
fn vestibule_returns_below_level_player_to_landing_before_penalty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(BELOW_LEVEL, || {
        scenario(BELOW_LEVEL, INGRESS, false)
    })
}

#[test]
fn unpaid_blood_entrance_uses_the_retained_donor_placement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(DONOR_ENTRANCE, || {
        scenario(DONOR_ENTRANCE, SOURCE_ENTRANCE, false)
    })
}

pub(super) fn callback(trigger: Position) -> TestResult<Callback> {
    let selector = if trigger == SOURCE_ENTRANCE {
        json!({"position":trigger,"ordinal":SOURCE_ORDINAL,"appearance":SOURCE_APPEARANCE})
    } else {
        json!({"position":trigger,"ordinal":0})
    };
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/fixture/vestibule-ejection",
        "quest":QUEST, "trigger":"step_in", "actor_kind":"player",
        "selector":selector,
        "passages":[], "storage":[{"track":ACCESS,"minimum":0,"maximum":3,"set":0,"read_only":true}],
        "offerings":[], "return_on_failure":true, "reversible":false,
        "teleport":EJECT,
        "minimum_level":250, "failure_message":LEVEL_REFUSAL, "success_message":UNPAID,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rotten_blood_quest/movements_blood_entrance.lua:7-44"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    harness: &Harness,
    name: &str,
) -> TestResult {
    let trigger = if name == DONOR_ENTRANCE { SOURCE_ENTRANCE } else { INGRESS };
    let origin = Position { x: trigger.x - 1, ..trigger };
    let mut unpaid = serde_json::to_value(callback(trigger)?)?;
    let mut paid = unpaid.clone();
    paid["key"] = json!("oteryn:quest-callback/fixture/vestibule-admission");
    paid["storage"][0]["minimum"] = json!(4);
    paid["storage"][0]["maximum"] = json!(5);
    paid["teleport"] = json!(DESTINATION);
    paid["success_message"] = json!(null);
    // Crystal movements_blood_entrance.lua:34-36 emits the effect only at the paid landing.
    paid["effects"]["magic"] = json!({"position":DESTINATION,"effect":TELEPORT_EFFECT});
    unpaid["effects"]["step_health_loss"] = json!({"percent":20,"health_floor":10});
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":["Crystal 201189f8 movements_blood_entrance.lua:7-44"],
            "tracks":[{"key":ACCESS,"initial":if name == PAID { 4 } else { 0 },"minimum":0,"maximum":5}],
            "stages":[
                {"key":"unpaid-ingress","completes":false,
                    "trigger":{"kind":"step_in","callback":unpaid}},
                {"key":"paid-ingress","completes":false,
                    "trigger":{"kind":"step_in","callback":paid}}
            ]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("Vestibule fence")?;
    let level: i64 = if name == BELOW_LEVEL { 249 } else { 250 };
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx).await?;
    let updated = sqlx::query(
        "UPDATE game_character_progression_state SET level=$1 WHERE character_id=encode($2,'hex')::uuid",
    )
    .bind(level)
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(&mut *tx).await?;
    assert_eq!(updated.rows_affected(), 1);
    tx.commit().await?;
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
        local(origin)
    );
    let before = health().await?;
    let expected = if name == HEALTH_FLOOR {
        assert_eq!(before, 12);
        before
    } else if matches!(name, PAID | BELOW_LEVEL) {
        before
    } else {
        assert!(before > 13);
        before - (before / 5).max(1)
    };
    let landing_effects = || -> TestResult<usize> {
        let map = owner.bundle_map().ok_or("Vestibule fixture map")?;
        let count = map
            .facts
            .movement_effects_since(0)
            .into_iter()
            .filter(|(_, at, effect)| {
                (at.x, at.y, at.floor) == (DESTINATION.x, DESTINATION.y, DESTINATION.floor)
                    && *effect == TELEPORT_EFFECT
            })
            .count();
        Ok(count)
    };
    let effects_before = landing_effects()?;
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
        local(match name {
            PAID => DESTINATION,
            BELOW_LEVEL => origin,
            _ => EJECT,
        })
    );
    assert_eq!(health().await?, expected);
    let effects_after = effects_before + usize::from(name == PAID);
    assert_eq!(landing_effects()?, effects_after);
    let messages = owner.states.chat.drain(session);
    if name == PAID {
        assert!(messages.is_empty());
    } else {
        assert_eq!(messages, vec![oteryn_protocol_oteryn::chat::ChatLine::Private {
            speaker_name: "Game".into(),
            text: if name == BELOW_LEVEL { LEVEL_REFUSAL } else { UNPAID }.into(),
        }]);
    }
    assert_eq!(wire(owner, admitted, 1, 1, step).await?, reply);
    assert_eq!(health().await?, expected);
    assert_eq!(landing_effects()?, effects_after);
    assert!(owner.states.chat.drain(session).is_empty());
    Ok(())
}
