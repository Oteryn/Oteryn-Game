use super::*;
use oteryn_client_domain::world::{START, START_FLOOR};

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn an_encoded_fluid_value_selects_the_fluid_pattern() -> Result<(), String> {
    let root = repo();
    let store = AssetStore::open(
        &root.join("content/assets/files"),
        &root.join("imports/official/client-assets/15.30/manifest.json"),
    )
    .map_err(|e| e.to_string())?;
    let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
    let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
    let sheets = SpriteSheets::new(store, catalog);
    let (x, y) = START;
    assert_eq!(placement(x, y, Some(8), START_FLOOR).sub_type, 8);
    assert_eq!(placement(x, y, None, START_FLOOR).count, 1);
    // Fluid appearance 2524 is on the start area with encoded values 1 and 8. Sub-types 8
    // and 12 share colour 5; sub-type 1 is colour 1.
    let mut resolved = Vec::new();
    for encoded in [1, 8, 12] {
        let entry = index
            .resolve(&sheets, 2524, placement(x, y, Some(encoded), START_FLOOR))
            .map_err(|e| e.to_string())?;
        resolved.push(entry.cells.iter().map(|c| c.sprite_id).collect::<Vec<_>>());
    }
    assert_ne!(resolved[0], resolved[1]);
    assert_eq!(resolved[1], resolved[2]);
    Ok(())
}

#[test]
fn a_tile_over_the_draw_cap_is_refused() {
    assert!(within_window_cap(MAX_WINDOW_CELLS).is_ok());
    assert!(within_window_cap(MAX_WINDOW_CELLS + 1).is_err());
    assert!(within_draw_cap(MAX_TILE_DRAWS).is_ok());
    assert!(within_draw_cap(MAX_TILE_DRAWS + 1).is_err());
}
