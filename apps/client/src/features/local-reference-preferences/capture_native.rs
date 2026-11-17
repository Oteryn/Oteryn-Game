//! Screenshot settings dialog. Child controls and callback state stay on their UI thread.
#![allow(
    unsafe_code,
    reason = "Win32 handles and callback storage stay on their creating thread"
)]
use super::{Shared, control, lock, text, wide};
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

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 1801 && w >> 16 == BN_CLICKED as usize {
            let pending = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut bool;
            if !pending.is_null() {
                *pending = true;
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
            wide("Screenshots").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
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
        let mut pending = Box::new(false);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *pending as *mut bool) as isize);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let label = control(
            hwnd,
            "STATIC",
            "Save folder (absolute path; blank uses the default folder)",
            0,
            [16, 16, 660, 24],
            0,
        );
        let folder = control(
            hwnd,
            "EDIT",
            "",
            1802,
            [16, 48, 660, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(folder, EM_SETLIMITTEXT, 1024, 0);
        let game = control(
            hwnd,
            "BUTTON",
            "Game map only (otherwise the entire client window)",
            1803,
            [16, 90, 660, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let events = [
            "Capture on level increase",
            "Capture on death",
            "Capture on achievement",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| {
            control(
                hwnd,
                "BUTTON",
                label,
                1804 + i,
                [16, 130 + i as i32 * 30, 660, 26],
                BS_AUTOCHECKBOX as u32,
            )
        });
        let events: Vec<HWND> = events.collect();
        let backlog_label = control(
            hwnd,
            "STATIC",
            "Pre-event backlog in seconds (0 disables; maximum 10)",
            0,
            [16, 230, 660, 24],
            0,
        );
        let backlog = control(
            hwnd,
            "EDIT",
            "0",
            1807,
            [16, 260, 100, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        SendMessageW(backlog, EM_SETLIMITTEXT, 2, 0);
        let apply = control(hwnd, "BUTTON", "Apply", 1801, [516, 260, 160, 28], 0);
        let info = control(
            hwnd,
            "STATIC",
            "Print Screen saves a BMP capture. Automatic captures use confirmed level, death and achievement events. Backlog keeps one frame per second, up to 64 MiB; high resolutions may retain a shorter history. Captures can contain visible player information.",
            0,
            [16, 310, 660, 72],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 390, 660, 56], 0);
        if [
            label,
            folder,
            game,
            backlog_label,
            backlog,
            apply,
            info,
            feedback,
        ]
        .iter()
        .chain(events.iter())
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        let mut held = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !lock(&shared).shutdown {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let mut state = lock(&shared);
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != state.character_scope())
            {
                *pending = false;
            }
            if std::mem::take(&mut *pending) {
                let mut prefs = state.preferences.clone();
                prefs.screenshots.directory = text(folder);
                prefs.screenshots.game_only = SendMessageW(game, BM_GETCHECK, 0, 0) == 1;
                prefs.screenshots.on_level = SendMessageW(events[0], BM_GETCHECK, 0, 0) == 1;
                prefs.screenshots.on_death = SendMessageW(events[1], BM_GETCHECK, 0, 0) == 1;
                prefs.screenshots.on_achievement = SendMessageW(events[2], BM_GETCHECK, 0, 0) == 1;
                match text(backlog).parse::<u8>() {
                    Ok(seconds) => {
                        prefs.screenshots.backlog_seconds = seconds;
                        prefs.screenshots =
                            prefs.screenshots.reconciled(&state.preferences.screenshots);
                        if prefs.screenshots.valid() {
                            state.apply(prefs);
                        } else {
                            state.feedback = "Choose an absolute folder (or blank default), and backlog from 0 to 10 seconds.".into();
                        }
                    }
                    Err(_) => {
                        state.feedback = "Enter whole-number backlog seconds from 0 to 10.".into()
                    }
                }
            }
            let snapshot = (
                state.character_scope(),
                state.preferences.screenshots.effective(),
            );
            if held.as_ref() != Some(&snapshot) {
                SetWindowTextW(folder, wide(&snapshot.1.directory).as_ptr());
                SendMessageW(game, BM_SETCHECK, usize::from(snapshot.1.game_only), 0);
                for (handle, enabled) in events.iter().zip([
                    snapshot.1.on_level,
                    snapshot.1.on_death,
                    snapshot.1.on_achievement,
                ]) {
                    SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
                }
                SetWindowTextW(
                    backlog,
                    wide(&snapshot.1.backlog_seconds.to_string()).as_ptr(),
                );
                held = Some(snapshot);
            }
            SetWindowTextW(
                feedback,
                wide(&format!("{}\r\n{}", state.scope_label(), state.feedback)).as_ptr(),
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
        SetForegroundWindow(parent);
    }
}
