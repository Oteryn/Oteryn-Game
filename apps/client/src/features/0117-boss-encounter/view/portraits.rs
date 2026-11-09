//! Use the same hash-pinned appearance/sheet owner as the world renderer.
use oteryn_client_assets::DrawCell;
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets};
use std::collections::BTreeMap;
pub(super) struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}
pub(super) struct Portraits {
    source: Option<(AppearanceIndex, SpriteSheets)>,
    cache: BTreeMap<(u32, bool), Option<Image>>,
}
impl Portraits {
    pub fn new() -> Self {
        Self {
            source: std::env::var_os(crate::map::ASSET_DIR_ENV).and_then(|root| {
                let root = std::path::PathBuf::from(root);
                let store = AssetStore::open_pinned(
                    &root.join("content/assets/files"),
                    &root.join(oteryn_client_assets::world::MANIFEST_PATH),
                    oteryn_client_assets::world::MANIFEST_SHA256,
                )
                .ok()?;
                let catalog = Catalog::load(&store).ok()?;
                let index = AppearanceIndex::load(&store, &catalog).ok()?;
                Some((index, SpriteSheets::new(store, catalog)))
            }),
            cache: BTreeMap::new(),
        }
    }
    pub fn image(&mut self, look: u32, object: bool) -> Option<&Image> {
        if look == 0 {
            return None;
        }
        if !self.cache.contains_key(&(look, object)) {
            if self.cache.len() >= 64 {
                self.cache.clear();
            }
            let image = self.source.as_mut().and_then(|(index, sheets)| {
                let entry = if object {
                    index.resolve(sheets, look, Placement::at(0, 0, -7)).ok()?
                } else {
                    index.resolve_outfit(sheets, look, 2).ok()?
                };
                compose(&entry.cells, |cell| {
                    sheets
                        .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                        .ok()
                })
            });
            self.cache.insert((look, object), image);
        }
        self.cache.get(&(look, object))?.as_ref()
    }
}
fn compose(
    cells: &[DrawCell],
    mut load: impl FnMut(&DrawCell) -> Option<Vec<u8>>,
) -> Option<Image> {
    if cells.is_empty() || cells.len() > oteryn_client_assets::MAX_ENTRY_CELLS {
        return None;
    }
    let left = cells.iter().map(|c| c.offset_x).min()?;
    let top = cells.iter().map(|c| c.offset_y).min()?;
    let width = u32::try_from(
        cells
            .iter()
            .map(|c| c.offset_x)
            .max()?
            .checked_sub(left)?
            .checked_add(32)?,
    )
    .ok()?;
    let height = u32::try_from(
        cells
            .iter()
            .map(|c| c.offset_y)
            .max()?
            .checked_sub(top)?
            .checked_add(32)?,
    )
    .ok()?;
    if width > 256 || height > 256 {
        return None;
    }
    let mut bgra = vec![0x18; (width * height * 4) as usize];
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    for cell in cells {
        let rgba = load(cell)?;
        if rgba.len() != 32 * 32 * 4 {
            return None;
        }
        for y in 0..32 {
            for x in 0..32 {
                let src = (y * 32 + x) * 4;
                let dx = usize::try_from(cell.offset_x - left).ok()? + x;
                let dy = usize::try_from(cell.offset_y - top).ok()? + y;
                let dst = (dy * width as usize + dx) * 4;
                let alpha = u32::from(rgba[src + 3]);
                for (target, source) in [(0, 2), (1, 1), (2, 0)] {
                    bgra[dst + target] = ((u32::from(rgba[src + source]) * alpha
                        + u32::from(bgra[dst + target]) * (255 - alpha))
                        / 255) as u8;
                }
            }
        }
    }
    Some(Image {
        width,
        height,
        bgra,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boss_portrait_without_a_server_appearance_never_draws_a_sprite() {
        assert!(Portraits::new().image(0, false).is_none());
    }
    #[test]
    fn boss_portrait_composes_actual_cell_offsets_layers_and_alpha_in_bgra() {
        let cells = [
            DrawCell {
                sprite_id: 1,
                cell_x: 0,
                cell_y: 0,
                offset_x: -32,
                offset_y: -32,
            },
            DrawCell {
                sprite_id: 2,
                cell_x: 0,
                cell_y: 0,
                offset_x: 0,
                offset_y: 0,
            },
        ];
        let image = compose(&cells, |cell| {
            Some(if cell.sprite_id == 1 {
                [255, 0, 0, 255].repeat(1024)
            } else {
                [0, 255, 0, 128].repeat(1024)
            })
        });
        assert!(image.is_some_and(|i| i.width == 64
            && i.height == 64
            && i.bgra[..4] == [0, 0, 255, 255]
            && i.bgra[(63 * 64 + 63) * 4 + 1] > 128));
        assert!(compose(&cells, |_| Some(vec![0; 4])).is_none());
    }
}
