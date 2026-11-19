//! Crystal 201189f feaster_of_souls/creaturescripts_pale_worm_death.lua:1-19.
use super::TestResult;
use super::fixture::{Harness, run};

const QUEST: &str = "oteryn:quest.poltergeist_outfits_quest";
const RECEIVED: &str = "oteryn:quest-progress/quest/u12_30/poltergeist_outfits/received";
const TRANSITION: &str = "oteryn:quest-transition/source-death/pale-worm-poltergeist";
const RULE: &str = r#"{
    "creature":"oteryn:creature.the_pale_worm",
    "transition":"oteryn:quest-transition/source-death/pale-worm-poltergeist",
    "effects":[{"track":"oteryn:quest-progress/quest/u12_30/poltergeist_outfits/received",
                "default":-1,"from":{"kind":"any"},"set":1}],
    "room":[33794,31493,33816,31515,-14],
    "define_transition":true,
    "delivery":"poltergeist_and_beyonder"
}"#;

#[test]
fn pale_worm_room_death_qualifies_two_players_and_grants_poltergeist_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("pale-worm-poltergeist", None, 1)
}

#[test]
fn pale_worm_room_death_keeps_existing_outfit_storage_and_still_awards_beyonder() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    // The published track permits 0. The donor awards the base outfit only
    // from -1, and still grants Beyonder to any witness with an existing value.
    scenario("pale-worm-existing-outfit-storage", Some((RECEIVED, 0)), 0)
}

fn scenario(name: &'static str, prior: Option<(&'static str, i64)>, outfits: i64) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async move |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "pale_worm_outfit", false, false)
                    .await?;
            let result = exercise(&harness, prior, outfits).await;
            let cleaned = harness.cleanup().await;
            result?;
            cleaned?;
            Ok(())
        })
    })
}

async fn exercise(
    harness: &Harness,
    prior: Option<(&str, i64)>,
    expected_outfits: i64,
) -> TestResult {
    super::srvenc_source_deaths::exercise_delivery_with_prior(
        harness, TRANSITION, QUEST, RULE, prior,
    )
    .await?;
    let (outfits, achievements): (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM game_account_appearance_unlocks
             WHERE unlock_key='oteryn:outfit.poltergeist/quest-grant/0'),
            (SELECT count(*) FROM game_account_achievements
             WHERE achievement_key='oteryn:achievement/beyonder')",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!((outfits, achievements), (expected_outfits, 1));
    Ok(())
}
