use super::*;
#[test]
fn prey_precedes_stamina_and_thresholds_are_exact() {
    assert_eq!(hunting_experience(101, 40, 2520, true), Ok(211));
    assert_eq!(hunting_experience(101, 40, 2520, false), Ok(141));
    assert_eq!(hunting_experience(101, 40, 2340, true), Ok(141));
    assert_eq!(hunting_experience(101, 40, 841, true), Ok(141));
    assert_eq!(hunting_experience(101, 40, 840, true), Ok(70));
    assert_eq!(hunting_experience(101, 40, 0, true), Ok(0));
    assert!(hunting_experience(i64::MAX, 100, 2520, true).is_err());
    for (share, percent, stamina) in [(-1, 0, 2520), (1, -1, 2520), (1, 101, 2520), (1, 0, 2521)] {
        assert!(hunting_experience(share, percent, stamina, true).is_err());
    }
}
