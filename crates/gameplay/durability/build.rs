fn main() {
    // sqlx::migrate! tracks existing files, but adding a migration must also
    // rebuild the embedded ledger used by every dependent gameplay crate.
    println!("cargo:rerun-if-changed=../../../apps/game-server/migrations");
}
fn main() {
    // sqlx::migrate! embeds this directory; adding a migration must rebuild the
    // owner crate as well as the game-server that runs it.
    println!("cargo:rerun-if-changed=../../../apps/game-server/migrations");
}
