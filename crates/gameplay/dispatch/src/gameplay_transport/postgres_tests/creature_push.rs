//! PUSH-ROUTE-1: command 98 through the real serve router, composed owner and PostgreSQL session.
use super::TestResult;
use super::creature_push_fixtures::scenario;

#[test]
fn creature_push_real_dispatch_moves_and_refuses_on_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            // One qualification spawn per runtime: the unpushable profile gets its own run.
            scenario("creature_push_np", true, None)?;
            scenario("creature_push", false, None)
        })?
        .join()
        .map_err(|_| "creature push e2e panicked")?
        .map_err(Into::into)
}

// Crystal 201189f8 data/scripts/movements/trap.lua:12-44,59-69: forced movement
// has the same creature StepIn damage/transform and StepOut reopening as walking.
#[test]
fn reaudit_relocation_tiles_monster_push_trap_entry_exit_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("srv_b3_relocation_tiles", false, Some(3944)))?
        .join()
        .map_err(|_| "relocation tiles e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn reaudit_relocation_tiles_monster_push_wolf_capture_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("srv_b3_relocation_wolf", false, Some(12368)))?
        .join()
        .map_err(|_| "relocation wolf e2e panicked")?
        .map_err(Into::into)
}
