//! Annual admission through the same wire commands as ordinary world callbacks.
use super::fixture::{Harness, debug};
use super::world_callback_clocks::{step, use_};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-annual-calendar-window";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const PHASE: &str = "oteryn:quest-progress/annual-calendar/phase";
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const APRIL_1_2026: i64 = 1_775_001_600;

#[test]
fn annual_calendar_window_step_in_and_use_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario(NAME)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let mut use_callback = super::authored_world_rewards::placeholder(AT)?;
    use_callback.key = "oteryn:quest-callback/annual-calendar-use".into();
    use_callback.requires_world_admission = true;
    use_callback.storage[0].track = PHASE.into();
    use_callback.storage[0].maximum = 1;
    let mut step_callback = use_callback.clone();
    step_callback.key = "oteryn:quest-callback/annual-calendar-step".into();
    step_callback.trigger = Trigger::StepIn;
    step_callback.selector.position = Some(AT);
    step_callback.selector.unique = None;
    step_callback.selector.appearance = None;
    step_callback.return_on_failure = true;
    step_callback.storage[0].preserve = true;
    step_callback.storage[0].set = 0;
    let window = serde_json::json!({"start_month":4,"start_day":1,"end_month":5,"end_day":1});
    let mut use_callback = serde_json::to_value(use_callback)?;
    let mut step_callback = serde_json::to_value(step_callback)?;
    use_callback["calendar_window"] = window.clone();
    step_callback["calendar_window"] = window;
    let document = serde_json::json!({"schema":1,"quests":[{"quest":QUEST,
    "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
    "sources":["synthetic annual calendar dispatch fixture"],
    "tracks":[{"key":PHASE,"initial":0,"minimum":0,"maximum":1}],
    "stages":[
        {"key":"use","completes":false,"trigger":{"kind":"map_use","callback":use_callback}},
        {"key":"enter","completes":false,"trigger":{"kind":"step_in","callback":step_callback}}
    ]}]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let revision = owner
        .quest_catalogue
        .as_deref()
        .ok_or("catalogue")?
        .content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    h.root
        .set_test_quest_clock(APRIL_1_2026 - 1)
        .map_err(debug)?;
    assert_eq!(use_(owner, admitted, 1).await?, UseDisposition::Rejected);
    step(owner, admitted, 2, StepDirection::East).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    h.root.advance_test_quest_clock(1).map_err(debug)?;
    step(owner, admitted, 3, StepDirection::East).await?;
    assert_ne!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    assert_eq!(use_(owner, admitted, 4).await?, UseDisposition::Committed);
    Ok(())
}
