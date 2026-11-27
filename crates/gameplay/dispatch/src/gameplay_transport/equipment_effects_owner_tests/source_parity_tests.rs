//! Production equipment facts from Crystal 201189f8, data/items/items.xml.
use super::*;
use oteryn_gameplay_combat_rules::{
    attack::formulas::DefenceSource,
    resolution::{SwingWeapon, swing_weapon},
};
use oteryn_gameplay_content::content::ReferenceCells;

#[test]
fn crystal_equipped_combat_stats_survive_production_loading() -> TestResult {
    let content = native(&input()?)?;
    let key = |id| format!("oteryn:item.tibia.i{id}");
    for (id, slot, armor) in [
        (3362, 3, 2),
        (3382, 3, 8),
        (3563, 2, 1),
        (6531, 1, 1),
        (8063, 2, 12),
        (23477, 4, 2),
        (34092, 3, 10),
        (34093, 3, 10),
        (34094, 2, 18),
        (34095, 2, 17),
        (34096, 2, 17),
        (34097, 4, 4),
        (34098, 4, 3),
        (39147, 2, 20),
        (39148, 1, 12),
        (39149, 1, 11),
        (39151, 1, 9),
        (39153, 1, 9),
        (43876, 3, 12),
        (43881, 3, 11),
        (43884, 4, 3),
        (43887, 4, 3),
        (50146, 3, 6),
        (50188, 1, 7),
        (50240, 4, 2),
    ] {
        assert_eq!(
            equipped_armor(&snapshot(&content, &key(id), slot), &content),
            Some(armor),
            "armor for {id}",
        );
    }
    for (id, defense) in [
        (3059, 23),
        (3409, 28),
        (3410, 23),
        (3411, 21),
        (3412, 19),
        (3413, 30),
        (3414, 49),
        (3415, 39),
        (3416, 41),
        (3418, 37),
        (3419, 42),
        (3420, 46),
        (3421, 33),
        (3424, 29),
        (3425, 34),
        (3426, 20),
        (3428, 42),
        (3429, 24),
        (3430, 25),
        (3431, 29),
        (3432, 36),
        (3433, 38),
        (3434, 45),
        (3435, 37),
        (3436, 43),
        (3439, 45),
        (3440, 33),
        (3441, 26),
        (3442, 47),
        (3443, 36),
        (6131, 34),
        (7460, 37),
        (8072, 29),
        (8073, 36),
        (8074, 26),
        (8075, 32),
        (13998, 33),
        (14000, 47),
        (14042, 43),
        (14088, 47),
        (21175, 41),
        (21400, 13),
        (22726, 49),
        (25699, 26),
        (39152, 36),
        (39154, 36),
        (45639, 31),
        (45640, 31),
        (45643, 47),
    ] {
        assert_eq!(
            equipped_defence(&snapshot(&content, &key(id), 6), &content),
            Some(Some(HandDefence {
                source: DefenceSource::Shield,
                value: defense,
                skill_index: Some(5),
            })),
            "shield defence for {id}",
        );
    }
    let staff = key(3289);
    assert_eq!(
        equipped_defence(&snapshot(&content, &staff, 5), &content),
        Some(Some(HandDefence {
            source: DefenceSource::Weapon,
            value: 14,
            skill_index: Some(0),
        })),
    );
    for (id, attack) in [
        (3289, 22),
        (10389, 27),
        (10391, 39),
        (14250, 42),
        (17828, 38),
        (50181, 21),
        (50182, 31),
        (50183, 36),
        (50239, 44),
    ] {
        let key = key(id);
        assert!(
            matches!(swing_weapon(&content, |slot| {
                (slot == 5).then_some((key.as_str(), "definition-r1"))
            }), Some(SwingWeapon::Melee { attack: actual, .. }) if actual == attack),
            "melee attack for {id}",
        );
    }
    for (id, range) in [(3072, 3), (3075, 3), (8092, 3), (22866, 6), (25700, 5)] {
        let policy = content.item_policy_for_source_id(id).ok_or("item policy")?;
        let ReferenceItemField::Known(weapon) = &policy.record().semantics.weapon else {
            return Err(format!("missing ranged weapon {id}").into());
        };
        assert_eq!(
            weapon.range,
            ReferenceItemField::Known(ReferenceCells(range)),
            "range for {id}"
        );
    }
    Ok(())
}

#[test]
fn crystal_forge_cohort_survives_production_loading() -> TestResult {
    let content = native(&input()?)?;
    for (classification, max_tier, groups) in [
        (
            1,
            1,
            &[
                3310..=3310,
                3345..=3345,
                7774..=7774,
                17813..=17813,
                25915..=25959,
                25961..=25969,
                25971..=25973,
                25983..=26000,
                26014..=26031,
                26044..=26061,
            ][..],
        ),
        (
            2,
            2,
            &[
                3208..=3208,
                3319..=3319,
                3323..=3323,
                7416..=7416,
                9373..=9373,
                9375..=9375,
                9385..=9385,
                9387..=9387,
                9396..=9396,
                11693..=11693,
                16161..=16161,
                16175..=16175,
                20067..=20067,
                20070..=20070,
                20073..=20073,
                20079..=20079,
                27449..=27453,
                31614..=31614,
            ][..],
        ),
        (
            3,
            3,
            &[
                7423..=7423,
                7453..=7453,
                8096..=8096,
                8100..=8100,
                11692..=11692,
                20068..=20068,
                20071..=20071,
                20074..=20074,
                20080..=20080,
                22762..=22762,
                28829..=28832,
                29419..=29419,
                50164..=50164,
            ][..],
        ),
        (
            4,
            10,
            &[
                3309..=3309,
                7455..=7455,
                8099..=8099,
                20066..=20066,
                20072..=20072,
                20078..=20078,
                28725..=28725,
                30395..=30396,
                30398..=30398,
                34155..=34155,
                34253..=34254,
                35514..=35514,
                36657..=36662,
                39155..=39157,
                49523..=49527,
                49530..=49530,
                49864..=49881,
                50158..=50158,
                50160..=50160,
                50165..=50165,
                50167..=50167,
                50169..=50170,
                50176..=50176,
                50250..=50253,
                50270..=50270,
                52333..=52334,
                52338..=52338,
            ][..],
        ),
    ] {
        for id in groups.iter().flat_map(|group| group.clone()) {
            let policy = content
                .item_policy_for_source_id(id)
                .ok_or_else(|| format!("missing Forge item {id}"))?;
            let record = policy.record();
            let profile = record
                .authoring
                .forge
                .ok_or_else(|| format!("missing Forge profile {id}"))?;
            assert_eq!(
                (profile.classification, profile.max_tier),
                (classification, max_tier),
                "Forge profile for {id}"
            );
            assert!(
                in_own_slot(&record.semantics, ReferenceEquipmentSlot::Weapon),
                "Forge weapon slot for {id}"
            );
            assert!(
                record.admission.materializable
                    && record.admission.legal_destinations.contains(
                        &crate::content::native_gameplay::NativeItemDestination::CharacterInventory
                    ),
                "Forge inventory admission for {id}"
            );
        }
    }
    Ok(())
}
