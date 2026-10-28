use super::*;
#[test]
fn prey_precedes_stamina_and_thresholds_are_exact() {
    assert_eq!(hunting_experience(101, 40, 0, 2520, true, 50), Ok(211));
    assert_eq!(hunting_experience(101, 40, 0, 2520, false, 50), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 2340, true, 50), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 841, true, 50), Ok(141));
    assert_eq!(hunting_experience(101, 40, 0, 840, true, 50), Ok(70));
    assert_eq!(hunting_experience(101, 40, 0, 0, true, 50), Ok(0));
    assert!(hunting_experience(i64::MAX, 100, 0, 2520, true, 50).is_err());
    for (share, percent, stamina) in [(-1, 0, 2520), (1, -1, 2520), (1, 101, 2520), (1, 0, 2521)] {
        assert!(hunting_experience(share, percent, 0, stamina, true, 50).is_err());
    }
}

#[test]
fn daily_xp_boost_is_fifty_percent_before_stamina() {
    assert_eq!(hunting_experience(100, 0, 50, 2520, false, 50), Ok(150));
    assert_eq!(hunting_experience(100, 0, 50, 2520, true, 50), Ok(225));
    assert_eq!(hunting_experience(100, 40, 50, 840, true, 50), Ok(105));
    assert_eq!(hunting_experience(100, 40, 50, 0, true, 50), Ok(0));
    assert!(hunting_experience(100, 0, 49, 2520, true, 50).is_err());
}

#[test]
fn low_level_bonus_follows_the_global_table() {
    // TibiaWiki Experience Point rev 1161628, "Actual XP bonus" column.
    for (level, award) in [
        (1, 2400),
        (2, 2400),
        (3, 2375),
        (14, 2100),
        (15, 2075),
        (26, 1800),
        (49, 1225),
        (50, 1200),
        (51, 1200),
        (u32::MAX, 1200),
    ] {
        assert_eq!(
            hunting_experience(1200, 0, 0, 2000, false, level),
            Ok(award),
            "level {level}"
        );
    }
    let bonus: Vec<i128> = (1..=50).map(low_level_bonus_1200ths).collect();
    assert!(bonus.windows(2).all(|pair| pair[0] >= pair[1]));
}

#[test]
fn low_level_bonus_adds_to_the_daily_boost_before_stamina() {
    assert_eq!(hunting_experience(100, 0, 50, 2000, false, 1), Ok(250));
    assert_eq!(hunting_experience(100, 0, 50, 2520, true, 1), Ok(375));
    assert_eq!(hunting_experience(100, 40, 0, 2000, false, 26), Ok(210));
    assert_eq!(hunting_experience(100, 0, 0, 840, false, 1), Ok(100));
    assert_eq!(hunting_experience(100, 0, 0, 0, false, 1), Ok(0));
}
