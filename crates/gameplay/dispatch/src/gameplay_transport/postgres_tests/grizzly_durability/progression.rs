//! NPC:462 awards XP before NPC:485 chooses point caps and catch-up bonuses.
use super::*;
use crate::domain::progression::FiniteProgressionPolicy;

#[test]
fn grizzly_task_points_use_native_post_xp_level_and_pending_batch_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("GRIZZLY_POST_XP_POINTS POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("grizzly-post-xp", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        // Both cap boundaries, the highest cap, and the separate level-130 catch-up.
        for (level, points, expected, batch) in [(49, 40, 41, false), (79, 70, 71, false),
            (129, 100, 101, false), (129, 20, 24, false), (49, 40, 41, true)] {
            let h = Harness::create(admin.clone(), "grizzly_post_xp_points", true).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let sequencer = CharacterRevisionSequencer::new();
            let catalogue = catalogue()?;
            let gate = PremiumProbe::new(true);
            let (controller, _pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
                fence(1)?.runtime_scope.world_id(), None);
            let policy = controller.active().and_then(|generation| generation.native_gameplay())
                .and_then(|native| native.progression()).ok_or("native progression")?
                .policy_for("profile-1", "ruleset-1", "content-1").map_err(debug)?;
            let threshold = policy.thresholds.iter().find(|entry| entry.level == level + 1)
                .ok_or("next native level")?.minimum_experience.get();
            let before_xp = threshold - if batch { 1200 } else { 800 };
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_character_progression_state SET level=$2,total_experience=$3,simulation_revision=$4,evidence_revision=$5,declaration_revision=$6,policy_revision=$7,reward_revision=$8 WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(i64::from(level)).bind(before_xp)
                .bind(&policy.context.simulation).bind(&policy.context.evidence)
                .bind(&policy.context.declaration).bind(&policy.policy_revision)
                .bind(&policy.reward_revision).execute(&mut *tx).await?;
            tx.commit().await?;
            committed(apply(&h, &authority, &sequencer, &catalogue,
                command(1)?, Action::Join, &gate).await?)?;
            seed(&h, None, &[(quest::POINTS_STORAGE, points),
                (quest::QUEST_STORAGE_BASE + 1, 1), (quest::KILLS_STORAGE_BASE + 1, 0),
                (quest::KILL_COUNTER_BASE + 1, 300),
                (quest::QUEST_STORAGE_BASE + 2, 1), (quest::KILLS_STORAGE_BASE + 2, 0),
                (quest::KILL_COUNTER_BASE + 2, 300)]).await?;

            // No policy means no guessed level, no point delta and no XP obligation.
            let missing = sequencer.acquire(fence(1)?.character_id).await
                .commit_grizzly_transition(&h.root, &authority, &h.node, fence(1)?, command(2)?,
                    Action::CompleteTask(1), Arc::clone(&catalogue), &gate).await.map_err(debug)?;
            assert_eq!(missing, QuestTransitionOutcome::Refused(QuestRefusal::NotSupported));
            let mut wrong = policy.clone();
            wrong.reward_revision.push_str("-other");
            assert_eq!(complete(&h, &authority, &sequencer, &catalogue, &gate, &wrong, 3, 1).await?,
                QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch));
            assert!(state(&h, &authority).await?.xp_obligations().is_empty());
            assert_eq!(value(&h, &authority, quest::POINTS_STORAGE).await?, points);

            let first = committed(complete(&h, &authority, &sequencer, &catalogue, &gate, &policy, 4, 1).await?)?;
            assert_eq!(first.experience.ok_or("crocodile XP")?.amount, 800);
            assert_eq!(value(&h, &authority, quest::POINTS_STORAGE).await?, if batch { points } else { expected });
            let rebuilt = CharacterRevisionSequencer::new();
            assert_eq!(complete(&h, &authority, &rebuilt, &catalogue, &gate, &policy, 4, 1).await?,
                QuestTransitionOutcome::AlreadyCommitted(first));
            if batch {
                // Same NPC utterance, different source task. The first 800 XP remains
                // pending; its later 500 XP must still cross level 50 for this reward.
                let second = committed(complete(&h, &authority, &rebuilt, &catalogue, &gate, &policy, 4, 2).await?)?;
                assert_eq!(second.experience.ok_or("badger XP")?.amount, 500);
            }
            assert_eq!(value(&h, &authority, quest::POINTS_STORAGE).await?, expected);
            let before_delivery: (i64, i64) = sqlx::query_as("SELECT level,total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(before_delivery, (i64::from(level), before_xp), "the quest writer must not write XP");
            let mut copy = state(&h, &authority).await?;
            assert_eq!(copy.xp_obligations().len(), if batch { 2 } else { 1 });
            assert!(!crate::durability::quest_state::request_pending_quest_experience(
                &rebuilt, &h.root, &authority, &h.node, fence(1)?, &policy, &mut copy).await);
            assert!(copy.xp_obligations().is_empty());
            let delivered: (i64, i64) = sqlx::query_as("SELECT level,total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(delivered, (i64::from(level + 1), before_xp + if batch { 1300 } else { 800 }));
            assert_eq!(value(&h, &authority, quest::POINTS_STORAGE).await?, expected);
            drop(authority); drop(seal); h.cleanup().await?;
        }
        eprintln!("GRIZZLY_POST_XP_POINTS POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
async fn complete<const N: usize>(
    h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>, sequencer: &CharacterRevisionSequencer,
    catalogue: &Arc<QuestStateCatalogue>, gate: &PremiumProbe,
    policy: &FiniteProgressionPolicy<String, N>, cause: u64, task: u8,
) -> TestResult<QuestTransitionOutcome> {
    sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_transition_with_progression(
        &h.root, authority, &h.node, fence(1)?, command(cause)?, Action::CompleteTask(task),
        Arc::clone(catalogue), gate, Some(policy)).await.map_err(debug)
}
