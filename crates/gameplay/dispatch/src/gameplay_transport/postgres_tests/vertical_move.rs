//! WAVE-A/M11: the parameter dispatcher consumes qualified vertical-movement plans.
//! Pinned source tiles and booted bundle tiles exercise the real parameter dispatcher.
use super::vertical_move_fixtures::{scenario};

use super::fixture::run;
use super::TestResult;

#[test]
fn vertical_move_postgres_parameter_dispatch_levitate_and_magic_rope() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            run(async |admin| {
                for case in 0..5 {
                    scenario(admin.clone(), case, false).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "vertical movement thread panicked")?
        .map_err(Into::into)
}
