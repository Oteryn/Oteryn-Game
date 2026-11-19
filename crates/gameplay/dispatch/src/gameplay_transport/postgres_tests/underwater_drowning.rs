use super::{TestResult, underwater_drowning_fixtures};

use underwater_drowning_fixtures::{on_thread, scenario, scenario_in_world};

#[test]
fn fixm10_swimming_and_map_traps_postgres() -> TestResult {
    on_thread(|| {
        scenario(None, false)?;
        scenario_in_world(None, false, false, None)
    })
}

#[test]
fn fixm10_swimming_quest_relocation_postgres() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 5, false)), false)?;
        scenario(Some((7, 4, false)), false)?;
        scenario(Some((6, 4, true)), false)
    })
}

#[test]
fn underwater_drowning_postgres_step_in_drowns_helmets_protect_step_out_removes() -> TestResult {
    on_thread(|| scenario(None, false))
}

#[test]
fn underwater_drowning_postgres_quest_relocation_enters_and_leaves_water() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 2, false)), false)?;
        scenario(Some((3, 4, false)), false)
    })
}

#[test]
fn underwater_drowning_postgres_refused_quest_step_restores_water_state() -> TestResult {
    on_thread(|| {
        scenario(Some((2, 1, true)), false)?;
        scenario(Some((1, 2, true)), false)
    })
}

#[test]
fn underwater_drowning_postgres_portal_landings_enter_and_leave_water() -> TestResult {
    on_thread(|| scenario(None, true))
}

#[test]
fn m10_move_yellow_pillow_postgres_real_dispatch() -> TestResult {
    underwater_drowning_fixtures::m10_move_yellow_pillow_postgres_real_dispatch_scenario()
}
