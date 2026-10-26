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

#[test]
fn soulpit_animus_raw_multiplier_stacks_and_rounds_once() {
    use super::hunting_experience_with_animus as award;
    for basis in [200, 210, 390, 400] {
        assert_eq!(
            award(10_000, 0, 0, 1000, false, basis),
            Ok(10_000 + i64::from(basis))
        );
    }
    assert_eq!(award(1000, 40, 50, 2520, true, 400), Ok(3276));
    assert_eq!(award(1000, 40, 50, 840, true, 400), Ok(1092));
    assert_eq!(award(1000, 40, 50, 0, true, 400), Ok(0));
    assert_eq!(award(999, 40, 50, 2520, true, 210), Ok(3212));
    assert_eq!(award(1, 40, 50, 2520, true, 200), Ok(3));
    assert_eq!(
        award(1, 40, 50, 2520, true, 0),
        hunting_experience(1, 40, 50, 2520, true)
    );
    for basis in [-1, 1, 190, 201, 401] {
        assert!(award(1000, 0, 0, 1000, false, basis).is_err());
    }
    assert!(award(i64::MAX, 100, 50, 2520, true, 400).is_err());
}
