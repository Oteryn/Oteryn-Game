//! PAR-W11: activated Soul War and Rotten Blood boss profiles against Crystal `201189f8`
//! `data-global/monster/quests/{soul_war,rotten_blood_quest}` (the shipped `data-global`
//! pack): (hp, xp, speed, armor, defense).
use super::TestResult;
use super::fixture::{debug, id};

type Stats = (u64, u64, u32, u32, u32);

#[test]
fn w11_crystal_soul_war_rotten_blood_boss_parity() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    let rows: [(&str, Stats, Stats); 17] = [
        ("bakragore", (660_000, 15_000_000, 100, 305, 305), (660_000, 15_000_000, 250, 135, 135)),
        ("chagorz", (350_000, 3_250_000, 100, 305, 305), (600_000, 3_250_000, 250, 105, 105)),
        ("ichgahal", (350_000, 3_250_000, 100, 305, 305), (600_000, 3_250_000, 250, 105, 105)),
        ("murcion", (350_000, 3_250_000, 100, 305, 305), (600_000, 3_250_000, 250, 105, 105)),
        ("vemiath", (350_000, 3_250_000, 100, 305, 305), (600_000, 3_250_000, 250, 105, 105)),
        ("echo_of_chagorz", (350_000, 0, 100, 105, 105), (90_000, 0, 100, 0, 65)),
        ("echo_of_ichgahal", (350_000, 0, 100, 105, 105), (90_000, 0, 100, 0, 65)),
        ("echo_of_murcion", (350_000, 0, 100, 105, 105), (90_000, 0, 100, 0, 65)),
        ("echo_of_vemiath", (350_000, 0, 100, 105, 105), (90_000, 0, 100, 0, 65)),
        ("goshnar_s_cruelty", (300_000, 75_000, 200, 160, 160), (550_000, 75_000, 200, 160, 160)),
        ("goshnar_s_greed", (300_000, 150_000, 200, 160, 160), (550_000, 150_000, 200, 160, 160)),
        ("goshnar_s_hatred", (300_000, 75_000, 150, 160, 160), (550_000, 75_000, 150, 160, 160)),
        ("goshnar_s_malice", (300_000, 75_000, 150, 160, 160), (550_000, 75_000, 150, 160, 160)),
        ("goshnar_s_spite", (300_000, 75_000, 150, 160, 160), (550_000, 75_000, 150, 160, 160)),
        ("goshnar_s_megalomania_blue", (620_000, 3_000_000, 0, 55, 55), (620_000, 3_000_000, 0, 55, 55)),
        ("goshnar_s_megalomania_green", (620_000, 3_000_000, 250, 55, 55), (620_000, 3_000_000, 250, 55, 55)),
        ("goshnar_s_megalomania_purple", (620_000, 0, 250, 25, 25), (620_000, 0, 250, 25, 25)),
    ];
    let mut matches = 0;
    for (name, crystal, oteryn) in rows {
        let key = format!("oteryn:creature.{name}");
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == key)
            .ok_or("boss profile")?;
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        else {
            return Err("boss has no Creature profile".into());
        };
        let activated = (
            profile.health.ok_or("health")?,
            profile.experience.ok_or("experience")?,
            profile.speed.ok_or("speed")?,
            profile.armor.ok_or("armor")?,
            profile.details.as_ref().ok_or("details")?.defense,
        );
        assert_eq!(activated, oteryn, "{key}");
        matches += usize::from(activated == crystal);
    }
    // Only the three Megalomania phases match Crystal on every field.
    assert_eq!(matches, 3);
    Ok(())
}
