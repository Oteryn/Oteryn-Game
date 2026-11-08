//! Explicit resource review, receipt and selectable history. No owning PlayLink is made here.
#![allow(
    unsafe_code,
    reason = "Win32 controls and userdata remain on their creating thread"
)]
use crate::{
    combat_input::PlayCommand,
    play::{
        feature_0152_item_forge::input::controller::{self, Action},
        mailbox::{SharedMailbox, lock},
    },
};
use oteryn_client_ui::item_forge::{
    reply::Reply,
    resource_dialog::{Content, dialog_text, operation_name},
};
use std::{
    ptr::{null, null_mut},
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
#[derive(Default)]
struct Events {
    close: bool,
    confirm: bool,
    row: bool,
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Events;
        if msg == WM_CLOSE {
            if !state.is_null() {
                (*state).close = true;
            }
            return 0;
        }
        if msg == WM_COMMAND && !state.is_null() {
            let id = w & 65535;
            let notification = (w >> 16) & 65535;
            if id == 100 && notification == BN_CLICKED as usize {
                (*state).confirm = true;
                return 0;
            }
            if id == 101 && notification == BN_CLICKED as usize {
                (*state).close = true;
                return 0;
            }
            if id == 200 && notification == LBN_SELCHANGE as usize {
                (*state).row = true;
                return 0;
            }
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe fn control(
    parent: HWND,
    class: &str,
    label: &str,
    id: usize,
    b: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
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
pub(in crate::play) fn open(mailbox: &SharedMailbox, commands: &UnboundedSender<PlayCommand>) {
    if commands.is_closed() {
        return;
    }
    let (serial, content) = {
        let mut m = lock(mailbox);
        let p = &mut m.item_forge;
        if !p.selected || p.resource_dialog.native_open {
            return;
        }
        let Some(content) = p.resource_dialog.content.clone() else {
            return;
        };
        p.resource_dialog.native_open = true;
        (p.resource_dialog.serial, content)
    };
    let weak = Arc::downgrade(mailbox);
    let rollback = weak.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("forge-resource-dialog".into())
        .spawn(move || run(weak, commands, serial, content))
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        let mut m = lock(&shared);
        m.item_forge.resource_dialog.dismiss(serial);
        m.item_forge.feedback = Some("Cannot open the Forge resource dialog".into());
    }
}
fn run(
    weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: UnboundedSender<PlayCommand>,
    serial: u64,
    content: Content,
) {
    unsafe {
        let confirm = matches!(content, Content::Confirm { .. });
        let history = matches!(content, Content::Result(Reply::History(_)));
        let title = match &content {
            Content::Confirm { .. } => "Review Forge resource operation",
            Content::Result(Reply::History(_)) => "Forge operation history",
            _ => "Forge resource result",
        };
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            750,
            640,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !hwnd.is_null() {
            let mut state = Box::<Events>::default();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *state as *mut Events) as isize);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            let list = control(
                hwnd,
                "LISTBOX",
                "",
                200,
                [12, 12, 710, 145],
                WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
            );
            if let Content::Result(Reply::History(rows)) = &content {
                for (ordinal, receipt) in rows {
                    SendMessageW(
                        list,
                        LB_ADDSTRING,
                        0,
                        wide(&format!(
                            "#{ordinal} · {} · {}",
                            operation_name(receipt.operation),
                            if receipt.succeeded {
                                "Succeeded"
                            } else if receipt.applied {
                                "Attempt applied"
                            } else {
                                "Not applied"
                            }
                        ))
                        .as_ptr() as isize,
                    );
                }
                if !rows.is_empty() {
                    SendMessageW(list, LB_SETCURSEL, 0, 0);
                }
            }
            if !history {
                ShowWindow(list, SW_HIDE);
            }
            let body = control(
                hwnd,
                "EDIT",
                "",
                201,
                [
                    12,
                    if history { 170 } else { 12 },
                    710,
                    if history { 350 } else { 510 },
                ],
                WS_BORDER
                    | WS_VSCROLL
                    | WS_TABSTOP
                    | ES_MULTILINE as u32
                    | ES_READONLY as u32
                    | ES_AUTOVSCROLL as u32,
            );
            SetWindowTextW(
                body,
                wide(&dialog_text(&content, 0).replace('\n', "\r\n")).as_ptr(),
            );
            if confirm {
                control(
                    hwnd,
                    "BUTTON",
                    "Confirm operation",
                    100,
                    [12, 540, 250, 32],
                    WS_TABSTOP,
                );
            }
            control(
                hwnd,
                "BUTTON",
                if confirm { "Cancel" } else { "Close" },
                101,
                [460, 540, 250, 32],
                WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            );
            loop {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if message.message == WM_KEYDOWN
                        && (message.wParam == 13 || message.wParam == 27)
                    {
                        state.close = true;
                    } else if IsDialogMessageW(hwnd, &message) == 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                if IsWindow(hwnd) == 0 || commands.is_closed() {
                    break;
                }
                let Some(shared) = weak.upgrade() else { break };
                let mut m = lock(&shared);
                if !m.item_forge.selected
                    || m.quest_shutdown
                    || m.item_forge.resource_dialog.serial != serial
                {
                    break;
                }
                if confirm
                    && m.item_forge
                        .resource_dialog
                        .expires
                        .is_none_or(|at| at <= std::time::Instant::now())
                {
                    controller::activate(&mut m, &commands, Action::DismissResource(serial));
                    m.item_forge.feedback = Some(
                        "Resource review expired. Choose the operation again to refresh it".into(),
                    );
                    break;
                }
                if state.close {
                    controller::activate(&mut m, &commands, Action::DismissResource(serial));
                    break;
                }
                if state.confirm && confirm {
                    state.confirm = false;
                    controller::activate(&mut m, &commands, Action::ConfirmResource(serial));
                    break;
                }
                if state.row {
                    state.row = false;
                    if let Ok(row) = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)) {
                        SetWindowTextW(
                            body,
                            wide(&dialog_text(&content, row).replace('\n', "\r\n")).as_ptr(),
                        );
                    }
                }
                drop(m);
                MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
            }
            if IsWindow(hwnd) != 0 {
                DestroyWindow(hwnd);
            }
        }
    }
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        if m.item_forge.resource_dialog.serial == serial {
            m.item_forge.resource_dialog.dismiss(serial);
        }
    }
}
