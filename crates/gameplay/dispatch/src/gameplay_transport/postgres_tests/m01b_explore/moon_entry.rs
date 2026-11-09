//! A native island visit consumes the current fenced World decision, never a retained moon snapshot.
use super::koshei::{relocate, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const NAME: &str = "m01b/grimvale-mini-world-change/s1/entry";
const QUEST: &str = "oteryn:quest.grimvale_mini_world_change";
const TRACK: &str = "oteryn:quest-progress/chosen-source/grimvale_mini_world_change/s1";
const START: Position = Position {
    x: 33336,
    y: 31690,
    floor: -7,
};

#[test]
fn m01b_grimvale_native_visit_requires_current_moon_premium_and_reset() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-grimvale-entry", || {
        super::super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 33335,
                y: 31690,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_grimvale_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z07-x130-y123.b3"
            )),
            region: (7, 130, 123),
            bounds: (33335, 31690, 33337, 31691),
            start: (33336, 31690),
            expected: vec![
                ExpectedTile {
                    position: (33335, 31690),
                    ids: vec![4515],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33336, 31690),
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
    let actor = admitted.runtime_actor.ok_or("moon actor")?;
    // Same administrator arrangement as worldchg_grimvale; all decisions are then
    // created/read by the existing durable World owner and native commands stay fenced.
    for (epoch, month, day) in [(0_i64, 6_i32, 12_i32), (1, 6, 13), (2, 6, 14), (3, 7, 13)] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,$4,$5,12,0,0,'UTC'),$6)")
            .bind(admitted.world_id.as_bytes().as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(month).bind(day).bind(format!("2031-{month:02}")).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(admitted.world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    activate(h, owner, admitted.world_id, 0).await?;
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "inactive moon cannot credit an ordinary native STEP"
    );
    activate(h, owner, admitted.world_id, 1).await?;
    step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "activation cannot retro-credit an old command"
    );
    relocate(owner, actor, admitted.game_session_id, START).await?;
    activate(h, owner, admitted.world_id, 2).await?;
    owner.reset_epoch = 1;
    step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "a retained active moon cannot cross the current reset fence"
    );
    activate(h, owner, admitted.world_id, 3).await?;
    relocate(owner, actor, admitted.game_session_id, START).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    let refused = step(owner, admitted, 3, StepDirection::West).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    relocate(owner, actor, admitted.game_session_id, START).await?;
    step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "fresh current-moon entry credits exactly once"
    );
    step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    step(owner, admitted, 5, StepDirection::East).await?;
    step(owner, admitted, 6, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "revisiting never manufactures another visit or completion"
    );
    Ok(())
}

async fn activate(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    world: crate::foundation::WorldId,
    epoch: u64,
) -> TestResult {
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .bind(i64::try_from(epoch)?)
    .execute(&h.pool)
    .await?;
    let day = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("current World day")?;
    assert_eq!(day.grimvale_full_moon, epoch == 1 || epoch == 3);
    owner.reset_epoch = epoch;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/grimvale-mini-world-change/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
