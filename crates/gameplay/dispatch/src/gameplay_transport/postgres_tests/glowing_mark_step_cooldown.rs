//! Crystal 201189f8 data-global/scripts/quests/grimvale/movements_strangely_glowing_mark.lua:1-28.
use super::TestResult;
use super::fixture::{Harness, run};
use super::solo_quest_room_ejection::exercise_step_buff;
use oteryn_gameplay_quests::quest::world_callbacks::{StepBenefit, StepBuff};

#[test]
fn glowing_mark_applies_its_benefit_then_allows_steps_during_the_character_cooldown() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                for (name, buff) in [
                    (
                        "glowing_mark_heal",
                        StepBuff {
                            magic_effect: 15,
                            benefit: StepBenefit::Heal { amount: 200 },
                        },
                    ),
                    (
                        "glowing_mark_speed",
                        StepBuff {
                            magic_effect: 13,
                            benefit: StepBenefit::Speed {
                                duration_ms: 22_000,
                                slope_per_thousand: 700,
                                intercept: -56,
                            },
                        },
                    ),
                ] {
                    let h = Harness::create_with_admission_seed(admin.clone(), name, false, false)
                        .await?;
                    let result = Box::pin(exercise_step_buff(&h, buff)).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "glowing mark step case panicked")??;
    Ok(())
}
