//! Exact turnips custody; the guarded Ram interaction is a separate quest slice.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

#[test]
fn questpc5_targuna_exact_turnips_enter_native_backpack_without_quest_progress() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-targuna-material")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    super::questpc5_oramond_roots::exercise_item(
        owner,
        admitted,
        h,
        53073,
        1,
        "targuna_turnip_material_custody",
    )
    .await
}
