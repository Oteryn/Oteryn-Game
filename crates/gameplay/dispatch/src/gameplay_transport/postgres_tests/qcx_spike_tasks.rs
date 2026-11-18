//! The retained Kazordoon sign is on a decorative crystal, so the return callback
//! must keep its source action binding until a walkable source placement is qualified.
use super::TestResult;

#[test]
fn spike_return_selector_requires_its_source_action() -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::{Position, load};

    let callback = load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/spike-portal-4227")
        .ok_or("Spike return callback")?;
    assert_eq!(callback.selector.position, None);
    assert_eq!(callback.selector.action, Some(4227));
    assert_eq!(
        callback.teleport,
        Some(Position {
            x: 32624,
            y: 31855,
            floor: -11,
        })
    );

    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z08-x125-y127.b3"
    ));
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        bytes,
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let sign_tile = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|tile| (tile.x, tile.y) == (32242, 32597))
        .ok_or("To: Kazordoon sign tile")?;
    assert!(
        sign_tile
            .items
            .iter()
            .any(|item| item.attrs.text.as_deref() == Some("To: Kazordoon"))
    );
    assert!(
        sign_tile
            .items
            .iter()
            .all(|item| item.attrs.action != Some(4227))
    );
    Ok(())
}
