mod build_mechanics;
#[path = "../../../apps/game-server/build_mounts.rs"]
mod build_mounts;
mod build_pg_fixture;
mod build_pg_groups;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    build_mechanics::generate(&root, &out)?;
    build_pg_fixture::generate(&root, &out)?;
    build_mounts::mount_directory(
        "src/gameplay_transport/connection/routing",
        "mechanic_routing_modules.rs",
        "",
    )?;
    // The shared mount helper emits private test modules; routing bodies are visible to
    // the sibling serve phases, retaining their previous pub(super) module visibility.
    let routing = out.join("mechanic_routing_modules.rs");
    let mounts = std::fs::read_to_string(&routing)?.replace("\nmod ", "\npub(super) mod ");
    std::fs::write(routing, mounts)?;
    // A new mechanic drops its PostgreSQL test file into this directory instead of appending a
    // `#[path] mod` line to `monk_save_postgres_tests.rs`, which every mechanic also edits.
    build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests",
        "postgres_tests_mounts.rs",
        "",
    )?;
    let path = out.join("postgres_tests_mounts.rs");
    let mounts = std::fs::read_to_string(&path)?;
    let mut grouped = String::new();
    for line in mounts.lines() {
        if let Some(name) = line
            .strip_prefix("mod ")
            .and_then(|line| line.strip_suffix(';'))
        {
            grouped.push_str(build_pg_groups::attributes(name)?);
        }
        grouped.push_str(line);
        grouped.push('\n');
    }
    std::fs::write(path, grouped)?;
    Ok(())
}
