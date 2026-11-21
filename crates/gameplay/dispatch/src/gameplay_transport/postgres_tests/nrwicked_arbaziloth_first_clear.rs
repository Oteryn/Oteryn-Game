//! Arbaziloth's native lethal capture records the No Rest for the Wicked first clear.
//! Kept beside the M24 source-death census, which counts only its macro-generated rooms.
use super::TestResult;

#[test]
fn nrwicked_arbaziloth_death_records_first_clear() -> TestResult {
    super::srvenc_source_deaths_fixtures::scenario(
        "nrwicked_arbaziloth_death_records_first_clear",
        "arbaziloth",
    )
}
