//! The Mintwallin hall is visited from outside the retained locked cell.
use super::koshei::{relocate, seed_level, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
const NAME: &str = "m01b/mad-mage-room/s1/entry";
const QUEST: &str = "oteryn:quest.mad_mage_room_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/mad_mage_room_quest/s1";
#[test]
fn m01b_prison_native_hall_entry_requires_durable_level_and_replays_once() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-prison-hall", || {
        super::super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32396,
                y: 32138,
                floor: -13,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_prison_hall duration={:?} success={}",
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
                "/../../../content/world/placements/region-z13-x126-y125.b3"
            )),
            region: (13, 126, 125),
            bounds: (32396, 32138, 32398, 32139),
            start: (32397, 32138),
            expected: vec![
                ExpectedTile {
                    position: (32396, 32138),
                    ids: vec![408],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32397, 32138),
                    ids: vec![432],
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
    let actor = admitted.runtime_actor.ok_or("prison hall actor")?;
    seed_level(h, 39).await?;
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    seed_level(h, 40).await?;
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "level eligibility cannot retro-credit an old command"
    );
    relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32397,
            y: 32138,
            floor: -13,
        },
    )
    .await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "fixture relocation is not a visit"
    );
    step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 3, StepDirection::East).await?;
    step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "the hall visit neither farms progress nor completes the quest"
    );
    Ok(())
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/mad-mage-room/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
