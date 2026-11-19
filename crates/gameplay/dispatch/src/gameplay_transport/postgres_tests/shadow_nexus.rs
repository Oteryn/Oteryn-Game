use super::{TestResult, shadow_nexus_fixtures};

#[test]
fn shadow_nexus_native_use_with_decays_and_spends_only_the_shared_charge_count() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    shadow_nexus_fixtures::shadow_nexus_native_use_with_decays_and_spends_only_the_shared_charge_count_scenario()
}
