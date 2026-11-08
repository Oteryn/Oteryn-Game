//! Native selectors consume only the current server observation.
#![allow(unsafe_code, reason = "Win32 handles stay on their creating thread")]
use super::super::super::input::controller::{self, Action};
use super::lifecycle::{WindowEvents as Events, close_with_surfaces};
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::cui3::item_forge::Operation;
use std::{
    ptr::{null, null_mut},
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[path = "icons.rs"]
mod icons;
#[path = "window_layout.rs"]
mod layout;
#[path = "window_selection.rs"]
mod selection;
use layout::control;
use selection::{choices, populate};
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata is a boxed Events owned by this thread until HWND destruction.
    unsafe {
        if matches!(msg, WM_CLOSE | WM_COMMAND) {
            let events = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Events;
            if !events.is_null() {
                if msg == WM_CLOSE {
                    (*events).request_close();
                } else {
                    (*events).command(w);
                }
            }
            // Never let DefWindowProcW destroy child HWNDs while GPU surfaces borrow them.
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
const OPS: [(Operation, &str); 4] = [
    (Operation::Fusion, "Fusion"),
    (Operation::ConvergenceFusion, "Convergence fusion"),
    (Operation::Transfer, "Transfer"),
    (Operation::ConvergenceTransfer, "Convergence transfer"),
];
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut shared = lock(&link.mailbox);
        if shared.item_forge.open {
            return;
        }
        shared.item_forge.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("forge-selection".into())
        .spawn(move || {
            // SAFETY: all window, event, and renderer access stays on this thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Forge — select carried items").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    920,
                    790,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !hwnd.is_null() {
                    let mut events = Box::<Events>::default();
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *events as *mut Events) as isize);
                    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                    let (
                        layout::Controls {
                            operation,
                            first,
                            second,
                            boost,
                            protect,
                            bags,
                            open,
                            refresh,
                            next,
                            quote,
                            forge,
                            retry,
                            secondary,
                            status,
                        },
                        mut icons,
                    ) = layout::create(hwnd);
                    let mut held = None;
                    let mut items = Vec::new();
                    let mut containers = Vec::new();
                    loop {
                        let mut message: MSG = std::mem::zeroed();
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                            if IsDialogMessageW(hwnd, &message) == 0 {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                        if events.close_requested || IsWindow(hwnd) == 0 || commands.is_closed() {
                            break;
                        }
                        let Some(shared) = weak.upgrade() else {
                            break;
                        };
                        {
                            let mut mailbox = lock(&shared);
                            if mailbox.quest_shutdown || !mailbox.item_forge.open {
                                break;
                            }
                            for event in std::mem::take(&mut events.commands) {
                                let id = event & 0xffff;
                                let notification = (event >> 16) & 0xffff;
                                let selected = |combo: HWND, entries: &[(u64, String)]| {
                                    usize::try_from(SendMessageW(combo, CB_GETCURSEL, 0, 0))
                                        .ok()
                                        .and_then(|i| entries.get(i))
                                        .map(|i| i.0)
                                };
                                let action = match (id, notification) {
                                    (100, n) if n == CBN_SELCHANGE as usize => {
                                        usize::try_from(SendMessageW(operation, CB_GETCURSEL, 0, 0))
                                            .ok()
                                            .and_then(|i| OPS.get(i))
                                            .map(|p| Action::Operation(p.0))
                                    }
                                    (101, n) if n == CBN_SELCHANGE as usize => {
                                        selected(first, &items).map(Action::First)
                                    }
                                    (102, n) if n == CBN_SELCHANGE as usize => {
                                        selected(second, &items).map(Action::Second)
                                    }
                                    (103 | 104, n) if n == BN_CLICKED as usize => {
                                        Some(Action::Cores {
                                            boost: SendMessageW(boost, BM_GETCHECK, 0, 0) == 1,
                                            protect: SendMessageW(protect, BM_GETCHECK, 0, 0) == 1,
                                        })
                                    }
                                    (106, n) if n == BN_CLICKED as usize => {
                                        selected(bags, &containers).map(Action::OpenBag)
                                    }
                                    (107, n) if n == BN_CLICKED as usize => Some(Action::Refresh),
                                    (108, n) if n == BN_CLICKED as usize => Some(Action::Next),
                                    (109, n) if n == BN_CLICKED as usize => Some(Action::Quote),
                                    (110, n) if n == BN_CLICKED as usize => Some(Action::Forge),
                                    (111, n) if n == BN_CLICKED as usize => Some(Action::Retry),
                                    (113, n) if n == BN_CLICKED as usize => {
                                        Some(Action::CurrentDust)
                                    }
                                    (114, n) if n == BN_CLICKED as usize => {
                                        Some(Action::Legacy(Operation::QueryHistory))
                                    }
                                    (115, n) if n == BN_CLICKED as usize => {
                                        Some(Action::Legacy(Operation::DustToSlivers))
                                    }
                                    (116, n) if n == BN_CLICKED as usize => {
                                        Some(Action::Legacy(Operation::SliversToCores))
                                    }
                                    (117, n) if n == BN_CLICKED as usize => {
                                        Some(Action::Legacy(Operation::DustLimit))
                                    }
                                    _ => None,
                                };
                                if let Some(action) = action {
                                    controller::activate(&mut mailbox, &commands, action);
                                }
                            }
                            if mailbox.item_forge.preview.refresh
                                && mailbox.item_forge.preview.pending.is_none()
                                && mailbox.item_forge.preview.navigation.is_none()
                            {
                                mailbox.item_forge.preview.refresh = false;
                                controller::activate(&mut mailbox, &commands, Action::Refresh);
                            }
                        }
                        let panel = lock(&shared).item_forge.clone();
                        if held.as_ref() != Some(&panel) {
                            items = choices(&panel, false);
                            containers = choices(&panel, true);
                            populate(first, &items, panel.first_item);
                            populate(second, &items, panel.second_item);
                            populate(bags, &containers, 0);
                            SendMessageW(
                                operation,
                                CB_SETCURSEL,
                                OPS.iter().position(|p| p.0 == panel.operation).unwrap_or(0),
                                0,
                            );
                            SendMessageW(boost, BM_SETCHECK, usize::from(panel.boost), 0);
                            SendMessageW(protect, BM_SETCHECK, usize::from(panel.protect), 0);
                            let editable = panel.editing() && panel.preview.pending.is_none();
                            for handle in [first, second, operation, bags, open, refresh] {
                                EnableWindow(handle, i32::from(editable));
                            }
                            for handle in [boost, protect] {
                                EnableWindow(
                                    handle,
                                    i32::from(editable && panel.operation == Operation::Fusion),
                                );
                            }
                            EnableWindow(
                                next,
                                i32::from(
                                    editable
                                        && panel
                                            .preview
                                            .page
                                            .as_ref()
                                            .is_some_and(|p| p.next.is_some()),
                                ),
                            );
                            EnableWindow(
                                quote,
                                i32::from(
                                    editable
                                        && panel.preview.selected
                                        && panel.selection().is_some(),
                                ),
                            );
                            EnableWindow(
                                forge,
                                i32::from(
                                    editable && (!panel.preview.selected || panel.verified_quote()),
                                ),
                            );
                            for handle in secondary {
                                EnableWindow(handle, i32::from(!panel.pending));
                            }
                            EnableWindow(retry, i32::from(!panel.pending && panel.retry.is_some()));
                            let mut text = oteryn_client_ui::item_forge::selected_pair_text(&panel);
                            text.push_str("\n");
                            text.push_str(&oteryn_client_ui::item_forge::panel_text(&panel));
                            for (index, handle) in [panel.first_item, panel.second_item]
                                .into_iter()
                                .enumerate()
                            {
                                if let Err(error) = icons[index]
                                    .select(panel.preview.item(handle).map(|i| i.appearance))
                                {
                                    text.push_str(&format!("\nItem preview unavailable: {error}"));
                                }
                            }
                            SetWindowTextW(status, wide(&text.replace('\n', "\r\n")).as_ptr());
                            held = Some(panel);
                        }
                        for icon in &mut icons {
                            icon.render();
                        }
                        MsgWaitForMultipleObjects(0, null(), 0, 40, QS_ALLINPUT);
                    }
                    close_with_surfaces(icons, || {
                        if IsWindow(hwnd) != 0 {
                            DestroyWindow(hwnd);
                        }
                    });
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).item_forge.open = false;
            }
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).item_forge.open = false;
    }
}
