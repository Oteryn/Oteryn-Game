//! Crystal 201189f8 data-global/scripts/quests/dark_trails/creaturescripts_think_the_ravager_heal.lua:3-13.

use super::TestResult;
use super::creature_tile_heal_fixtures::{HealFixture, exercise};
use super::fixture::{Harness, run};

#[test]
fn creature_think_heals_only_a_wounded_creature_standing_on_the_bound_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h =
                    Harness::create_with_admission_seed(admin, "creature_tile_heal", false, false)
                        .await?;
                let result = Box::pin(exercise(&h, HealFixture::Tile)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "creature tile heal case panicked")??;
    Ok(())
}
