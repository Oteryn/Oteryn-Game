//! Crystal 201189f8 data-global/scripts/spells/monster/the_ravager_heal.lua:6-16.
use super::TestResult;
use super::creature_tile_heal::{HealFixture, exercise};
use super::fixture::{Harness, run};

#[test]
fn scheduled_defence_heals_the_caster_only_when_the_bound_creature_is_nearby() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                for (tag, present) in [
                    ("creature_heal_without_jar", false),
                    ("creature_heal_with_jar", true),
                ] {
                    let h = Harness::create_with_admission_seed(admin.clone(), tag, false, false)
                        .await?;
                    let result = Box::pin(exercise(&h, HealFixture::Nearby { present })).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "creature proximity heal case panicked")??;
    Ok(())
}
