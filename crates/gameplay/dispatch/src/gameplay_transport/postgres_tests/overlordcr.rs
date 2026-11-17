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
