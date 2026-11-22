//! Quest package coverage for the Renegade Orc source death.

use super::{TestResult, srvenc_source_deaths_fixtures::scenario};

#[test]
fn srvenc_m24_postgres_renegade_orc_damage() -> TestResult {
    scenario("srvenc_m24_postgres_renegade_orc_damage", "renegade_orc")
}
