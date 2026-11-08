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

#[test]
fn m01b_geomancer_real_native_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-geomancer-native-step", || {
        super::quest_world_callbacks::scenario(
            "m01b/geomancer/s1/entry",
            Position {
                x: 32418,
                y: 32017,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_geomancer_native_step duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_asura_native_entry_requires_current_premium_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-asura-premium-step", || {
        super::quest_world_callbacks::scenario(
            "m01b/asura-palace/s1/entry",
            Position {
                x: 32953,
                y: 32685,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_asura_premium duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_alawar_native_approach_credits_once_without_replacing_portal() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run("m01b-alawar-source-approach", || {
            super::quest_world_callbacks::scenario(
                "m01b/alawar-s-vault/s1/entry",
                Position {
                    x: 32187,
                    y: 31623,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_alawar_approach duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_elvenbane_native_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-elvenbane-entry", || {
        super::quest_world_callbacks::scenario(
            "m01b/elvenbane/s1/entry",
            Position {
                x: 32590,
                y: 31645,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_elvenbane_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_edron_goblin_native_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-edron-goblin-entry", || {
        super::quest_world_callbacks::scenario(
            "m01b/edron-goblin/s1/entry",
            Position {
                x: 33108,
                y: 31850,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_edron_goblin_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m01b_lions_rock_native_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m01b-lions-rock-entry", || {
        super::quest_world_callbacks::scenario(
            "m01b/lions-rock/s1/entry",
            Position {
                x: 33138,
                y: 32358,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_lions_rock_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    if name == "m01b/edron-goblin/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y124.b3"
                )),
                region: (7, 129, 124),
                bounds: (33108, 31849, 33110, 31851),
                start: (33109, 31850),
                expected: vec![ExpectedTile {
                    position: (33108, 31850),
                    ids: vec![355],
                    counted_entry: None,
                }],
            },
        );
    }
    if name == "m01b/lions-rock/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y126.b3"
                )),
                region: (7, 129, 126),
                bounds: (33138, 32357, 33140, 32359),
                start: (33139, 32358),
                expected: vec![ExpectedTile {
                    position: (33138, 32358),
                    ids: vec![4399],
                    counted_entry: None,
                }],
            },
        );
    }
    if name == "m01b/elvenbane/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x127-y123.b3"
                )),
                region: (7, 127, 123),
                bounds: (32589, 31644, 32593, 31647),
                start: (32591, 31645),
                expected: vec![ExpectedTile {
                    position: (32590, 31645),
                    ids: vec![436],
                    counted_entry: None,
                }],
            },
        );
    }
    if name == "m01b/alawar-s-vault/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x125-y123.b3"
                )),
                region: (8, 125, 123),
                bounds: (32186, 31623, 32190, 31625),
                start: (32188, 31623),
                expected: vec![ExpectedTile {
                    position: (32187, 31623),
                    ids: vec![409],
                    counted_entry: None,
                }],
            },
        );
    }
    if name == "m01b/asura-palace/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x128-y127.b3"
                )),
                region: (7, 128, 127),
                bounds: (32952, 32684, 32955, 32687),
                start: (32954, 32685),
                expected: vec![ExpectedTile {
                    position: (32953, 32685),
                    ids: vec![923],
                    counted_entry: None,
                }],
            },
        );
    }
    if name == "m01b/geomancer/s1/entry" {
        return native_crop(
            native,
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x126-y125.b3"
                )),
                region: (7, 126, 125),
                bounds: (32417, 32016, 32420, 32018),
                start: (32419, 32017),
                expected: vec![ExpectedTile {
                    position: (32418, 32017),
                    ids: vec![4529],
                    counted_entry: None,
                }],
            },
        );
    }
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
    if callback.admission.as_ref().is_some_and(|a| a.premium) {
        return exercise_premium(owner, admitted, h, callback).await;
    }
    let elvenbane = callback.key.ends_with("m01b/elvenbane/s1/entry");
    let alawar = callback.key.ends_with("m01b/alawar-s-vault/s1/entry");
    let geomancer = callback.key.ends_with("m01b/geomancer/s1/entry");
    assert!(elvenbane || alawar || geomancer || callback.key.ends_with(ENTRY));
    let counter = if elvenbane {
        "oteryn:quest-progress/authored/elvenbane_quest/s1"
    } else if alawar {
        "oteryn:quest-progress/chosen-source/alawar_s_vault_quest/s1"
    } else if geomancer {
        "oteryn:quest-progress/chosen-source/geomancer_quest/s1"
    } else {
        COUNTER
    };
    assert_eq!(
        owner
            .quest_catalogue
            .as_ref()
            .and_then(|c| c.track(counter))
            .ok_or("explore counter")?
            .initial,
        0
    );
    let counts = || async {
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
             (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$2), \
             (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$3 AND completed_character_revision IS NOT NULL)")
            .bind(&callback.key).bind(counter).bind(&callback.quest).fetch_one(&h.pool).await
    };
    assert_eq!(counts().await?, (0, 0, 0));
    // A nearby non-target route tile is not an objective.
    let longsword_moves = [
        StepDirection::South,
        StepDirection::North,
        StepDirection::West,
        StepDirection::East,
        StepDirection::West,
    ];
    let geomancer_moves = [
        StepDirection::West,
        StepDirection::East,
        StepDirection::West,
    ];
    let moves: &[StepDirection] = if elvenbane || alawar || geomancer {
        &geomancer_moves
    } else {
        &longsword_moves
    };
    let first_entry = if elvenbane || alawar || geomancer {
        0
    } else {
        2
    };
    for (index, &direction) in moves.iter().enumerate() {
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
            if index < first_entry {
                (0, 0, 0)
            } else {
                (1, 1, 0)
            }
        );
        if index == first_entry {
            // Replay while the command outcome is still retained. The following
            // fresh movements separately prove that a later visit cannot add credit.
            let replay = super::quest_world_callbacks::wire(
                owner,
                admitted,
                u64::try_from(index)? + 1,
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

async fn exercise_premium(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("current Premium fixture")?;
    let counter = callback
        .storage
        .iter()
        .find(|s| s.increment)
        .ok_or("stage counter")?
        .track
        .as_str();
    let counts = || async {
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT \
          (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
          (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$2)",
        )
        .bind(&callback.key)
        .bind(counter)
        .fetch_one(&h.pool)
        .await
    };
    for (index, direction) in [
        StepDirection::West,
        StepDirection::East,
        StepDirection::West,
        StepDirection::East,
        StepDirection::West,
    ]
    .into_iter()
    .enumerate()
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let command = u64::try_from(index)? + 1;
        let intent = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 1, intent.clone()).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            counts().await?,
            if index < 2 { (0, 0) } else { (1, 1) },
            "movement without Current Premium may not grant imported exploration"
        );
        if index == 0 {
            owner.premium_coordinator = Some(premium);
            let replay =
                super::quest_world_callbacks::wire(owner, admitted, command, 1, intent).await?;
            assert_eq!(replay, reply);
            assert_eq!(
                counts().await?,
                (0, 0),
                "gaining Premium cannot retroactively credit a replay"
            );
        } else if index == 2 {
            let replay =
                super::quest_world_callbacks::wire(owner, admitted, command, 1, intent).await?;
            assert_eq!(replay, reply);
            assert_eq!(counts().await?, (1, 1));
        }
    }
    Ok(())
}
