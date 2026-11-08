//! Minimap transfer and marker editing on the settings window thread.
#![allow(
    unsafe_code,
    reason = "Win32 controls and callback data stay on this UI thread"
)]
use super::{Shared, control, lock, text, wide};
use oteryn_client::play::{MinimapMark, MinimapMemory};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND
            && matches!(w & 0xffff, 902 | 903 | 909 | 910)
            && w >> 16 == BN_CLICKED as usize
        {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn open(shared: Arc<Mutex<Shared>>, parent: HWND) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Minimap transfer and markers").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            780,
            620,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let help = control(
            hwnd,
            "STATIC",
            "Local observations only: remembered terrain and named markers never supply world geometry.\r\nFiles preserve world/channel/content scope. A different scope is hidden during play.\r\nMemory keeps the latest 8,192 tiles and up to 128 markers; older tiles are discarded.",
            0,
            [16, 12, 720, 65],
            0,
        );
        let path = control(
            hwnd,
            "EDIT",
            "",
            901,
            [16, 90, 720, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(path, EM_SETLIMITTEXT, 240, 0);
        let import = control(hwnd, "BUTTON", "Import map...", 902, [16, 130, 220, 30], 0);
        let export = control(
            hwnd,
            "BUTTON",
            "Export new map file",
            903,
            [250, 130, 240, 30],
            0,
        );
        let marks = control(
            hwnd,
            "LISTBOX",
            "",
            904,
            [16, 180, 720, 180],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let label = control(
            hwnd,
            "STATIC",
            "Marker: X / Y / Floor / Label",
            0,
            [16, 375, 720, 24],
            0,
        );
        let x = control(
            hwnd,
            "EDIT",
            "",
            905,
            [16, 405, 110, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let y = control(
            hwnd,
            "EDIT",
            "",
            906,
            [140, 405, 110, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let floor = control(
            hwnd,
            "EDIT",
            "",
            907,
            [265, 405, 85, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let name = control(
            hwnd,
            "EDIT",
            "",
            908,
            [365, 405, 370, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(name, EM_SETLIMITTEXT, 64, 0);
        let add = control(hwnd, "BUTTON", "Add marker", 909, [16, 445, 220, 30], 0);
        let remove = control(
            hwnd,
            "BUTTON",
            "Remove selected marker",
            910,
            [250, 445, 300, 30],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 495, 720, 65], 0);
        if [
            help, path, import, export, marks, label, x, y, floor, name, add, remove, feedback,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        let mut shown = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !lock(&shared).shutdown {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let mut state = lock(&shared);
            let snapshot = (state.character_scope(), state.preferences.minimap.clone());
            if shown
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, MinimapMemory)| {
                    old.0 != snapshot.0
                        || old.1.binding != snapshot.1.binding
                        || old.1.marks != snapshot.1.marks
                })
            {
                actions.0.clear();
            }
            if let Some(action) = std::mem::take(&mut actions.0).into_iter().next() {
                let file = std::path::PathBuf::from(text(path));
                if action == 903 {
                    state.feedback = match state.preferences.minimap.export_new(&file) {
                        Ok(()) => "Minimap exported. Existing files are never overwritten.".into(),
                        Err(error) => format!("Export failed: {error}"),
                    };
                } else if action == 902 {
                    let scope = state.character_scope();
                    drop(state);
                    let imported = MinimapMemory::import(&file);
                    let confirmed = imported.is_ok() && MessageBoxW(hwnd,wide("Replace remembered map tiles and markers in this character's settings?").as_ptr(),wide("Minimap import").as_ptr(),MB_YESNO | MB_ICONQUESTION) == IDYES;
                    state = lock(&shared);
                    match imported {
                        Ok(map) if confirmed && state.character_scope() == scope => {
                            let mut prefs = state.preferences.clone();
                            prefs.minimap = map;
                            state.apply(prefs);
                        }
                        Err(error) => state.feedback = format!("Import failed: {error}"),
                        _ => {}
                    }
                } else {
                    let mut prefs = state.preferences.clone();
                    let result = if action == 909 {
                        match (
                            text(x).parse::<i32>(),
                            text(y).parse::<i32>(),
                            text(floor).parse::<i16>(),
                        ) {
                            (Ok(x), Ok(y), Ok(floor)) => prefs.minimap.add_mark(MinimapMark {
                                x,
                                y,
                                floor,
                                label: text(name),
                            }),
                            _ => Err(std::io::Error::other("Enter integer X, Y and floor values")),
                        }
                    } else {
                        usize::try_from(SendMessageW(marks, LB_GETCURSEL, 0, 0))
                            .ok()
                            .filter(|row| prefs.minimap.remove_mark(*row))
                            .map_or_else(
                                || Err(std::io::Error::other("Select a marker to remove")),
                                |_| Ok(()),
                            )
                    };
                    match result {
                        Ok(()) => state.apply(prefs),
                        Err(error) => state.feedback = error.to_string(),
                    }
                }
            }
            let snapshot = (state.character_scope(), state.preferences.minimap.clone());
            if shown.as_ref() != Some(&snapshot) {
                let row = SendMessageW(marks, LB_GETCURSEL, 0, 0);
                SendMessageW(marks, LB_RESETCONTENT, 0, 0);
                for mark in &snapshot.1.marks {
                    let label = format!(
                        "{}, {} / floor {}: {}",
                        mark.x, mark.y, mark.floor, mark.label
                    );
                    SendMessageW(marks, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                SendMessageW(marks, LB_SETCURSEL, usize::try_from(row).unwrap_or(0), 0);
                shown = Some(snapshot);
            }
            SetWindowTextW(
                feedback,
                wide(&format!(
                    "{} remembered tiles. {}",
                    state.preferences.minimap.tiles.len(),
                    state.feedback
                ))
                .as_ptr(),
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
    }
}
