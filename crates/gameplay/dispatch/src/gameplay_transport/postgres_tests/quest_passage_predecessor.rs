use super::TestResult;
use super::quest_world_callbacks::{callback, literal_uses};

#[test]
fn order_of_the_stag_use_preserves_main_era_quest_save() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("stag-main-era-use", || {
        let boat = callback("order-of-the-stag-boat-to-ada")?;
        assert!(boat.is_read_only());
        let minimum = boat.storage[0].minimum;
        literal_uses::exercise(boat.clone(), 6391, 4, Some(minimum - 1), true)?;
        literal_uses::exercise(boat, 6391, 4, Some(minimum), true)
    })
}
