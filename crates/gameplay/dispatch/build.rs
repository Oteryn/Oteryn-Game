#[path = "../../../apps/game-server/build_mounts.rs"]
mod build_mounts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A new mechanic drops its PostgreSQL test file into this directory instead of appending a
    // `#[path] mod` line to `monk_save_postgres_tests.rs`, which every mechanic also edits.
    build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests",
        "postgres_tests_mounts.rs",
        "",
    )
}
