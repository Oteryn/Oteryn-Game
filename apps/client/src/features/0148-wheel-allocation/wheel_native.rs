//! Original graph plus accessible slot controls. Every data/edit callback uses the live mailbox.
#![allow(
    unsafe_code,
    reason = "Native controls/userdata and renderer remain on one thread"
)]
use crate::{
    combat_input::PlayCommand,
    play::{
        feature_0148_wheel_allocation::input::wheel_presentation::{self as callbacks, Action},
        mailbox::{SharedMailbox, lock},
    },
};
use oteryn_client_ui::wheel_allocation::{WheelPanel, diagram};
use std::{
    ptr::{null, null_mut},
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[path = "wheel_window.rs"]
mod window;
use window::{Events, control, graph_procedure, procedure, wide};
#[path = "wheel_launch.rs"]
mod launch;
pub(in crate::play) use launch::open;
fn run(
    weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    s: UnboundedSender<PlayCommand>,
    epoch: u64,
) {
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Wheel — graph, configured perks and draft points").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            790,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !h.is_null() {
            let mut events = Box::<Events>::default();
            SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *events as *mut Events) as isize);
            SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
            let surface = control(h, "STATIC", "", 1, [12, 12, 384, 384], SS_NOTIFY as u32);
            SetWindowLongPtrW(
                surface,
                GWLP_USERDATA,
                (&mut *events as *mut Events) as isize,
            );
            SetWindowLongPtrW(surface, GWLP_WNDPROC, graph_procedure as *const () as isize);
            control(
                h,
                "STATIC",
                "Select a node or choose its slot below. Edges are server unlock links.",
                2,
                [12, 405, 384, 50],
                0,
            );
            let slot = control(
                h,
                "COMBOBOX",
                "",
                10,
                [12, 460, 384, 280],
                WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            );
            let points = control(
                h,
                "EDIT",
                "0",
                11,
                [12, 505, 150, 30],
                WS_TABSTOP | WS_BORDER | ES_NUMBER as u32,
            );
            let set = control(
                h,
                "BUTTON",
                "Set draft points",
                100,
                [175, 505, 220, 30],
                WS_TABSTOP,
            );
            let save = control(
                h,
                "BUTTON",
                "Save draft",
                101,
                [12, 555, 184, 32],
                WS_TABSTOP,
            );
            let reset = control(
                h,
                "BUTTON",
                "Review reset draft",
                102,
                [210, 555, 184, 32],
                WS_TABSTOP,
            );
            let refresh = control(
                h,
                "BUTTON",
                "Discard / refresh",
                103,
                [12, 605, 184, 32],
                WS_TABSTOP,
            );
            let retry = control(
                h,
                "BUTTON",
                "Retry exact request",
                104,
                [210, 605, 184, 32],
                WS_TABSTOP,
            );
            let body = control(
                h,
                "EDIT",
                "",
                12,
                [415, 12, 650, 715],
                WS_TABSTOP
                    | WS_BORDER
                    | WS_VSCROLL
                    | ES_MULTILINE as u32
                    | ES_READONLY as u32
                    | ES_AUTOVSCROLL as u32,
            );
            let mut renderer = super::wheel_surface::Surface::new(surface);
            let mut held: Option<WheelPanel> = None;
            let mut painted_nonce = None;
            loop {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if IsDialogMessageW(h, &message) == 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                if events.close || IsWindow(h) == 0 || s.is_closed() {
                    break;
                }
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let mut m = lock(&shared);
                if !m.wheel.selected
                    || m.wheel_shutdown
                    || !m.wheel.diagram.open
                    || m.wheel.diagram.epoch != epoch
                {
                    break;
                }
                for index in std::mem::take(&mut events.clicks) {
                    if let Some(nonce) = painted_nonce {
                        callbacks::activate(&mut m, &s, Action::Slot { nonce, index });
                    }
                }
                for (id, n) in std::mem::take(&mut events.actions) {
                    let action = match (id, n) {
                        (10, n) if n == CBN_SELCHANGE as usize => {
                            usize::try_from(SendMessageW(slot, CB_GETCURSEL, 0, 0))
                                .ok()
                                .and_then(|index| {
                                    Some(Action::Slot {
                                        nonce: painted_nonce?,
                                        index,
                                    })
                                })
                        }
                        (100, n) if n == BN_CLICKED as usize => {
                            let mut text = [0_u16; 16];
                            let n = GetWindowTextW(points, text.as_mut_ptr(), 16);
                            usize::try_from(n)
                                .ok()
                                .and_then(|n| String::from_utf16(&text[..n]).ok())
                                .and_then(|v| v.trim().parse::<u16>().ok())
                                .map(Action::Points)
                        }
                        (101, n) if n == BN_CLICKED as usize => Some(Action::Apply),
                        (102, n) if n == BN_CLICKED as usize => Some(Action::ResetDraft),
                        (103, n) if n == BN_CLICKED as usize => Some(Action::Refresh),
                        (104, n) if n == BN_CLICKED as usize => Some(Action::Retry),
                        _ => None,
                    };
                    if let Some(action) = action {
                        callbacks::activate(&mut m, &s, action);
                    }
                }
                if m.wheel.diagram.selected && diagram::needs_query(&m.wheel) {
                    callbacks::query(&mut m, &s);
                }
                let p = m.wheel.clone();
                drop(m);
                if held.as_ref() != Some(&p) {
                    let nonce = diagram::metadata(&p).map(|r| r.request.nonce);
                    if nonce != painted_nonce {
                        SendMessageW(slot, CB_RESETCONTENT, 0, 0);
                        if let Some(r) = diagram::metadata(&p) {
                            for (index, n) in r.nodes.iter().enumerate() {
                                SendMessageW(
                                    slot,
                                    CB_ADDSTRING,
                                    0,
                                    wide(&format!(
                                        "Slot {} · {} · capacity {}",
                                        index + 1,
                                        n.name,
                                        n.capacity
                                    ))
                                    .as_ptr() as isize,
                                );
                            }
                        }
                        painted_nonce = nonce;
                    }
                    SendMessageW(slot, CB_SETCURSEL, p.slot, 0);
                    if held.as_ref().is_none_or(|old| {
                        old.slot != p.slot || old.points[p.slot] != p.points[p.slot]
                    }) {
                        SetWindowTextW(points, wide(&p.points[p.slot].to_string()).as_ptr());
                    }
                    SetWindowTextW(
                        save,
                        wide(if p.diagram.reset_requested {
                            "Apply reset"
                        } else {
                            "Save draft"
                        })
                        .as_ptr(),
                    );
                    let editing =
                        p.ready() && p.diagram.pending.is_none();
                    for button in [slot, points, set, reset] {
                        EnableWindow(
                            button,
                            i32::from(editing && diagram::metadata(&p).is_some()),
                        );
                    }
                    EnableWindow(
                        save,
                        i32::from(
                            editing
                                && p.dirty()
                                && diagram::current(&p).is_some_and(|r| r.draft_valid),
                        ),
                    );
                    EnableWindow(
                        refresh,
                        i32::from(p.pending.is_none() && p.diagram.pending.is_none()),
                    );
                    EnableWindow(
                        retry,
                        i32::from(
                            p.retry.is_some() && p.pending.is_none() && p.diagram.pending.is_none(),
                        ),
                    );
                    let mut text = diagram::view::text(&p);
                    if let Some(renderer) = &mut renderer {
                        if let Err(error) = renderer.update(&p) {
                            text.push_str(&format!("\nGraph unavailable: {error}"));
                        }
                    } else {
                        text.push_str("\nGraph renderer unavailable; accessible slot details remain available.");
                    }
                    SetWindowTextW(body, wide(&text.replace('\n', "\r\n")).as_ptr());
                    held = Some(p);
                }
                if let Some(r) = &mut renderer {
                    r.render();
                }
                MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
            }
            if let Some(r) = &mut renderer {
                r.close();
            }
            drop(renderer);
            DestroyWindow(h);
        }
    }
    if let Some(m) = weak.upgrade() {
        let mut m = lock(&m);
        if m.wheel.diagram.epoch == epoch {
            m.wheel.diagram.open = false;
            m.wheel.diagram.reset_requested = false;
        }
    }
}
