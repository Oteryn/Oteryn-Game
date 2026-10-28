#![allow(clippy::unwrap_used)]
use super::*;
fn rates() -> TrainingRates {
    TrainingRates::from_json(include_bytes!(
        "../../../../rulesets/training/global-v1.json"
    ))
    .unwrap()
}
#[test]
fn loop_training_verified_rates_and_unknown_vocation_fail_closed() {
    let rates = rates();
    assert!(rates.enabled);
    assert_eq!(rates.offline_gain("knight", 3, 600), Ok(150));
    assert_eq!(rates.offline_gain("paladin", 5, 600), Ok(75));
    assert_eq!(rates.offline_gain("sorcerer", 0, 600), Ok(600));
    assert!(rates.offline_gain("unknown", 3, 600).is_err());
    assert!(rates.offline_gain("knight", 7, 600).is_err());
    assert!(rates.offline_gain("knight", 3, 43201).is_err());
}
#[test]
fn loop_training_pool_delay_cap_restore_and_short_pool() {
    assert_eq!(
        plan_offline(43200, Some(3), 599),
        Ok(OfflinePlan {
            pool_after: 43200,
            training_seconds: 0
        })
    );
    assert_eq!(
        plan_offline(43200, Some(3), 600),
        Ok(OfflinePlan {
            pool_after: 42600,
            training_seconds: 600
        })
    );
    assert_eq!(
        plan_offline(30, Some(3), 600),
        Ok(OfflinePlan {
            pool_after: 570,
            training_seconds: 30
        })
    );
    assert_eq!(
        plan_offline(43200, Some(0), 43200),
        Ok(OfflinePlan {
            pool_after: 0,
            training_seconds: 43200
        })
    );
    assert_eq!(
        plan_offline(43200, Some(0), 86400),
        Ok(OfflinePlan {
            pool_after: 43200,
            training_seconds: 43200
        })
    );
    assert_eq!(
        plan_offline(0, None, u64::MAX),
        Ok(OfflinePlan {
            pool_after: 43200,
            training_seconds: 0
        })
    );
    assert!(plan_offline(43201, None, 600).is_err());
    assert!(plan_offline(43200, Some(6), 600).is_err());
}
#[test]
fn loop_training_configuration_refuses_unknown_fields_and_zero_divisors() {
    let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../rulesets/training/global-v1.json"
    ))
    .unwrap();
    value["offline_divisors"][3] = serde_json::json!(0);
    assert!(TrainingRates::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    value["offline_divisors"][3] = serde_json::json!(2);
    value["extra"] = serde_json::json!(1);
    assert!(TrainingRates::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn loop_training_configuration_byte_boundary() {
    let mut bytes = include_bytes!("../../../../rulesets/training/global-v1.json").to_vec();
    bytes.resize(32768, b' ');
    assert!(TrainingRates::from_json(&bytes).is_ok());
    bytes.push(b' ');
    assert!(TrainingRates::from_json(&bytes).is_err());
}

#[test]
fn crystal_exercise_dummy_percentages_match_per_charge_integer_conversion() {
    let rates = rates();
    assert_eq!(rates.exercise_gain(0, 100), Ok(600));
    assert_eq!(rates.exercise_gain(0, 110), Ok(660));
    for family in 1..=6 {
        assert_eq!(rates.exercise_gain(family, 100), Ok(7));
        assert_eq!(rates.exercise_gain(family, 110), Ok(7));
    }
    assert!(rates.exercise_gain(7, 100).is_err());
    assert!(rates.exercise_gain(0, 105).is_err());
}

#[test]
fn crystal_offline_hourly_rates_cover_every_vocation_and_skill() {
    let rates = rates();
    for vocation in &rates.vocations {
        let expected_magic =
            3_600_000_u64 * u64::from(vocation.mana_amount) / u64::from(vocation.top_mana_ticks_ms);
        assert_eq!(
            rates.offline_gain(&vocation.vocation, 0, 3_600),
            Ok(expected_magic),
            "{} magic",
            vocation.vocation
        );
        for family in 1..=6 {
            let expected = 3_600_000_u64
                / (u64::from(vocation.top_attack_ms) * u64::from(rates.offline_divisors[family]));
            assert_eq!(
                rates.offline_gain(&vocation.vocation, family, 3_600),
                Ok(expected),
                "{} family {family}",
                vocation.vocation
            );
        }
    }
}

#[test]
fn crystal_and_canary_training_item_populations_are_complete() {
    let rates = rates();
    let ids = |keys: Vec<&String>| {
        keys.into_iter()
            .map(|key| key.rsplit('i').next().unwrap().parse::<u32>().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        ids(rates.statues.keys().collect()),
        [16198, 16199, 16200, 16201, 16202, 50296].into()
    );
    assert_eq!(
        ids(rates.dummies.keys().collect()),
        [28558, 28559, 28560, 28561, 28562, 28563, 28564, 28565].into()
    );
    assert_eq!(
        ids(rates.weapons.keys().collect()),
        [
            28540, 28541, 28542, 28543, 28544, 28545, 28552, 28553, 28554, 28555, 28556, 28557,
            35279, 35280, 35281, 35282, 35283, 35284, 35285, 35286, 35287, 35288, 35289, 35290,
            44064, 44065, 44066, 44067, 50292, 50293, 50294, 50295,
        ]
        .into()
    );
}
