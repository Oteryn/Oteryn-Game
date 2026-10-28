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

/// PAR-C10: Crystal `201189f8` `BedItem::regeneratePlayer` as data: with `t` food ms left,
/// `regen = min(t / 1000, slept) / 30` points of health and of mana (rates 1.0), the food
/// condition keeps `t - regen * 30000` ms (removed at 0), and soul gains `slept / 900` up to
/// the maximum (`Player::changeSoul`).
fn crystal_bed(pools: [u32; 3], maxima: [u32; 3], food_ms: u32, slept: u64) -> ([u32; 3], u32) {
    let regen = (u64::from(food_ms / 1000).min(slept) / 30) as u32;
    let soul = (slept / 900).min(u64::from(maxima[2] - pools[2].min(maxima[2]))) as u32;
    (
        [
            (pools[0] + regen).min(maxima[0]),
            (pools[1] + regen).min(maxima[1]),
            pools[2] + soul,
        ],
        food_ms - regen * 30_000,
    )
}

#[test]
fn c10_bed_settlement_equals_crystal_regenerate_player() {
    let sleeps = [
        0, 1, 29, 30, 31, 59, 60, 899, 900, 901, 1_199, 1_200, 1_201, 3_600, 89_999, 90_000,
        180_000, 1_814_400,
    ];
    for food_ms in (0..=1_200_000)
        .step_by(997)
        .chain([999, 1_000, 29_999, 30_000, 1_200_000])
    {
        for slept in sleeps {
            for (pools, maxima) in [
                ([1, 0, 0], [1_000, 1_000, 100]),
                ([500, 7, 95], [510, 20, 100]),
                ([200, 200, 199], [200, 200, 200]),
            ] {
                assert_eq!(
                    settle(pools, maxima, food_ms, slept),
                    Some(crystal_bed(pools, maxima, food_ms, slept)),
                    "food {food_ms} ms, slept {slept} s, pools {pools:?}"
                );
            }
        }
    }
}
