//! Read existing pinned assets in memory; never copy sprites into the package.
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
pub(super) struct Art {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}
impl Art {
    pub(super) fn load() -> Result<Self, String> {
        let root = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .ok_or("Set the asset directory to load appearance previews")?;
        let root = std::path::Path::new(&root);
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
    pub(super) fn pixels(&mut self, look: u32) -> Result<Vec<u8>, String> {
        let entry = self
            .index
            .resolve_outfit(&self.sheets, look, 2)
            .map_err(|e| e.to_string())?;
        let left = entry
            .cells
            .iter()
            .map(|c| c.offset_x)
            .min()
            .ok_or("No sprite cells")?;
        let top = entry
            .cells
            .iter()
            .map(|c| c.offset_y)
            .min()
            .ok_or("No sprite cells")?;
        let width = entry
            .cells
            .iter()
            .map(|c| c.offset_x + 32)
            .max()
            .ok_or("No sprite cells")?
            - left;
        let height = entry
            .cells
            .iter()
            .map(|c| c.offset_y + 32)
            .max()
            .ok_or("No sprite cells")?
            - top;
        let side = width.max(height);
        let (left, top) = (left - (side - width) / 2, top - (side - height) / 2);
        let mut rgba = vec![36; 96 * 96 * 4];
        for cell in entry.cells {
            let source = self
                .sheets
                .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                .map_err(|e| e.to_string())?;
            for y in 0..96_i32 {
                let sy = top + y * side / 96 - cell.offset_y;
                if !(0..32).contains(&sy) {
                    continue;
                }
                for x in 0..96_i32 {
                    let sx = left + x * side / 96 - cell.offset_x;
                    if !(0..32).contains(&sx) {
                        continue;
                    }
                    let from = ((sy * 32 + sx) * 4) as usize;
                    let to = ((y * 96 + x) * 4) as usize;
                    let alpha = u16::from(source[from + 3]);
                    for channel in 0..3 {
                        rgba[to + channel] = ((u16::from(source[from + 2 - channel]) * alpha
                            + u16::from(rgba[to + channel]) * (255 - alpha))
                            / 255) as u8;
                    }
                    rgba[to + 3] = 255;
                }
            }
        }
        Ok(rgba)
    }
}
