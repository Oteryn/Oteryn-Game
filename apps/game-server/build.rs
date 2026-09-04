fn main() {
    const MIGRATION: &str = include_str!("migrations/0001_admission_reconnect_journal.sql");
    assert!(
        MIGRATION.contains(
            "candidate_reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(candidate_reconnect_attempt_ref) = 8)"
        ),
        "terminal replacement receipt must persist the exact candidate reconnect attempt ref"
    );
    println!("cargo:rerun-if-changed=migrations");
}
