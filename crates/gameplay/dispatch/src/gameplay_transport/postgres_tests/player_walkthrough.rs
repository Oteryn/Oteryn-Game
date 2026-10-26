//! SPAWNGLOBAL: Retro PvP collision through PostgreSQL policy and `serve_admitted`.

use super::TestResult;

#[test]
fn retro_world_blocks_an_outside_pz_step_into_another_player() -> TestResult {
    super::pvp_live::retro_walkthrough_scenario()
}
