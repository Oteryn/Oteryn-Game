//! M01b: imported exploration uses the native STEP owner, never a client-supplied counter.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

const ENTRY: &str = "m01b/longsword/s1/entry";
const COUNTER: &str = "oteryn:quest-progress/chosen-source/longsword_quest/s1";

#[test]
fn m01b_explore_native_catalogue_admits_zero_initial_counters_on_postgres() -> TestResult {
    use crate::bestiary_postgres_harness as pg;
    let Some(admin) = pg::configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run("m01b-explore-catalogue", move || {
            pg::runtime()?.block_on(async {
                let h = pg::Harness::create(admin, "m01b_explore", true).await?;
                let result: TestResult = async {
                    let version: String =
                        sqlx::query_scalar("SELECT current_setting('server_version')")
                            .fetch_one(&h.pool)
                            .await?;
                    eprintln!("POSTGRES_DATABASE_READY m01b_explore_catalogue version={version}");
                    let catalogue =
                        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state(
                            "content-1",
                        )
                        .map_err(debug)?;
                    let track = catalogue
                        .catalogue()
                        .track(COUNTER)
                        .ok_or("bound explore counter")?;
                    assert_eq!((track.initial, track.min, track.max), (0, 0, 1));
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let before = h
                        .root
                        .read_character_quest_state(&authority, pg::fence(1)?.character_id)
                        .await
                        .map_err(debug)?;
                    assert!(!before.tracks().contains_key(COUNTER));
                    let receipts: i64 =
                        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                            .fetch_one(&h.pool)
                            .await?;
                    assert_eq!(receipts, 0, "admission itself must not credit exploration");
                    Ok(())
                }
                .await;
                h.cleanup().await?;
                result
            })
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_explore_catalogue duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_explore_real_native_steps_credit_exact_anchor_once_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-explore-native-step", || {
        super::quest_world_callbacks::scenario(
            ENTRY,
            Position {
                x: 32663,
                y: 31970,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_explore_native_step duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x127-y124.b3"
            )),
            region: (7, 127, 124),
            bounds: (32662, 31969, 32666, 31972),
            start: (32664, 31970),
            expected: vec![
                ExpectedTile {
                    position: (32663, 31970),
                    ids: vec![4525],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32664, 31971),
                    ids: vec![4524],
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
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    assert!(callback.key.ends_with(ENTRY));
    assert_eq!(
        owner
            .quest_catalogue
            .as_ref()
            .and_then(|c| c.track(COUNTER))
            .ok_or("explore counter")?
            .initial,
        0
    );
    let counts = || async {
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
             (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$2), \
             (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$3 AND completed_character_revision IS NOT NULL)")
            .bind(&callback.key).bind(COUNTER).bind(&callback.quest).fetch_one(&h.pool).await
    };
    assert_eq!(counts().await?, (0, 0, 0));
    // A nearby non-target route tile is not an objective.
    let moves = [
        StepDirection::South,
        StepDirection::North,
        StepDirection::West,
        StepDirection::East,
        StepDirection::West,
    ];
    for (index, direction) in moves.into_iter().enumerate() {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            u64::try_from(index)? + 1,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            counts().await?,
            if index < 2 { (0, 0, 0) } else { (1, 1, 0) }
        );
        if index == 2 {
            // Replay while the command outcome is still retained. The following
            // fresh movements separately prove that a later visit cannot add credit.
            let replay = super::quest_world_callbacks::wire(
                owner,
                admitted,
                3,
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::West),
            )
            .await?;
            assert_eq!(replay, reply);
            assert_eq!(counts().await?, (1, 1, 0));
        }
    }
    assert_eq!(
        counts().await?,
        (1, 1, 0),
        "replay and revisits must not farm credit"
    );
    Ok(())
}
