use super::{TestResult, m06_keys_fixtures::source_key_door};

#[test]
fn outlaw_camp_key_3303_unlocks_the_mine_door() -> TestResult {
    source_key_door(32614, 32175, -9, 3303)
}

#[test]
fn outlaw_camp_key_3304_unlocks_the_treasure_door() -> TestResult {
    source_key_door(32620, 32199, -10, 3304)
}

#[test]
fn explorer_society_key_3802_unlocks_dwacatra_prison() -> TestResult {
    source_key_door(32602, 31975, -14, 3802)
}

#[test]
fn outlaw_camp_treasure_use_commits_bag_and_completion_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("outlaw-camp-treasure", || {
        super::quest_world_callbacks_fixtures::scenario(
            "outlaw-camp-treasure",
            super::outlaw_camp_treasure_fixtures::CHEST,
            false,
        )
    })
}

#[test]
fn explorer_society_enrollment_unlocks_dwacatra_through_npc_service() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("explorer-society-enrollment", || {
        super::quest_world_callbacks_fixtures::scenario(
            "explorer-society-enrollment",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
