use super::TestResult;

#[test]
fn p11_first_carried_potion_use_commits_once_without_container_read() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    crate::gameplay_transport::item_use::item_use_postgres_tests::run_potion_bag_read_budget_regression(admin)
}
