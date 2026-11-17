//! Match the server fragment guard: compare the actual build output byte for byte.
#[path = "../../../tools/architecture-check/src/client_fragments.rs"]
mod client_fragments;

use std::path::Path;

fn pc02_supported(mechanic: &str, command: u64) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (entries, _) = client_fragments::scan(&root)?;
    assert!(entries.iter().any(|entry| entry["mechanic"] == mechanic
        && entry["kind"] == "handlers"
        && entry["status"] == "supported"
        && entry["commands"] == serde_json::json!([command])));
    Ok(())
}

#[test]
fn pc02_player_report_registered() -> Result<(), String> {
    pc02_supported("0076-player-report", 93)
}

#[test]
fn pc02_guild_war_registered() -> Result<(), String> {
    pc02_supported("0103-guild-war", 119)
}

#[test]
fn pc02_container_sort_registered() -> Result<(), String> {
    pc02_supported("0146-container-sort", 168)
}

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
