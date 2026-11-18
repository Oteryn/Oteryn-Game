//! Crystal 201189f the_dream_courts/creaturescripts_dream_courts_death.lua:1-8,88-97.
use super::TestResult;
use super::fixture::{Harness, run};

const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const TRANSITION: &str = "oteryn:quest-transition/source-death/faceless-bane";
const RULE: &str = r#"{
    "creature":"oteryn:creature.faceless_bane",
    "transition":"oteryn:quest-transition/source-death/faceless-bane",
    "effects":[{"track":"oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/questline",
                "default":0,"from":{"kind":"any"},"increment_to":5}],
    "define_transition":true,
    "cooldown":{"boss":"oteryn:creature.faceless_bane","seconds":72000}
}"#;

#[test]
fn faceless_bane_death_advances_haunted_house_and_sets_its_personal_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("faceless-bane-credit", || {
        run(async |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "faceless_bane_credit", false, false)
                    .await?;
            let result = exercise(&harness).await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    })
}

async fn exercise(harness: &Harness) -> TestResult {
    super::srvenc_source_deaths::exercise_delivery(harness, TRANSITION, QUEST, RULE).await?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         FROM game_character_boss_cooldowns WHERE boss_key='oteryn:creature.faceless_bane'",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert!((71_990_000..=72_000_000).contains(&remaining));
    Ok(())
}
