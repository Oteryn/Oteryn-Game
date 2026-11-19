use super::TestResult;

#[test]
fn mechanical_fishing_whole_percent_miss_shocks_without_burning_nail() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("mechanical-fishing-shock", || {
        super::standard_fishing_fixtures::gathering_scenario(
            9306,
            4597,
            Some(4597),
            None,
            false,
            true,
            Some(11),
        )
    })
}

#[test]
fn mechanical_fishing_shock_reaches_sewer_water_exclusive_to_the_mechanical_rod() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    // Crystal mechanical_fishing.lua:11-24,203-237 includes 21312.
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "mechanical-fishing-shock-exclusive-water",
        || {
            super::standard_fishing_fixtures::gathering_scenario(
                9306,
                21312,
                Some(21312),
                None,
                false,
                true,
                Some(10),
            )
        },
    )
}

#[test]
fn mechanical_fishing_on_spent_sewer_water_shows_energy_without_training_or_consuming() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    // Crystal mechanical_fishing.lua:203-218 returns after the energy effect.
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "mechanical-fishing-spent-sewer-water",
        || {
            super::standard_fishing_fixtures::gathering_scenario(
                9306,
                4609,
                Some(4609),
                None,
                false,
                true,
                None,
            )
        },
    )
}

#[test]
fn mechanical_fishing_on_spent_water_outside_sewers_still_shows_energy() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    // Crystal mechanical_fishing.lua:270-273 gives the same cue without the sewer or nail.
    super::kill_reward_e2e_fixtures::scenario_thread::run("mechanical-fishing-spent-water", || {
        super::standard_fishing_fixtures::gathering_scenario(
            9306,
            4609,
            Some(4609),
            None,
            false,
            false,
            None,
        )
    })
}
