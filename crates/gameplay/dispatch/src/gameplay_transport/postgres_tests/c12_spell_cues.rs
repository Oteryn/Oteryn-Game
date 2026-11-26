use super::TestResult;

#[test]
fn committed_damaging_cast_publishes_words_and_actual_target_damage_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::parameter_spell_fixtures::cast_cues_scenario()
}
