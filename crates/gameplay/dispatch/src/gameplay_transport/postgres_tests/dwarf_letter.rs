use super::TestResult;

#[test]
fn both_dwarfs_drop_one_lootable_letter_through_live_combat() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    for creature in ["oteryn:creature.dwarf", "oteryn:creature.crazed_dwarf"] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(creature, move || {
            super::kill_reward_e2e_fixtures::dwarf_letter_scenario(creature)
        })?;
    }
    Ok(())
}
