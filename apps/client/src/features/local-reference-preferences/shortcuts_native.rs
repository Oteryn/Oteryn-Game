//! Available/displayed shortcut lists, edited through the validated preference owner.
#![allow(unsafe_code, reason = "Win32 controls remain on the settings thread")]
use super::{Shared, control, lock, wide};
use oteryn_client::play::SHORTCUTS;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && matches!(w & 0xffff, 803..=806) && w >> 16 == BN_CLICKED as usize {
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
            wide("Shortcut buttons").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            500,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let label = control(
            hwnd,
            "STATIC",
            "Available buttons                                  Displayed buttons (in order)",
            0,
            [16, 16, 680, 30],
            0,
        );
        let available = control(
            hwnd,
            "LISTBOX",
            "",
            801,
            [16, 50, 310, 280],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let displayed = control(
            hwnd,
            "LISTBOX",
            "",
            802,
            [370, 50, 310, 280],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let add = control(hwnd, "BUTTON", "Add", 803, [16, 345, 145, 30], 0);
        let remove = control(hwnd, "BUTTON", "Remove", 804, [175, 345, 145, 30], 0);
        let up = control(hwnd, "BUTTON", "Move up", 805, [370, 345, 145, 30], 0);
        let down = control(hwnd, "BUTTON", "Move down", 806, [530, 345, 145, 30], 0);
        let status = control(hwnd, "STATIC", "", 0, [16, 395, 680, 50], 0);
        if [label, available, displayed, add, remove, up, down, status]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        let mut held: Option<(Option<[u8; 16]>, Vec<String>)> = None;
        let mut candidates: Vec<&str> = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !lock(&shared).shutdown {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let mut state = lock(&shared);
            if held.as_ref().is_some_and(|old| {
                old.0 != state.character_scope() || old.1 != state.preferences.shortcuts
            }) {
                actions.0.clear();
            }
            let mut next_selection = None;
            if let Some(action) = std::mem::take(&mut actions.0).into_iter().next() {
                let row = usize::try_from(SendMessageW(displayed, LB_GETCURSEL, 0, 0)).ok();
                let mut prefs = state.preferences.clone();
                let result = match action {
                    803 => usize::try_from(SendMessageW(available, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|i| candidates.get(i))
                        .map(|id| prefs.add_shortcut(id)),
                    804 => row.map(|row| prefs.remove_shortcut(row)),
                    805 => row
                        .and_then(|row| row.checked_sub(1).map(|to| prefs.move_shortcut(row, to))),
                    806 => row.map(|row| prefs.move_shortcut(row, row.saturating_add(1))),
                    _ => None,
                };
                match result {
                    Some(Ok(())) => {
                        next_selection = match action {
                            803 => Some(prefs.shortcuts.len().saturating_sub(1)),
                            805 => row.and_then(|row| row.checked_sub(1)),
                            806 => row.map(|row| row.saturating_add(1)),
                            _ => row,
                        };
                        state.apply(prefs);
                    }
                    Some(Err(error)) => state.feedback = error.to_string(),
                    None => state.feedback = "Select an available or displayed button first".into(),
                }
                // Rebuild before another queued action can use stale row indices.
            }
            let snapshot = (state.character_scope(), state.preferences.shortcuts.clone());
            if held.as_ref() != Some(&snapshot) {
                let row = SendMessageW(displayed, LB_GETCURSEL, 0, 0);
                SendMessageW(available, LB_RESETCONTENT, 0, 0);
                SendMessageW(displayed, LB_RESETCONTENT, 0, 0);
                candidates.clear();
                for &(id, label) in SHORTCUTS {
                    if !snapshot.1.iter().any(|shown| shown == id) {
                        candidates.push(id);
                        SendMessageW(available, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                    }
                }
                for id in &snapshot.1 {
                    if let Some((_, label)) = SHORTCUTS.iter().find(|(known, _)| known == id) {
                        SendMessageW(displayed, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                    }
                }
                SendMessageW(available, LB_SETCURSEL, 0, 0);
                SendMessageW(
                    displayed,
                    LB_SETCURSEL,
                    next_selection
                        .unwrap_or_else(|| usize::try_from(row).unwrap_or(0))
                        .min(snapshot.1.len().saturating_sub(1)),
                    0,
                );
                held = Some(snapshot);
            }
            SetWindowTextW(status, wide(&state.feedback).as_ptr());
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
    }
}
