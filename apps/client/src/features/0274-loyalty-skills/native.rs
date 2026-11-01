//! Read-only Skills window using the existing Quest Log window ownership pattern.
#![allow(
    unsafe_code,
    reason = "Win32 controls stay on their creating UI thread"
)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::loyalty_skills::{panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions {
    refresh: bool,
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the creating thread keeps Actions alive until after DestroyWindow.
    unsafe {
        if message == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize && w & 0xffff == 1 {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() {
                (*actions).refresh = true;
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

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.loyalty_skills.open {
            return;
        }
        mailbox.loyalty_skills.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("loyalty-skills".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).loyalty_skills.open = false;
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: every HWND and userdata access is confined to this thread.
    // Win32 copies the title strings; the Actions allocation outlives the window.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Skills — Ctrl+S").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            520,
            440,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let text = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE,
            16,
            16,
            470,
            330,
            hwnd,
            null_mut(),
            null_mut(),
            null(),
        );
        let refresh = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("Refresh").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            16,
            355,
            120,
            32,
            hwnd,
            1 as _,
            null_mut(),
            null(),
        );
        if text.is_null() || refresh.is_null() {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held = None;
        loop {
            let mut message: MSG = std::mem::zeroed();
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
            let mut mailbox = lock(&shared);
            if mailbox.quest_shutdown {
                break;
            }
            if std::mem::take(&mut actions.refresh)
                && let Some(payload) = request(&mut mailbox.loyalty_skills)
                && commands.send(PlayCommand::LoyaltySkills(payload)).is_err()
            {
                mailbox.loyalty_skills.pending = false;
                mailbox.loyalty_skills.error = Some("Session ended");
            }
            let panel = &mailbox.loyalty_skills;
            if held.as_ref() != Some(panel) {
                SetWindowTextW(text, wide(&panel_text(panel)).as_ptr());
                EnableWindow(refresh, i32::from(panel.selected && !panel.pending));
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).loyalty_skills.open = false;
    }
}
