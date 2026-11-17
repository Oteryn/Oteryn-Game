//! Window-thread owned title selector over the server's confirmed self profile.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::character_info::{Intent, TitleChoice, TitleFilter, title_rows};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
fn w(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 32 {
                (*a).0.push(wp & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, wp, lp)
    }
}
pub(in crate::play) fn open_titles(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.properties.character_info.titles_open {
            return;
        }
        m.properties.character_info.titles_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("character-titles".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
type Shared = Weak<Mutex<Mailbox>>;
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: HWNDs, userdata and destruction remain on the creating thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            w("STATIC").as_ptr(),
            w(tr!("Character titles — Ctrl+Alt+Shift+Y")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            560,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let search = ctl(
            h,
            "EDIT",
            "",
            201,
            [10, 10, 360, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let filter = ctl(
            h,
            "COMBOBOX",
            "",
            202,
            [380, 10, 320, 160],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in [
            tr!("All titles"),
            tr!("Earned"),
            tr!("Locked"),
            tr!("Permanent"),
            tr!("Temporary"),
        ] {
            SendMessageW(filter, CB_ADDSTRING, 0, w(label).as_ptr() as isize);
        }
        SendMessageW(filter, CB_SETCURSEL, 0, 0);
        let list = ctl(
            h,
            "LISTBOX",
            "",
            203,
            [10, 50, 690, 300],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let status = ctl(h, "STATIC", "", 204, [10, 360, 690, 95], 0);
        let refresh = ctl(h, "BUTTON", tr!("Refresh"), 100, [10, 465, 160, 28], 0);
        let unlock = ctl(
            h,
            "BUTTON",
            tr!("Unlock selected"),
            101,
            [185, 465, 160, 28],
            0,
        );
        let select = ctl(
            h,
            "BUTTON",
            tr!("Use selected"),
            102,
            [360, 465, 160, 28],
            0,
        );
        let clear = ctl(
            h,
            "BUTTON",
            tr!("Clear selection"),
            103,
            [535, 465, 160, 28],
            0,
        );
        if [search, filter, list, status, refresh, unlock, select, clear]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(h);
            close(&weak);
            return;
        }
        let mut held: Vec<TitleChoice> = Vec::new();
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let chosen = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| held.get(i))
                .cloned();
            let search_value = text(search);
            let filter_value = match SendMessageW(filter, CB_GETCURSEL, 0, 0) {
                1 => TitleFilter::Earned,
                2 => TitleFilter::Locked,
                3 => TitleFilter::Permanent,
                4 => TitleFilter::Temporary,
                _ => TitleFilter::All,
            };
            let p = {
                let mut m = lock(&shared);
                let p = &mut m.properties.character_info;
                p.title_search = search_value;
                p.title_filter = filter_value;
                for id in actions.0.drain(..) {
                    if m.properties.character_info.pending {
                        continue;
                    }
                    let intent = match id {
                        100 => {
                            let p = &mut m.properties.character_info;
                            p.fields[0] = "0".into();
                            p.fields[1] = "1".into();
                            p.fields[2].clear();
                            Intent::Query
                        }
                        101 | 102 => {
                            let Some(row) = &chosen else { continue };
                            m.properties.character_info.fields[3] = row.title.id().to_string();
                            if id == 101 {
                                Intent::Unlock
                            } else {
                                Intent::Select
                            }
                        }
                        103 => {
                            m.properties.character_info.fields[3] = "0".into();
                            Intent::Select
                        }
                        _ => continue,
                    };
                    super::super::input::submit(&mut m, &commands, intent);
                }
                m.properties.character_info.clone()
            };
            let rows = title_rows(&p);
            if rows != held {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &rows {
                    let label = format!(
                        "{} — {}{}",
                        row.title.name(),
                        if row.earned {
                            tr!("Earned")
                        } else {
                            tr!("Locked")
                        },
                        if row.selected {
                            tr!(" — Selected")
                        } else {
                            ""
                        }
                    );
                    SendMessageW(list, LB_ADDSTRING, 0, w(&label).as_ptr() as isize);
                }
                held = rows;
            }
            let current = p
                .own_titles
                .as_ref()
                .and_then(|o| o.selected)
                .map_or(tr!("None"), |t| t.name());
            let message = if p.pending {
                tr!("Waiting for the server")
            } else if !p.selected {
                tr!("Feature unavailable in this session")
            } else if p.own_titles.is_none() {
                tr!("Refresh to load your earned titles")
            } else {
                &p.feedback
            };
            let selected_info =
                chosen
                    .as_ref()
                    .map_or(tr!("Select a title to unlock or use it").into(), |r| {
                        lfmt!(
                            "{} · {}. The server checks unlock requirements.",
                            r.title.name(),
                            if r.title.permanent() {
                                tr!("Permanent")
                            } else {
                                tr!("Temporary")
                            }
                        )
                    });
            SetWindowTextW(
                status,
                w(&lfmt!(
                    "Current title: {current}\r\n{selected_info}\r\n{message}",
                    current = current,
                    message = message,
                    selected_info = selected_info
                ))
                .as_ptr(),
            );
            let ready = p.selected && !p.pending;
            EnableWindow(refresh, i32::from(ready));
            EnableWindow(clear, i32::from(ready && p.own_titles.is_some()));
            EnableWindow(
                unlock,
                i32::from(ready && chosen.as_ref().is_some_and(|r| !r.earned)),
            );
            EnableWindow(
                select,
                i32::from(ready && chosen.as_ref().is_some_and(|r| r.earned)),
            );
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak);
    }
}
fn close(weak: &Shared) {
    if let Some(m) = weak.upgrade() {
        lock(&m).properties.character_info.titles_open = false;
    }
}
unsafe fn ctl(h: HWND, c: &str, t: &str, id: usize, r: [i32; 4], s: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            w(c).as_ptr(),
            w(t).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | s,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h).max(0);
        let mut b = vec![0; n as usize + 1];
        let n = GetWindowTextW(h, b.as_mut_ptr(), n + 1).max(0) as usize;
        String::from_utf16_lossy(&b[..n])
    }
}
