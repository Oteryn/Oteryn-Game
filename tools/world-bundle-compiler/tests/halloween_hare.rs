//! Source-Hare B3 fixture is shared with the gameplay-world source physics regression.
#[path = "support/halloween_hare_fixture.rs"]
mod actual_fixture;
#[test]
fn halloween_hare_bundle_preserves_real_source_landing_and_observer()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = actual_fixture::compile(&root)?;
    if let Some(path) = std::env::var_os("OTERYN_SRV7_HARE_BUNDLE") {
        std::fs::write(path, bytes)?;
    }
    Ok(())
}
