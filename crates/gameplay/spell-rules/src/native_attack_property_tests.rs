#![allow(clippy::expect_used)]
use crate::harmony::{HarmonyMultiplier, MAX_HARMONY, MULTIPLIER_DENOMINATOR};
use crate::native_actor_states::{
    ActorSkill, DamageElement, EquipmentFacts, HandItem, HandKind, equipment_attack,
};
use oteryn_test_support::property::for_each_case;
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn property_harmony_outputs_are_bounded_monotone_and_saturate() {
    for_each_case("spell harmony bounds", |rng, _| {
        let level = rng.u64() as u32;
        let charges = rng.below(u64::from(MAX_HARMONY) + 1) as u8;
        let virtue = rng.bool();
        let serene = rng.bool();
        let multiplier =
            HarmonyMultiplier::new(level, charges, virtue, serene).expect("bounded charges");
        for bound in [0, 1, rng.range_i64(0, i64::MAX), i64::MAX] {
            let expected = (i128::from(bound) * i128::from(multiplier.numerator())
                / i128::from(MULTIPLIER_DENOMINATOR))
            .min(i128::from(i64::MAX));
            assert_eq!(multiplier.apply(bound), expected as i64);
            assert!(multiplier.apply(bound) >= bound);
            let higher = HarmonyMultiplier::new(level.saturating_add(1), charges, virtue, serene)
                .expect("charges");
            assert!(higher.apply(bound) >= multiplier.apply(bound));
        }
        assert!(HarmonyMultiplier::new(level, MAX_HARMONY + 1, virtue, serene).is_none());
        Ok(())
    });
}

#[test]
fn property_equipment_spell_bounds_and_all_element_bonds_are_preserved() {
    let export: Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .expect("profiles");
    let profile = export["profiles"]
        .as_array()
        .expect("profiles array")
        .iter()
        .find(|p| {
            p["name"]
                .as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case("flurry of blows"))
                && p["carrier"] == "instant"
        })
        .expect("flurry");
    let parameters = &profile["execution"]["native_behavior"]["parameters"];
    for_each_case("equipment spell elements", |rng, _| {
        let facts = EquipmentFacts {
            level: 1 + rng.below(2000) as u32,
            left: Some(HandItem {
                kind: HandKind::Weapon(ActorSkill::Fist),
                attack: rng.below(101) as u32,
                defense: 0,
                bond: None,
            }),
            right: None,
            effective_skills: BTreeMap::from([(ActorSkill::Fist, rng.below(1001) as u32)]),
            flurry_enlarged: false,
            shield_slam_grade: 0,
            harmony: Some(HarmonyMultiplier::ONE),
        };
        for bond in [
            None,
            Some(DamageElement::Physical),
            Some(DamageElement::Energy),
            Some(DamageElement::Earth),
            Some(DamageElement::Fire),
            Some(DamageElement::Ice),
            Some(DamageElement::Holy),
            Some(DamageElement::Death),
        ] {
            let mut facts = facts.clone();
            facts.left.as_mut().expect("weapon").bond = bond;
            let plan = equipment_attack(parameters, &facts).expect("qualified facts");
            assert_eq!(plan.element, bond.unwrap_or(DamageElement::Physical));
            assert!(!plan.components.is_empty());
            for component in plan.components {
                assert!(component.bounds.minimum >= 0);
                assert!(component.bounds.minimum <= component.bounds.maximum);
                assert!(component.bounds.maximum <= i64::from(i32::MAX));
            }
        }
        Ok(())
    });
}
