//! Crystal 201189f8 data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:31-58.

use super::TestResult;
use super::solo_quest_room_ejection_fixtures::{CaveRoom, cave_case};

#[test]
fn solo_quest_room_ejects_the_admitted_player_after_the_last_monster_dies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "solo_quest_room_ejection", CaveRoom::EmptyAtTimeout)
}

#[test]
fn cave_spider_refuses_occupied_entry_then_ejects_only_the_admitted_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "cave_spider_room_cleanup", CaveRoom::OccupiedEntry)
}

#[test]
fn cave_spider_first_deadline_cleans_new_monsters_and_second_deadline_ejects_second_player()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "cave_spider_room_reentry", CaveRoom::TwoAdmissions)
}
