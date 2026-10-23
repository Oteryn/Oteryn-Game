#![allow(clippy::unwrap_used)]
use super::*;
fn rates() -> TrainingRates {
    TrainingRates::from_json(include_bytes!(
        "../../../../rulesets/training/provisional-v1.json"
    ))
    .unwrap()
}
#[test]
fn loop_training_gate_and_unknown_vocation_fail_closed() {
    let mut rates = rates();
    assert!(!rates.enabled);
    assert!(rates.offline_gain("knight", 3, 600).is_err());
    rates.enabled = true;
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
        "../../../../rulesets/training/provisional-v1.json"
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
    let mut bytes = include_bytes!("../../../../rulesets/training/provisional-v1.json").to_vec();
    bytes.resize(32768, b' ');
    assert!(TrainingRates::from_json(&bytes).is_ok());
    bytes.push(b' ');
    assert!(TrainingRates::from_json(&bytes).is_err());
}
