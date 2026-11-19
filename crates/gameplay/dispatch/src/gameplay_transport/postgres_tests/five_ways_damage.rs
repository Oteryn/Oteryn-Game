use super::{TestResult, five_ways_damage_fixtures};

#[test]
fn five_ways_step_in_uses_base_vocation_and_final_field_element_protection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    five_ways_damage_fixtures::five_ways_step_in_uses_base_vocation_and_final_field_element_protection_scenario()
}

#[test]
fn five_ways_final_field_spends_one_ordinary_equipped_protection_charge_per_hit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    five_ways_damage_fixtures::five_ways_final_field_spends_one_ordinary_equipped_protection_charge_per_hit_scenario()
}
