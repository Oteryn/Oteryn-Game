//! Minimal native Task Board list and action buttons.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::task_board::Intent;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const LIST: usize = 101;
const REFRESH: usize = 102;
const SELECT: usize = 103;
const CLAIM: usize = 104;
const UPGRADE: usize = 105;
#[derive(Default)]
struct Actions(Vec<usize>);
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.task_board.open {
            return;
        }
        mailbox.task_board.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("task-board-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn run(
    weak: WeakMailbox,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Task Board").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            480,
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
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            LIST,
            [10, 10, 580, 300],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        for (id, label, x) in [
            (REFRESH, "Refresh", 10),
            (SELECT, "Select task", 140),
            (CLAIM, "Claim task", 290),
            (UPGRADE, "Upgrade talisman", 430),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 325, 140, 30], WS_TABSTOP);
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
            for id in std::mem::take(&mut actions.0) {
                let row = u8::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|v| v.checked_add(1));
                let intent = match id {
                    REFRESH => Some(Intent::Refresh),
                    SELECT => row.map(Intent::Select),
                    CLAIM => row.map(Intent::Claim),
                    UPGRADE => row.filter(|v| *v <= 4).map(Intent::UpgradeTalisman),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) =
                        oteryn_client_ui::task_board::request(&mut mailbox.task_board, intent)
                    && commands
                        .send(crate::combat_input::PlayCommand::TaskBoard(command))
                        .is_err()
                {
                    mailbox.task_board.pending = false;
                    mailbox.task_board.feedback = Some("The session has ended");
                }
            }
            if held.as_ref() != mailbox.task_board.state.as_ref() {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                if let Some(state) = &mailbox.task_board.state {
                    for task in &state.tasks {
                        SendMessageW(
                            list,
                            LB_ADDSTRING,
                            0,
                            wide(&format!("{} ({})", task.race, task.required)).as_ptr() as isize,
                        );
                    }
                }
                held = mailbox.task_board.state.clone();
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        close(&weak);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).task_board.open = false;
    }
}
