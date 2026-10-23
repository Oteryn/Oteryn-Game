#![allow(clippy::expect_used)]
use crate::cast::{CastRejection, CasterPremium, CasterState, Cooldowns, mana_cost};
use crate::harmony::HarmonyMultiplier;
use oteryn_gameplay_content::content::spell_data::{
    CooldownGroup, ManaCost, SpellDefinition, Vocation, authoring::spell_from_bundle,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use oteryn_test_support::property::for_each_case;
use std::collections::BTreeSet;

fn spell() -> SpellDefinition {
    let spell = serde_json::from_str(include_str!("../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-light_healing/spell.json")).expect("spell");
    let dependencies = serde_json::from_str(include_str!("../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-light_healing/dependencies.json")).expect("dependencies");
    spell_from_bundle(&spell, &dependencies).expect("admitted spell")
}

fn caster(max_mana: u32) -> CasterState {
    CasterState {
        vocation: Vocation::Druid,
        level: 100,
        magic_level: 10,
        premium: CasterPremium::Current,
        mana: max_mana,
        max_mana,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 10,
        attack_value: 7,
        attack_factor: 1.0,
        shielding_skill: 10,
        melee_weapon: false,
        shield_defense: None,
        harmony_multiplier: HarmonyMultiplier::ONE,
    }
}

#[test]
fn property_mana_cost_uses_wide_arithmetic_and_saturates_instead_of_wrapping() {
    let mut spell = spell();
    for_each_case("spell mana costs", |rng, _| {
        let max_mana = rng.u64() as u32;
        let caster = caster(max_mana);
        for cost in [0, 1, 100, 101, rng.u64() as u32, u32::MAX] {
            spell.mana = ManaCost::Fixed(cost);
            assert_eq!(mana_cost(&spell, &caster), cost);
            spell.mana = ManaCost::PercentOfMaximum(cost);
            let expected =
                (u128::from(max_mana) * u128::from(cost) / 100).min(u128::from(u32::MAX));
            assert_eq!(mana_cost(&spell, &caster), expected as u32);
            let higher = CasterState {
                max_mana: max_mana.saturating_add(1),
                ..caster.clone()
            };
            assert!(mana_cost(&spell, &higher) >= mana_cost(&spell, &caster));
        }
        Ok(())
    });
}

#[test]
fn property_cooldown_groups_share_deadlines_and_overflow_is_atomic() {
    let mut spell = spell();
    for_each_case("spell cooldown groups", |rng, _| {
        let now = *rng.pick(&[0, 1, u64::MAX - 1, u64::MAX]);
        let duration = rng.u64();
        spell.cooldown_micros = rng.below(1000);
        spell.groups = vec![
            CooldownGroup {
                key: spell.key.clone(),
                cooldown_micros: duration,
            },
            CooldownGroup {
                key: "secondary".into(),
                cooldown_micros: rng.u64(),
            },
        ];
        let mut cooldowns = Cooldowns::default();
        cooldowns
            .spells
            .insert("unrelated".into(), SemanticTimeMicros::from_micros(42));
        cooldowns
            .groups
            .insert("unrelated".into(), SemanticTimeMicros::from_micros(43));
        let before = cooldowns.clone();
        let fits = now.checked_add(spell.cooldown_micros).is_some()
            && spell
                .groups
                .iter()
                .all(|group| now.checked_add(group.cooldown_micros).is_some());
        let result = cooldowns.rearm(&spell, SemanticTimeMicros::from_micros(now));
        if fits {
            assert_eq!(result, Ok(()));
            assert_eq!(
                cooldowns.spell_ready_at(&spell.key).map(|at| at.get()),
                Some(now + spell.cooldown_micros)
            );
            for group in &spell.groups {
                assert_eq!(
                    cooldowns.group_ready_at(&group.key).map(|at| at.get()),
                    Some(now + group.cooldown_micros)
                );
            }
            assert_eq!(
                cooldowns.spell_ready_at("unrelated"),
                before.spell_ready_at("unrelated")
            );
            assert_eq!(
                cooldowns.group_ready_at("unrelated"),
                before.group_ready_at("unrelated")
            );
            let deadlines = cooldowns.canonical_deadlines();
            assert_eq!(deadlines.len(), 5);
            assert!(deadlines.windows(2).all(|pair| pair[0].0 < pair[1].0));
            let mut other = spell.clone();
            other.key = "other".into();
            assert_eq!(cooldowns.spell_ready_at(&other.key), None);
            assert_eq!(
                cooldowns.rearm(&other, SemanticTimeMicros::from_micros(now)),
                Ok(())
            );
            assert_eq!(
                cooldowns.spell_ready_at(&other.key),
                cooldowns.spell_ready_at(&spell.key)
            );
            assert_eq!(
                cooldowns.group_ready_at(&other.groups[0].key),
                cooldowns.group_ready_at(&spell.groups[0].key)
            );
        } else {
            assert_eq!(result, Err(CastRejection::TimeOverflow));
            assert_eq!(cooldowns, before);
        }
        // Force a late group overflow after a representable individual deadline.
        spell.cooldown_micros = 0;
        spell.groups[0].cooldown_micros = 0;
        spell.groups[1].cooldown_micros = 1;
        let before = cooldowns.clone();
        assert_eq!(
            cooldowns.rearm(&spell, SemanticTimeMicros::from_micros(u64::MAX)),
            Err(CastRejection::TimeOverflow)
        );
        assert_eq!(cooldowns, before);
        Ok(())
    });
}
