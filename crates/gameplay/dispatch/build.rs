mod build_mechanics;
#[path = "../../../apps/game-server/build_mounts.rs"]
mod build_mounts;
mod build_pg_shards;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    build_mechanics::generate(&root, &out)?;
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
        "src/gameplay_transport/postgres_tests/support",
        "postgres_test_support_mounts.rs",
        "",
    )?;
    build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests",
        "postgres_tests_mounts.rs",
        "",
    )?;
    build_pg_shards::configure(&root, &out)?;

    const ARCHIVE_ENV: &str = "OTERYN_DISPATCH_TEST_ARCHIVE";
    build_mechanics::emit_cargo_directive("rerun-if-env-changed", ARCHIVE_ENV);
    build_mechanics::emit_cargo_directive(
        "rustc-check-cfg",
        "cfg(oteryn_dispatch_test_archive, values(\"common\", \"dedicated\"))",
    );
    match std::env::var(ARCHIVE_ENV) {
        Err(std::env::VarError::NotPresent) => {}
        value => match value?.as_str() {
            mode @ ("common" | "dedicated") => build_mechanics::emit_cargo_directive(
                "rustc-cfg",
                format_args!("oteryn_dispatch_test_archive=\"{mode}\""),
            ),
            mode => {
                return Err(format!("{ARCHIVE_ENV}={mode:?} must be common or dedicated").into());
            }
        },
    }
    Ok(())
}
