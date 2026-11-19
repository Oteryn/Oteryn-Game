//! Crystal 201189f8 data-global/npc/tonar.lua:66-81, data/items/items.xml:13001-13016.

use super::TestResult;
use super::fixture::{Harness, run};
use super::tonar_scarab_shell_exchange_fixtures::exercise;

#[test]
fn tonar_refuses_a_fresh_shell_and_exchanges_each_mature_shell() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "tonar_scarab_shell_exchange",
                    false,
                    false,
                )
                .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Tonar shell exchange case panicked")??;
    Ok(())
}
