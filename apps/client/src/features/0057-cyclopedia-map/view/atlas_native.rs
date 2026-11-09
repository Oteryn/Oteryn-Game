//! Large atlas viewport. The thread owns every HWND and GDI resource.
#![allow(
    unsafe_code,
    reason = "Win32 atlas resources stay on their owning thread"
)]
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !state.is_null() && (*state).0.len() < 16 {
                (*state).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.properties.cyclopedia_map.atlas_open {
            return;
        }
        mailbox.properties.cyclopedia_map.atlas_open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("cyclopedia-atlas".into())
        .spawn(move || unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Cyclopedia — Region atlas").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                80,
                80,
                1200,
                850,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if !hwnd.is_null() {
                let mut actions = Box::<Actions>::default();
                SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    actions.as_mut() as *mut Actions as isize,
                );
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                for (i, label) in [
                    "Load atlas",
                    "Next atlas page",
                    "Previous floor",
                    "Next floor",
                    "Toggle POIs",
                ]
                .iter()
                .enumerate()
                {
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(label).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        12 + i as i32 * 155,
                        10,
                        150,
                        30,
                        hwnd,
                        (100 + i) as _,
                        null_mut(),
                        null(),
                    );
                }
                let mut msg: MSG = std::mem::zeroed();
                while IsWindow(hwnd) != 0 && !commands.is_closed() {
                    while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    if IsWindow(hwnd) == 0 {
                        break;
                    }
                    let Some(shared) = weak.upgrade() else {
                        break;
                    };
                    let panel = {
                        let mut mailbox = lock(&shared);
                        for action in actions.0.drain(..) {
                            match action {
                                100 | 101 => super::super::input::submit(
                                    &mut mailbox,
                                    &commands,
                                    if action == 100 {
                                        oteryn_client_ui::cyclopedia_map::Intent::Atlas
                                    } else {
                                        oteryn_client_ui::cyclopedia_map::Intent::AtlasNext
                                    },
                                ),
                                102 | 103 => mailbox
                                    .properties
                                    .cyclopedia_map
                                    .select_floor(action == 102),
                                104 => {
                                    let panel = &mut mailbox.properties.cyclopedia_map;
                                    panel.hide_pois = !panel.hide_pois;
                                }
                                _ => {}
                            }
                        }
                        mailbox.properties.cyclopedia_map.clone()
                    };
                    paint(hwnd, &panel);
                    std::thread::sleep(std::time::Duration::from_millis(80));
                }
                if IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).properties.cyclopedia_map.atlas_open = false;
            }
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).properties.cyclopedia_map.atlas_open = false;
    }
}
unsafe fn label(dc: HDC, x: i32, y: i32, text: &str) {
    unsafe {
        let text = wide(text);
        TextOutW(dc, x, y, text.as_ptr(), text.len().saturating_sub(1) as i32);
    }
}
unsafe fn fill(dc: HDC, rect: RECT, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        if !brush.is_null() {
            FillRect(dc, &rect, brush);
            DeleteObject(brush);
        }
    }
}
unsafe fn paint(hwnd: HWND, panel: &oteryn_client_ui::cyclopedia_map::Panel) {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return;
        }
        fill(
            dc,
            RECT {
                left: 0,
                top: 46,
                right: rect.right,
                bottom: rect.bottom,
            },
            0x181818,
        );
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0xeeeeee);
        label(
            dc,
            12,
            50,
            &format!("Floor {} • north up • {}", panel.floor, panel.feedback),
        );
        let width = rect.right.saturating_sub(320).max(1) as u32;
        let height = rect.bottom.saturating_sub(110).max(1) as u32;
        for run in oteryn_client_ui::cyclopedia_map::atlas_runs(panel, width, height) {
            let color = color(run.area);
            fill(
                dc,
                RECT {
                    left: 12 + run.left as i32,
                    top: 85 + run.top as i32,
                    right: 12 + run.right.max(run.left + 1) as i32,
                    bottom: 85 + run.bottom.max(run.top + 1) as i32,
                },
                color,
            );
        }
        for poi in oteryn_client_ui::cyclopedia_map::atlas_pois(panel, width, height) {
            let x = 12 + poi.column as i32;
            let y = 85 + poi.row as i32;
            fill(
                dc,
                RECT {
                    left: x - 3,
                    top: y - 3,
                    right: x + 4,
                    bottom: y + 4,
                },
                0x00ffff,
            );
        }
        if let Some(atlas) = &panel.region_atlas {
            let ids: std::collections::BTreeSet<_> = atlas
                .runs
                .iter()
                .filter(|r| r.floor == panel.floor)
                .map(|r| r.area)
                .collect();
            let mut y = 85;
            for area in atlas.areas.iter().filter(|a| ids.contains(&a.id)) {
                fill(
                    dc,
                    RECT {
                        left: rect.right - 285,
                        top: y,
                        right: rect.right - 270,
                        bottom: y + 15,
                    },
                    color(area.id),
                );
                label(dc, rect.right - 263, y, &area.name);
                y += 24;
                if y > rect.bottom - 60 {
                    break;
                }
            }
            label(
                dc,
                rect.right - 285,
                rect.bottom - 40,
                if atlas.next.is_some() {
                    "More atlas pages available"
                } else {
                    "Complete admitted coverage"
                },
            );
        } else {
            label(
                dc,
                30,
                100,
                "Load atlas to display the server's named regions and subareas.",
            );
        }
        ReleaseDC(hwnd, dc);
    }
}
fn color(area: u32) -> u32 {
    let n = area.wrapping_mul(0x9e3779b9);
    (0x40 + (n & 0x7f)) | ((0x40 + ((n >> 8) & 0x7f)) << 8) | ((0x40 + ((n >> 16) & 0x7f)) << 16)
}
