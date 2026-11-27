//! Qualify the shipped classic quest placements with the production dispatch owners.
use super::TestResult;

#[test]
fn classic_quest_native_draconia_fixed_portals() -> TestResult {
    super::classic_quest_native_fixtures::run("portals")
}

#[test]
fn classic_quest_native_draconia_key_custody_and_doors() -> TestResult {
    super::classic_quest_native_fixtures::run("keys")
}

#[test]
fn classic_quest_native_draconia_levers_escape_and_expertise() -> TestResult {
    super::classic_quest_native_fixtures::run("puzzle")
}

#[test]
fn classic_quest_native_devil_helmet_sewer_and_rewards() -> TestResult {
    super::classic_quest_native_fixtures::run("devil")
}

#[test]
fn classic_quest_native_dragon_corpse_tools_and_reward() -> TestResult {
    super::classic_quest_native_fixtures::run("corpse")
}

#[test]
fn classic_quest_native_dragon_corpse_predecessor_tools_and_reward() -> TestResult {
    super::classic_quest_native_fixtures::run("corpse-predecessor")
}

#[test]
fn classic_quest_native_draconia_bags_complete_in_either_order() -> TestResult {
    for route in ["bags", "bags-reverse"] {
        super::classic_quest_native_fixtures::run(route)?;
    }
    Ok(())
}

#[test]
fn classic_quest_native_dragon_tower_backpacks_complete_in_either_order() -> TestResult {
    for route in ["tower", "tower-reverse"] {
        super::classic_quest_native_fixtures::run(route)?;
    }
    Ok(())
}

#[test]
fn classic_quest_native_dragon_corpse_shovel_rolls_back_with_quest_credit() -> TestResult {
    super::classic_quest_native_fixtures::run("corpse-shovel-rollback")
}

#[test]
fn classic_quest_native_dragon_corpse_scythe_rolls_back_with_quest_credit() -> TestResult {
    super::classic_quest_native_fixtures::run("corpse-scythe-rollback")
}
