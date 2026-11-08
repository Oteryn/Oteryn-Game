//! Screen-space bridge across the three native windows. Only the shell polls button edges.
// SAFETY: window handles are observed as integers; all control messages run on their owner thread.
#![allow(unsafe_code)]
use crate::play::{
    PlayLink, PlayView,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::inventory::movement::DragItem;
use oteryn_session::{ItemEntry, ItemMoveDestination, item_movement::WorldTilePosition};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    System::Threading::GetCurrentProcessId,
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::*,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

#[path = "count.rs"]
mod count;

#[derive(Debug, Clone)]
struct Surface {
    hwnd: usize,
    bounds: [i32; 4],
    rows: Vec<([i32; 4], DragItem)>,
    destination: ItemMoveDestination,
}
#[derive(Debug, Default)]
pub(in crate::play) struct DragNative {
    surfaces: Vec<Surface>,
    down: bool,
    primary_button: u16,
    escape_down: bool,
    start: Option<(i32, i32)>,
    prompt_open: bool,
}
fn contains(bounds: [i32; 4], point: POINT) -> bool {
    point.x >= bounds[0] && point.y >= bounds[1] && point.x < bounds[2] && point.y < bounds[3]
}
/// Called on the list's creating thread, after its displayed snapshot changes.
pub(in crate::play) fn list_surface(
    mailbox: &mut Mailbox,
    hwnd: HWND,
    rows: &[ItemEntry],
    destination: ItemMoveDestination,
) {
    unsafe {
        let mut bounds: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut bounds) == 0 {
            return;
        }
        let rows = rows
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let mut row: RECT = std::mem::zeroed();
                if SendMessageW(hwnd, LB_GETITEMRECT, i, (&mut row as *mut RECT) as isize)
                    == LB_ERR as isize
                {
                    return None;
                }
                Some((
                    [
                        bounds.left + row.left + 1,
                        bounds.top + row.top + 1,
                        bounds.left + row.right + 1,
                        bounds.top + row.bottom + 1,
                    ],
                    DragItem {
                        equipped: matches!(destination, ItemMoveDestination::Equipment(_)),
                        handle: item.handle,
                        count: item.count.get(),
                    },
                ))
            })
            .collect();
        let surface = Surface {
            hwnd: hwnd as usize,
            bounds: [bounds.left, bounds.top, bounds.right, bounds.bottom],
            rows,
            destination,
        };
        let surfaces = &mut mailbox.item_drag_native.surfaces;
        surfaces.retain(|s| IsWindow(s.hwnd as HWND) != 0 && s.hwnd != surface.hwnd);
        surfaces.push(surface);
    }
}
impl PlayLink {
    /// Called by the production shell between native input and redraw, including while a panel has focus.
    pub fn item_drag_tick(&self, view: &PlayView, window: &Window) {
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let main = handle.hwnd.get() as HWND;
        let Ok(origin) = window.inner_position() else {
            return;
        };
        let mut point = POINT { x: 0, y: 0 };
        // No capture is needed: screen polling spans all three native top-level windows.
        let (down, escape, under, primary_button) = unsafe {
            if GetCursorPos(&mut point) == 0 {
                return;
            }
            let under = WindowFromPoint(point);
            let primary_button = {
                let mailbox = lock(&self.mailbox);
                if mailbox.item_drag_native.down {
                    mailbox.item_drag_native.primary_button
                } else if view
                    .preferences()
                    .controls
                    .drag_button(GetAncestor(under, GA_ROOT) == main)
                    == oteryn_input_actions::MouseButton::SECONDARY
                {
                    VK_RBUTTON
                } else {
                    VK_LBUTTON
                }
            };
            (
                GetAsyncKeyState(i32::from(primary_button)) < 0,
                GetAsyncKeyState(i32::from(VK_ESCAPE)) < 0,
                under,
                primary_button,
            )
        };
        let tile = (unsafe { GetAncestor(under, GA_ROOT) } == main)
            .then(|| {
                crate::input::click_tile(
                    view.scene().view(),
                    f64::from(point.x - origin.x),
                    f64::from(point.y - origin.y),
                )
            })
            .flatten();
        let (source, destination, changed, start, prompt_open, cancel) = {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.inventory_shutdown {
                return;
            }
            let native = &mut mailbox.item_drag_native;
            native
                .surfaces
                .retain(|s| unsafe { IsWindow(s.hwnd as HWND) } != 0);
            let escape_pressed = escape && !native.escape_down;
            native.escape_down = escape;
            let foreground = unsafe { GetForegroundWindow() };
            let mut foreground_process = 0;
            unsafe {
                GetWindowThreadProcessId(foreground, &mut foreground_process);
            }
            let client_foreground = foreground_process == unsafe { GetCurrentProcessId() };
            let cancel = escape_pressed && client_foreground;
            let surface = native
                .surfaces
                .iter()
                .rev()
                .find(|s| s.hwnd == under as usize && contains(s.bounds, point));
            let source = surface
                .and_then(|s| {
                    s.rows
                        .iter()
                        .find(|(r, _)| contains(*r, point))
                        .map(|(_, item)| *item)
                })
                .or_else(|| tile.and_then(|tile| view.item_drag_source(tile)));
            let destination = surface.map(|s| s.destination).or_else(|| {
                tile.map(|tile| {
                    ItemMoveDestination::Ground(WorldTilePosition {
                        x: tile.x,
                        y: tile.y,
                        floor: view.item_floor(),
                    })
                })
            });
            let changed = native.down != down;
            native.down = down;
            let start = native.start;
            if changed && down {
                native.primary_button = primary_button;
                native.start = Some((point.x, point.y));
            }
            if !down {
                native.start = None;
            }
            (
                source,
                destination,
                changed,
                start,
                native.prompt_open,
                cancel,
            )
        };
        let state = self.item_movement();
        if cancel && (state.dragged.is_some() || state.prompt.is_some()) {
            self.item_move_confirm(None);
        } else if changed
            && down
            && let Some(source) = source
        {
            self.item_drag_begin(source);
        } else if changed && !down && self.item_movement().dragged.is_some() {
            let moved =
                start.is_some_and(|(x, y)| x.abs_diff(point.x) >= 4 || y.abs_diff(point.y) >= 4);
            if moved {
                self.item_drag_drop(destination);
            } else {
                self.item_drag_abort();
            }
        }
        let state = self.item_movement();
        if let Some((item, _)) = state.prompt
            && !prompt_open
        {
            lock(&self.mailbox).item_drag_native.prompt_open = true;
            count::open(self, item.count.min(100));
        }
    }
}
/// Every open-view header is a drop destination, so another container need not be active.
pub(in crate::play) fn view_surfaces(
    mailbox: &mut Mailbox,
    hwnd: HWND,
    panel: &oteryn_client_ui::container_tree::ContainerPanel,
) {
    unsafe {
        let mut bounds: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut bounds) == 0 {
            return;
        }
        mailbox
            .item_drag_native
            .surfaces
            .retain(|s| s.hwnd != hwnd as usize);
        if let Some(views) = &panel.views {
            for (i, view) in views.views.iter().enumerate() {
                let mut row: RECT = std::mem::zeroed();
                if SendMessageW(hwnd, LB_GETITEMRECT, i, (&mut row as *mut RECT) as isize)
                    == LB_ERR as isize
                {
                    continue;
                }
                mailbox.item_drag_native.surfaces.push(Surface {
                    hwnd: hwnd as usize,
                    bounds: [
                        bounds.left + row.left + 1,
                        bounds.top + row.top + 1,
                        bounds.left + row.right + 1,
                        bounds.top + row.bottom + 1,
                    ],
                    rows: vec![],
                    destination: ItemMoveDestination::Container(view.container_handle),
                });
            }
        }
    }
}

pub(in crate::play) fn clear_surfaces(mailbox: &mut Mailbox, handles: &[HWND]) {
    mailbox
        .item_drag_native
        .surfaces
        .retain(|s| !handles.iter().any(|h| *h as usize == s.hwnd));
}
