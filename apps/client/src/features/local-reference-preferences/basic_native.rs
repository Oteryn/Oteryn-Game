//! Grouped Basic Options editor. HWNDs and callback storage stay on this dialog thread.
#![allow(
    unsafe_code,
    reason = "Win32 controls are accessed only on their creating thread"
)]
use super::{Shared, control, lock, text, wide};
#[path = "arcs_options.rs"]
mod arcs_window;
#[derive(Default)]
struct Pending {
    apply: bool,
    arcs: bool,
}
use oteryn_client::play::{BASIC_GROUPS, BasicOptions, MousePreset, MovementKeys};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::{BST_CHECKED, EM_SETLIMITTEXT},
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::*,
    },
};
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && matches!(w & 0xffff, 901 | 909) && w >> 16 == BN_CLICKED as usize {
            let pending = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Pending;
            if !pending.is_null() {
                if w & 0xffff == 901 {
                    (*pending).apply = true;
                } else {
                    (*pending).arcs = true;
                }
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
            wide("Oteryn Basic Options").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            730,
            575,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut pending = Box::<Pending>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *pending as *mut Pending) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let mut controls = Vec::new();
        for (name, y) in BASIC_GROUPS.into_iter().zip([12, 127, 224, 338]) {
            controls.push(control(
                hwnd,
                "BUTTON",
                name,
                0,
                [16, y, 680, 95],
                BS_GROUPBOX as u32,
            ));
        }
        controls.push(control(
            hwnd,
            "STATIC",
            "Movement keys                         Mouse preset",
            0,
            [34, 38, 640, 22],
            0,
        ));
        let movement = control(
            hwnd,
            "COMBOBOX",
            "",
            902,
            [34, 65, 290, 150],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let mouse = control(
            hwnd,
            "COMBOBOX",
            "",
            903,
            [350, 65, 320, 120],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        controls.push(control(
            hwnd,
            "STATIC",
            "World zoom (80–180%, fitted to the game window)",
            0,
            [34, 156, 450, 28],
            0,
        ));
        controls.push(control(
            hwnd,
            "BUTTON",
            "Health and mana arcs...",
            909,
            [34, 188, 320, 25],
            0,
        ));
        let zoom = control(
            hwnd,
            "EDIT",
            "",
            904,
            [540, 153, 130, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        controls.push(control(
            hwnd,
            "STATIC",
            "Foreground FPS (1–360)             Background FPS (1–360)",
            0,
            [34, 250, 640, 22],
            0,
        ));
        let foreground = control(
            hwnd,
            "EDIT",
            "",
            905,
            [34, 277, 290, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let background = control(
            hwnd,
            "EDIT",
            "",
            906,
            [350, 277, 320, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        controls.push(control(
            hwnd,
            "STATIC",
            "Master volume (0–100%)",
            0,
            [34, 370, 300, 28],
            0,
        ));
        let volume = control(
            hwnd,
            "EDIT",
            "",
            907,
            [350, 366, 130, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let mute = control(
            hwnd,
            "BUTTON",
            tr!("Mute"),
            908,
            [540, 366, 130, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let apply = control(
            hwnd,
            "BUTTON",
            "Apply all groups",
            901,
            [480, 447, 210, 30],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 486, 680, 50], 0);
        controls.extend([
            movement, mouse, zoom, foreground, background, volume, mute, apply, feedback,
        ]);
        if controls.iter().any(|h| h.is_null()) {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        for field in [zoom, foreground, background, volume] {
            SendMessageW(field, EM_SETLIMITTEXT, 3, 0);
        }
        for (field, labels) in [
            (movement, ["Arrows", "Arrows and WASD (chat closed)"]),
            (mouse, ["Standard", "Left-handed"]),
        ] {
            for label in labels {
                SendMessageW(field, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
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
            let snapshot = (
                state.character_scope(),
                BasicOptions::from(&state.preferences),
            );
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != snapshot.0)
            {
                pending.apply = false;
            }
            if std::mem::take(&mut pending.arcs) {
                drop(state);
                arcs_window::open(Arc::clone(&shared), hwnd);
                state = lock(&shared);
                pending.apply = false;
                held = None;
            }
            if std::mem::take(&mut pending.apply) {
                match (
                    text(zoom).parse::<u16>(),
                    text(foreground).parse::<u16>(),
                    text(background).parse::<u16>(),
                    text(volume).parse::<u8>(),
                ) {
                    (Ok(world_zoom), Ok(foreground_fps), Ok(background_fps), Ok(master_volume)) => {
                        let options = BasicOptions {
                            world_zoom,
                            foreground_fps,
                            background_fps,
                            master_volume,
                            movement: if SendMessageW(movement, CB_GETCURSEL, 0, 0) == 1 {
                                MovementKeys::ArrowsAndWasd
                            } else {
                                MovementKeys::Arrows
                            },
                            mouse: if SendMessageW(mouse, CB_GETCURSEL, 0, 0) == 1 {
                                MousePreset::LeftHanded
                            } else {
                                MousePreset::Standard
                            },
                            muted: SendMessageW(mute, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                        };
                        if let Err(error) = state.apply_basic(&options) {
                            state.feedback = format!("{error}. Check zoom, FPS and volume ranges.");
                        }
                    }
                    _ => {
                        state.feedback =
                            "Enter whole numbers in the displayed ranges. No changes were applied."
                                .into()
                    }
                }
            }
            let snapshot = (
                state.character_scope(),
                BasicOptions::from(&state.preferences),
            );
            if held.as_ref() != Some(&snapshot) {
                let p = snapshot.1;
                SendMessageW(
                    movement,
                    CB_SETCURSEL,
                    usize::from(p.movement == MovementKeys::ArrowsAndWasd),
                    0,
                );
                SendMessageW(
                    mouse,
                    CB_SETCURSEL,
                    usize::from(p.mouse == MousePreset::LeftHanded),
                    0,
                );
                for (field, value) in [
                    (zoom, p.world_zoom),
                    (foreground, p.foreground_fps),
                    (background, p.background_fps),
                    (volume, u16::from(p.master_volume)),
                ] {
                    SetWindowTextW(field, wide(&value.to_string()).as_ptr());
                }
                SendMessageW(mute, BM_SETCHECK, usize::from(p.muted), 0);
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
