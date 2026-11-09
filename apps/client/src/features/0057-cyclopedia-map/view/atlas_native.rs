//! Original Win32 atlas renderer; geometry is supplied by the admitted server.
#![allow(
    unsafe_code,
    reason = "HWND and GDI resources stay on this window thread"
)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_map::{Intent, Panel, atlas_action, atlas_detail};
use std::{
    ptr::{null, null_mut},
    sync::Arc,
};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
#[path = "atlas_native/paint.rs"]
mod paint;
#[derive(Default)]
struct Events {
    actions: Vec<usize>,
    dirty: bool,
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let events = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Events;
        if !events.is_null() {
            if msg == WM_PAINT {
                (*events).dirty = true;
            }
            if msg == WM_COMMAND
                && (w >> 16) & 65535 == BN_CLICKED as usize
                && (*events).actions.len() < 32
            {
                (*events).actions.push(w & 65535);
                return 0;
            }
            if msg == WM_LBUTTONDOWN && (*events).actions.len() < 32 {
                let x = (l & 65535) as usize;
                let y = ((l >> 16) & 65535) as usize;
                if (10..778).contains(&x) && (90..570).contains(&y) {
                    (*events)
                        .actions
                        .push(1000 + (y - 90) / 12 * 64 + (x - 10) / 12);
                }
            }
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.cyclopedia_map;
        if p.atlas_open {
            return;
        }
        p.atlas_open = true;
        p.atlas_layers = 15;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let commands = link.commands.clone();
    let rollback = weak.clone();
    if std::thread::Builder::new()
        .name("cyclopedia-atlas".into())
        .spawn(move || unsafe {
            let class = wide("OterynCyclopediaAtlas");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                lpszClassName: class.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("Cyclopedia atlas").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1080,
                790,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                if let Some(shared) = weak.upgrade() {
                    lock(&shared).properties.cyclopedia_map.atlas_open = false;
                }
                return;
            }
            let mut events = Box::<Events>::default();
            events.dirty = true;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *events as *mut Events) as isize);
            for (i, title) in [
                "Refresh",
                "Center",
                "Floor up",
                "Floor down",
                "Zoom in",
                "Zoom out",
                "North",
                "South",
                "West",
                "East",
                "NPC layer",
                "Passages",
                "POI layer",
                "Regions",
                "Prev markers",
                "Next markers",
            ]
            .iter()
            .enumerate()
            {
                control(
                    hwnd,
                    "BUTTON",
                    title,
                    100 + i,
                    [10 + (i % 8) as i32 * 130, 10 + (i / 8) as i32 * 32, 125, 28],
                    WS_TABSTOP,
                );
            }
            let legend = control(
                hwnd,
                "EDIT",
                "",
                400,
                [790, 90, 250, 480],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
            );
            let detail = control(
                hwnd,
                "EDIT",
                "",
                401,
                [10, 585, 1030, 150],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
            );
            // The tooltip owns a borrow of this buffer until the parent is destroyed.
            let mut speed_tip = wide(&oteryn_client_ui::cyclopedia_map::discovery_speed_tooltip(
                &Panel::default(),
            ));
            let tooltip = {
                use windows_sys::Win32::UI::Controls::*;
                let common = INITCOMMONCONTROLSEX {
                    dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_WIN95_CLASSES,
                };
                InitCommonControlsEx(&common);
                let tooltip = CreateWindowExW(
                    WS_EX_TOPMOST,
                    TOOLTIPS_CLASSW,
                    null(),
                    WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    hwnd,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !tooltip.is_null() {
                    let mut info: TTTOOLINFOW = std::mem::zeroed();
                    info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                    info.hwnd = hwnd;
                    info.uId = detail as usize;
                    info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                    info.lpszText = speed_tip.as_mut_ptr();
                    SendMessageW(tooltip, TTM_ADDTOOLW, 0, &info as *const _ as isize);
                    SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, 0, 400);
                }
                tooltip
            };
            let mut held: Option<Panel> = None;
            let mut msg: MSG = std::mem::zeroed();
            if let Some(shared) = weak.upgrade() {
                let mut m = lock(&shared);
                super::super::input::submit(&mut m, &commands, Intent::Atlas);
            }
            while IsWindow(hwnd) != 0 {
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let mut m = lock(&shared);
                if !m.properties.cyclopedia_map.atlas_open {
                    break;
                }
                for action in std::mem::take(&mut events.actions) {
                    let p = &mut m.properties.cyclopedia_map;
                    if action >= 1000 {
                        p.atlas_cell = action - 1000;
                    } else if let Some(action) = action.checked_sub(100)
                        && atlas_action(p, action)
                    {
                        super::super::input::submit(&mut m, &commands, Intent::Atlas);
                    }
                }
                let p = &m.properties.cyclopedia_map;
                if events.dirty || held.as_ref() != Some(p) {
                    if !tooltip.is_null() {
                        use windows_sys::Win32::UI::Controls::*;
                        let old = std::mem::replace(
                            &mut speed_tip,
                            wide(&oteryn_client_ui::cyclopedia_map::discovery_speed_tooltip(
                                p,
                            )),
                        );
                        let mut info: TTTOOLINFOW = std::mem::zeroed();
                        info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                        info.hwnd = hwnd;
                        info.uId = detail as usize;
                        info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                        info.lpszText = speed_tip.as_mut_ptr();
                        SendMessageW(tooltip, TTM_UPDATETIPTEXTW, 0, &info as *const _ as isize);
                        drop(old);
                    }
                    paint::draw(hwnd, p);
                    SetWindowTextW(legend, wide(&paint::legend(p)).as_ptr());
                    SetWindowTextW(detail, wide(&atlas_detail(p)).as_ptr());
                    held = Some(p.clone());
                    events.dirty = false;
                }
                drop(m);
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).properties.cyclopedia_map.atlas_open = false;
            }
            DestroyWindow(hwnd);
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).properties.cyclopedia_map.atlas_open = false;
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}
unsafe fn control(
    hwnd: HWND,
    class: &str,
    title: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(title).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            hwnd,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
