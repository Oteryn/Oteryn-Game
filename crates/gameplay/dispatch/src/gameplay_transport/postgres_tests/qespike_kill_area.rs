//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/creaturescripts_lower_spike_kill.lua:1-21.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::native_kills::NativeDeathSource;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-qespike-kill-area";
pub(super) const OUTSIDE_NAME: &str = "qengine-rewards-qespike-kill-area-outside";
const QUEST: &str = "oteryn:quest.spike_task";
const CREATURE: &str = "oteryn:creature.drillworm";
const TRANSITION: &str = "oteryn:quest-transition/native-kills/qespike-lower-drillworm";
const GATE: &str = "oteryn:quest-progress/spike/lower-hunt-active";
const COUNT: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_lower_kill_main";
const EDGE: Position = Position {
    x: 32120,
    y: 32470,
    floor: -13,
};

#[test]
fn spike_drillworm_death_keeps_actual_lethal_cell_through_quest_settlement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, EDGE, false)
    })
}

#[test]
fn spike_drillworm_death_outside_lower_area_leaves_quest_progress_unchanged() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(OUTSIDE_NAME, || {
        super::quest_world_callbacks::scenario(
            OUTSIDE_NAME,
            Position {
                x: EDGE.x - 1,
                ..EDGE
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    inside_area: bool,
) -> TestResult {
    let bindings = Bindings::parse(json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":"cff7d45a40c4302abea54b2e4860cfeafa4d44229e922216ddd2eebd1bd3e6dd",
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/creaturescripts_lower_spike_kill.lua:1-21"],
        "tracks":[{"key":GATE,"initial":1,"minimum":0,"maximum":1},
            {"key":COUNT,"initial":0,"minimum":-1,"maximum":7}],
        "stages":[{"key":"drillworms","completes":false,"trigger":{"kind":"kill","counter":{
            "key":TRANSITION,"creature":CREATURE,"track":COUNT,"gate":GATE,
            "gate_value":1,"minimum":0,"maximum":7,
            "death_area":{"minimum":[32120,32470,13],"maximum":[32345,32710,15]}
        }}}]
    }]}).to_string().as_bytes())?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let target = super::qengine_kill_stories::queue_death(owner, admitted, CREATURE, 89430).await?;
    let captured = owner
        .states
        .attack
        .lock()
        .await
        .kills()
        .1
        .lock()
        .map_err(|_| "kill cache")?
        .counter_death_fixture(target, TRANSITION);
    if inside_area {
        assert!(
            matches!(
                captured.ok_or("captured native counter")?.source,
                NativeDeathSource::PositionedParty {
                    position: (32120, 32470, -13),
                    ..
                }
            ),
            "the qualified death must retain the exact lethal cell"
        );
    } else {
        assert!(
            captured.is_none(),
            "an out-of-area death cannot capture quest credit"
        );
    }
    super::qengine_kill_stories::drain_deaths(owner, admitted, &[target]).await?;
    let amount: Option<i64> =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(COUNT)
            .fetch_optional(&harness.pool)
            .await?;
    assert_eq!(amount, inside_area.then_some(1));
    Ok(())
}
