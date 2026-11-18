//! Foolish Quest reward USE through both shipped chest placements.
use super::TestResult;

#[test]
fn foolish_watch_and_bag_use_shipped_chests() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let cases: Vec<_> = super::quest_catalogue_chests::cases()?
        .into_iter()
        .filter(|chest| matches!(chest.unique, 6259 | 6269))
        .collect();
    assert_eq!(
        cases
            .iter()
            .map(|chest| chest.unique)
            .collect::<std::collections::BTreeSet<_>>(),
        [6259, 6269].into()
    );
    super::quest_catalogue_chests::execute(cases)
}
