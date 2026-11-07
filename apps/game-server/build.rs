#[path = "build_mounts.rs"]
mod build_mounts;

use build_mounts::mount_directory;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    const MIGRATION: &str = include_str!("migrations/0001_admission_reconnect_journal.sql");
    assert!(
        MIGRATION.contains(
            "candidate_reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(candidate_reconnect_attempt_ref) = 8)"
        ),
        "terminal replacement receipt must persist the exact candidate reconnect attempt ref"
    );
    println!("cargo:rerun-if-changed=migrations");

    // A new mechanic drops its PostgreSQL case file into one of these directories instead of
    // appending a `#[path] mod` line to the shared mount file every other mechanic also edits.
    mount_directory(
        "tests/support/authority_cases",
        "authority_cases_mounts.rs",
        "",
    )?;
    mount_directory(
        "src/lifted_tests/auto",
        "lifted_tests_auto_mounts.rs",
        "#[cfg(test)]\n",
    )
}
