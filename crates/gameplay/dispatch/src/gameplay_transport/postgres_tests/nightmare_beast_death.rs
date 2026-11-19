//! Crystal 201189f creaturescripts_dream_courts_death.lua:50-54,67-99.
use super::TestResult;
use super::fixture::{Harness, run};

const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const PROGRESS: &str =
    "oteryn:quest-progress/quest/u12_00/the_dream_courts/ward_stones/questline";
const TRANSITION: &str = "oteryn:quest-transition/source-death/nightmare-beast";
const RULE: &str = r#"{
    "creature":"oteryn:creature.the_nightmare_beast",
    "transition":"oteryn:quest-transition/source-death/nightmare-beast",
    "effects":[{"track":"oteryn:quest-progress/quest/u12_00/the_dream_courts/ward_stones/questline",
                "default":0,"from":{"kind":"any"},"increment_to":3}],
    "define_transition":true,
    "delivery":"dream_warrior",
    "cooldown":{"boss":"oteryn:creature.the_nightmare_beast","seconds":72000}
}"#;

#[test]
fn nightmare_beast_death_awards_dream_warrior_to_damage_recipients_and_advances_ward_stones()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("nightmare-beast-death-advances-ward-stones", 2)
}

#[test]
fn nightmare_beast_capped_deaths_refresh_cooldown_and_announce_the_first_outfit_only() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("nightmare-beast-death-at-ward-stones-cap", 3)
}

fn scenario(name: &'static str, before: i64) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async move |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "nightmare_beast_death", false, false)
                    .await?;
            let result = exercise(&harness, before).await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn exercise(harness: &Harness, before: i64) -> TestResult {
    super::srvenc_source_deaths::exercise_delivery_with_prior(
        harness,
        TRANSITION,
        QUEST,
        RULE,
        Some((PROGRESS, before)),
        None,
    )
    .await?;
    let (bare_unlocks, other_unlocks): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE unlock_key='oteryn:outfit.dream_warrior/quest-grant/0'
                                  AND content_revision='crystal-9f5a72c-quest-rewards-v1'
                                  AND source_kind='quest'),
                count(*) FILTER (WHERE unlock_key<>'oteryn:outfit.dream_warrior/quest-grant/0')
         FROM game_account_appearance_unlock_receipts
         WHERE unlock_key LIKE 'oteryn:outfit.dream_warrior/%'",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!((bare_unlocks, other_unlocks), (1, 0));
    let remaining: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         FROM game_character_boss_cooldowns
         WHERE boss_key='oteryn:creature.the_nightmare_beast'",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert!((71_990_000..=72_000_000).contains(&remaining));
    Ok(())
}
