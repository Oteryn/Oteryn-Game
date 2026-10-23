use super::*;
use oteryn_test_support::property::for_each_case;

#[test]
fn property_cast_mana_and_soul_spend_is_exact_and_insufficient_pools_refuse() {
    let book = book();
    let template = book.spoken("exura").expect("starter healing").spell;
    for_each_case("cast resource spend", |rng, _| {
        let mut spell = template.clone();
        let mana = rng.u64() as u32;
        let soul = rng.u64() as u32;
        spell.mana = ManaCost::Fixed(mana);
        spell.soul = soul;
        let mut caster = caster(Vocation::Druid, 100, 100, mana);
        caster.soul = soul;
        let resolution = resolve_cast(
            &spell,
            &caster,
            &Cooldowns::default(),
            at(0),
            false,
            &mut lowest,
        )
        .expect("exact resources");
        assert_eq!((resolution.mana_spent, resolution.soul_spent), (mana, soul));
        assert_eq!(caster.mana - resolution.mana_spent, 0);
        assert_eq!(caster.soul - resolution.soul_spent, 0);
        if mana > 0 {
            caster.mana = mana - 1;
            assert_eq!(
                resolve_cast(
                    &spell,
                    &caster,
                    &Cooldowns::default(),
                    at(0),
                    false,
                    &mut lowest
                ),
                Err(CastRejection::NotEnoughMana { required: mana })
            );
            caster.mana = mana;
        }
        if soul > 0 {
            caster.soul = soul - 1;
            assert_eq!(
                resolve_cast(
                    &spell,
                    &caster,
                    &Cooldowns::default(),
                    at(0),
                    false,
                    &mut lowest
                ),
                Err(CastRejection::NotEnoughSoul { required: soul })
            );
        }
        // Zero and maximum costs are deterministic boundary cases too.
        for cost in [0, u32::MAX] {
            spell.mana = ManaCost::Fixed(cost);
            spell.soul = cost;
            caster.mana = cost;
            caster.soul = cost;
            let resolution = resolve_cast(
                &spell,
                &caster,
                &Cooldowns::default(),
                at(0),
                false,
                &mut lowest,
            )
            .expect("boundary costs");
            assert_eq!((resolution.mana_spent, resolution.soul_spent), (cost, cost));
        }
        Ok(())
    });
}
