//! Decode existing pinned assets only after the server resolves public artwork IDs.
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets};
use oteryn_session::property_services::house_podium::preview::Artwork;
use std::path::Path;
pub(super) struct Assets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}
impl Assets {
    pub(super) fn load(root: &Path) -> Result<Self, String> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|e| e.to_string())?;
        let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    pub(super) fn render(&mut self, artwork: Artwork) -> Result<Vec<Vec<u8>>, String> {
        let count = if artwork.looks[0] == artwork.looks[1] {
            1
        } else {
            2
        };
        artwork
            .looks
            .into_iter()
            .take(count)
            .map(|look| {
                let entry = if artwork.object {
                    self.index
                        .resolve(&self.sheets, look, Placement::at(0, 0, -7))
                } else {
                    self.index
                        .resolve_outfit(&self.sheets, look, u32::from(artwork.direction))
                }
                .map_err(|e| e.to_string())?;
                if entry.cells.len() > 64 {
                    return Err("Artwork exceeds the preview bound".into());
                }
                let cells = entry
                    .cells
                    .into_iter()
                    .map(|cell| {
                        self.sheets
                            .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                            .map(|rgba| (cell.offset_x, cell.offset_y, rgba))
                            .map_err(|e| e.to_string())
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(super::preview::compose(cells))
            })
            .collect()
    }
}
