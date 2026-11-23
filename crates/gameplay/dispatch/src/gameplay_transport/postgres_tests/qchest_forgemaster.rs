//! Positive USE witnesses for the remaining Forgemaster weapon choices.

use super::TestResult;
use super::quest_container_chests_fixtures::{Case, execute};

// Crystal 201189f8: data-global/scripts/quests/no_rest_for_the_wicked/
// no_rest_for_the_wicked_rewards.lua:2-12,16-24.
#[test]
fn qchest_forgemaster_placed_choices_use_once_and_refusal() -> TestResult {
    for (x, reward) in [
        (34087, "oteryn:item.tibia.i49520"),
        (34089, "oteryn:item.tibia.i49522"),
        (34091, "oteryn:item.tibia.i49523"),
        (34093, "oteryn:item.tibia.i49524"),
        (34095, "oteryn:item.tibia.i49525"),
        (34097, "oteryn:item.tibia.i49526"),
        (34099, "oteryn:item.tibia.i49527"),
        (34103, "oteryn:item.tibia.i49528"),
        (34105, "oteryn:item.tibia.i49529"),
        (34107, "oteryn:item.tibia.i50250"),
    ] {
        execute(
            Case::PlacedForgemaster { x, reward },
            "qchest_forgemaster_placed_choices_use_once_and_refusal",
        )
        .map_err(|error| format!("Forgemaster chest x={x}, {reward}: {error}"))?;
    }
    Ok(())
}
