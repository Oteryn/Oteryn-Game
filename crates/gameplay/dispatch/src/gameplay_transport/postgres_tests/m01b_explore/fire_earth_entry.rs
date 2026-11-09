//! The volcano survey consumes actual admitted eruption days from the World owner.
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.authored.fire_from_the_earth_mini_world_change";
const TRACK: &str = "oteryn:quest-progress/authored/fire_from_the_earth_mini_world_change/s1";
const START: Position = Position {
    x: 32072,
    y: 32605,
    floor: -2,
};

#[test]
fn m01b_fire_earth_native_survey_requires_current_eruption_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-fire-earth-survey", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/fire-from-the-earth-mini-world-change/s1/entry",
                Position {
                    x: 32072,
                    y: 32604,
                    floor: -2,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_fire_earth_survey duration={:?} success={}",
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
                "/../../../content/world/placements/region-z02-x125-y127.b3"
            )),
            region: (2, 125, 127),
            bounds: (32071, 32603, 32074, 32606),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (32072, 32604),
                    ids: vec![4401],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (START.x, START.y),
                    ids: vec![4396],
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
    let actor = admitted.runtime_actor.ok_or("Fire Earth survey actor")?;
    owner.reset_epoch = 0;
    koshei::step(owner, admitted, 1, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "missing current World admission refuses survey credit"
    );
    let [first, second] = eruption_epochs(h, admitted.world_id).await?;
    activate(h, owner, admitted.world_id, first).await?;
    koshei::step(owner, admitted, 1, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "World admission cannot retro-credit an old visit"
    );
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    activate(h, owner, admitted.world_id, second).await?;
    owner.reset_epoch = first;
    koshei::step(owner, admitted, 2, StepDirection::North).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "an old reset cannot borrow a current counter World"
    );
    owner.reset_epoch = second;
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium provider")?;
    let refused = koshei::step(owner, admitted, 3, StepDirection::North).await;
    owner.premium_coordinator = Some(premium);
    refused?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    koshei::step(owner, admitted, 3, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 4, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 4, StepDirection::North).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 5, StepDirection::South).await?;
    koshei::step(owner, admitted, 6, StepDirection::North).await?;
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
        oteryn_gameplay_world_change::owner::is_active(&day, "fire_from_the_earth"),
        "real World owner admits the current Fire Earth phase"
    );
    owner.reset_epoch = epoch;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/fire-from-the-earth-mini-world-change/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}

// Use the existing deterministic producer to select admitted days, then read them
// through the real durable World owner. No activation flag or probability is overridden.
async fn eruption_epochs(h: &Harness, world: crate::foundation::WorldId) -> TestResult<[u64; 2]> {
    let base_ms: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM make_timestamptz(2031,6,1,12,0,0,'UTC'))*1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let mut selected = Vec::new();
    for epoch in 1..=512_u64 {
        let day = crate::durability::world_event::changes::roll(
            world,
            epoch,
            base_ms + i64::try_from(epoch)? * 86_400_000,
        )
        .map_err(debug)?;
        if oteryn_gameplay_world_change::owner::is_active(&day, "fire_from_the_earth") {
            selected.push(epoch);
            if selected.len() == 2 {
                break;
            }
        }
    }
    let epochs: [u64; 2] = selected
        .try_into()
        .map_err(|_| "deterministic producer has no bounded eruption pair")?;
    for epoch in epochs {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::bigint,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,1,12,0,0,'UTC')+($2::bigint)*INTERVAL '1 day',to_char(make_timestamptz(2031,6,1,12,0,0,'UTC')+($2::bigint)*INTERVAL '1 day','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind(i64::try_from(epoch)?).bind([1_u8;32].as_slice()).execute(&h.pool).await?;
    }
    Ok(epochs)
}
