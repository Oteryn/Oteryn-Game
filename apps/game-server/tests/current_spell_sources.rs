//! The source import must pass the real manifest loader and content compiler.
use oteryn_game_server::{content, foundation::WorldId};

#[test]
fn current_sources_qualify_through_existing_loader() -> Result<(), String> {
    let mut bytes = [1_u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    let world =
        WorldId::decode(&bytes).map_err(|error| format!("invalid test WorldId: {error:?}"))?;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/spells.manifest.json"
    ));
    content::qualify_native_entry_room_from_gameplay_manifest(world, path)
        .map_err(|error| format!("current source manifest was refused: {error:?}"))?;
    Ok(())
}
