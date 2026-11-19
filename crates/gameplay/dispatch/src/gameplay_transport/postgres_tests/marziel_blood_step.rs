use super::TestResult;
use super::marziel_blood_step_fixtures::{NAME, TRIGGER};

#[test]
fn marziel_step_requires_female_character_and_exact_blood_before_spawning() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        for name in ["marziel-blood-step-male", "marziel-blood-step-female"] {
            super::quest_world_callbacks_fixtures::scenario(name, TRIGGER, false)?;
        }
        Ok(())
    })
}
