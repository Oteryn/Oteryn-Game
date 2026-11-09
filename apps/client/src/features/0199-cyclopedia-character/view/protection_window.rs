//! Forecast cards use server values only; settlement still owns the actual death.
#![allow(unsafe_code)]
use super::controls::{control, wide};
use crate::{
    combat_input::PlayCommand,
    play::mailbox::{Mailbox, lock},
};
use oteryn_client_ui::cyclopedia_character::{
    Intent, PROTECTION_NOTE, protection_cards, protection_heading, request,
};
use std::{
    ptr::{null, null_mut},
    sync::{Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
type Shared = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(bool);
unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 1 && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() {
                (*a).0 = true;
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(in crate::play) fn open(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    let Some(shared) = weak.upgrade() else {
        return;
    };
    let generation = {
        let mut m = lock(&shared);
        let p = &mut m.cyclopedia_character;
        if !p.selected || p.protection.open {
            return;
        }
        p.protection.open = true;
        p.protection.forecast = None;
        p.profile_generation
    };
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("death-loss-preview".into())
        .spawn(move || run(weak, commands, generation))
        .is_err()
    {
        close(&failed, generation);
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>, generation: u64) {
    unsafe {
        // SAFETY: HWNDs and the action userdata live on this thread until destruction.
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Your death loss preview").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            820,
            680,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, generation);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let header = control(
            h,
            "STATIC",
            "Waiting for a server forecast",
            10,
            [10, 10, 780, 75],
            0,
        );
        let mut cards = Vec::new();
        for i in 0..6 {
            cards.push(control(
                h,
                "STATIC",
                "",
                20 + i,
                [
                    10 + (i % 2) as i32 * 395,
                    100 + (i / 2) as i32 * 115,
                    385,
                    105,
                ],
                WS_BORDER,
            ));
        }
        control(h, "STATIC", PROTECTION_NOTE, 30, [10, 455, 780, 70], 0);
        control(
            h,
            "BUTTON",
            "Refresh forecast",
            1,
            [10, 540, 200, 30],
            WS_TABSTOP,
        );
        let status = control(h, "STATIC", "", 31, [10, 585, 780, 35], 0);
        let mut initialized = false;
        let mut painted = None;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let (forecast, pending, feedback) = {
                let mut m = lock(&shared);
                if m.cyclopedia_character_shutdown {
                    break;
                }
                let p = &mut m.cyclopedia_character;
                if p.profile_generation != generation || !p.selected {
                    break;
                }
                let refresh = std::mem::take(&mut actions.0);
                if !p.pending && (!initialized || refresh) {
                    initialized = true;
                    if let Some(command) = request(p, Intent::Protection)
                        && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        p.pending = false;
                        p.protection.awaiting = false;
                        p.feedback = Some("The session has ended");
                    }
                }
                (p.protection.forecast, p.pending, p.feedback)
            };
            if forecast != painted {
                if let Some(f) = forecast {
                    SetWindowTextW(header, wide(&protection_heading(&f)).as_ptr());
                    for (handle, (label, value)) in cards.iter().zip(protection_cards(&f)) {
                        SetWindowTextW(*handle, wide(&format!("{label}\r\n\r\n{value}")).as_ptr());
                    }
                }
                painted = forecast;
            }
            SetWindowTextW(
                status,
                wide(if pending {
                    "Waiting for the server"
                } else {
                    feedback.unwrap_or("")
                })
                .as_ptr(),
            );
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        DestroyWindow(h);
        close(&weak, generation);
    }
}
fn close(weak: &Shared, generation: u64) {
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        if m.cyclopedia_character.profile_generation == generation {
            m.cyclopedia_character.protection.open = false;
        }
    }
}
