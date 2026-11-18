//! Qualify source rewards through USE on native placements without retained UIDs.
use super::{TestResult, quest_catalogue_chests};

#[test]
fn untagged_pits_reward_placements_grant_once_through_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let cases: Vec<_> = quest_catalogue_chests::cases()?
        .into_iter()
        .filter(|case| {
            (case.unique == 5024 || (6202..=6212).contains(&case.unique))
                && case.unique != 6203
        })
        .map(|mut case| {
            case.unique = 0;
            case
        })
        .collect();
    assert_eq!(cases.len(), 11);
    quest_catalogue_chests::execute(cases)
}
