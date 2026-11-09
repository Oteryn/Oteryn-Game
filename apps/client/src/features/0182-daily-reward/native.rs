//! Native daily reward wall and one-click claim controls.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_reward::{Intent, panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
const REFRESH: usize = 101;
const CLAIM: usize = 102;
const DETAILS: usize = 104;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && matches!(id, REFRESH | CLAIM | DETAILS)
                && note == BN_CLICKED as usize
            {
                (*actions).0.push(id);
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
        if m.daily_reward.open {
            return;
        }
        m.daily_reward.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("daily-reward-panel".into())
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
            wide(tr!("Daily Reward")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            430,
            640,
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
        let text = control(
            hwnd,
            "EDIT",
            "",
            103,
            [10, 10, 390, 480],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Refresh"),
            REFRESH,
            [10, 505, 180, 30],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Claim reward"),
            CLAIM,
            [220, 505, 180, 30],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Catalogue / account history",
            DETAILS,
            [10, 550, 390, 30],
            WS_TABSTOP,
        );
        let mut held = None;
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
            if mailbox.daily_reward_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if action == DETAILS {
                    super::super::input::enqueue_details(&mut mailbox, &commands);
                    continue;
                }
                let intent = if action == CLAIM {
                    Intent::Claim
                } else {
                    Intent::Refresh
                };
                if let Some(command) = request(&mut mailbox.daily_reward, intent)
                    && commands.send(PlayCommand::DailyReward(command)).is_err()
                {
                    mailbox.daily_reward.pending = false;
                    mailbox.daily_reward.feedback = Some(tr!("The session has ended"));
                }
            }
            if held.as_ref() != Some(&mailbox.daily_reward) {
                SetWindowTextW(text, wide(&panel_text(&mailbox.daily_reward)).as_ptr());
                held = Some(mailbox.daily_reward);
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(25));
        }
        DestroyWindow(hwnd);
        close(&weak);
        drop(actions);
    }
}
fn control(parent: HWND, class: &str, label: &str, id: usize, rect: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).daily_reward.open = false;
    }
}
