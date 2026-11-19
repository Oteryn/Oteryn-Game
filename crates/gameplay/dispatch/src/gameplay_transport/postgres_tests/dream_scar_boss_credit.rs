//! Crystal 201189f the_dream_courts/creaturescripts_dream_courts_death.lua:17-33,88-97.
use super::TestResult;
use super::fixture::{Harness, run};

const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const TRANSITION: &str = "oteryn:quest-transition/source-death/izcandar-winter";
const BOSS_COUNT: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/dream_scar/boss_count";
const RULE: &str = r#"{
    "creature":"oteryn:creature.izcandar_champion_of_winter",
    "transition":"oteryn:quest-transition/source-death/izcandar-winter",
    "effects":[{"track":"oteryn:quest-progress/quest/u12_00/the_dream_courts/dream_scar/boss_count",
                "default":-1,"from":{"kind":"any"},"increment_to":6}],
    "define_transition":true,
    "cooldown":{"boss":"oteryn:creature.izcandar_the_banished","seconds":72000}
}"#;

#[test]
fn izcandar_form_death_credits_the_damaging_player_and_refreshes_the_original_boss_timer()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("dream-scar-boss-credit", None)
}

#[test]
fn izcandar_form_death_refreshes_the_original_boss_timer_at_the_progress_cap() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("dream-scar-capped-boss-credit", Some((BOSS_COUNT, 6)))
}

fn scenario(name: &'static str, prior: Option<(&'static str, i64)>) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async move |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "dream_scar_credit", false, false)
                    .await?;
            let result = exercise(&harness, prior).await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    })
}

async fn exercise(harness: &Harness, prior: Option<(&str, i64)>) -> TestResult {
    super::srvenc_source_deaths::exercise_delivery_with_prior(
        harness, TRANSITION, QUEST, RULE, prior, None,
    ).await?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         FROM game_character_boss_cooldowns WHERE boss_key='oteryn:creature.izcandar_the_banished'",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert!((71_990_000..=72_000_000).contains(&remaining));
    Ok(())
}
