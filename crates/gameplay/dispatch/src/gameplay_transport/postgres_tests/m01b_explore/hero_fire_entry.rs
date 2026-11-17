//! First survey uses native STEP; only the separate evidence owner's condition is arranged.
use super::koshei::{relocate, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.hero_of_rathleton_quest";
const PREFIX: &str = "oteryn:quest-progress/chosen-source/hero_of_rathleton_quest/";
const START: Position = Position {
    x: 33554,
    y: 32049,
    floor: -15,
};

#[test]
fn m01b_hero_fire_native_visit_requires_first_survey_evidence_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-hero-fire-visit", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/hero-of-rathleton/s3/entry",
                Position {
                    x: START.x - 1,
                    ..START
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_hero_fire_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let crops = vec![
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z15-x131-y125.b3"
            )),
            region: (15, 131, 125),
            bounds: (33551, 32048, 33555, 32051),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (33552, 32049),
                    ids: vec![20538],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33553, 32049),
                    ids: vec![20537],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (START.x, START.y),
                    ids: vec![20533],
                    counted_entry: None,
                },
            ],
        },
        hero_entry::crop(),
    ];
    native_scene(native, crops, &[])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Hero fire survey actor")?;
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 33534,
            y: 31951,
            floor: -14,
        },
    )
    .await?;
    step(owner, admitted, 2, StepDirection::West).await?;
    let first:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(super::super::fixture::id(41).as_slice()).bind(format!("{PREFIX}s1")).fetch_one(&h.pool).await?;
    assert_eq!(first, 1, "first survey is a real native visit");
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(owner, actor, admitted.game_session_id, START).await?;
    step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "first survey cannot replace workshop evidence"
    );
    seed_evidence(h).await?;
    step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "new evidence cannot retro-credit a refused visit"
    );
    relocate(owner, actor, admitted.game_session_id, START).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    let refused = step(owner, admitted, 4, StepDirection::West).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(owner, actor, admitted.game_session_id, START).await?;
    step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 6, StepDirection::East).await?;
    step(owner, admitted, 7, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "revisits cannot farm or complete the whole quest"
    );
    Ok(())
}

// Arrange only the separate USE owner's evidence after the real first survey.
// Neither exploration progress nor the live root revision is seeded.
async fn seed_evidence(h: &Harness) -> TestResult {
    let before = counts(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
        .bind(super::super::fixture::id(41).as_slice()).bind(format!("{PREFIX}s2")).bind(QUEST).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(counts(h).await?, before);
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/hero-of-rathleton/s3/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(format!("{PREFIX}s3")).bind(QUEST).fetch_one(&h.pool).await?)
}
