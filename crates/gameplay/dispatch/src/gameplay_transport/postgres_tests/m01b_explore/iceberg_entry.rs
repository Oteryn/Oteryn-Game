//! The fixed personal iceberg entry still requires Current Premium.
use super::*;

const QUEST: &str = "oteryn:quest.authored.chakoya_iceberg_mini_world_change";
const TRACK: &str = "oteryn:quest-progress/authored/chakoya_iceberg_mini_world_change/s1";

#[test]
fn m01b_chakoya_native_personal_iceberg_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-iceberg-entry", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/chakoya-iceberg-mini-world-change/s1/entry",
            Position {
                x: 32622,
                y: 32585,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_iceberg_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z07-x127-y127.b3"
            )),
            region: (7, 127, 127),
            bounds: (32619, 32582, 32625, 32588),
            start: (32622, 32586),
            expected: vec![
                ExpectedTile {
                    position: (32623, 32584),
                    ids: vec![799, 7152],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32622, 32585),
                    ids: vec![6585],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32622, 32586),
                    ids: vec![6582],
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
    callback: &Callback,
) -> TestResult {
    assert_eq!(later_stage_rows(h).await?, 0);
    assert_eq!(legacy_states(h).await?, 0);
    super::exercise_premium(owner, admitted, h, callback).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    assert_eq!(
        later_stage_rows(h).await?,
        0,
        "entry never credits kills, the chamber or completion"
    );
    assert_eq!(
        legacy_states(h).await?,
        0,
        "personal expedition never activates a global iceberg spawn"
    );
    Ok(())
}

async fn later_stage_rows(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key LIKE 'oteryn:quest-progress/authored/chakoya_iceberg_mini_world_change/%' AND track_key <> $1 AND track_key <> $2")
        .bind(TRACK).bind(format!("{TRACK}/visited/entry")).fetch_one(&h.pool).await?)
}

async fn legacy_states(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_world_change_states WHERE change_key='chakoya_iceberg'",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/chakoya-iceberg-mini-world-change/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
