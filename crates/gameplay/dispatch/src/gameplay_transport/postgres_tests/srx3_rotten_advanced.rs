//! Actual source entry and native callback effects for the three advanced bosses.
use super::srx3_rotten_advanced_fixtures::{Extra, scenario_with_step};

use super::TestResult;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_vemiath_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
    )
}
#[test]
fn srx3_chagorz_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
        false,
        false,
    )
}
#[test]
fn srx3_bakragore_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        false,
    )
}

#[test]
fn srx3_vemiath_actual_source_teleport_exit_return_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        true,
    )
}
#[test]
fn qmrevfix_rotten_control_lost_players_block_placement_and_allow_children_postgres() -> TestResult
{
    scenario_with_step(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
        Extra::ControlLost,
    )
}

#[test]
fn srx3_vemiath_actual_source_portal_step_postgres() -> TestResult {
    scenario_with_step(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
        Extra::PortalStep,
    )
}
#[test]
fn srx3_vemiath_original_portal_due_precedes_death_and_late_mint_postgres() -> TestResult {
    scenario_with_step(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
        Extra::DueBeforeDeath,
    )
}
fn scenario(
    kind: PinnedActivitySourceRoom,
    origin: MovementLocalPosition,
    boss: &'static str,
    final_boss: bool,
    return_only: bool,
) -> TestResult {
    scenario_with_step(kind, origin, boss, final_boss, return_only, Extra::None)
}
