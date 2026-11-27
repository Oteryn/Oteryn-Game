use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{TestResult, m06_keys_fixtures::source_key_door};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn circle_room_source_expertise_door_requires_level_32() -> TestResult {
    source_key_door(32510, 31956, -13, 1648, 1032)
}

#[test]
fn crusader_helmet_source_expertise_door_requires_level_35() -> TestResult {
    source_key_door(32475, 31946, -13, 1648, 1035)
}

#[test]
fn crystal_wand_source_expertise_door_requires_level_60() -> TestResult {
    source_key_door(32483, 31722, -15, 5111, 1060)
}

#[test]
fn captain_iglues_left_chest_renews_letter_and_twelve_salmon() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("captain-iglues-left-chest", || {
        super::quest_world_callbacks_fixtures::scenario(
            "captain-iglues-left-chest",
            Position {
                x: 32038,
                y: 32121,
                floor: -13,
            },
            false,
        )
    })
}

#[test]
fn children_greasy_oil_shelf_uses_the_native_claim() -> TestResult {
    use super::quest_catalogue_chests_fixtures::{cases_including, execute};
    const CLAIM: &str =
        "oteryn:reward-claim.quest.u8_54.children_of_the_revolution.flask_of_extra_greasy_oil";
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let shelf = cases_including(|key| key == CLAIM)?
        .into_iter()
        .find(|chest| chest.claim == CLAIM)
        .ok_or("source grease shelf")?;
    assert_eq!(shelf.position, (33327, 31410, -8));
    assert_eq!(shelf.unique, 6290);
    assert_eq!(shelf.root, "oteryn:item.tibia.i10189");
    execute(vec![shelf])
}

#[test]
fn realm_draconia_chest_grants_written_book_once() -> TestResult {
    run_source_case(
        "m07/realm-draconia-book",
        Position {
            x: 32804,
            y: 31586,
            floor: 0,
        },
    )
}

#[test]
fn realm_corpse_grants_all_eight_items_and_completes_once() -> TestResult {
    run_source_case(
        "m07/realm-reward",
        Position {
            x: 32827,
            y: 31493,
            floor: -6,
        },
    )
}

#[test]
fn crystal_wand_room_grants_twinkiller_parchment_once() -> TestResult {
    run_source_case(
        "crystal-wand-twinkiller",
        Position {
            x: 32478,
            y: 31611,
            floor: -15,
        },
    )
}

fn run_source_case(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, position, false)
    })
}

#[test]
fn realm_source_lever_reuses_key_after_both_deadlines() -> TestResult {
    run_source_case(
        super::m07_realm_trials_fixtures::NAME,
        super::m07_realm_trials_fixtures::CONTROL,
    )
}

#[test]
fn children_greased_levers_keep_their_native_reset_lifecycle() -> TestResult {
    run_source_case(
        "qn1-children-lever-1/ordinary",
        super::m44_children_lifecycle_fixtures::AT,
    )
}

#[test]
fn spike_hunt_reports_fame_and_both_purchases_close_the_outfit() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-cave-explorer-story")
}
