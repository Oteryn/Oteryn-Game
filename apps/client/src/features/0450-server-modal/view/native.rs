//! Native controls use local indices; only server identities enter the answer wire.
#![allow(
    unsafe_code,
    reason = "Window and controls remain on their creating thread"
)]
#[cfg(all(windows, not(test)))]
use super::{Arc, Dialog, DialogInput, Entry, Mutex, Submission};
#[cfg(all(windows, not(test)))]
use oteryn_client_ui::server_modal::Answer;
#[cfg(all(windows, not(test)))]
use std::ptr::{null, null_mut};
#[cfg(all(windows, not(test)))]
use tokio::sync::{mpsc::Sender, oneshot};
#[cfg(all(windows, not(test)))]
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[cfg(all(windows, not(test)))]
#[path = "image.rs"]
mod image;
#[cfg(all(windows, not(test)))]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
#[cfg(all(windows, not(test)))]
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
    confirm: bool,
}
impl Action {
    fn command(&mut self, command: usize) -> bool {
        // Win32 LBN_DBLCLK is 2 and the choice list's control ID is 11.
        if (command >> 16) & 0xffff == 2 && command & 0xffff == 11 {
            self.confirm = true;
            return true;
        }
        // BN_CLICKED is zero; keep notification decoding testable without Win32.
        if (command >> 16) & 0xffff == 0 {
            self.button = (command & 0xffff).checked_sub(100);
            return true;
        }
        false
    }

