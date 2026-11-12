//! Win32 player-typing controls and live indicator rendering.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
const ON: usize = 101;
const OFF: usize = 102;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
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
        let mut m = lock(&link.mailbox);
        if m.player_typing.open {
            return;
        }
        m.player_typing.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("player-typing-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}
fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Player typing")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            360,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        control(hwnd, "BUTTON", tr!("Typing on"), ON, [10, 10, 130, 30]);
        control(hwnd, "BUTTON", tr!("Typing off"), OFF, [150, 10, 130, 30]);
        let state = control(hwnd, "EDIT", "", 103, [10, 50, 580, 250]);
        let mut held = String::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.player_typing_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if matches!(action, ON | OFF) {
                    super::super::input::enqueue(&mut mailbox, &commands, action == ON);
                }
            }
            let text = oteryn_client_ui::player_typing::panel_text(&mailbox.player_typing);
            if text != held {
                SetWindowTextW(state, wide(&text).as_ptr());
                held = text;
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(m) = weak.upgrade() {
        lock(&m).player_typing.open = false;
    }
}
unsafe fn control(parent: HWND, class: &str, text: &str, id: usize, b: [i32; 4]) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            b[0],
            b[1],
            b[2],
            b[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
