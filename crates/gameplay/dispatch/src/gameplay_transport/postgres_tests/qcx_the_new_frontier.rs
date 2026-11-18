//! The TibiaWiki spoiler (curid 41756, oldid 1099208), Mission 10.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, loader::load_embedded_quest_state,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10_1";
const CARPET: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10/magic_carpet_door";
const READY: &str = "oteryn:quest-transition/qcx-new-frontier-reported-to-chrak";

#[test]
fn new_horizons_grants_experience_and_carpet_access_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx-new-frontier", || {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "qcx_new_frontier", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut admitted = deps.admitted;
            let mut owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;

            let source = load_embedded_quest_state("content-1").map_err(debug)?;
            let catalogue = Arc::new(source.catalogue().with_fixture_transition(QuestTransition {
                world_change: None,
                key: READY.into(),
                quest: QUEST.into(),
                effects: [QUESTLINE, MISSION].into_iter().zip([28, 1]).map(|(track, value)| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                }).collect(),
                completes: false,
                experience: None,
            }).map_err(debug)?);
            owner.quest_catalogue = Some(Arc::clone(&catalogue));
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            let [ongulf] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.ongulf"]).await?;
            let before = experience(&h).await?;
            super::qengine_rewards::talk(&mut owner, &mut admitted, ongulf, 101, "hi").await?;
            super::qengine_rewards::talk(&mut owner, &mut admitted, ongulf, 102, "mission").await?;
            assert_eq!(experience(&h).await?, before);

            let fence = owner.current_quest_fence(admitted.game_session_id).await
                .map_err(debug)?.ok_or("quest fence")?;
            {
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let outcome = slot.commit_quest_transition(
                    owner.root, owner.character, owner.holder, fence,
                    QuestTransitionRequest {
                        transition_key: READY.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id, CommandId::new(103).map_err(debug)?,
                        )),
                    }, catalogue,
                ).await.map_err(debug)?;
                assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            }
            owner.refresh_quest_session(admitted.game_session_id, false).await;
            super::qengine_rewards::talk(&mut owner, &mut admitted, ongulf, 104, "mission").await?;
            assert_eq!(experience(&h).await?, before + 8000);
            for (track, stage) in [(QUESTLINE, 29), (MISSION, 2), (CARPET, 1)] {
                assert_eq!(super::m39_bone_journey::stage(&h, track).await?, Some(stage));
            }
            super::qengine_rewards::talk(&mut owner, &mut admitted, ongulf, 105, "mission").await?;
            assert_eq!(experience(&h).await?, before + 8000);
            println!("new_horizons_grants_experience_and_carpet_access_once: {:?}", started.elapsed());
            Ok(())
        })
    })
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
        .fetch_one(&h.pool).await?)
}
