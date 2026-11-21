//! M01b: imported exploration uses the native STEP owner, never a client-supplied counter.
use super::m01b_explore_fixtures::{COUNTER, ENTRY};

use super::fixture::debug;
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[path = "m01b_explore/area_entries.rs"]
mod area_entries;

#[path = "m01b_explore/cauldron_entry.rs"]
mod cauldron_entry;

#[path = "m01b_explore/claim_entries.rs"]
mod claim_entries;

#[path = "m01b_explore/camp_temple_entries.rs"]
mod camp_temple_entries;

#[path = "m01b_explore/key_entries.rs"]
mod key_entries;

#[path = "m01b_explore/minotaur_entry.rs"]
mod minotaur_entry;

#[path = "m01b_explore/fire_axe.rs"]
mod fire_axe;

#[path = "m01b_explore/koshei.rs"]
mod koshei;

#[path = "m01b_explore/spawn_entries.rs"]
mod spawn_entries;

#[path = "m01b_explore/treasure_entries.rs"]
mod treasure_entries;

#[path = "m01b_explore/cave_entry.rs"]
mod cave_entry;

#[path = "m01b_explore/iceberg_chamber.rs"]
mod iceberg_chamber;

#[path = "m01b_explore/iceberg_entry.rs"]
mod iceberg_entry;

#[path = "m01b_explore/infernal_entry.rs"]
mod infernal_entry;

#[path = "m01b_explore/breach_entry.rs"]
mod breach_entry;

#[path = "m01b_explore/followup_entries.rs"]
mod followup_entries;

#[path = "m01b_explore/grave_danger.rs"]
mod grave_danger;

#[path = "m01b_explore/cookie_entry.rs"]
mod cookie_entry;

#[path = "m01b_explore/moon_entry.rs"]
mod moon_entry;

#[path = "m01b_explore/crypt_entries.rs"]
mod crypt_entries;

#[path = "m01b_explore/prison_entry.rs"]
mod prison_entry;

#[path = "m01b_explore/bear_entry.rs"]
mod bear_entry;

#[path = "m01b_explore/ghoul_entry.rs"]
mod ghoul_entry;

#[path = "m01b_explore/hero_entry.rs"]
mod hero_entry;

#[path = "m01b_explore/elvenbane_dragon.rs"]
mod elvenbane_dragon;

#[path = "m01b_explore/lion_trophy.rs"]
mod lion_trophy;

#[path = "m01b_explore/deeplings_entry.rs"]
mod deeplings_entry;

#[path = "m01b_explore/fire_earth_entry.rs"]
mod fire_earth_entry;

#[path = "m01b_explore/fury_entry.rs"]
mod fury_entry;

#[path = "m01b_explore/cobra_entry.rs"]
mod cobra_entry;

#[path = "m01b_explore/hero_fire_entry.rs"]
mod hero_fire_entry;

#[path = "m01b_explore/station_entry.rs"]
mod station_entry;

#[test]
fn m01b_explore_native_catalogue_admits_zero_initial_counters_on_postgres() -> TestResult {
    use crate::bestiary_postgres_harness as pg;
    let Some(admin) = pg::configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-explore-catalogue", move || {
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-explore-native-step", || {
        super::quest_world_callbacks_fixtures::scenario(
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-geomancer-native-step", || {
        super::quest_world_callbacks_fixtures::scenario(
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-asura-premium-step", || {
        super::quest_world_callbacks_fixtures::scenario(
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
        super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-alawar-source-approach", || {
            super::quest_world_callbacks_fixtures::scenario(
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-elvenbane-entry", || {
        super::quest_world_callbacks_fixtures::scenario(
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-edron-goblin-entry", || {
        super::quest_world_callbacks_fixtures::scenario(
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-lions-rock-entry", || {
        super::quest_world_callbacks_fixtures::scenario(
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
