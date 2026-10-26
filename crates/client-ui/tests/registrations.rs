//! Match the server fragment guard: compare the actual build output byte for byte.
#[path = "../../../tools/architecture-check/src/client_fragments.rs"]
mod client_fragments;

use std::path::Path;

#[test]
fn build_output_equals_discovered_fragments_and_mechanic_coverage() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (entries, coverage) = client_fragments::scan(&root)?;
    let (rust, report) = client_fragments::render(&entries, &coverage)?;
    assert_eq!(
        rust,
        include_str!(concat!(env!("OUT_DIR"), "/client_registrations.rs"))
    );
    assert_eq!(
        report,
        include_str!(concat!(env!("OUT_DIR"), "/client-mechanic-coverage.json"))
    );
    assert_eq!(coverage.len(), client_fragments::mechanics(&root)?.len());
    Ok(())
}
