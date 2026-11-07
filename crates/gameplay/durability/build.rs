fn main() {
    // sqlx::migrate! embeds a directory's files, but adding a migration must also
    // invalidate this crate when no Rust source changes. Every mounted PG harness
    // borrows GAME_MIGRATOR from this one owner.
    let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/game-server/migrations");
    println!("cargo:rerun-if-changed={}", migrations.display());
}
