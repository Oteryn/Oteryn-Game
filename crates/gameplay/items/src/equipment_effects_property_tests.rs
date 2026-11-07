use crate::equipment_effects::apply_protection;
use oteryn_gameplay_content::content::ReferenceRationalPercent;
use oteryn_test_support::property::for_each_case;

#[test]
fn property_protection_clamps_percentages_and_never_produces_negative_damage() {
    for_each_case("protection percent bounds", |rng, _| {
        let damage = rng.range_i64(0, 1_000_000);
        let numerator = rng.range_i64(-100_000, 100_000);
        let denominator = 1 + rng.below(100);
        for numerator in [numerator, -10100, -100, 0, 100, 10100] {
            let percent = ReferenceRationalPercent {
                numerator,
                denominator,
            };
            if percent.validate().is_err() {
                assert_eq!(apply_protection(damage, percent), None);
                continue;
            }
            let bounded = i128::from(numerator).clamp(
                -100 * i128::from(denominator),
                100 * i128::from(denominator),
            );
            let expected =
                i128::from(damage) - i128::from(damage) * bounded / (100 * i128::from(denominator));
            assert_eq!(apply_protection(damage, percent), Some(expected as i64));
            assert!((0..=2 * i128::from(damage)).contains(&expected));
        }
        assert_eq!(
            apply_protection(
                -1,
                ReferenceRationalPercent {
                    numerator: 0,
                    denominator: 1
                }
            ),
            None
        );
        assert_eq!(
            apply_protection(
                damage,
                ReferenceRationalPercent {
                    numerator: 1,
                    denominator: 0
                }
            ),
            None
        );
        assert_eq!(
            apply_protection(
                i64::MAX,
                ReferenceRationalPercent {
                    numerator: 100,
                    denominator: 1
                }
            ),
            None,
            "unrepresentable intermediate refuses rather than wraps"
        );
        Ok(())
    });
}
