use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.orc_fortress_quest";
const TRACK: &str = "oteryn:quest-progress/orc-fortress/king-greeted";
const CLAIM: &str = "oteryn:quest-transition/orc-fortress/king-greeted";

#[test]
fn orc_king_first_greeting_claims_personal_flag_and_ambushes_only_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("orc-king-first-greeting", || {
        super::quest_world_callbacks_fixtures::npc_scenario(
            Position { x: 33294, y: 31677, floor: -15 },
            |owner, admitted, harness| Box::pin(exercise(owner, admitted, harness)),
        )
    })
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(catalogue.with_quest(
        vec![QuestTrack { key: TRACK.into(), quest: QUEST.into(), initial: 0, min: 0, max: 1 }],
        vec![QuestTransition {
            key: CLAIM.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    ).map_err(debug)?));
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let [king] = install(owner, admitted, ["oteryn:npc.the_orc_king"]).await?;
    let before = owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.len();

    let first = talk(owner, admitted, king, 6301, "hi").await?;
    assert_eq!(first, ["Arrrrgh! A dirty paleskin! To me my children! Kill them my guards!"]);
    assert!(owner.states.chat.engaged(admitted.game_session_id).is_none());
    let value: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    ).bind(TRACK).fetch_one(&harness.pool).await?;
    assert_eq!(value, 1);
    let spawned = owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.len();
    assert_eq!(spawned - before, 8);

    let second = talk(owner, admitted, king, 6302, "hi").await?;
    assert_eq!(second, ["Harrrrk! You think you are strong now? You shall never escape my wrath! I am immortal!"]);
    assert!(owner.states.chat.engaged(admitted.game_session_id).is_some());
    assert_eq!(owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.len(), spawned);
    Ok(())
}
