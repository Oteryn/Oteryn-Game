//! Party portraits use the same pinned assets and colour/addon compositor as Appearance.
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
use oteryn_client_ui::party::party::PartyListEntity;

pub(super) struct Image {
    pub rgba: Vec<u8>,
}
pub(super) struct PreviewAssets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}
#[path = "../0025-appearance/view/preview_pixels.rs"]
mod pixels;
impl PreviewAssets {
    pub(super) fn load() -> Option<Self> {
        let root = std::path::PathBuf::from(std::env::var_os(crate::map::ASSET_DIR_ENV)?);
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(oteryn_client_assets::world::MANIFEST_PATH),
            oteryn_client_assets::world::MANIFEST_SHA256,
        )
        .ok()?;
        let catalog = Catalog::load(&store).ok()?;
        let index = AppearanceIndex::load(&store, &catalog).ok()?;
        Some(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    pub(super) fn image(&mut self, row: &PartyListEntity) -> Option<Image> {
        let oteryn_session::Look::Outfit {
            look_type, addons, ..
        } = row.portrait.as_ref()?.look
        else {
            return None;
        };
        let mut image = Image {
            rgba: [24, 24, 24, 255].repeat(128 * 128),
        };
        self.draw_look(
            look_type,
            addons,
            Some(row.portrait_colours),
            false,
            &mut image,
        )
        .ok()?;
        for pixel in image.rgba.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        Some(image)
    }
}
