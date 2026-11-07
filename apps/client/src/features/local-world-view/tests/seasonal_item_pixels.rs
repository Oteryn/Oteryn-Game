// Test-only actual official Item-look cells compared with real native quads.
fn seasonal_item_official_pixels(
    view: &PlayView,
    identity: [u8; 16],
    appearance_id: u32,
) -> Result<(), HarePlayError> {
    use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
    use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
    use sha2::Digest;
    let root = std::path::PathBuf::from(
        std::env::var_os(crate::map::ASSET_DIR_ENV)
            .ok_or("official pixel play requires genuine OTERYN_ASSET_DIR")?,
    );
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(MANIFEST_PATH),
        MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    let mut sheets = SpriteSheets::new(store, catalog);
    let hare = view
        .appearances
        .iter()
        .find(|entry| entry.entity.entity.identity == identity)
        .ok_or("actual seasonal body entity")?;
    assert!(
        matches!(hare.look, Some(oteryn_session::Look::Item { .. })),
        "actual server sends a typed Item look"
    );
    assert!(
        matches!(appearance_id, 2096 | 6393),
        "closed source presentation only"
    );
    assert_eq!(hare.entity.position.floor, view.map_center().2);
    let official = index.resolve(
        &sheets,
        appearance_id,
        oteryn_client_assets::Placement::at(0, 0, 0),
    )?;
    let scene = view.scene();
    let tile = oteryn_renderer::TileCoord::new(hare.entity.position.x, hare.entity.position.y);
    assert!(
        scene.view().contains(tile),
        "source Hare is inside the independently captured camera"
    );
    let at = scene.view().tile_to_screen(tile);
    let scale = scene.view().tile_px() as f32 / 32.0;
    let atlas = scene.atlas();
    let mut opaque_cells = 0;
    for cell in official.cells {
        let expected = sheets.cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)?;
        if !expected
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] != 0)
        {
            continue;
        }
        opaque_cells += 1;
        let position = [
            at[0] + cell.offset_x as f32 * scale,
            at[1] + cell.offset_y as f32 * scale,
        ];
        let shown = scene
            .sprites()
            .instances()
            .iter()
            .filter(|quad| quad.position == position)
            .any(|quad| {
                let x = (quad.uv[0] * atlas.width() as f32).round() as usize;
                let y = (quad.uv[1] * atlas.height() as f32).round() as usize;
                let width = ((quad.uv[2] - quad.uv[0]) * atlas.width() as f32).round() as usize;
                let height = ((quad.uv[3] - quad.uv[1]) * atlas.height() as f32).round() as usize;
                width == 32
                    && height == 32
                    && (0..32).all(|row| {
                        let start = ((y + row) * atlas.width() as usize + x) * 4;
                        atlas.rgba().get(start..start + 128)
                            == expected.get(row * 128..row * 128 + 128)
                    })
            });
        assert!(
            shown,
            "actual seasonal Item official sprite{} cell({},{}), appearance{} must be drawn at captured tile/offset {:?}",
            cell.sprite_id, cell.cell_x, cell.cell_y, appearance_id, position
        );
        eprintln!(
            "SRV7_SEASONAL_ITEM_OFFICIAL_PIXELS sprite={} appearance={} rgba_sha256={} native_floor={}",
            cell.sprite_id,
            appearance_id,
            sha2::Sha256::digest(&expected)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            hare.entity.position.floor
        );
    }
    assert!(
        opaque_cells > 0,
        "pinned official seasonal Item body must contain real nontransparent pixels"
    );
    Ok(())
}
