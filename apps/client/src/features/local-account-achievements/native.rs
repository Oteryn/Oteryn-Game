//! Account-wide achievement browser over validated server pages. HWNDs stay on their thread.
#![allow(unsafe_code)]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::account_achievements::{
    AchievementFilter, AchievementSort, Intent, catalog_next_request, catalog_rows, request,
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the window thread keeps Actions alive until the HWND is destroyed.
    unsafe {
        if m == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, m, w, l)
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[path = "view/backend.rs"]
mod backend;
use backend::Backend;
pub(in crate::play) fn open_window(link: &PlayLink) {
    open_shared(
        Arc::downgrade(&link.mailbox),
        link.commands.clone(),
        Backend::Account,
    );
}
pub(in crate::play) fn open_character(
    weak: Shared,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    let Some(shared) = weak.upgrade() else {
        return;
    };
    let generation = lock(&shared).cyclopedia_character.profile_generation;
    open_shared(weak, commands, Backend::Character(generation));
}
fn open_shared(
    weak: Shared,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    backend: Backend,
) {
    {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        let mut m = lock(&shared);
        if !backend.valid(&m) || !backend.ready(&m) || backend.panel(&mut m).open {
            return;
        }
        backend.panel(&mut m).open = true;
        if let Some(command) = backend.request(&mut m, Intent::Refresh)
            && commands.send(command).is_err()
        {
            backend.fail(&mut m);
        }
    }
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("achievement-browser".into())
        .spawn(move || run(weak, commands, backend))
        .is_err()
    {
        close(&failed, backend);
    }
}
type Shared = Weak<Mutex<Mailbox>>;
#[path = "native_localized_4.rs"]
mod native_localized_4;

use native_localized_4::run;
fn close(weak: &Shared, backend: Backend) {
    if let Some(m) = weak.upgrade() {
        let mut m = lock(&m);
        if backend.valid(&m) {
            backend.panel(&mut m).open = false;
        }
    }
}
unsafe fn ctl(h: HWND, c: &str, t: &str, id: usize, r: [i32; 4], s: u32) -> HWND {
    // SAFETY: the caller owns the live parent; Win32 copies the temporary strings.
    unsafe {
        CreateWindowExW(
            0,
            wide(c).as_ptr(),
            wide(t).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | s,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(h: HWND) -> String {
    // SAFETY: this thread owns the live edit control and the bounded output buffer.
    unsafe {
        let n = GetWindowTextLengthW(h).max(0);
        let mut b = vec![0; n as usize + 1];
        let n = GetWindowTextW(h, b.as_mut_ptr(), n + 1).max(0) as usize;
        String::from_utf16_lossy(&b[..n])
    }
}
