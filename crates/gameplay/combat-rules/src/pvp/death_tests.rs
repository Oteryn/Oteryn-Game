use super::death::*;

#[test]
fn twist_only_protects_regular_blessings_on_an_ordinary_pvp_death() {
    assert_eq!(
        blessing_disposition(true, false, false, 3, true),
        BlessingDisposition::ConsumeTwist
    );
    assert_eq!(
        blessing_disposition(true, false, false, 0, true),
        BlessingDisposition::ConsumeRegular
    );
    assert_eq!(
        blessing_disposition(true, true, false, 3, true),
        BlessingDisposition::ConsumeRegular
    );
}

#[test]
fn adventurer_precedes_skull_and_non_pvp_never_consumes_twist() {
    assert_eq!(
        blessing_disposition(true, true, true, 3, true),
        BlessingDisposition::KeepAll
    );
    assert_eq!(
        blessing_disposition(false, false, false, 3, true),
        BlessingDisposition::ConsumeRegular
    );
}
