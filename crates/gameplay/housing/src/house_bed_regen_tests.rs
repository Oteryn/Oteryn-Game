use super::settle;

#[test]
fn house_bed_regen_food_boundaries_caps_and_remainders() {
    for food in [0, 29, 30, 31, 1199, 1200] {
        for sleep in [0, 29, 30, 31, 899, 900, 1_814_400, 1_814_401, u64::MAX] {
            let points = food.min(sleep) / 30;
            assert_eq!(
                settle([1, 0, 0], [100, 100, 100], food as u32 * 1000, sleep),
                Some((
                    [
                        1 + points as u32,
                        points as u32,
                        (sleep.min(1_814_400) / 900).min(100) as u32
                    ],
                    (food - points * 30) as u32 * 1000
                ))
            );
        }
    }
    assert_eq!(
        settle([100, 100, 100], [100, 100, 100], 31_567, 900),
        Some(([100, 100, 100], 1_567))
    );
    assert_eq!(
        settle([100, 100, 100], [50, 20, 10], 30_000, 900),
        Some(([50, 20, 10], 0))
    );
    assert_eq!(settle([1, 0, 0], [100, 100, 100], 1_200_001, 900), None);
    assert_eq!(settle([0, 0, 0], [100, 100, 100], 1000, 900), None);
}
