//! Crystal 201189f8, a_piece_of_cake_config.lua:116-118,456-467 and
//! a_piece_of_cake/world_bonus.lua:3-42. The persisted active occurrence
//! is a fixture. The timed-item owner and attack settlement are production.
use super::TestResult;
use super::party_xp_fixtures as party_xp;

#[test]
fn cake_bonus_regenerates_both_players_and_scales_only_premium_shared_xp() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    party_xp::run(party_xp::Case::CakeBonus)
}
