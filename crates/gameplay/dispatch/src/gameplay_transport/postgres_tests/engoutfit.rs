//! Shared outfit owner proofs through the production NPC dispatcher and PostgreSQL.
use super::TestResult;

#[test]
fn engoutfit_battle_mage_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l2-battle-mage-story")
}

#[test]
fn engoutfit_beggar_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-beggar-female-story")
}

#[test]
fn engoutfit_assassin_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l7-assassin-story")
}

#[test]
fn engoutfit_deepling_atomic_addon() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-deepling")
}

#[test]
fn engoutfit_source_outfits_are_durable_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("engoutfit-source", || {
        super::quest_world_callbacks_fixtures::scenario(
            "engoutfit-source",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-source duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn engoutfit_source_outfits_rollback_when_insert_is_suppressed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("engoutfit-source-fault", || {
        super::quest_world_callbacks_fixtures::scenario(
            "engoutfit-source-fault",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-source-fault duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn engoutfit_rock_steady_collection_commits_exact_handins_and_weapon() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("engoutfit-rock-steady", || {
        super::quest_world_callbacks_fixtures::scenario(
            "engoutfit-rock-steady",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-rock-steady duration={:?}",
        started.elapsed()
    );
    Ok(())
}
