//! A personal cauldron survey never credits brewing, combat or a global event.
use super::koshei::step;
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.authored.bewitched";
const TRACK: &str = "oteryn:quest-progress/authored/bewitched/s1";

#[test]
fn m01b_bewitched_native_personal_cauldron_visit_does_not_credit_the_brew() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-cauldron-visit", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/bewitched/s1/entry",
            Position {
                x: 32733,
                y: 31935,
                floor: -12,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_cauldron_visit duration={:?} success={}",
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
                "/../../../content/world/placements/region-z12-x127-y124.b3"
            )),
            region: (12, 127, 124),
            bounds: (32729, 31932, 32735, 31937),
            start: (32734, 31935),
            expected: vec![
                ExpectedTile {
                    position: (32732, 31935),
                    ids: vec![7272, 12023],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32733, 31935),
                    ids: vec![7272],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32734, 31935),
                    ids: vec![7272],
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
    assert_eq!(later_stage_rows(h).await?, 0);
    let legacy = legacy_states(h).await?;
    assert_eq!(
        legacy, 0,
        "no global cauldron event state is arranged by this survey fixture"
    );
    let premium = owner.premium_coordinator.take();
    let visit = step(owner, admitted, 1, StepDirection::West).await;
    owner.premium_coordinator = premium;
    visit?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "the landmark survey needs no Premium/event activation"
    );
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 2, StepDirection::East).await?;
    step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "no farming or whole-quest completion"
    );
    assert_eq!(
        legacy_states(h).await?,
        legacy,
        "survey never activates global cauldron events/raids"
    );
    assert_eq!(
        later_stage_rows(h).await?,
        0,
        "survey cannot credit ingredients, brewing, combat or completion"
    );
    Ok(())
}

async fn later_stage_rows(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key LIKE 'oteryn:quest-progress/authored/bewitched/%' AND track_key <> $1 AND track_key <> $2")
        .bind(TRACK).bind(format!("{TRACK}/visited/entry")).fetch_one(&h.pool).await?)
}

async fn legacy_states(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_world_change_states WHERE change_key='bewitched'",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/bewitched/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
