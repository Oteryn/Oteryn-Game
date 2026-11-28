use super::TestResult;

pub(super) fn committed_attrs(
    position: (u16, u16, i8),
    appearance: u16,
) -> TestResult<oteryn_world_bundle::sector::Attrs> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (x, y, floor) = position;
    let path = root.join(format!(
        "content/world/placements/region-z{:02}-x{:03}-y{:03}.b3",
        -floor,
        x / 256,
        y / 256
    ));
    let mut budget = oteryn_world_bundle::bundle::BUNDLE_BUDGET;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        &std::fs::read(path)?,
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut budget,
    )?;
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let entries: Vec<_> = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|tile| tile.x == x && tile.y == y)
        .flat_map(|tile| &tile.items)
        .filter(|item| {
            item.depth == 0
                && index["palette"][item.palette as usize]["source_item_id"] == appearance
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "one committed source entry at {position:?}"
    );
    Ok(entries[0].attrs.clone())
}
