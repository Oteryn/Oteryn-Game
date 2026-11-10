//! Monday results and next-week difficulty, using the Task Board command queue.
#![allow(unsafe_code, reason = "Win32 controls stay on their creating thread")]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::task_board::{FullIntent, full_request};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

type Shared = Weak<Mutex<Mailbox>>;
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize && w & 0xffff == 1 {
            SetWindowLongPtrW(h, GWLP_USERDATA, 1);
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
unsafe fn control(h: HWND, class: &str, text: &str, id: usize, r: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
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
pub(in crate::play) fn open(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.task_board.weekly_results_open {
            return;
        }
        mailbox.task_board.weekly_results_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("weekly-results".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn close(weak: &Shared) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).task_board.weekly_results_open = false;
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Weekly results")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            580,
            300,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak);
            return;
        }
        SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
        let text = control(h, "STATIC", "", 0, [15, 15, 540, 100], 0);
        control(
            h,
            "STATIC",
            tr!("Choose next week's difficulty"),
            0,
            [15, 120, 300, 24],
            0,
        );
        let choice = control(
            h,
            "COMBOBOX",
            "",
            2,
            [15, 150, 250, 180],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for label in [tr!("Beginner"), tr!("Adept"), tr!("Expert"), tr!("Master")] {
            SendMessageW(choice, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(choice, CB_SETCURSEL, 0, 0);
        let start = control(
            h,
            "BUTTON",
            tr!("Start next week"),
            1,
            [285, 150, 250, 32],
            WS_TABSTOP,
        );
        let feedback = control(h, "STATIC", "", 0, [15, 200, 540, 40], 0);
        if text.is_null() || choice.is_null() || start.is_null() || feedback.is_null() {
            DestroyWindow(h);
            close(&weak);
            return;
        }
        loop {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&shared);
            let panel = &mut mailbox.task_board;
            if !panel.weekly_results_open || !panel.selected {
                break;
            }
            let Some(results) = panel
                .weekly
                .as_ref()
                .and_then(|reply| reply.weekly_results.as_ref())
            else {
                break;
            };
            SetWindowTextW(
                text,
                wide(&format!(
                    "{}\r\n{}: {}\r\nSoulseals: {}",
                    tr!("Weekly rewards received"),
                    tr!("Task points"),
                    results.points,
                    results.soulseals
                ))
                .as_ptr(),
            );
            SetWindowTextW(feedback, wide(panel.feedback.unwrap_or("")).as_ptr());
            EnableWindow(start, i32::from(!panel.pending));
            if GetWindowLongPtrW(h, GWLP_USERDATA) != 0 {
                SetWindowLongPtrW(h, GWLP_USERDATA, 0);
                if let Ok(difficulty) = u8::try_from(SendMessageW(choice, CB_GETCURSEL, 0, 0))
                    && let Some(command) = full_request(panel, FullIntent::StartWeekly(difficulty))
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    panel.pending = false;
                    panel.feedback = Some(tr!("The session has ended"));
                }
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
    }
    close(&weak);
}
