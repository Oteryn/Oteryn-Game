//! Uses the production asset verifier and Windows renderer. No substitute skin is drawn.
#![allow(
    unsafe_code,
    reason = "Win32 handles remain on their creating preview thread"
)]
use super::{State, layers};
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets};
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::{AtlasImage, SpriteBatch, TileBatch, TileCoord, WindowsRenderer};
use std::{
    num::NonZeroIsize,
    path::Path,
    ptr::{null, null_mut},
    sync::Arc,
};
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
#[derive(Clone, Copy)]
struct Target(NonZeroIsize);
impl HasWindowHandle for Target {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the creating thread keeps this HWND alive until the renderer is dropped.
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(self.0)))
        })
    }
}
impl HasDisplayHandle for Target {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows display handles carry no borrowed display pointer.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}
#[derive(Default)]
struct Close {
    requested: bool,
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points at this thread's live Close box through window destruction.
    unsafe {
        if message == WM_CLOSE {
            let close = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Close;
            if !close.is_null() {
                (*close).requested = true;
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
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
        let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    fn draw(&mut self, state: &State) -> Result<(AtlasImage, TileBatch, SpriteBatch), String> {
        use oteryn_renderer::world::{BLACK_CELL, WorldAtlas};
        let direction = state
            .projection
            .as_ref()
            .map_or(0, |p| u32::from(p.direction));
        let mut cells = Vec::new();
        for layer in layers(state).map_err(str::to_owned)? {
            let entry = if layer.kind == 1 {
                self.index
                    .resolve_outfit(&self.sheets, layer.appearance, direction)
            } else {
                self.index
                    .resolve(&self.sheets, layer.appearance, Placement::at(0, 0, 0))
            }
            .map_err(|e| e.to_string())?;
            cells.extend(entry.cells);
        }
        if cells.len() > 32 {
            return Err("Podium preview exceeds its sprite budget".into());
        }
        let prepared = WorldAtlas::prepare(cells.iter(), |sprite, x, y| {
            self.sheets
                .cell_rgba(sprite, x, y)
                .map_err(|e| e.to_string())
        })?;
        let slots = cells
            .iter()
            .map(|cell| {
                prepared
                    .slot(cell)
                    .map(|slot| (slot, [cell.offset_x, cell.offset_y]))
                    .ok_or("Missing verified preview sprite")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let atlas = prepared.into_image();
        let view = super::tile_view().map_err(|e| e.to_string())?;
        let tiles =
            TileBatch::from_cells(&view, &atlas, &[BLACK_CELL; 25]).map_err(|e| e.to_string())?;
        let mut sprites = SpriteBatch::new();
        for (slot, offset) in slots {
            sprites
                .push_offset(&view, &atlas, TileCoord::new(2, 2), slot, offset, 32)
                .map_err(|e| e.to_string())?;
        }
        Ok((atlas, tiles, sprites))
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.house_podium;
        if !panel.display_selected || panel.preview_open {
            return;
        }
        panel.preview_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("podium-preview".into())
        .spawn(move || {
            // SAFETY: all HWND/message accesses and renderer ownership stay on this thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Podium preview: request Preview draft or Committed view").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    360,
                    380,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if hwnd.is_null() {
                    if let Some(shared) = weak.upgrade() {
                        lock(&shared).properties.house_podium.preview_open = false;
                    }
                    return;
                }
                let mut close = Box::<Close>::default();
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *close as *mut Close) as isize);
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                let surface = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    10,
                    10,
                    320,
                    320,
                    hwnd,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                let generation = ProcessGeneration::new(1);
                let mut renderer = NonZeroIsize::new(surface as isize)
                    .and_then(|raw| WindowsRenderer::new(Target(raw), generation, 320, 320).ok());
                let mut assets = std::env::var_os(crate::map::ASSET_DIR_ENV)
                    .ok_or("The verified client asset root is not configured".to_owned())
                    .and_then(|root| Assets::load(Path::new(&root)));
                let mut held = None;
                let mut drawn = None;
                loop {
                    let mut message: MSG = std::mem::zeroed();
                    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    if close.requested || IsWindow(hwnd) == 0 || commands.is_closed() {
                        break;
                    }
                    let Some(shared) = weak.upgrade() else {
                        break;
                    };
                    let panel = lock(&shared).properties.house_podium.clone();
                    if !panel.preview_open || !panel.selected {
                        break;
                    }
                    if held != panel.preview {
                        drawn = None;
                        if let Some(state) = &panel.preview {
                            let result = match (&mut renderer, &mut assets) {
                                (Some(renderer), Ok(assets)) => {
                                    assets.draw(state).map(|(atlas, tiles, sprites)| {
                                        renderer.set_atlas(atlas);
                                        (tiles, sprites)
                                    })
                                }
                                (None, _) => Err("The production renderer could not start".into()),
                                (_, Err(error)) => Err(error.clone()),
                            };
                            match result {
                                Ok(frame) => {
                                    drawn = Some(frame);
                                    SetWindowTextW(
                                        hwnd,
                                        wide(if state.committed {
                                            "Podium preview — committed display"
                                        } else {
                                            "Podium preview — draft (not saved)"
                                        })
                                        .as_ptr(),
                                    );
                                }
                                Err(error) => {
                                    SetWindowTextW(
                                        hwnd,
                                        wide(&format!("Preview unavailable: {error}")).as_ptr(),
                                    );
                                    lock(&shared).properties.house_podium.feedback =
                                        format!("Preview unavailable: {error}");
                                }
                            }
                        }
                        if panel.preview.is_none() {
                            SetWindowTextW(
                                hwnd,
                                wide(
                                    "Podium preview — request a validated draft or committed view",
                                )
                                .as_ptr(),
                            );
                        }
                        held = panel.preview;
                    }
                    if let Some(renderer) = &mut renderer {
                        let result = if let Some((tiles, sprites)) = &drawn {
                            renderer.render_batches(generation, tiles, sprites)
                        } else {
                            renderer.render(generation)
                        };
                        if result.is_err() {
                            SetWindowTextW(
                                hwnd,
                                wide("Podium preview: renderer unavailable").as_ptr(),
                            );
                        }
                    }
                    MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
                }
                if let Some(renderer) = &mut renderer {
                    let _ = renderer.close(generation);
                }
                drop(renderer);
                if IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).properties.house_podium.preview_open = false;
            }
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).properties.house_podium.preview_open = false;
    }
}
