#[path = "../../../apps/game-server/build_mounts.rs"]
mod build_mounts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // sqlx tracks embedded files, but a newly added migration must also rebuild the catalog.
    println!("cargo:rerun-if-changed=../../../apps/game-server/migrations");
    build_mounts::mount_directory("src/tests/postgres", "postgres_read_cases_mounts.rs", "")
}
