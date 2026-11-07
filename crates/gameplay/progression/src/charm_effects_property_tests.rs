#![allow(clippy::expect_used)]
use super::{
    CharmAttackerFacts, CharmDamageKind, CharmEffect, CharmEffectOutcome, CharmElement,
    CharmHitSource, CharmHookEvent, CharmPercent, effect_outcome, hundredths_roll,
};
use oteryn_test_support::property::for_each_case;

#[test]
fn property_critical_percentages_and_elemental_procs_keep_their_typed_mapping() {
    for_each_case("charm critical and elements", |rng, _| {
        let draw = rng.u64();
        for draw in [0, draw, u64::MAX] {
            assert!(hundredths_roll(draw) < 10_000);
            assert!(hundredths_roll(draw.saturating_add(1)) >= hundredths_roll(draw));
        }
        assert_eq!(hundredths_roll(0), 0);
        assert_eq!(hundredths_roll(u64::MAX), 9_999);
        let raw = 1 + rng.below(100_000) as u32;
        let percent = CharmPercent::from_hundredths(raw).expect("catalogue percent");
        let base = rng.u64();
        for base in [0, base, u64::MAX] {
            let expected = (u128::from(base) * u128::from(raw) / 10_000).min(u128::from(u64::MAX));
            assert_eq!(percent.of(base), expected as u64);
        }
        assert_eq!(CharmPercent::from_hundredths(0), None);
        assert_eq!(CharmPercent::from_hundredths(100_001), None);
        let event = CharmHookEvent::AttackDamageCalculation {
            source: CharmHitSource::CharacterAttack,
        };
        assert_eq!(
            effect_outcome(CharmEffect::CriticalHitChance, percent, &event),
            Some(CharmEffectOutcome::CriticalHitChanceBonus { percent })
        );
        assert_eq!(
            effect_outcome(CharmEffect::CriticalExtraDamage, percent, &event),
            Some(CharmEffectOutcome::CriticalExtraDamageBonus { percent })
        );
        for element in [
            CharmElement::Physical,
            CharmElement::Fire,
            CharmElement::Earth,
            CharmElement::Ice,
            CharmElement::Energy,
            CharmElement::Death,
            CharmElement::Holy,
        ] {
            let damage = CharmDamageKind {
                element,
                ignores_resistances: rng.bool(),
                reduced_by_armor: rng.bool(),
            };
            let level = 1 + rng.below(2000) as u32;
            let multiplier = 1 + rng.below(100) as u32;
            let effect = CharmEffect::AttackProcDamage {
                damage,
                percent_of_creature_max_health: percent,
                damage_cap_level_multiplier: multiplier,
            };
            let event = CharmHookEvent::CommittedHit {
                source: CharmHitSource::CharacterAttack,
                attacker: CharmAttackerFacts {
                    level,
                    max_health: base,
                    max_mana: 0,
                },
                creature_max_health: base,
                health_before: 10,
                health_after: 1,
            };
            let result = effect_outcome(effect, percent, &event);
            assert_eq!(
                result,
                Some(CharmEffectOutcome::ProcDamage {
                    damage,
                    amount: percent
                        .of(base)
                        .min(u64::from(level) * u64::from(multiplier))
                })
            );
        }
        Ok(())
    });
}
