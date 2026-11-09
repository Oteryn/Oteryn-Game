//! Process-owned native panel geometry. Cross-thread changes are posted to HWND owners.
//! Enum callbacks are synchronous; their borrowed vectors live until enumeration returns.
use super::*;
use oteryn_client::play::{
    FIXED_PIXEL_FONT_PROPERTY, fit_ui_scale, rescale_ui_rect, scale_ui_rect,
};
use std::{collections::BTreeMap, time::Instant};
use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, ScreenToClient,
    },
    System::Threading::GetCurrentProcessId,
};
#[path = "native_ui_font.rs"]
mod native_ui_font;
use native_ui_font::UiFonts;

#[derive(Clone, Copy)]
struct Entry {
    base: [i32; 4],
    last: [i32; 4],
    observed: [i32; 4],
    scale: u16,
    tag: usize,
    font_scale: u16,
}

#[derive(Default)]
pub(super) struct UiScaler {
    entries: BTreeMap<usize, Entry>,
    fonts: UiFonts,
    next_tag: usize,
    refreshed: Option<Instant>,
    percentage: u16,
}

unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> i32 {
    // SAFETY: EnumWindows/EnumChildWindows synchronously invoke this callback on the caller.
    unsafe {
        (&mut *(data as *mut Vec<HWND>)).push(hwnd);
    }
    1
}

unsafe fn rect(hwnd: HWND, child: bool) -> Option<[i32; 4]> {
    unsafe {
        let mut rectangle: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rectangle) == 0 {
            return None;
        }
        let mut origin = POINT {
            x: rectangle.left,
            y: rectangle.top,
        };
        if child && ScreenToClient(GetParent(hwnd), &mut origin) == 0 {
            return None;
        }
        Some([
            origin.x,
            origin.y,
            rectangle.right - rectangle.left,
            rectangle.bottom - rectangle.top,
        ])
    }
}

unsafe fn work_area(hwnd: HWND) -> RECT {
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) != 0 {
            return info.rcWork;
        }
        RECT {
            left: 0,
            top: 0,
            right: GetSystemMetrics(SM_CXSCREEN),
            bottom: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

unsafe fn panel_class(hwnd: HWND) -> bool {
    unsafe {
        let mut name = [0u16; 128];
        let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
        let name = String::from_utf16_lossy(&name[..length.max(0) as usize]);
        name.eq_ignore_ascii_case("static") || name.to_ascii_lowercase().starts_with("oteryn")
    }
}

impl UiScaler {
    pub(super) fn font(&mut self, percentage: u16) -> Option<usize> {
        self.fonts.get(percentage)
    }

    unsafe fn visit(
        &mut self,
        hwnd: HWND,
        requested: u16,
        root: bool,
        geometry: bool,
    ) -> Option<u16> {
        unsafe {
            let mut owner = 0;
            GetWindowThreadProcessId(hwnd, &mut owner);
            if owner != GetCurrentProcessId() {
                return None;
            }
            if IsWindowVisible(hwnd) == 0 {
                return None;
            }
            let actual = rect(hwnd, !root)?;
            let key = wide("Oteryn.UiScale.v1");
            let previous = self
                .entries
                .get(&(hwnd as usize))
                .copied()
                .filter(|entry| GetPropW(hwnd, key.as_ptr()) as usize == entry.tag);
            let mut entry = if let Some(entry) = previous {
                entry
            } else {
                self.next_tag = self.next_tag.wrapping_add(1).max(1);
                if SetPropW(hwnd, key.as_ptr(), self.next_tag as *mut _) == 0 {
                    return None;
                }
                Entry {
                    base: actual,
                    last: actual,
                    observed: actual,
                    scale: 100,
                    tag: self.next_tag,
                    font_scale: 0,
                }
            };
            if geometry && actual != entry.last && actual != entry.observed {
                // Child MoveWindow calls use logical pixels; a root resize is a user's physical size.
                entry.base = if root {
                    rescale_ui_rect(actual, entry.scale, 100)
                } else {
                    actual
                };
            }
            let effective = if root && geometry {
                let work = work_area(hwnd);
                fit_ui_scale(
                    requested,
                    [entry.base[2], entry.base[3]],
                    [work.right - work.left, work.bottom - work.top],
                )
            } else {
                requested
            };
            if geometry {
                let mut desired = scale_ui_rect(entry.base, effective);
                if root {
                    let work = work_area(hwnd);
                    desired[0] =
                        actual[0].clamp(work.left, (work.right - desired[2]).max(work.left));
                    desired[1] =
                        actual[1].clamp(work.top, (work.bottom - desired[3]).max(work.top));
                }
                if actual != desired {
                    SetWindowPos(
                        hwnd,
                        null_mut(),
                        desired[0],
                        desired[1],
                        desired[2],
                        desired[3],
                        SWP_NOACTIVATE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS,
                    );
                }
                entry.last = desired;
            }
            if entry.font_scale != effective
                && GetPropW(hwnd, wide(FIXED_PIXEL_FONT_PROPERTY).as_ptr()).is_null()
                && let Some(font) = self.fonts.get(effective)
            {
                PostMessageW(hwnd, WM_SETFONT, font, 1);
                entry.font_scale = effective;
            }
            entry.observed = actual;
            entry.scale = effective;
            self.entries.insert(hwnd as usize, entry);
            Some(effective)
        }
    }

    pub(super) fn refresh(&mut self, parent: HWND, percentage: u16) {
        if parent.is_null() {
            return;
        }
        let now = Instant::now();
        if self.percentage == percentage
            && self.refreshed.is_some_and(|last| {
                now.saturating_duration_since(last) < Duration::from_millis(250)
            })
        {
            return;
        }
        self.percentage = percentage;
        self.refreshed = Some(now);
        unsafe {
            let key = wide("Oteryn.UiScale.v1");
            self.entries.retain(|handle, entry| {
                IsWindow(*handle as HWND) != 0
                    && GetPropW(*handle as HWND, key.as_ptr()) as usize == entry.tag
            });
            let mut roots = Vec::<HWND>::new();
            EnumWindows(Some(collect), &mut roots as *mut _ as isize);
            let process = GetCurrentProcessId();
            for root in roots {
                let mut owner = 0;
                GetWindowThreadProcessId(root, &mut owner);
                if owner != process
                    || root == parent
                    || IsIconic(root) != 0
                    || !panel_class(root)
                    || GetWindowLongPtrW(root, GWL_STYLE) as u32 & WS_CAPTION != WS_CAPTION
                {
                    continue;
                }
                let Some(effective) = self.visit(root, percentage, true, true) else {
                    continue;
                };
                let mut children = Vec::<HWND>::new();
                EnumChildWindows(root, Some(collect), &mut children as *mut _ as isize);
                for child in children {
                    // Native composite controls lay out their own internal edit/list/header windows.
                    let geometry = GetParent(child) == root || panel_class(GetParent(child));
                    self.visit(child, effective, false, geometry);
                }
            }
            // Main-window HUD geometry already uses preferences; scale its native typography only.
            let mut children = Vec::<HWND>::new();
            EnumChildWindows(parent, Some(collect), &mut children as *mut _ as isize);
            for child in children {
                self.visit(child, percentage, false, false);
            }
        }
    }
}
