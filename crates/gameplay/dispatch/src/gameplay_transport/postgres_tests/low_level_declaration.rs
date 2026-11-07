//! A declaration correction must preserve immutable XP receipts and allow the next award.
use super::TestResult;
use super::fixture::{Harness, debug, fence, id, run};
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

const OLD: &str = "character-progression-differences-v1-c6d0829911772c01b4a229a5bb0aa8f9";
const MIGRATION: &str = include_str!(
    "../../../../../../apps/game-server/migrations/0539_low_level_experience_declaration.sql"
);

fn award(tag: u8, declaration: &str) -> TestResult<ExperienceAwardRequest<2>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: declaration.into(),
    };
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: declaration.into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

#[test]
fn low_level_declaration_postgres_migrates_state_preserves_receipt_and_next_award() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("low-level-declaration", || {
        run(async |admin| {
            let h = Harness::create(admin.clone(), "low_level_declaration", true).await?;
            let unrelated = Harness::create(admin, "unrelated_declaration", true).await?;
            // Seed the pre-migration declaration on the revision-one initializer, which has no receipt.
            let mut seed = h.pool.begin().await?;
            sqlx::raw_sql("ALTER TABLE game_character_progression_state DISABLE TRIGGER game_character_progression_state_guard")
                .execute(&mut *seed).await?;
            sqlx::query("UPDATE game_character_progression_state SET declaration_revision=$1")
                .bind(OLD)
                .execute(&mut *seed)
                .await?;
            sqlx::raw_sql("SET CONSTRAINTS ALL IMMEDIATE")
                .execute(&mut *seed)
                .await?;
            sqlx::raw_sql("ALTER TABLE game_character_progression_state ENABLE TRIGGER game_character_progression_state_guard")
                .execute(&mut *seed).await?;
            seed.commit().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let request = award(91, OLD)?;
            let committed = h
                .root
                .commit_character_experience(&authority, &h.node, fence(1)?, request.clone())
                .await
                .map_err(debug)?;
            let receipt_before: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(x) FROM game_character_xp_receipts x")
                    .fetch_one(&h.pool)
                    .await?;
            let before: serde_json::Value = sqlx::query_scalar(
                "SELECT to_jsonb(s)-'declaration_revision' FROM game_character_progression_state s",
            )
            .fetch_one(&h.pool)
            .await?;
            let mut tx = h.pool.begin().await?;
            sqlx::raw_sql(MIGRATION).execute(&mut *tx).await?;
            sqlx::raw_sql(MIGRATION).execute(&mut *tx).await?;
            tx.commit().await?;
            let current: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../../rulesets/character/experience/declared-differences.json"
            ))?;
            let expected = current["revision"].as_str().ok_or("declaration revision")?;
            assert_ne!(expected, OLD);
            let stored = h
                .root
                .read_character_progression(&authority, fence(2)?.character_id)
                .await
                .map_err(debug)?
                .ok_or("state")?;
            assert_eq!(stored.context.declaration, expected);
            let after: serde_json::Value = sqlx::query_scalar(
                "SELECT to_jsonb(s)-'declaration_revision' FROM game_character_progression_state s",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(after, before);
            let receipt_after: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(x) FROM game_character_xp_receipts x")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(receipt_before, receipt_after);
            let replay = h
                .root
                .commit_character_experience(&authority, &h.node, fence(1)?, request)
                .await
                .map_err(debug)?;
            let ExperienceCommitOutcome::Committed(committed) = committed else {
                return Err("first award was replayed".into());
            };
            assert_eq!(replay, ExperienceCommitOutcome::AlreadyCommitted(committed));
            assert!(
                h.root
                    .commit_character_experience(&authority, &h.node, fence(2)?, award(92, OLD)?)
                    .await
                    .is_err()
            );
            h.root
                .commit_character_experience(&authority, &h.node, fence(2)?, award(92, expected)?)
                .await
                .map_err(debug)?;
            let triggers: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_trigger WHERE tgrelid='game_character_progression_state'::regclass AND tgenabled='D'")
                .fetch_one(&h.pool).await?;
            assert_eq!(triggers, 0);
            let unrelated_before: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_character_progression_state s")
                    .fetch_one(&unrelated.pool)
                    .await?;
            let mut tx = unrelated.pool.begin().await?;
            sqlx::raw_sql(MIGRATION).execute(&mut *tx).await?;
            tx.commit().await?;
            let unrelated_after: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_character_progression_state s")
                    .fetch_one(&unrelated.pool)
                    .await?;
            assert_eq!(unrelated_before, unrelated_after);
            eprintln!(
                "PKG3 declaration migration: stored revision, unchanged receipt, replay, stale refusal and next award executed"
            );
            drop(authority);
            drop(seal);
            unrelated.cleanup().await?;
            h.cleanup().await
        })
    })
}
