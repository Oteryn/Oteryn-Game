//! Shared native editor and explicit confirmation; no mailbox lock is held here.
#![allow(
    unsafe_code,
    reason = "Dialog handles and userdata stay on their creating thread"
)]
use super::super::dialog_preferences;
use super::{control, text, wide};
use oteryn_client_ui::modal_dialogs::TextEditor;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::EM_SETLIMITTEXT,
        Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled, SetActiveWindow, SetFocus},
        WindowsAndMessaging::*,
    },
};
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND
            && (w >> 16) & 0xffff == BN_CLICKED as usize
            && matches!(w & 0xffff, 1 | 2)
        {
            let action = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut usize;
            if !action.is_null() {
                *action = w & 0xffff;
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
struct Prompt {
    hwnd: HWND,
    parent: HWND,
    restore_parent: bool,
    action: Box<usize>,
}
impl Drop for Prompt {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.hwnd) != 0 {
                DestroyWindow(self.hwnd);
            }
            if self.restore_parent && IsWindow(self.parent) != 0 {
                EnableWindow(self.parent, 1);
                SetActiveWindow(self.parent);
            }
        }
    }
}
fn run(
    parent: HWND,
    title: &str,
    body: &str,
    initial: Option<(&str, usize)>,
    suppressible: bool,
) -> Option<(String, bool)> {
    unsafe {
        let editing = initial.is_some();
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            if editing { 440 } else { 250 },
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return None;
        }
        let mut prompt = Prompt {
            hwnd,
            parent,
            restore_parent: !parent.is_null() && IsWindowEnabled(parent) != 0,
            action: Box::new(0),
        };
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            prompt.action.as_mut() as *mut usize as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        if prompt.restore_parent {
            EnableWindow(parent, 0);
        }
        let label = control(
            hwnd,
            "STATIC",
            body,
            0,
            [12, 10, 640, if editing { 30 } else { 85 }],
            0,
        );
        if label.is_null() {
            return None;
        }
        let mut edit = null_mut();
        if let Some((initial, maximum)) = initial {
            edit = control(
                hwnd,
                "EDIT",
                initial,
                10,
                [12, 45, 640, 290],
                WS_BORDER
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | ES_MULTILINE as u32
                    | ES_AUTOVSCROLL as u32
                    | ES_WANTRETURN as u32,
            );
            if edit.is_null() {
                return None;
            }
            SendMessageW(edit, EM_SETLIMITTEXT, maximum, 0);
            SetFocus(edit);
        }
        let checkbox = if !editing {
            let checkbox = control(
                hwnd,
                "BUTTON",
                "Don't show this confirmation again",
                3,
                [12, 105, 640, 28],
                WS_TABSTOP | BS_AUTOCHECKBOX as u32,
            );
            if checkbox.is_null() {
                return None;
            }
            EnableWindow(checkbox, i32::from(suppressible));
            checkbox
        } else {
            null_mut()
        };
        let y = if editing { 350 } else { 150 };
        if control(
            hwnd,
            "BUTTON",
            if editing { "Save text" } else { "Confirm" },
            1,
            [12, y, 180, 30],
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        )
        .is_null()
            || control(hwnd, "BUTTON", "Cancel", 2, [210, y, 180, 30], WS_TABSTOP).is_null()
        {
            return None;
        }
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            match *prompt.action {
                2 => return None,
                1 => {
                    let value = if editing { text(edit) } else { String::new() };
                    if let Some((_, maximum)) = initial
                        && TextEditor::new(&value, maximum).submit().is_none()
                    {
                        *prompt.action = 0;
                        MessageBoxW(
                            hwnd,
                            wide(&format!("Text must fit within {maximum} UTF-8 bytes.")).as_ptr(),
                            wide("Text editor").as_ptr(),
                            MB_OK | MB_ICONWARNING,
                        );
                        continue;
                    }
                    return Some((
                        value,
                        !checkbox.is_null() && SendMessageW(checkbox, BM_GETCHECK, 0, 0) == 1,
                    ));
                }
                _ => {}
            }
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        None
    }
}
pub(in crate::play) fn edit_text(
    parent: HWND,
    label: &str,
    initial: &str,
    maximum: usize,
) -> Option<String> {
    run(
        parent,
        "Text editor",
        label,
        Some((initial, maximum)),
        false,
    )
    .map(|(text, _)| text)
}
pub(in crate::play) fn confirm(parent: HWND, key: &str, title: &str, message: &str) -> bool {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .map(|root| root.join("Oteryn/client-modal-notices.json"));
    if let Some(path) = &path
        && !dialog_preferences::should_ask(path, key).unwrap_or(true)
    {
        return true;
    }
    let result = run(parent, title, message, None, path.is_some());
    let accepted = result.is_some();
    if let Some(path) = &path
        && let Err(error) = dialog_preferences::record(
            path,
            key,
            accepted,
            result.is_some_and(|(_, checked)| checked),
        )
    {
        unsafe {
            MessageBoxW(
                parent,
                wide(&format!(
                    "Could not save the confirmation preference: {error}"
                ))
                .as_ptr(),
                wide(title).as_ptr(),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    accepted
}
