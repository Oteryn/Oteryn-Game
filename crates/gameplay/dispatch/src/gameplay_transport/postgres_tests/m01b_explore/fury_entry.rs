//! The Fury survey consumes the selected city from the committed World owner.
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.fury_gates_mini_world_change";
const TRACK: &str = "oteryn:quest-progress/chosen-source/fury_gates_mini_world_change/s1";
const START: Position = Position {
    x: 32265,
    y: 32165,
    floor: -7,
};

#[test]
fn m01b_fury_native_survey_requires_selected_city_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-fury-survey", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/fury-gates-mini-world-change/s1/entry",
            Position {
                x: 32264,
                y: 32165,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_fury_survey duration={:?} success={}",
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
            bounds: (32263, 32164, 32266, 32166),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (32264, 32165),
                    ids: vec![7066],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (START.x, START.y),
                    ids: vec![7063],
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
    let actor = admitted.runtime_actor.ok_or("Fury survey actor")?;
    koshei::seed_level(h, 60).await?;
    owner.reset_epoch = 0;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "missing current World admission refuses survey credit"
    );
    let [first, second] = selected_city_epochs(h, admitted.world_id).await?;
    activate(h, owner, admitted.world_id, first).await?;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "World admission cannot retro-credit an old visit"
    );
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    activate(h, owner, admitted.world_id, second).await?;
    owner.reset_epoch = first;
    koshei::step(owner, admitted, 2, StepDirection::West).await?;
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
        oteryn_gameplay_world_change::owner::is_active(&day, "fury_gates/thais"),
        "real World owner admits the current Fury phase"
    );
    owner.reset_epoch = epoch;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/fury-gates-mini-world-change/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}

// Use the existing deterministic producer to select admitted days, then read them
// through the real durable World owner. No activation flag or probability is overridden.
async fn selected_city_epochs(
    h: &Harness,
    world: crate::foundation::WorldId,
) -> TestResult<[u64; 2]> {
    let base_ms: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM make_timestamptz(2031,6,1,12,0,0,'UTC'))*1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let mut selected = Vec::new();
    for epoch in 1..=4096_u64 {
        let day = crate::durability::world_event::changes::roll(
            world,
            epoch,
            base_ms + i64::try_from(epoch)? * 86_400_000,
        )
        .map_err(debug)?;
        if oteryn_gameplay_world_change::owner::is_active(&day, "fury_gates/thais") {
            selected.push(epoch);
            if selected.len() == 2 {
                break;
            }
        }
    }
    let epochs: [u64; 2] = selected
        .try_into()
        .map_err(|_| "deterministic producer has no bounded selected-city pair")?;
    for epoch in epochs {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::bigint,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,1,12,0,0,'UTC')+($2::bigint)*INTERVAL '1 day',to_char(make_timestamptz(2031,6,1,12,0,0,'UTC')+($2::bigint)*INTERVAL '1 day','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind(i64::try_from(epoch)?).bind([1_u8;32].as_slice()).execute(&h.pool).await?;
    }
    Ok(epochs)
}
