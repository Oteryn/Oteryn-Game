//! Named controls select confirmed IDs before invoking the existing progression caller.
#![allow(
    unsafe_code,
    reason = "Native controls and message userdata stay on their window thread"
)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::charm_progression::Intent;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
#[derive(Default)]
struct Actions(Vec<usize>);
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
unsafe fn control(h: HWND, class: &str, label: &str, id: usize, r: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.charm_progression;
        if !p.selected || p.selector.open {
            return;
        }
        p.selector.open = true;
        p.open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("charm-selector".into())
        .spawn(move || dialog::run(weak, commands))
        .is_err()
        && let Some(shared) = failed.upgrade()
    {
        let mut m = lock(&shared);
        m.properties.charm_progression.selector.open = false;
        m.properties.charm_progression.open = false;
    }
}
unsafe fn populate(h: HWND, rows: &[(u32, String)]) {
    unsafe {
        SendMessageW(h, CB_RESETCONTENT, 0, 0);
        for (_, label) in rows {
            SendMessageW(h, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
    }
}

#[path = "selector_dialog.rs"]
mod dialog;
