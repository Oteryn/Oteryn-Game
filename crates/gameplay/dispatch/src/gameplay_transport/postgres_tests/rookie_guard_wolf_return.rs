//! Crystal 201189f8 data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:93-96,137-167.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const NAME: &str = "rookie-guard-wolf-return";
const EXIT: Position = Position {
    x: 32147,
    y: 32125,
    floor: -11,
};
const START: Position = Position {
    x: 32116,
    y: 32131,
    floor: -11,
};
const RETURN: Position = Position {
    x: 32109,
    y: 32131,
    floor: -11,
};
const BOOST_APPROACH: Position = Position { x: START.x - 1, ..START };
const EXIT_APPROACH: Position = Position { x: EXIT.x - 1, ..EXIT };
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission06";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:137-168";
const EXIT_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:93-96";
const BOOST_TILE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/tile.lua:1623-1634";
const EXIT_TILE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/tile.lua:2220-2224";
const TIMEOUT_MESSAGE: &str =
    "Oh no... you were too slow and the wolves caught up with you. You may try again.";

#[test]
fn wolf_escape_timer_projects_return_movement_and_allows_only_unfinished_attempts_to_retry()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, START, false)
    })
}

fn document() -> serde_json::Value {
    use serde_json::json;
    json!({"schema":1,"callbacks":[
        {"key":"oteryn:quest-callback/rookie-guard-wolf-return", "quest":QUEST, "trigger":"step_in",
         "selector":{"position":START,"action":50334},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "storage":[{"track":TRACK,"minimum":4,"maximum":4,"set":5}],
         "success_message":"RUUUUUUUUUUUUUUUUUN!",
         "effects":{"personal_return":{"delay_ms":25000,"landing":RETURN,"retry_value":4,
             "failure_message":TIMEOUT_MESSAGE,
             "haste":{"multiplier":300,"offset":-24}}},
         "sources":[SOURCE,BOOST_TILE]},
        {"key":"oteryn:quest-callback/rookie-guard-wolf-exit", "quest":QUEST, "trigger":"step_in",
         "selector":{"position":EXIT,"unique":25027,"appearance":7542},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "storage":[{"track":TRACK,"minimum":5,"maximum":5,"set":6}],
         "success_message":"You reached the exit in time! Phew.. back to Tom.",
         "sources":[EXIT_SOURCE,EXIT_TILE]}
    ]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_cells(
        callback,
        START,
        100,
        1,
        &[(EXIT, 7542, oteryn_world_bundle::sector::Attrs {
            unique: Some(25027),
            ..Default::default()
        })],
        &[],
        None,
        &[RETURN, EXIT_APPROACH],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let document = document();
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 4,
            min: -1,
            max: 6,
        }],
        vec![],
    )
    .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture,
        &document.to_string(),
    )?));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let step = encode_step_intent(StepDirection::East);
    wire(owner, admitted, 1, 1, step.clone()).await?;
    assert_eq!(stage(h).await?, 5);
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states.get(&runtime, actor, session).ok_or("player")?;
        assert!(
            player
                .owned_conditions()
                .active_speed_delta(owner.owner_now().get())
                > 0
        );
    }
    owner.drain_quest_timers(actor, session).await;
    assert_eq!(stage(h).await?, 5, "a fresh deadline cannot trigger");
    assert!(drowning(owner, actor, session).await?);
    expire(h).await?;
    owner.drain_quest_timers(actor, session).await;
    assert_eq!(stage(h).await?, 4);
    assert!(
        !drowning(owner, actor, session).await?,
        "the scheduler return runs the underwater tile's StepOut"
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(RETURN)
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(26))
        .ok_or("clock")?;
    let relocate = super::l6_native_signs::relocate;
    relocate(owner, actor, session, BOOST_APPROACH).await?;
    wire(owner, admitted, 2, 1, step.clone()).await?;
    assert_eq!(stage(h).await?, 5);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let back = encode_step_intent(StepDirection::West);
    wire(owner, admitted, 3, 1, back.clone()).await?;
    relocate(owner, actor, session, RETURN).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(RETURN)
    );
    assert_eq!(stage(h).await?, 5);
    owner.states.chat.drain(session);
    expire(h).await?;
    owner.drain_quest_timers(actor, session).await;
    assert_eq!(stage(h).await?, 4);
    assert!(owner.states.chat.drain(session).iter().any(|line| matches!(
        line,
        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == TIMEOUT_MESSAGE
    )));
    owner.drain_quest_timers(actor, session).await;
    assert!(owner.states.chat.drain(session).is_empty());

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(26))
        .ok_or("clock")?;
    relocate(owner, actor, session, BOOST_APPROACH).await?;
    wire(owner, admitted, 4, 1, step.clone()).await?;
    assert_eq!(stage(h).await?, 5);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    wire(owner, admitted, 5, 1, back).await?;
    relocate(owner, actor, session, EXIT_APPROACH).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    wire(owner, admitted, 6, 1, step).await?;
    assert_eq!(stage(h).await?, 6);
    expire(h).await?;
    owner.drain_quest_timers(actor, session).await;
    assert_eq!(stage(h).await?, 6);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(EXIT)
    );
    let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='server_timer' AND transition_key='oteryn:quest-personal-return/rookie-guard-wolf-return'")
        .fetch_one(&h.pool).await?;
    assert_eq!(returns, 2);
    Ok(())
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn drowning(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<bool> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let player = states.get(&runtime, actor, session).ok_or("player")?;
    Ok(player
        .owned_conditions()
        .instances()
        .iter()
        .any(|condition| is_environment_drown(condition.provenance())))
}

async fn expire(h: &Harness) -> TestResult {
    let updated = sqlx::query(
        "UPDATE game_quest_personal_returns SET due_at=clock_timestamp()-interval '1 millisecond'",
    )
    .execute(&h.pool)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    Ok(())
}
