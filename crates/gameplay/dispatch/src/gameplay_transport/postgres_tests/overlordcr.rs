//! Elemental overlord production lethal queue and durable quest receipt regression.
use super::TestResult;

#[test]
fn overlordcr_all_four_overlords_credit_two_damagers_exclude_observer_and_replay_postgres()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    for creature in [
        "earth_overlord",
        "energy_overlord",
        "fire_overlord",
        "ice_overlord",
    ] {
        let key = format!("oteryn:creature.{creature}");
        let rule = oteryn_gameplay_quests::quest::source_deaths::rules()?
            .iter()
            .find(|rule| rule.creature == key)
            .ok_or("overlord donor rule missing")?;
        assert_eq!(rule.effects.len(), 1);
        assert_eq!(
            rule.effects[0].track,
            "oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage"
        );
        assert_eq!(rule.effects[0].default, -1);
        assert_eq!(rule.effects[0].set, Some(1));
        assert!(rule.room.is_none());
        assert!(!rule.top_damage_only);
        super::srvenc_source_deaths::scenario("overlordcr", creature)?;
    }
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn overlordcr_non_overlord_does_not_set_flag_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    super::srvenc_source_deaths::scenario("overlordcr_non_overlord", "goblin_leader")?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn overlordcr_non_overlord_retains_anti_leech_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::boss_rewards::run_case(super::boss_rewards::Case::NonOverlordHealing)
}

#[test]
fn overlordcr_atomic_all_recipients_and_death_rollback_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_atomic", "earth_overlord")
}

#[test]
fn overlordcr_seventeenth_distinct_damager_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_seventeen", "earth_overlord")
}

#[test]
fn overlordcr_missing_quest_copy_retains_damager_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_missing_cache", "earth_overlord")
}

#[test]
fn overlordcr_without_reward_row_credits_damage_participants_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_rowless", "earth_overlord")
}

#[test]
fn overlordcr_atomic_chests_before_xp_fault_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::boss_rewards::run_case(super::boss_rewards::Case::AtomicDeathFault)
}

#[test]
fn overlordcr_crash_after_commit_replay_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_crash_replay", "earth_overlord")
}

#[test]
fn overlordcr_compatible_existing_pin_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_old_pin", "earth_overlord")
}

#[test]
fn overlordcr_proficiency_fallback_storage_without_xp_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_fallback", "earth_overlord")
}

#[test]
fn overlordcr_ordinary_boss_damager_chest_excludes_healer_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::boss_rewards::run_case(super::boss_rewards::Case::OrdinaryDamagerAndHealer)
}

#[test]
fn overlordcr_ended_secondary_keeps_principal_xp_chest_and_storage_postgres() -> TestResult {
    super::srvenc_source_deaths::scenario("overlordcr_ended_secondary_xp", "earth_overlord")
}

#[test]
fn overlordcr_ended_secondary_no_xp_completes_without_requeue_postgres() -> TestResult {
    super::srvenc_source_deaths::scenario("overlordcr_ended_secondary_no_xp", "earth_overlord")
}
