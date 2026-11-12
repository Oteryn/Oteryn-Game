//! Local marker editor. Window handles and action storage remain on the creating thread.
#![allow(unsafe_code)]
use crate::play::feature_local_minimap::state::{SharedMinimap, lock};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};
#[derive(Default)]
struct Actions(Vec<(usize, usize)>);
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 64 {
                (*actions).0.push((w & 0xffff, (w >> 16) & 0xffff));
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn open(shared: SharedMinimap, own: (i32, i32, i16)) {
    {
        let mut map = lock(&shared);
        if map.editor_open {
            return;
        }
        map.editor_open = true;
    }
    let failed = shared.clone();
    if std::thread::Builder::new()
        .name("minimap-marker-editor".into())
        .spawn(move || run(shared, own))
        .is_err()
    {
        lock(&failed).editor_open = false;
    }
}
fn run(shared: SharedMinimap, own: (i32, i32, i16)) {
    // SAFETY: controls copy strings; Actions outlives the HWND and all dispatch stays here.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Minimap and markers — Ctrl+Alt+Shift+M")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            640,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            lock(&shared).editor_open = false;
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        for (i, label) in [
            tr!("Zoom in"),
            tr!("Zoom out"),
            tr!("Floor up"),
            tr!("Floor down"),
            tr!("Recenter"),
            tr!("West"),
            tr!("East"),
            tr!("North"),
            tr!("South"),
            tr!("Use map centre"),
        ]
        .iter()
        .enumerate()
        {
            control(
                h,
                "BUTTON",
                label,
                100 + i,
                [10 + (i % 5) as i32 * 140, 10 + (i / 5) as i32 * 34, 130, 28],
                0,
            );
        }
        let status = control(h, "STATIC", "", 300, [10, 85, 690, 48], 0);
        let list = control(
            h,
            "LISTBOX",
            "",
            207,
            [10, 140, 690, 230],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let mut fields = Vec::new();
        for (i, (label, value)) in [
            ("X", own.0.to_string()),
            ("Y", own.1.to_string()),
            (tr!("Floor"), own.2.to_string()),
            (tr!("Label"), String::new()),
        ]
        .iter()
        .enumerate()
        {
            let y = 385 + i as i32 * 32;
            control(h, "STATIC", label, 0, [10, y, 75, 25], 0);
            let field = control(
                h,
                "EDIT",
                value,
                201 + i,
                [90, y, 350, 25],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(field, EM_SETLIMITTEXT, if i == 3 { 128 } else { 8 }, 0);
            fields.push(field);
        }
        let color = control(
            h,
            "COMBOBOX",
            "",
            205,
            [460, 385, 230, 160],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for name in [tr!("Red"), tr!("Blue"), tr!("Green"), tr!("Gold")] {
            SendMessageW(color, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
        }
        SendMessageW(color, CB_SETCURSEL, 0, 0);
        control(h, "BUTTON", tr!("Save marker"), 110, [460, 425, 230, 28], 0);
        control(
            h,
            "BUTTON",
            tr!("Delete selected"),
            111,
            [460, 460, 230, 28],
            0,
        );
        control(
            h,
            "BUTTON",
            tr!("Go to selected"),
            112,
            [460, 495, 230, 28],
            0,
        );
        let feedback = control(h, "STATIC", "", 301, [10, 530, 690, 55], 0);
        if fields
            .iter()
            .chain([&status, &list, &color, &feedback])
            .any(|h| h.is_null())
        {
            DestroyWindow(h);
            lock(&shared).editor_open = false;
            return;
        }
        let mut held: Vec<crate::play::feature_local_minimap::state::Marker> = Vec::new();
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if IsWindow(h) == 0 {
                break;
            }
            let chosen = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| held.get(i))
                .cloned();
            let values: Vec<_> = fields.iter().map(|h| text(*h)).collect();
            let selected_color = u8::try_from(SendMessageW(color, CB_GETCURSEL, 0, 0)).unwrap_or(0);
            let mut map = lock(&shared);
            for (id, notification) in actions.0.drain(..) {
                if id == 207 && notification == LBN_SELCHANGE as usize {
                    if let Some(marker) = &chosen {
                        for (field, value) in fields.iter().zip([
                            marker.x.to_string(),
                            marker.y.to_string(),
                            marker.floor.to_string(),
                            marker.label.clone(),
                        ]) {
                            SetWindowTextW(*field, wide(&value).as_ptr());
                        }
                        SendMessageW(color, CB_SETCURSEL, usize::from(marker.color), 0);
                    }
                    continue;
                }
                if notification != BN_CLICKED as usize {
                    continue;
                }
                match id {
                    100 => map.zoom = oteryn_client_ui::minimap::zoom_in(map.zoom),
                    101 => map.zoom = oteryn_client_ui::minimap::zoom_out(map.zoom),
                    102 => {
                        map.tracking = false;
                        map.floor = oteryn_client_ui::minimap::floor_up(map.floor);
                    }
                    103 => {
                        map.tracking = false;
                        map.floor = oteryn_client_ui::minimap::floor_down(map.floor);
                    }
                    104 => {
                        map.tracking = true;
                        let (x, y, f) = map.own;
                        map.follow(x, y, f);
                    }
                    105 => map.pan(-5, 0),
                    106 => map.pan(5, 0),
                    107 => map.pan(0, -5),
                    108 => map.pan(0, 5),
                    109 => {
                        for (field, value) in fields.iter().zip([
                            map.x.to_string(),
                            map.y.to_string(),
                            map.floor.to_string(),
                        ]) {
                            SetWindowTextW(*field, wide(&value).as_ptr());
                        }
                    }
                    110 => {
                        let position = values[0]
                            .parse::<i32>()
                            .ok()
                            .zip(values[1].parse::<i32>().ok())
                            .zip(values[2].parse::<i16>().ok());
                        map.feedback = match position {
                            Some(((x, y), f)) => {
                                match map.edit_marker(x, y, f, values[3].clone(), selected_color) {
                                    Ok(()) => tr!("Marker saved locally").into(),
                                    Err(e) => e.to_string(),
                                }
                            }
                            None => tr!("Enter valid X, Y and floor values").into(),
                        };
                    }
                    111 => {
                        if let Some(marker) = &chosen {
                            map.feedback = match map.remove_marker(marker.x, marker.y, marker.floor)
                            {
                                Ok(()) => tr!("Marker deleted").into(),
                                Err(e) => e.to_string(),
                            };
                        }
                    }
                    112 => {
                        if let Some(marker) = &chosen {
                            map.tracking = false;
                            map.x = marker.x;
                            map.y = marker.y;
                            map.floor = marker.floor;
                        }
                    }
                    _ => {}
                }
            }
            SetWindowTextW(status, wide(&lfmt!("Map centre: {}, {} · Floor {} · {} tiles per cell\r\nGreen: remembered ground. Select a marker to edit its label.", map.x, map.y, map.floor, map.zoom)).as_ptr());
            SetWindowTextW(feedback, wide(&map.feedback).as_ptr());
            let displayed = map.displayed_markers();
            if held != displayed {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for marker in &displayed {
                    SendMessageW(
                        list,
                        LB_ADDSTRING,
                        0,
                        wide(&lfmt!(
                            "{} · {}, {} · floor {}",
                            marker.label,
                            marker.x,
                            marker.y,
                            marker.floor
                        ))
                        .as_ptr() as isize,
                    );
                }
                held = displayed;
            }
            drop(map);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        let mut map = lock(&shared);
        map.editor_open = false;
        if let Err(error) = map.save() {
            map.feedback = error.to_string();
        }
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    label: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            r[0],
            r[1],
            r[2],
            r[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h).max(0);
        let mut buffer = vec![0; n as usize + 1];
        let read = GetWindowTextW(h, buffer.as_mut_ptr(), n + 1).max(0) as usize;
        String::from_utf16_lossy(&buffer[..read])
    }
}
