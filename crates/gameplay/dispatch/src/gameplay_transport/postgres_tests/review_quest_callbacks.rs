use super::{
    TestResult, kill_reward_e2e_fixtures::scenario_thread,
    quest_world_callbacks_fixtures as fixture,
};

#[test]
fn published_coffin_recovers_until_its_count_death_commits() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("published_coffin_recovery", || {
        let trigger = fixture::callback(super::inquisition_coffin_recovery_fixtures::RESTART)?
            .selector
            .position
            .ok_or("coffin selector")?;
        fixture::scenario(
            "inquisition-weakened-coffin#published_restart",
            trigger,
            false,
        )
    })
}

#[test]
fn serpentine_close_resolves_a_lost_ack_when_the_first_receipt_read_is_unavailable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("serpentine_close_receipt", || {
        fixture::scenario(
            "serpentine-tower-lever#lost_ack",
            super::serpentine_tower_fixtures::LEVER,
            false,
        )
    })
}

#[test]
fn sarina_vines_refuse_pz_locked_use_on_ordinary_ground() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    for name in [
        "thieves-guild-sarina-vine-climb",
        "thieves-guild-sarina-vine-descend",
    ] {
        scenario_thread::run(name, move || {
            let selector = fixture::callback(name)?
                .selector
                .native
                .ok_or("vine selector")?;
            fixture::scenario(name, selector.position, false)
        })?;
    }
    Ok(())
}
