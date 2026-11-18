use super::TestResult;
use super::quest_world_callbacks::{callback, literal_uses};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn order_of_the_stag_use_preserves_main_era_quest_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(
        !admin.is_empty(),
        "PostgreSQL route must be nonempty when configured"
    );
    super::kill_reward_e2e::scenario_thread::run("stag-main-era-use", || {
        let boat = callback("order-of-the-stag-boat-to-ada")?;
        assert!(boat.is_read_only());
        // Crystal isle_of_ada.lua:27-39 and order_of_the_stag.lua:60-62.
        assert_eq!(
            boat.selector.position,
            Some(Position {
                x: 32393,
                y: 32513,
                floor: -7
            })
        );
        assert_eq!(
            boat.teleport,
            Some(Position {
                x: 33875,
                y: 32249,
                floor: -7
            })
        );
        let minimum = boat.storage[0].minimum;
        assert_eq!(minimum, 3);
        literal_uses::exercise(boat.clone(), 6391, 4, Some(minimum - 1), true)?;
        literal_uses::exercise(boat, 6391, 4, Some(minimum), true)
    })
}
