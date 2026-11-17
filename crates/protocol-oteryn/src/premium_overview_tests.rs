use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn active() -> Overview {
    Overview {
        state: State::Active,
        authority_revision: 7,
        observed_at_us: 1_000_000,
        expires_at_us: Some(3_601_000_000),
        valid_for_us: 10_000_000,
        features: vec![Feature {
            title: "Premium channels".into(),
            description: "Subject to the world's channel rules.".into(),
            enabled: true,
        }],
    }
}
#[test]
fn premium_overview_codec_preserves_subscription_end_and_bounds() -> TestResult {
    let v = active();
    let bytes = v.encode().ok_or("encode")?;
    assert_eq!(Overview::decode(&bytes), Some(v.clone()));
    assert!(Overview::decode(&bytes[..bytes.len() - 1]).is_none());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Overview::decode(&trailing).is_none());
    let mut wrong = bytes;
    wrong[0] = 2;
    assert!(Overview::decode(&wrong).is_none());
    for mut bad in [v.clone(), v.clone(), v.clone(), v.clone()]
        .into_iter()
        .enumerate()
    {
        match bad.0 {
            0 => bad.1.valid_for_us = 60_000_001,
            1 => bad.1.state = State::None,
            2 => bad.1.features.push(bad.1.features[0].clone()),
            _ => bad.1.features[0].description = "x".repeat(513),
        };
        assert!(bad.1.encode().is_none());
    }
    let mut stale = v;
    stale.valid_for_us = 0;
    assert!(stale.encode().is_none());
    stale.features[0].enabled = false;
    assert!(stale.encode().is_some());
    Ok(())
}
