use super::*;

#[test]
fn rotten_blood_postgres_literal_altar_consumption_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("rotten-blood-altar", || {
        super::scenario(
            "rotten-blood/sacrifice",
            Position {
                x: 100,
                y: 100,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn rotten_blood_postgres_npc_detour_coin_bank_fee_atomicity_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("rotten-blood-detour", || {
        super::scenario(
            "rotten-blood-access-purchase",
            Position {
                x: 100,
                y: 100,
                floor: -7,
            },
            false,
        )
    })
}
