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

#[test]
fn cyclopedia_navigation_is_collected_from_feature_fragments() {
    let pages = oteryn_client_ui::cyclopedia_pages();
    assert_eq!(
        pages
            .iter()
            .map(|page| (page.mechanic, page.id, page.parent))
            .collect::<Vec<_>>(),
        [
            ("0001-bestiary-charms", "cyclopedia-bestiary", "root"),
            ("0001-bestiary-charms", "cyclopedia-charms", "root"),
            ("0057-cyclopedia-map", "cyclopedia-map", "root"),
        ]
    );
}