    fn take_button_id(
        &mut self,
        dialog: &oteryn_client_ui::server_modal::Dialog,
        selected: Option<usize>,
    ) -> Option<u8> {
        let clicked = self.button.take();
        let confirm = std::mem::take(&mut self.confirm);
        if std::mem::take(&mut self.close) {
            return dialog.cancel_button;
        }
        let index = if confirm {
            match key_action(dialog, 13, selected, true) {
                KeyAction::Confirm(index) => Some(index),
                _ => None,
            }
        } else {
            clicked
        }?;
        dialog.buttons.get(index).map(|(id, _)| *id)
    }
}
#[cfg(all(windows, not(test)))]
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let action = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Action;
        if !action.is_null() {
            if msg == WM_CLOSE {
                (*action).close = true;
                return 0;
            }
            if msg == WM_COMMAND && (*action).command(w) {
                return 0;
            }
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
#[cfg(all(windows, not(test)))]
fn read_text(edit: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(edit).max(0) as usize;
        let mut text = vec![0u16; length + 1];
        let copied = GetWindowTextW(edit, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&text[..copied])
    }
}
#[cfg(all(windows, not(test)))]
pub(super) fn run(
    dialog: Dialog,
    input: Option<DialogInput>,
    remember: bool,
    sender: Sender<Submission>,
    entry: Arc<Mutex<Entry>>,
) {
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
            // A dialog with a value field leaves the space below the list for it.
            [120, 204, 572, if input.is_some() { 90 } else { 210 }],
            WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        if input.is_some() && dialog.choices.is_empty() {
            ShowWindow(list, SW_HIDE);
        }
        // Capability 490: the value field sits below the choice list, above the buttons.
        let field = input.as_ref().map(|input| {
            let field = super::input_control(input, |text, rect, style| {
                control(hwnd, "EDIT", text, 13, rect, style)
            });
            control(
                hwnd,
                "STATIC",
                &oteryn_client_ui::server_modal_input::prompt(input),
                15,
                [120, 300, 572, 22],
                0,
            );
            field
        });
        // Capability 490: the choice is independent of the value field.
        let (remember, feedback) = super::footer_controls(
            remember,
            |rect| {
                control(
                    hwnd,
                    "BUTTON",
                    "Don't show again",
                    14,
                    rect,
                    WS_TABSTOP | BS_AUTOCHECKBOX as u32,
                )
            },
            |rect| control(hwnd, "STATIC", "", 12, rect, 0),
        );
        fill_choices(
            &dialog,
            |label| {
                SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            },
            |index| {
                SendMessageW(list, LB_SETCURSEL, index, 0);
            },
        );
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
        let mut pending: Option<oneshot::Receiver<bool>> = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !sender.is_closed() {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
                let multiline = field == Some(message.hwnd)
                    && input.as_ref().is_some_and(|input| {
                        input.kind == oteryn_client_runtime::server_modal::input::InputKind::Edit
                    });
                let allow_confirm = !multiline
                    && (message.hwnd == list
                        || field == Some(message.hwnd)
                        || message.hwnd == hwnd);
                let key = if message.message == WM_KEYDOWN {
                    key_action(&dialog, message.wParam, selected, allow_confirm)
                } else {
                    KeyAction::Dispatch
                };
                match key {
                    KeyAction::Close => action.close = true,
                    KeyAction::Confirm(index) => action.button = Some(index),
                    KeyAction::Dispatch => {
                        if IsDialogMessageW(hwnd, &message) == 0 {
                            TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                    }
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
            let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
            let button = action.take_button_id(&dialog, selected);
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
                let value = match (&input, field) {
                    (Some(input), Some(field)) if dialog.cancel_button != Some(button_id) => {
                        let text = read_text(field);
                        oteryn_client_ui::server_modal_input::parse_value(input, &text)
                    }
                    _ => None,
                };
                let wants_value = input.is_some() && dialog.cancel_button != Some(button_id);
                if !dialog.accepts(answer) {
                    SetWindowTextW(feedback, wide("Select one option first.").as_ptr());
                } else if wants_value && value.is_none() {
                    SetWindowTextW(feedback, wide("Enter a value within the limits.").as_ptr());
                } else {
                    *entry
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Entry {
                        value,
                        dont_show_again: remember
                            .is_some_and(|box_| SendMessageW(box_, BM_GETCHECK, 0, 0) == 1),
                    };
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

fn fill_choices(
    dialog: &oteryn_client_ui::server_modal::Dialog,
    mut add: impl FnMut(&str),
    select: impl FnOnce(usize),
) {
    for (_, label) in &dialog.choices {
        add(label);
    }
    if !dialog.choices.is_empty() {
        select(0);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum KeyAction {
    Close,
    Confirm(usize),
    Dispatch,
}

fn key_action(
    dialog: &oteryn_client_ui::server_modal::Dialog,
    key: usize,
    selected: Option<usize>,
    allow_confirm: bool,
) -> KeyAction {
    if key == 27 {
        return KeyAction::Close;
    }
    if key == 13
        && allow_confirm
        && let Some(index) = dialog.default_button
            .and_then(|button| dialog.buttons.iter().position(|(id, _)| *id == button))
            .or_else(|| dialog.buttons.iter().position(|(id, _)| Some(*id) != dialog.cancel_button))
    {
        let answer = oteryn_client_ui::server_modal::Answer {
            dialog_id: dialog.id,
            button_id: dialog.buttons[index].0,
            choice_id: if Some(dialog.buttons[index].0) == dialog.cancel_button {
                None
            } else {
                selected.and_then(|row| dialog.choices.get(row).map(|(id, _)| *id))
            },
        };
        if dialog.accepts(answer) {
            return KeyAction::Confirm(index);
        }
    }
    KeyAction::Dispatch
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_ui::server_modal::Dialog;
    fn dialog() -> Dialog {
        Dialog {
            id: 73,
            title: "Choose".into(),
            message: "Server options".into(),
            choices: vec![(9, "Nine".into())],
            buttons: vec![(8, "Cancel".into()), (4, "Accept".into())],
            default_button: None,
            cancel_button: Some(8),
            image: None,
        }
    }
    #[test]
    fn review_native_modal_enter_submits_the_server_default_even_when_it_is_close() {
        let mut dialog = dialog();
        dialog.choices.clear();
        dialog.buttons = vec![(1, "Close".into())];
        dialog.default_button = Some(1);
        dialog.cancel_button = Some(1);
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Confirm(0));
        assert_eq!(key_action(&dialog, 27, None, true), KeyAction::Close);
        let mut action = Action { button: Some(0), ..Default::default() };
        assert_eq!(action.take_button_id(&dialog, None), Some(1));
    }

    #[test]
    fn review_native_modal_choice_double_click_submits_the_server_default() {
        let mut dialog = dialog();
        dialog.buttons.push((7, "Apply".into()));
        dialog.default_button = Some(7);
        let mut action = Action::default();
        assert!(action.command(11 | (2 << 16)));
        assert_eq!(action.take_button_id(&dialog, Some(0)), Some(7));
        assert_eq!(action.take_button_id(&dialog, Some(0)), None);
        assert_eq!(key_action(&dialog, 27, Some(0), true), KeyAction::Close);
        action.close = true;
        assert_eq!(action.take_button_id(&dialog, Some(0)), Some(8));
    }
    #[test]
    fn native_modal_initially_selects_first_server_choice() {
        let mut labels = Vec::new();
        let mut selected = None;
        fill_choices(
            &dialog(),
            |label| labels.push(label.to_owned()),
            |row| selected = Some(row),
        );
        assert_eq!(labels, ["Nine"]);
        assert_eq!(selected, Some(0));
        selected = None;
        let mut empty = dialog();
        empty.choices.clear();
        fill_choices(
            &empty,
            |label| labels.push(label.to_owned()),
            |row| selected = Some(row),
        );
        assert_eq!(selected, None);
    }
    #[test]
    fn native_modal_enter_validates_confirmation_and_preserves_multiline_editing() {
        let mut dialog = dialog();
        assert_eq!(
            key_action(&dialog, 13, Some(0), true),
            KeyAction::Confirm(1)
        );
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 13, Some(3), true), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 13, Some(0), false), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 27, None, false), KeyAction::Close);
        dialog.choices.clear();
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Confirm(1));
        dialog.buttons.pop();
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Dispatch);
    }
}
