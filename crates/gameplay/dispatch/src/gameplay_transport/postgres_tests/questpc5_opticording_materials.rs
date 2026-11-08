//! Exact Opticording material custody, independently of its unfinished puzzle route.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

#[test]
fn questpc5_opticording_initial_sphere_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-opticording-material-initial")
}
#[test]
fn questpc5_opticording_broken_sphere_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-opticording-material-broken")
}
#[test]
fn questpc5_opticording_complete_sphere_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-opticording-material-complete")
}

#[test]
fn questpc5_opticording_blue_spectacles_enter_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-opticording-material-spectacles")
}
#[test]
fn questpc5_opticording_supreme_cube_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-opticording-material-cube")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let item = match name {
        "qengine-rewards-opticording-material-initial" => 19063,
        "qengine-rewards-opticording-material-broken" => 23573,
        "qengine-rewards-opticording-material-complete" => 23572,
        "qengine-rewards-opticording-material-spectacles" => 27522,
        "qengine-rewards-opticording-material-cube" => 33313,
        _ => return Err("unknown Opticording material".into()),
    };
    super::questpc5_oramond_roots::exercise_item(
        owner,
        admitted,
        h,
        item,
        1,
        "opticording_material_custody",
    )
    .await
}
