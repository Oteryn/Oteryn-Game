//! Elemental overlord production lethal queue and durable quest receipt regression.
use super::TestResult;

#[test]
fn overlordcr_each_overlord_sets_donor_flag_and_replay_is_idempotent_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    for creature in ["earth_overlord", "energy_overlord", "fire_overlord", "ice_overlord"] {
        let key = format!("oteryn:creature.{creature}");
        let rule = oteryn_gameplay_quests::quest::source_deaths::rules()?
            .iter().find(|rule| rule.creature == key).ok_or("overlord donor rule missing")?;
        assert_eq!(rule.effects.len(), 1);
        assert_eq!(rule.effects[0].track, "oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage");
        assert_eq!(rule.effects[0].default, -1);
        assert_eq!(rule.effects[0].set, Some(1));
        assert!(rule.room.is_none());
        assert!(!rule.top_damage_only);
        super::srvenc_source_deaths::scenario("overlordcr", creature)?;
    }
    Ok(())
}

#[test]
fn overlordcr_non_overlord_does_not_set_flag_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_non_overlord", "goblin_leader")
}
