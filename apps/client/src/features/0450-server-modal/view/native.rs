//! Native controls use local indices; only server identities enter the answer wire.
#![allow(
    unsafe_code,
    reason = "Window and controls remain on their creating thread"
)]
use super::{Dialog, Submission};
use oteryn_client_ui::server_modal::Answer;
use std::ptr::{null, null_mut};
use tokio::sync::{mpsc::Sender, oneshot};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[path = "image.rs"]
mod image;
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
#[derive(Default)]
struct Action {
    button: Option<usize>,
    close: bool,
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let action = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Action;
        if !action.is_null() {
            if msg == WM_CLOSE {
                (*action).close = true;
                return 0;
            }
            if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
                (*action).button = (w & 0xffff).checked_sub(100);
                return 0;
            }
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn run(dialog: Dialog, sender: Sender<Submission>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(&dialog.title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            640,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        let mut action = Box::<Action>::default();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, action.as_mut() as *mut Action as isize);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        control(
            hwnd,
            "EDIT",
            &dialog.message,
            10,
            [12, 12, 680, 180],
            WS_BORDER
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
        );
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            11,
            [120, 204, 572, 210],
            WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        for (_, label) in &dialog.choices {
            SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let mut buttons = Vec::new();
        for (index, (_, label)) in dialog.buttons.iter().enumerate() {
            buttons.push(control(
                hwnd,
                "BUTTON",
                label,
                100 + index,
                [
                    12 + (index % 4) as i32 * 172,
                    435 + (index / 4) as i32 * 42,
                    164,
                    34,
                ],
                WS_TABSTOP,
            ));
        }
        let feedback = control(hwnd, "STATIC", "", 12, [12, 535, 680, 40], 0);
        let mut pending: Option<oneshot::Receiver<bool>> = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !sender.is_closed() {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_KEYDOWN && message.wParam == 27 {
                    action.close = true;
                } else if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if let Some(image) = &dialog.image {
                image::paint(hwnd, image);
            }
            if let Some(receiver) = &mut pending {
                match receiver.try_recv() {
                    Ok(true) => break,
                    Ok(false) => {
                        pending = None;
                        for button in &buttons {
                            EnableWindow(*button, 1);
                        }
                        SetWindowTextW(
                            feedback,
                            wide("The server refused the action. Choose again or cancel.").as_ptr(),
                        );
                    }
                    Err(oneshot::error::TryRecvError::Closed) => break,
                    Err(oneshot::error::TryRecvError::Empty) => {}
                }
            }
            let clicked = action
                .button
                .take()
                .and_then(|index| dialog.buttons.get(index).map(|(id, _)| *id));
            let button = if std::mem::take(&mut action.close) {
                dialog.cancel_button
            } else {
                clicked
            };
            if pending.is_none()
                && let Some(button_id) = button
            {
                let selected = SendMessageW(list, LB_GETCURSEL, 0, 0);
                let choice_id = if dialog.cancel_button == Some(button_id) {
                    None
                } else {
                    usize::try_from(selected)
                        .ok()
                        .and_then(|index| dialog.choices.get(index).map(|(id, _)| *id))
                };
                let answer = Answer {
                    dialog_id: dialog.id,
                    button_id,
                    choice_id,
                };
                if !dialog.accepts(answer) {
                    SetWindowTextW(feedback, wide("Select one option first.").as_ptr());
                } else {
                    let (tx, rx) = oneshot::channel();
                    if sender.try_send((answer, tx)).is_ok() {
                        pending = Some(rx);
                        for button in &buttons {
                            EnableWindow(*button, 0);
                        }
                    }
                }
            }
            MsgWaitForMultipleObjects(0, null(), 0, 30, QS_ALLINPUT);
        }
        DestroyWindow(hwnd);
    }
}
