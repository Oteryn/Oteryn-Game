//! P08 perk modification through the production command owner (commands 379, 380, 392-395)
//! with the Global shaping values and migration 0575's activation: MODIFY spends 250 dust and
//! lands on rank 0, ORB_RANK burns one carried Lunar Ascension Orb with its line, a replay burns
//! nothing, RANK_UP advances with donor-evidenced values, CLEAR frees the slot and ORB_RANK without an orb is
//! `NO_ORB`.
use super::proficiency_modification_fixtures::{scenario_with_state};

use super::TestResult;

#[test]
fn proficiency_modification_spends_dust_burns_an_orb_and_clears() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency_modification".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency modification scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    scenario_with_state(false)
}
