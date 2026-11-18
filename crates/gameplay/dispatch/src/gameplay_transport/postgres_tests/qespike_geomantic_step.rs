//! Crystal 201189f8 data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:1-45.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    CAPABILITY_DIAGONAL_MOVEMENT_V1, CAPABILITY_PACED_MOVEMENT_V1,
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
    decode_step_result_profile, encode_step_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "qespike-geomantic-step";
const CHARGE_ONLY: &str = "qespike-geomantic-step-charge-only";
const QUEST: &str = "oteryn:quest.spike_task";
const NEST: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_middle_nest_main";
const CHARGE: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_middle_charge_main";
// Test geometry for Crystal's action ID, independent of the source spawn map.
const CELL: Position = Position {
    x: 32624,
    y: 31855,
    floor: -11,
};
const ORIGIN: Position = Position {
    x: CELL.x - 1,
    ..CELL
};

#[test]
fn spike_geomantic_step_checks_outfit_before_charge_then_applies_both() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, CELL, false)
    })
}

#[test]
fn spike_geomantic_step_charges_without_nest_and_refuses_after_charge() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CHARGE_ONLY, || {
        super::quest_world_callbacks::scenario(CHARGE_ONLY, CELL, false)
    })
}

pub(super) fn matches(name: &str) -> bool {
    matches!(name, NAME | CHARGE_ONLY)
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qespike/geomantic-nest-and-charge","quest":QUEST,
        "trigger":"step_in","selector":{"action":56421},
        "passages":[],"offerings":[],"reversible":false,"return_on_failure":true,
        "requires_world_admission":true,
        "storage":[{"track":NEST,"minimum":0,"maximum":7,"set":0,"preserve":true},
            {"track":CHARGE,"minimum":1,"maximum":1,"set":2}],
        "effects":{"timed_outfit":{"look_type":307,"duration_ms":120000,"effect":11},"step_effect":12},
        "success_message":"You have charged your body with geomantic energy and can report about it.",
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:1-45"]
    }))?)
}

fn phase_catalogue(revision: &str, combined: &Callback, nest: i64) -> TestResult<QuestStateCatalogue> {
    let mut phases = vec![combined.clone()];
    // Crystal movements_geomantic_charges.lua:13-38 treats the two tasks
    // independently. Split the inactive values into disjoint intervals.
    for (suffix, minimum, maximum) in [("unstarted", -1, 0), ("charged", 2, 2)] {
        let mut phase = combined.clone();
        phase.key = format!("oteryn:quest-callback/qespike/geomantic-nest-{suffix}");
        phase.storage[1].minimum = minimum;
        phase.storage[1].maximum = maximum;
        phase.storage[1].set = minimum;
        phase.storage[1].preserve = true;
        phase.effects.step_effect = None;
        phase.success_message = None;
        phases.push(phase);
    }
    for (suffix, inactive) in [("unstarted", -1), ("finished", 8)] {
        let mut phase = combined.clone();
        phase.key = format!("oteryn:quest-callback/qespike/geomantic-charge-{suffix}");
        phase.storage[0].minimum = inactive;
        phase.storage[0].maximum = inactive;
        phase.storage[0].set = inactive;
        phase.effects.timed_outfit = None;
        phases.push(phase);
    }
    let stages: Vec<_> = phases
        .iter()
        .map(|phase| json!({
            "key": phase.key,
            "completes": false,
            "trigger": {"kind": "step_in", "callback": phase}
        }))
        .collect();
    let digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let input = serde_json::to_vec(&json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": digest, "sources": combined.sources,
        "tracks": [
            {"key": NEST, "initial": nest, "minimum": -1, "maximum": 8},
            {"key": CHARGE, "initial": 1, "minimum": -1, "maximum": 2}
        ],
        "stages": stages
    }]}))?;
    let base = QuestStateCatalogue::empty(revision).ok_or("geomantic base catalogue")?;
    Ok(Bindings::parse(&input)?.attach(&base)?)
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(
        owner, admitted, command, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    let capabilities = admitted.continuity.selected_capabilities;
    assert_eq!(
        decode_step_result_profile(&result, capabilities.contains(CAPABILITY_PACED_MOVEMENT_V1),
            capabilities.contains(CAPABILITY_DIAGONAL_MOVEMENT_V1)).map_err(debug)?.disposition,
        StepDisposition::Moved,
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    let charge_only = name == CHARGE_ONLY;
    let initial_nest = if charge_only { -1 } else { 0 };
    owner.quest_catalogue = Some(Arc::new(phase_catalogue(revision, callback, initial_nest)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("geomantic player")?;
    let session = admitted.game_session_id;
    {
        let runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        let current = spells.get(&runtime, actor, session).ok_or("player state")?;
        let next = crate::spell::actor_conditions::stage_item_costume(
            current,
            307,
            1000,
            owner.owner_now().get() / 1000,
        )
        .map_err(debug)?;
        assert!(spells.commit(&runtime, actor, session, next));
    }
    async fn charge(harness: &Harness) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),1)",
        )
        .bind(CHARGE)
        .fetch_one(&harness.pool)
        .await?)
    }

    let movement_effects = |admission: &ComposedFreshAdmission<'_, '_, '_>| {
        admission.bundle_map().map(|map| {
            map.facts
                .movement_effects_since(0)
                .into_iter()
                .filter_map(|(_, position, effect)| {
                    (position.x == CELL.x && position.y == CELL.y && position.floor == CELL.floor)
                        .then_some(effect)
                })
                .collect::<Vec<_>>()
        })
    };
    step(owner, admitted, StepDirection::East).await?;
    if charge_only {
        assert_eq!(charge(harness).await?, 2);
        assert_eq!(movement_effects(owner), Some(vec![12]));
        assert_eq!(
            owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
            super::quest_world_callbacks::local(CELL)
        );
        for direction in [StepDirection::West, StepDirection::East] {
            owner.clock_origin -= Duration::from_secs(5);
            step(owner, admitted, direction).await?;
        }
        assert_eq!(charge(harness).await?, 2);
        assert_eq!(movement_effects(owner), Some(vec![12]));
        assert_eq!(
            owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
            super::quest_world_callbacks::local(ORIGIN)
        );
        return Ok(());
    }
    assert_eq!(charge(harness).await?, 1);
    assert_eq!(movement_effects(owner), Some(Vec::new()));
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(ORIGIN)
    );

    owner.clock_origin -= Duration::from_secs(5);
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(charge(harness).await?, 2);
    assert_eq!(movement_effects(owner), Some(vec![11, 12]));
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(CELL)
    );
    {
        let spells = owner.states.spell_states.lock().await;
        let conditions = spells
            .get(&runtime, actor, session)
            .ok_or("player state")?
            .owned_conditions();
        let now = owner.owner_now().get();
        assert_eq!(conditions.outfit_at(now), Some(307));
        assert_eq!(conditions.outfit_at(now + 120_000_000), None);
    }
    drop(runtime);
    owner.clock_origin -= Duration::from_secs(120);
    step(owner, admitted, StepDirection::West).await?;
    owner.clock_origin -= Duration::from_secs(5);
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(charge(harness).await?, 2);
    assert_eq!(movement_effects(owner), Some(vec![11, 12, 11]));
    Ok(())
}
