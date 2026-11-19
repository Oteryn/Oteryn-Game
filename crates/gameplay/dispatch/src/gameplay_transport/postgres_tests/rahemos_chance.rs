use super::{TestResult, rahemos_chance_fixtures};

#[test]
fn rahemos_hat_roll_controls_damage_carrot_and_passage_to_the_oasis() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    rahemos_chance_fixtures::rahemos_hat_roll_controls_damage_carrot_and_passage_to_the_oasis_scenario()
}
