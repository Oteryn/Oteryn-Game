fn main() {
    // sqlx::migrate! tracks existing files, but adding a migration must also
    // rebuild the embedded ledger used by every dependent gameplay crate.
    println!("cargo:rerun-if-changed=../../../apps/game-server/migrations");
}
