//! Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:5-10,30-55.

use super::TestResult;

#[test]
fn coal_basin_sacrifices_only_the_arriving_split_and_replays_the_other_players_landing()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::coal_basin_arrival(admin)
}
