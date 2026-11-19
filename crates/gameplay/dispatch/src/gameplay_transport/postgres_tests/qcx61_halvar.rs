//! Crystal 201189f8 halvar.lua:70-115 and movements_arena_enter.lua:3-40.
//! The Barbarian Test prerequisite is confirmed by npc/sven.lua:107-115 and
//! the Svargrond Arena entry requirements in the Tibia Fandom archive.
use super::TestResult;

#[test]
fn halvar_payment_opens_the_native_forcefield_without_a_second_fee() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-halvar",
        super::arena_match_fixtures::halvar_paid_entrance,
    )
}
