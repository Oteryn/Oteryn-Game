//! The selector owns its HWNDs on the blocking form thread.
// Native controls require Win32 FFI, confined to this module.
#![allow(unsafe_code)]
use super::{CharacterRow, SelectionAction};
use oteryn_foundation::CancellationToken;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_CLOSE {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 4);
            return 0;
        }
        if message == WM_COMMAND && (1..=5).contains(&(w & 0xffff)) {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (w & 0xffff) as isize);
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
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

pub(super) fn select(
    rows: &[CharacterRow],
    preselect: &str,
    cancellation: CancellationToken,
) -> SelectionAction {
    unsafe {
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            wide("STATIC").as_ptr(),
            wide(tr!("Oteryn — Select character")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            690,
            440,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return SelectionAction::Cancel;
        }
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        let label = control(
            hwnd,
            "STATIC",
            if rows.is_empty() {
                tr!(
                    "No characters on this account. Create a character in your browser, then refresh."
                )
            } else {
                tr!("Choose a character (name — world — availability) and optional channel.")
            },
            0,
            [15, 15, 650, 40],
            0,
        );
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            10,
            [15, 60, 650, 180],
            WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let channel_label = control(hwnd, "STATIC", tr!("Channel"), 0, [15, 250, 80, 25], 0);
        let channels = control(
            hwnd,
            "COMBOBOX",
            "",
            11,
            [100, 245, 400, 200],
            WS_TABSTOP | CBS_DROPDOWNLIST as u32,
        );
        let play = control(
            hwnd,
            "BUTTON",
            tr!("Play"),
            1,
            [15, 295, 100, 32],
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        );
        let create = control(
            hwnd,
            "BUTTON",
            tr!("Create character"),
            2,
            [130, 295, 160, 32],
            WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            tr!("Refresh"),
            3,
            [305, 295, 100, 32],
            WS_TABSTOP,
        );
        let cancel = control(
            hwnd,
            "BUTTON",
            tr!("Cancel"),
            4,
            [420, 295, 100, 32],
            WS_TABSTOP,
        );
        let services = control(
            hwnd,
            "BUTTON",
            tr!("Bazaar / account services"),
            5,
            [15, 345, 280, 32],
            WS_TABSTOP,
        );
        let train = control(
            hwnd,
            "BUTTON",
            tr!("Train selected character"),
            6,
            [310, 345, 280, 32],
            WS_TABSTOP,
        );
        if [
            train,
            label,
            list,
            channel_label,
            channels,
            play,
            create,
            refresh,
            cancel,
            services,
        ]
        .iter()
        .any(|handle| handle.is_null())
        {
            DestroyWindow(hwnd);
            return SelectionAction::Cancel;
        }
        for row in rows {
            let text = format!(
                "{} — {} — {}",
                row.character.name,
                row.world_name,
                if row.available {
                    "AVAILABLE"
                } else {
                    "UNAVAILABLE"
                }
            );
            SendMessageW(list, LB_ADDSTRING, 0, wide(&text).as_ptr() as LPARAM);
        }
        if let Some(index) = rows
            .iter()
            .position(|row| row.character.character_id == preselect)
        {
            SendMessageW(list, LB_SETCURSEL, index, 0);
        }
        let mut previous = -2;
        let mut message: MSG = std::mem::zeroed();
        let action = loop {
            if cancellation.is_cancelled() {
                break SelectionAction::Cancel;
            }
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let index = SendMessageW(list, LB_GETCURSEL, 0, 0);
            let row = usize::try_from(index)
                .ok()
                .and_then(|index| rows.get(index));
            if index != previous {
                SendMessageW(channels, CB_RESETCONTENT, 0, 0);
                SendMessageW(
                    channels,
                    CB_ADDSTRING,
                    0,
                    wide(tr!("Automatic")).as_ptr() as LPARAM,
                );
                if let Some(row) = row {
                    for (_, name) in &row.channels {
                        SendMessageW(channels, CB_ADDSTRING, 0, wide(name).as_ptr() as LPARAM);
                    }
                }
                SendMessageW(channels, CB_SETCURSEL, 0, 0);
                previous = index;
            }
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                play,
                i32::from(row.is_some_and(|row| row.available)),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                train,
                i32::from(row.is_some_and(|row| row.available)),
            );
            let command = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            match command {
                1 | 6 => {
                    if let Some(row) = row.filter(|row| row.available) {
                        let selected = SendMessageW(channels, CB_GETCURSEL, 0, 0);
                        let channel_id = usize::try_from(selected)
                            .ok()
                            .and_then(|index| index.checked_sub(1))
                            .and_then(|index| row.channels.get(index))
                            .map(|channel| channel.0.clone());
                        let character_id = row.character.character_id.clone();
                        break if command == 6 {
                            SelectionAction::Train {
                                character_id,
                                channel_id,
                            }
                        } else {
                            SelectionAction::Play {
                                character_id,
                                channel_id,
                            }
                        };
                    }
                }
                2 => break SelectionAction::Create,
                3 => break SelectionAction::Refresh,
                4 => break SelectionAction::Cancel,
                5 => break SelectionAction::AccountServices,
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        };
        DestroyWindow(hwnd);
        action
    }
}
