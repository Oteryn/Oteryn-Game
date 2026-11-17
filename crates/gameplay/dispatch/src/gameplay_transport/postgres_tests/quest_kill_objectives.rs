//! Native quest objectives must be content-bound before any death can advance them.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};

#[test]
fn quest_kill_objectives_postgres_content_and_native_authority() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-kill-objectives", || {
        run(async |admin| {
            reset();
            let started = std::time::Instant::now();
            let h =
                Harness::create_with_admission_seed(admin, "quest_kill_objectives", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            eprintln!("QUEST_KILL_OBJECTIVES POSTGRES_CASE_OBSERVED server_version_num={version}");
            let catalogue =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            assert!(
                catalogue
                    .catalogue()
                    .transition("oteryn:quest-transition/elvenbane/dragon")
                    .is_some(),
                "authored Dragon objective needs its production native-death binding"
            );
            let deps = Dependencies::new()?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|_| "current quest fence")?
                .ok_or("current session")?;
            let before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            use crate::durability::character_progression::{
                CharacterProgressionError, ExperienceRewardOccurrence,
            };
            use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
            use crate::foundation::{CommandId, CommandRef};
            for transition in [
                "oteryn:quest-transition/elvenbane/dragon",
                "oteryn:quest-transition/native-kill/elvenbane-dragon",
            ] {
                for cause in [
                    QuestCause::Command(CommandRef::new(
                        deps.admitted.game_session_id,
                        CommandId::new(9000).map_err(debug)?,
                    )),
                    QuestCause::CreatureDeath(
                        ExperienceRewardOccurrence::from_bytes(super::fixture::id(91))
                            .map_err(debug)?,
                    ),
                ] {
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    let result = slot
                        .commit_quest_transition(
                            owner.root,
                            owner.character,
                            owner.holder,
                            fence,
                            QuestTransitionRequest {
                                transition_key: transition.into(),
                                cause,
                            },
                            std::sync::Arc::new(catalogue.catalogue().clone()),
                        )
                        .await;
                    assert!(
                        matches!(result, Err(CharacterProgressionError::InvalidInput)),
                        "generic command/death occurrence cannot forge native source admission: {result:?}"
                    );
                }
            }
            let after: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                after, before,
                "forged native objectives write no Quest receipt"
            );
            eprintln!(
                "QUEST_KILL_OBJECTIVES POSTGRES_CASE_COMPLETED elapsed_ms={}",
                started.elapsed().as_millis()
            );
            Ok(())
        })
    })
}
