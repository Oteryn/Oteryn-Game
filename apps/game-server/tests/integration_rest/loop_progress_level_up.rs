//! Native public-library arithmetic qualification; no copied modules or curve/authority facade.
use oteryn_game_server::domain::progression::{
    LevelUpVitalChange, LevelUpVitalError, LevelUpVitalMaxima, calculate_level_up_vital_change,
};

fn maxima(level: u32, max_health: u32, max_mana: u32) -> LevelUpVitalMaxima {
    LevelUpVitalMaxima {
        level,
        max_health,
        max_mana,
    }
}

#[test]
fn multi_level_gain_uses_supplied_owner_totals_without_a_formula() {
    assert_eq!(
        calculate_level_up_vital_change(maxima(8, 185, 90), maxima(10, 215, 100)),
        Ok(LevelUpVitalChange {
            health_maximum_gain: 30,
            mana_maximum_gain: 10
        })
    );
}

#[test]
fn unchanged_maxima_are_valid_without_refilling_current_pools() {
    assert_eq!(
        calculate_level_up_vital_change(maxima(8, 185, 90), maxima(9, 185, 90)),
        Ok(LevelUpVitalChange {
            health_maximum_gain: 0,
            mana_maximum_gain: 0
        })
    );
}

#[test]
fn zero_same_and_decreasing_levels_fail_closed() {
    for (before, after) in [(0, 1), (8, 8), (8, 7)] {
        assert_eq!(
            calculate_level_up_vital_change(maxima(before, 185, 90), maxima(after, 215, 100)),
            Err(LevelUpVitalError::InvalidLevels)
        );
    }
}

#[test]
fn each_maximum_regression_is_rejected() {
    for after in [maxima(9, 184, 90), maxima(9, 185, 89)] {
        assert_eq!(
            calculate_level_up_vital_change(maxima(8, 185, 90), after),
            Err(LevelUpVitalError::RegressingMaximum)
        );
    }
}
