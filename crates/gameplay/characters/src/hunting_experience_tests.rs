use super::*;
#[test]
fn prey_precedes_stamina_and_thresholds_are_exact() {
    assert_eq!(hunting_experience(101, 40, 0, 2520, true), Ok(211));
    assert_eq!(hunting_experience(101, 40, 0, 2520, false), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 2340, true), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 841, true), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 840, true), Ok(70));
    assert_eq!(hunting_experience(101, 40, 0, 0, true), Ok(0));
    assert!(hunting_experience(i64::MAX, 100, 0, 2520, true).is_err());
    for (share, percent, stamina) in [(-1, 0, 2520), (1, -1, 2520), (1, 101, 2520), (1, 0, 2521)] {
        assert!(hunting_experience(share, percent, 0, stamina, true).is_err());
    }
}

#[test]
fn daily_xp_boost_is_fifty_percent_before_stamina() {
    assert_eq!(hunting_experience(100, 0, 50, 2520, false), Ok(150));
    assert_eq!(hunting_experience(100, 0, 50, 2520, true), Ok(225));
    assert_eq!(hunting_experience(100, 40, 50, 840, true), Ok(105));
    assert_eq!(hunting_experience(100, 40, 50, 0, true), Ok(0));
    assert!(hunting_experience(100, 0, 49, 2520, true).is_err());
}
