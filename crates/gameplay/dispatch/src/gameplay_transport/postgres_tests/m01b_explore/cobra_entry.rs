//! All twelve graves use native STEP; only the separate boss-owner conditions are arranged.
use super::koshei::{relocate, seed_level, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.grave_danger_quest";
const PREFIX: &str = "oteryn:quest-progress/chosen-source/grave_danger_quest/";
const START: Position = Position {
    x: 33301,
    y: 32649,
    floor: -7,
};

#[test]
fn m01b_grave_danger_cobra_native_visit_requires_twelve_graves_and_both_boss_stages() -> TestResult
{
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-cobra-visit", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/grave-danger/s4/entry",
            Position {
                y: START.y - 1,
                ..START
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_cobra_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let mut crops = vec![NativeCrop {
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x130-y127.b3"
        )),
        region: (7, 130, 127),
        bounds: (33300, 32647, 33302, 32650),
        start: (START.x, START.y),
        expected: vec![
            ExpectedTile {
                position: (33301, 32647),
                ids: vec![231, 20255],
                counted_entry: None,
            },
            ExpectedTile {
                position: (33301, 32648),
                ids: vec![231],
                counted_entry: None,
            },
            ExpectedTile {
                position: (START.x, START.y),
                ids: vec![231],
                counted_entry: None,
            },
        ],
    }];
    crops.extend(grave_danger::crops());
    native_scene(native, crops, &[])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Cobra survey actor")?;
    seed_level(h, 250).await?;
    let mut command = 1;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    // No administrative exploration progress: physically visit every retained grave.
    for (index, &(x, y, floor, direction)) in grave_danger::POINTS.iter().enumerate() {
        relocate(
            owner,
            actor,
            admitted.game_session_id,
            Position { x: x + 1, y, floor },
        )
        .await?;
        command += 1;
        step(owner, admitted, command, direction).await?;
        let visited: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(super::super::fixture::id(41).as_slice()).bind(format!("{PREFIX}s1")).fetch_one(&h.pool).await?;
        assert_eq!(visited, i64::try_from(index + 1)?);
        assert_eq!(counts(h).await?, (0, 0, 0));
    }
    for (lich, zelos) in [(0, 0), (4, 1), (5, 0)] {
        seed_boss_conditions(h, lich, zelos).await?;
        relocate(owner, actor, admitted.game_session_id, START).await?;
        command += 1;
        step(owner, admitted, command, StepDirection::North).await?;
        assert_eq!(
            counts(h).await?,
            (0, 0, 0),
            "all predecessor stages must match exactly"
        );
    }
    seed_boss_conditions(h, 5, 1).await?;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "new boss progress cannot retro-credit a refused visit"
    );
    seed_level(h, 249).await?;
    relocate(owner, actor, admitted.game_session_id, START).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "Cobra still requires durable level250"
    );
    seed_level(h, 250).await?;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "level cannot retro-credit a visit"
    );
    relocate(owner, actor, admitted.game_session_id, START).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    command += 1;
    let refused = step(owner, admitted, command, StepDirection::North).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(owner, actor, admitted.game_session_id, START).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    command += 1;
    step(owner, admitted, command, StepDirection::South).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "no farming or whole-quest completion"
    );
    Ok(())
}

// Fixture only for the separate encounter owners, after actual twelve-grave progress.
// Never advance the live character root or seed either exploration counter.
async fn seed_boss_conditions(h: &Harness, lich: i64, zelos: i64) -> TestResult {
    let before = counts(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (stage, value) in [("s2", lich), ("s3", zelos)] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(super::super::fixture::id(41).as_slice()).bind(format!("{PREFIX}{stage}")).bind(QUEST).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    assert_eq!(
        counts(h).await?,
        before,
        "boss arrangement cannot grant exploration"
    );
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/grave-danger/s4/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(format!("{PREFIX}s4")).bind(QUEST).fetch_one(&h.pool).await?)
}
