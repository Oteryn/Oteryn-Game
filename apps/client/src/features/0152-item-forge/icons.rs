//! Sprite pixels are resolved through the production SHA-pinned asset store.
#![allow(
    unsafe_code,
    reason = "HWND lifetime is confined to its renderer thread"
)]
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets};
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::{AtlasImage, SpriteBatch, TileBatch, TileCoord, WindowsRenderer};
use std::{num::NonZeroIsize, path::Path};
use windows_sys::Win32::Foundation::HWND;
use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
#[derive(Clone, Copy)]
struct Target(NonZeroIsize);
impl HasWindowHandle for Target {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: creating thread retains the surface until after renderer destruction.
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(self.0)))
        })
    }
}
impl HasDisplayHandle for Target {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows display has no borrowed pointer.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}
struct Assets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}
impl Assets {
    fn load(root: &Path) -> Result<Self, String> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|e| e.to_string())?;
        let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
        Ok(Self {
            index: AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    fn frame(&mut self, appearance: u32) -> Result<(AtlasImage, TileBatch, SpriteBatch), String> {
        use oteryn_renderer::world::{BLACK_CELL, WorldAtlas};
        if appearance == 0 {
            return Err("No verified item appearance is bound".into());
        }
        let entry = self
            .index
            .resolve(&self.sheets, appearance, Placement::at(0, 0, 0))
            .map_err(|e| e.to_string())?;
        if entry.cells.is_empty() || entry.cells.len() > 32 {
            return Err("Item sprite exceeds its preview budget".into());
        }
        let prepared = WorldAtlas::prepare(entry.cells.iter(), |sprite, x, y| {
            self.sheets
                .cell_rgba(sprite, x, y)
                .map_err(|e| e.to_string())
        })?;
        let slots = entry
            .cells
            .iter()
            .map(|cell| {
                prepared
                    .slot(cell)
                    .map(|slot| (slot, [cell.offset_x, cell.offset_y]))
                    .ok_or("Missing verified sprite")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let atlas = prepared.into_image();
        let view = super::super::preview_grid::item_preview_view().map_err(|e| e.to_string())?;
        let tiles =
            TileBatch::from_cells(&view, &atlas, &[BLACK_CELL; 16]).map_err(|e| e.to_string())?;
        let mut sprites = SpriteBatch::new();
        for (slot, offset) in slots {
            sprites
                .push_offset(&view, &atlas, TileCoord::new(2, 2), slot, offset, 32)
                .map_err(|e| e.to_string())?;
        }
        Ok((atlas, tiles, sprites))
    }
}
pub(super) struct Icon {
    renderer: Option<WindowsRenderer<Target>>,
    assets: Result<Assets, String>,
    frame: Option<(TileBatch, SpriteBatch)>,
}
impl Icon {
    pub(super) unsafe fn new(parent: HWND, bounds: [i32; 4]) -> Self {
        // SAFETY: surface belongs to calling window thread and is destroyed by its parent.
        let surface = unsafe { super::control(parent, "STATIC", "", 0, bounds, 0) };
        let renderer = NonZeroIsize::new(surface as isize).and_then(|raw| {
            WindowsRenderer::new(Target(raw), ProcessGeneration::new(1), 128, 128).ok()
        });
        let assets = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .ok_or("Verified client asset root is not configured".into())
            .and_then(|root| Assets::load(Path::new(&root)));
        Self {
            renderer,
            assets,
            frame: None,
        }
    }
    pub(super) fn select(&mut self, appearance: Option<u32>) -> Result<(), String> {
        self.frame = None;
        let Some(appearance) = appearance else {
            return Ok(());
        };
        let renderer = self
            .renderer
            .as_mut()
            .ok_or("Production renderer unavailable")?;
        let assets = self.assets.as_mut().map_err(|e| e.clone())?;
        let (atlas, tiles, sprites) = assets.frame(appearance)?;
        renderer.set_atlas(atlas);
        self.frame = Some((tiles, sprites));
        Ok(())
    }
    pub(super) fn render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let generation = ProcessGeneration::new(1);
            let _ = if let Some((tiles, sprites)) = &self.frame {
                renderer.render_batches(generation, tiles, sprites)
            } else {
                renderer.render(generation)
            };
        }
    }
}
impl Drop for Icon {
    fn drop(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let _ = renderer.close(ProcessGeneration::new(1));
        }
    }
}
