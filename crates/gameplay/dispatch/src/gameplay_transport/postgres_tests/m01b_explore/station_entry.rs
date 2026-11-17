//! A permanent pasture visit never activates the held horse escape/taming cycle.
use super::koshei::step;
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.horse_station_world_change";
const TRACK: &str = "oteryn:quest-progress/chosen-source/horse_station_world_change/s1";

#[test]
fn m01b_horse_station_native_landmark_visit_never_activates_escape_state() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-station-visit", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/horse-station-world-change/s1/entry",
            Position {
                x: 32442,
                y: 32229,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_station_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x126-y125.b3"
            )),
            region: (7, 126, 125),
            bounds: (32441, 32228, 32443, 32232),
            start: (32442, 32228),
            expected: vec![
                ExpectedTile {
                    position: (32442, 32229),
                    ids: vec![4521],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32442, 32228),
                    ids: vec![4515],
                    counted_entry: None,
                },
            ],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let legacy = legacy_states(h).await?;
    assert_eq!(
        legacy, 0,
        "no horse escape state is arranged by this survey fixture"
    );
    let premium = owner.premium_coordinator.take();
    let visit = step(owner, admitted, 1, StepDirection::South).await;
    owner.premium_coordinator = premium;
    visit?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "the landmark survey needs no Premium/event activation"
    );
    step(owner, admitted, 1, StepDirection::South).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 2, StepDirection::North).await?;
    step(owner, admitted, 3, StepDirection::South).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "no farming or whole-quest completion"
    );
    assert_eq!(
        legacy_states(h).await?,
        legacy,
        "survey never activates horse escapes/raids"
    );
    Ok(())
}

async fn legacy_states(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_world_change_states WHERE change_key='horse_station'",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/horse-station-world-change/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
