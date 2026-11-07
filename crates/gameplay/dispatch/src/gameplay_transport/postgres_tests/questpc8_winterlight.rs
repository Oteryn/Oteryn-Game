//! Finite selected Winterlight route over native island geometry and existing owners.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn questpc8_winterlight_native_disguise_four_fires_queen_and_base_outfit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-winterlight", || {
        super::quest_world_callbacks::scenario(
            "authored-winterlight",
            Position {
                x: 33745,
                y: 31061,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_winterlight_native_route duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    super::questpc8_native_slice::world(
        active.native_gameplay().ok_or("native")?,
        &[
            (33740, 31058, 33747, 31063, -8),
            (33752, 31065, 33764, 31078, -9),
        ],
        &[],
        TilePos {
            x: 33744,
            y: 31061,
            floor: -8,
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    _h: &Harness,
) -> TestResult {
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33744,
            y: 31061,
            floor: -8,
        },
    )
    .await?;
    let target = super::questpc8_authored_programmes::object_target(
        owner,
        TilePos {
            x: 33743,
            y: 31061,
            floor: -8,
        },
        1,
    )?;
    assert_eq!(
        super::questpc8_authored_programmes::use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed,
        "the native western bonfire must grant the selected three physical disguise components"
    );
    Ok(())
}
