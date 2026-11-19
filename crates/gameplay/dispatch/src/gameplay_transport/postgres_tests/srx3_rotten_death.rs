//! Loaded mechanics death events retain their real corpse and optional one-unit loot.
use super::TestResult;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;

use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_murcion_empty_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
        PinnedActivitySourceRoom::Murcion,
        MovementLocalPosition {
            x: 32974,
            y: 32365,
            floor: 15,
        },
        "oteryn:creature.murcion",
        false,
        false,
    )
}

#[test]
fn srx3_ichgahal_empty_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
        PinnedActivitySourceRoom::Ichgahal,
        MovementLocalPosition {
            x: 32978,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.ichgahal",
        false,
        false,
    )
}

#[test]
fn srx3_vemiath_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
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
fn srx3_chagorz_copied_death_preserves_credited_corpse_owner_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
        PinnedActivitySourceRoom::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
        true,
        false,
    )
}

#[test]
fn srx3_bakragore_uncredited_death_retains_actual_first_think_essence_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        false,
        true,
    )
}

#[test]
fn srx3_bakragore_credited_death_retains_actual_first_think_essence_postgres() -> TestResult {
    super::srx3_rotten_advanced_fixtures::copied_death(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        true,
    )
}

#[test]
fn srx3_bakragore_full_reward_queue_finishes_copied_death_without_mint_or_xp_postgres() -> TestResult
{
    super::srx3_rotten_advanced_fixtures::copied_death_with_full_queue()
}
