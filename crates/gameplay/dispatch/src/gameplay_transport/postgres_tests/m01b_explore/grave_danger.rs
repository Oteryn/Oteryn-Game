//! Twelve distinct grave visits share one bounded counter and cannot substitute for one another.
use super::koshei::{relocate, seed_level, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.grave_danger_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/grave_danger_quest/s1";
pub(super) const POINTS: [(u16, u16, i8, StepDirection); 12] = [
    (32645, 32394, -8, StepDirection::West),
    (32542, 31845, -6, StepDirection::West),
    (33376, 32805, -8, StepDirection::West),
    (32960, 31534, -7, StepDirection::West),
    (32777, 31817, -8, StepDirection::West),
    (32012, 31557, -7, StepDirection::West),
    (33813, 31628, -9, StepDirection::West),
    (33456, 31493, -13, StepDirection::West),
    (33456, 31413, -13, StepDirection::West),
    (33517, 31444, -13, StepDirection::West),
    (33423, 31493, -13, StepDirection::West),
    (33425, 31413, -13, StepDirection::West),
];

#[test]
fn m01b_grave_danger_twelve_native_visits_require_level_premium_and_cannot_be_farmed() -> TestResult
{
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-grave-danger-visits", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/grave-danger/s1/dark-cathedral",
                Position {
                    x: 32645,
                    y: 32394,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_grave_danger_visits duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    super::super::l6_world_pickups::native_scene(native, crops(), &[])
}

pub(super) fn crops() -> Vec<super::super::l6_world_pickups::NativeCrop> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop};
    let bytes: [&'static [u8]; 12] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z08-x127-y126.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z06-x127-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z08-x130-y128.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x128-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z08-x128-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x125-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z09-x132-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y122.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y122.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y122.b3"
        )),
    ];
    let appearances = [
        410, 11417, 418, 7062, 4409, 436, 469, 429, 429, 429, 429, 429,
    ];
    let starts = [
        410, 11417, 418, 7065, 4399, 436, 469, 429, 429, 429, 429, 429,
    ];
    POINTS
        .iter()
        .enumerate()
        .map(|(i, &(x, y, floor, _))| NativeCrop {
            bytes: bytes[i],
            region: ((-floor) as u8, x / 256, y / 256),
            bounds: (x, y, x + 2, y + 1),
            start: (x + 1, y),
            expected: vec![
                ExpectedTile {
                    position: (x, y),
                    ids: vec![appearances[i]],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (x + 1, y),
                    ids: vec![starts[i]],
                    counted_entry: None,
                },
            ],
        })
        .collect()
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Grave Danger actor")?;
    // Arrange durable levels, not the cached client/runtime level: the World
    // predicate must read the actual PostgreSQL progression owner.
    seed_level(h, 249).await?;
    let mut command = 1;
    step(owner, admitted, command, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    seed_level(h, 250).await?;
    // Raising the level cannot retroactively turn the old refused visit into credit.
    step(owner, admitted, command, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32646,
            y: 32394,
            floor: -8,
        },
    )
    .await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    command += 1;
    let refused = step(owner, admitted, command, StepDirection::West).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "cached Premium cannot authorize the visit"
    );
    step(owner, admitted, command, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "restoring Premium cannot retro-credit an old command"
    );
    for (index, &(x, y, floor, direction)) in POINTS.iter().enumerate() {
        let start = Position {
            x: if direction == StepDirection::West {
                x + 1
            } else {
                x - 1
            },
            y,
            floor,
        };
        relocate(owner, actor, admitted.game_session_id, start).await?;
        let prior = i64::try_from(index)?;
        assert_eq!(
            counts(h).await?,
            (prior, prior, 0),
            "fixture positioning grants no exploration credit"
        );
        command += 1;
        step(owner, admitted, command, direction).await?;
        let expected = i64::try_from(index + 1)?;
        assert_eq!(counts(h).await?, (expected, expected, 0));
        step(owner, admitted, command, direction).await?;
        assert_eq!(
            counts(h).await?,
            (expected, expected, 0),
            "replay cannot farm a grave"
        );
        let reverse = if direction == StepDirection::West {
            StepDirection::East
        } else {
            StepDirection::West
        };
        command += 1;
        step(owner, admitted, command, reverse).await?;
        command += 1;
        step(owner, admitted, command, direction).await?;
        assert_eq!(
            counts(h).await?,
            (expected, expected, 0),
            "revisiting one grave cannot replace another"
        );
    }
    assert_eq!(
        counts(h).await?,
        (12, 12, 0),
        "exploration does not finish the whole quest"
    );
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/m01b/grave-danger/s1/%'), (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1), (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
