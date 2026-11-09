//! Preview only: pinned pixels from server-resolved public looks, never local grants.
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets};
use oteryn_client_runtime::cui3::store::browse::Artwork;
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
    pub(super) fn render(
        &mut self,
        art: Artwork,
        gender: usize,
        direction: u8,
    ) -> Result<Vec<u8>, String> {
        if !(1..=3).contains(&art.kind) || gender > 1 || direction > 3 {
            return Err("This service has no item or character model".into());
        }
        let look = art.looks[gender];
        let entry = if art.kind == 1 {
            self.index
                .resolve(&self.sheets, look, Placement::at(0, 0, -7))
        } else {
            self.index
                .resolve_outfit(&self.sheets, look, u32::from(direction))
        }
        .map_err(|e| e.to_string())?;
        if entry.cells.len() > 64 {
            return Err("Artwork exceeds the preview bound".into());
        }
        let mut image = [32, 32, 32, 255].repeat(128 * 128);
        for cell in entry.cells {
            let rgba = self
                .sheets
                .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                .map_err(|e| e.to_string())?;
            for y in 0..32i32 {
                for x in 0..32i32 {
                    let (dx, dy) = (64 + cell.offset_x + x, 64 + cell.offset_y + y);
                    if !(0..128).contains(&dx) || !(0..128).contains(&dy) {
                        continue;
                    }
                    let s = ((y * 32 + x) * 4) as usize;
                    let t = ((dy * 128 + dx) * 4) as usize;
                    let alpha = u32::from(rgba[s + 3]);
                    for c in 0..3 {
                        image[t + c] = ((u32::from(rgba[s + c]) * alpha
                            + u32::from(image[t + c]) * (255 - alpha))
                            / 255) as u8;
                    }
                }
            }
        }
        Ok(image)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_preview_renders_real_item_outfit_and_mount_sprites()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut assets = Assets::load(&root)?;
        for art in [
            Artwork {
                kind: 1,
                looks: [28552; 2],
            },
            Artwork {
                kind: 2,
                looks: [128, 136],
            },
            Artwork {
                kind: 3,
                looks: [368; 2],
            },
        ] {
            for gender in 0..2 {
                for direction in 0..4 {
                    let image = assets.render(art, gender, direction)?;
                    assert_eq!(image.len(), 128 * 128 * 4);
                    assert!(
                        image
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .any(|p| *p != [32, 32, 32, 255])
                    );
                }
            }
        }
        let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("rulesets/economy/store/catalogue.json"),
        )?)?;
        for offer in catalogue["offers"].as_array().ok_or("offers")? {
            if let Some(key) = offer["item"]["key"].as_str() {
                let look = key.strip_prefix("oteryn:item.tibia.i").ok_or("item identity")?.parse()?;
                let image = assets.render(Artwork { kind: 1, looks: [look; 2] }, 0, 0)?;
                assert!(image.as_chunks::<4>().0.iter().any(|p| *p != [32, 32, 32, 255]), "{key}");
            }
        }
        assert!(assets.render(Artwork::default(), 0, 0).is_err());
        Ok(())
    }
}
