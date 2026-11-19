use super::{TestResult, shadow_nexus_fixtures};

#[test]
fn shadow_nexus_native_use_with_decays_and_spends_only_the_shared_charge_count() -> TestResult {
    shadow_nexus_fixtures::shadow_nexus_native_use_with_decays_and_spends_only_the_shared_charge_count_scenario()
}
