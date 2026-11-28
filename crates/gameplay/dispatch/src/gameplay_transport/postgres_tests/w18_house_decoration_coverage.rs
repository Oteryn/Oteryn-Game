use super::TestResult;
use super::house_balloon_fixtures::{Case, run_case};

#[test]
fn w18_house_wall_lamp_use_replays_the_original_framed_command() -> TestResult {
    run_case(Case::Decoration(39446, 39447))
}

#[test]
fn w18_house_folded_carpet_use_replays_the_original_framed_command() -> TestResult {
    run_case(Case::Decoration(22737, 22736))
}

#[test]
fn w18_house_bed_kit_colours_replay_after_the_kit_is_consumed() -> TestResult {
    for kit in [832, 833, 834, 17972] {
        run_case(Case::BedKit(kit))?;
    }
    Ok(())
}
