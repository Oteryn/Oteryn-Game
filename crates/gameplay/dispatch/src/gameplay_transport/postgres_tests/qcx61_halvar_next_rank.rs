//! Crystal 201189f8 movements_arena_pit.lua:26-48 and halvar.lua:74-107.
use super::TestResult;

#[test]
fn halvar_admits_scrapper_after_greenhorn_without_a_leave_command() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-halvar-next-rank",
        super::arena_match_fixtures::halvar_next_rank,
    )
}
