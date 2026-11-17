//! The survey consumes the existing counter-World owner, not a fabricated event flag.
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.deeplings_worldchange";
const TRACK: &str = "oteryn:quest-progress/chosen-source/deeplings_worldchange/s1";
const START: Position = Position {
    x: 33642,
    y: 31379,
    floor: -9,
};

#[test]
fn m01b_deeplings_native_survey_requires_current_counter_world_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-deeplings-survey", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/deeplings-worldchange/s1/entry",
                Position {
                    x: 33641,
                    y: 31379,
                    floor: -9,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_deeplings_survey duration={:?} success={}",
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
                "/../../../content/world/placements/region-z09-x131-y122.b3"
            )),
            region: (9, 131, 122),
            bounds: (33640, 31379, 33644, 31380),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (33641, 31379),
                    ids: vec![8287],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (START.x, START.y),
                    ids: vec![8276],
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
    let actor = admitted.runtime_actor.ok_or("Deeplings survey actor")?;
    owner.reset_epoch = 0;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "missing current World admission refuses survey credit"
    );
    for (epoch, day) in [(1_i64, 13_i32), (2, 14)] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,$4,12,0,0,'UTC'),'2031-06')")
            .bind(admitted.world_id.as_bytes().as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(day).execute(&h.pool).await?;
    }
    activate(h, owner, admitted.world_id, 1).await?;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "World admission cannot retro-credit an old visit"
    );
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    activate(h, owner, admitted.world_id, 2).await?;
    owner.reset_epoch = 1;
    koshei::step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "an old reset cannot borrow a current counter World"
    );
    owner.reset_epoch = 2;
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    let refused = koshei::step(owner, admitted, 3, StepDirection::West).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    koshei::step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 5, StepDirection::East).await?;
    koshei::step(owner, admitted, 6, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "revisits do not farm survey or complete the whole quest"
    );
    Ok(())
}

async fn activate(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    world: crate::foundation::WorldId,
    epoch: u64,
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(i64::try_from(epoch)?).execute(&h.pool).await?;
    let day = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("current counter World")?;
    assert!(
        oteryn_gameplay_world_change::owner::is_active(&day, "deeplings"),
        "real World owner admits the current Deeplings phase"
    );
    owner.reset_epoch = epoch;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/deeplings-worldchange/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
