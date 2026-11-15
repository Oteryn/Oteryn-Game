//! Read-only native daily boost window; HWNDs stay on the creating thread.
// Win32 controls require FFI, confined to this feature's native adapter.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::{panel_text, xp_boost_lines};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points at this window thread's live refresh flag.
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 1 && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let refresh = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut bool;
            if !refresh.is_null() {
                *refresh = true;
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("daily-boost-panel".into())
        .spawn(move || {
            // SAFETY: all windows, strings and messages stay on this thread. The APIs
            // consume the UTF-16 pointers synchronously; no HWND escapes this scope.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide(tr!("Daily boost (Ctrl+B)")).as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    480,
                    260,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if hwnd.is_null() {
                    close(&weak);
                    return;
                }
                let mut refresh_requested = Box::new(false);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *refresh_requested as *mut bool) as isize);
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                let button = CreateWindowExW(0, wide("BUTTON").as_ptr(), wide("Refresh").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP, 16, 175, 120, 30, hwnd, 1 as _, null_mut(), null());
                if button.is_null() {
                    DestroyWindow(hwnd);
                    close(&weak);
                    return;
                }
                let label = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    null(),
                    WS_CHILD | WS_VISIBLE,
                    16,
                    16,
                    440,
                    150,
                    hwnd,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !label.is_null() {
                    let mut previous = String::new();
                    let mut refreshed = Instant::now();
                    let mut message: MSG = std::mem::zeroed();
                    while IsWindow(hwnd) != 0 && !commands.is_closed() {
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                            TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                        if IsWindow(hwnd) == 0 {
                            break;
                        }
                        let Some(shared) = weak.upgrade() else {
                            break;
                        };
                        let text = {
                            let mut mailbox = lock(&shared);
                            if !super::super::input::window_open(&mailbox) {
                                break;
                            }
                            // Open observations also pick up grants made while this window is visible.
                            if *refresh_requested || refreshed.elapsed() >= Duration::from_secs(5) {
                                super::super::input::refresh(&mut mailbox, &commands);
                                *refresh_requested = false;
                                refreshed = Instant::now();
                            }
                            let mut text = panel_text(&mailbox.daily_boost);
                            let potions = &mailbox.potion_durations;
                            if let (Some(snapshot), Some(at)) =
                                (&potions.snapshot, potions.received_at)
                            {
                                let elapsed =
                                    u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX);
                                text.push_str(&xp_boost_lines(&snapshot.active, elapsed));
                            }
                            text
                        };
                        if text != previous {
                            SetWindowTextW(label, wide(&text).as_ptr());
                            previous = text;
                        }
                        std::thread::sleep(Duration::from_millis(30));
                    }
                }
                if IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
                close(&weak);
            }
        })
        .is_err()
    {
        close(&rollback);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).daily_boost.open = false;
    }
}
