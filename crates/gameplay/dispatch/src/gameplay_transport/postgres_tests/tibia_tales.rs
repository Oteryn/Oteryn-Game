use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::TestResult;

fn player_journey(name: &'static str, target: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, target, false)
    })
}

#[test]
fn tibia_tales_appease_requires_both_envoys_before_paying() -> TestResult {
    player_journey(
        "qengine-rewards-tibia-tales-appease",
        Position {
            x: 33294,
            y: 31677,
            floor: -15,
        },
    )
}

#[test]
fn tibia_tales_ultimate_booze_accept_fill_and_redeem() -> TestResult {
    player_journey(
        "qengine-rewards-tibia-tales-booze",
        Position {
            x: 32573,
            y: 31906,
            floor: -4,
        },
    )
}

#[test]
fn tibia_tales_exterminator_accept_poison_and_redeem() -> TestResult {
    player_journey(
        "qengine-rewards-tibia-tales-exterminator",
        Position {
            x: 32306,
            y: 31821,
            floor: -9,
        },
    )
}

#[test]
fn tibia_tales_outfox_accept_retrieve_and_exchange() -> TestResult {
    player_journey(
        "qengine-rewards-tibia-tales-outfox",
        Position {
            x: 32467,
            y: 31970,
            floor: -5,
        },
    )
}

#[test]
fn tibia_tales_hallowed_counts_distinct_graves_and_claims_potions() -> TestResult {
    player_journey(
        "qengine-rewards-tibia-tales-hallowed",
        Position {
            x: 33188,
            y: 31688,
            floor: -6,
        },
    )
}
