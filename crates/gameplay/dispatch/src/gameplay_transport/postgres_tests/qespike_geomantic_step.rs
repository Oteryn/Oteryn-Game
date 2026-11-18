//! Crystal 201189f8 data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:1-45.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "qespike-geomantic-step";
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

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qespike/geomantic-nest-and-charge","quest":QUEST,
        "trigger":"step_in","selector":{"action":56421},
        "passages":[],"offerings":[],"reversible":false,"return_on_failure":true,
        "requires_world_admission":true,
        "storage":[{"track":NEST,"minimum":0,"maximum":7,"set":0,"preserve":true},
            {"track":CHARGE,"minimum":1,"maximum":1,"set":2}],
        "effects":{"timed_outfit":{"look_type":307,"duration_ms":120000}},
        "success_message":"You have charged your body with geomantic energy and can report about it.",
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:1-45"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    callback: &Callback,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            revision,
            vec![
                QuestTrack {
                    key: NEST.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: -1,
                    max: 8,
                },
                QuestTrack {
                    key: CHARGE.into(),
                    quest: QUEST.into(),
                    initial: 1,
                    min: -1,
                    max: 2,
                },
            ],
            Vec::new(),
        )
        .map_err(debug)?
        .with_world_callback_fixture(callback.clone())?,
    ));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback.clone()])
        .map_err(|_| "geomantic fixture")?;
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

    let command = admitted.continuity.next_command_id;
    super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(charge(harness).await?, 1);
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
    let command = admitted.continuity.next_command_id;
    super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(charge(harness).await?, 2);
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(CELL)
    );
    let spells = owner.states.spell_states.lock().await;
    let conditions = spells
        .get(&runtime, actor, session)
        .ok_or("player state")?
        .owned_conditions();
    let now = owner.owner_now().get();
    assert_eq!(conditions.outfit_at(now), Some(307));
    assert_eq!(conditions.outfit_at(now + 120_000_000), None);
    Ok(())
}
