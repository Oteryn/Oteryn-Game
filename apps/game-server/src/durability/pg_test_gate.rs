//! Test-only gate shared by the PostgreSQL test fixtures.
//!
//! A PostgreSQL test runs when `OTERYN_TEST_POSTGRES_ADMIN_URL` is set and skips otherwise.
//! A job that sets `OTERYN_REQUIRE_PG=1` must run its PostgreSQL tests, so a missing admin
//! URL there fails the test instead of letting it pass silently.

/// Decides whether a PostgreSQL test runs (`Ok(Some)`), skips (`Ok(None)`) or fails (`Err`).
pub(crate) fn admin_url_decision(
    required: bool,
    admin_url: Option<String>,
) -> Result<Option<String>, &'static str> {
    match admin_url {
        Some(value) => Ok(Some(value)),
        None if required => Err(
            "OTERYN_REQUIRE_PG=1 but OTERYN_TEST_POSTGRES_ADMIN_URL is not configured: \
             this job must run its PostgreSQL tests instead of skipping them",
        ),
        None => Ok(None),
    }
}

/// Whether this process must run its PostgreSQL tests (`OTERYN_REQUIRE_PG=1`).
pub(crate) fn postgres_required() -> bool {
    std::env::var("OTERYN_REQUIRE_PG").is_ok_and(|value| value == "1")
}

/// The configured test-admin URL, or `None` to skip.
///
/// # Panics
///
/// Panics when `OTERYN_REQUIRE_PG=1` and the admin URL is missing.
pub(crate) fn configured_admin() -> Option<String> {
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL").ok();
    match admin_url_decision(postgres_required(), admin_url) {
        Ok(Some(value)) => Some(value),
        Ok(None) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            None
        }
        Err(message) => panic!("{message}"),
    }
}

#[test]
fn a_required_postgres_job_without_an_admin_url_reports_an_error() {
    let url = "postgresql://oteryn_test_admin:x@127.0.0.1:5432/postgres".to_owned();
    assert!(admin_url_decision(true, None).is_err());
    assert_eq!(admin_url_decision(false, None), Ok(None));
    assert_eq!(
        admin_url_decision(true, Some(url.clone())),
        Ok(Some(url.clone()))
    );
    assert_eq!(admin_url_decision(false, Some(url.clone())), Ok(Some(url)));
}
