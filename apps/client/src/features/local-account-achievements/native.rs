//! Account-wide achievement browser over validated server pages. HWNDs stay on their thread.
#![allow(unsafe_code)]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::account_achievements::{
    AchievementFilter, AchievementSort, Intent, catalog_next_request, catalog_rows, request,
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the window thread keeps Actions alive until the HWND is destroyed.
    unsafe {
        if m == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, m, w, l)
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[path = "view/backend.rs"]
mod backend;
use backend::Backend;
pub(in crate::play) fn open_window(link: &PlayLink) {
    open_shared(
        Arc::downgrade(&link.mailbox),
        link.commands.clone(),
        Backend::Account,
    );
}
pub(in crate::play) fn open_character(
    weak: Shared,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    let Some(shared) = weak.upgrade() else {
        return;
    };
    let generation = lock(&shared).cyclopedia_character.profile_generation;
    open_shared(weak, commands, Backend::Character(generation));
}
fn open_shared(
    weak: Shared,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    backend: Backend,
) {
    {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        let mut m = lock(&shared);
        if !backend.valid(&m) || !backend.ready(&m) || backend.panel(&mut m).open {
            return;
        }
        backend.panel(&mut m).open = true;
        if let Some(command) = backend.request(&mut m, Intent::Refresh)
            && commands.send(command).is_err()
        {
            backend.fail(&mut m);
        }
    }
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("achievement-browser".into())
        .spawn(move || run(weak, commands, backend))
        .is_err()
    {
        close(&failed, backend);
    }
}
type Shared = Weak<Mutex<Mailbox>>;
fn run(weak: Shared, commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>, backend: Backend) {
    // SAFETY: handles and action storage remain on this thread until after DestroyWindow.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Account achievements — Ctrl+Shift+A").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            640,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, backend);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let search = ctl(
            h,
            "EDIT",
            "",
            10,
            [10, 10, 330, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let filter = ctl(
            h,
            "COMBOBOX",
            "",
            11,
            [350, 10, 180, 150],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in ["All achievements", "Regular", "Secret"] {
            SendMessageW(filter, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(filter, CB_SETCURSEL, 0, 0);
        let sort = ctl(
            h,
            "COMBOBOX",
            "",
            12,
            [540, 10, 230, 150],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in ["Name", "Points", "Grade", "Recently earned"] {
            SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(sort, CB_SETCURSEL, 0, 0);
        let header = ctl(h, "STATIC", "", 13, [10, 48, 760, 45], 0);
        let list = ctl(
            h,
            "LISTBOX",
            "",
            14,
            [10, 100, 760, 275],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let detail = ctl(
            h,
            "EDIT",
            "",
            15,
            [10, 385, 760, 140],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let refresh = ctl(
            h,
            "BUTTON",
            "Load / refresh account",
            2,
            [10, 540, 240, 32],
            0,
        );
        let previous = ctl(h, "BUTTON", "Previous page", 3, [270, 540, 240, 32], 0);
        let next = ctl(h, "BUTTON", "Next page", 4, [530, 540, 240, 32], 0);
        if [
            search, filter, sort, header, list, detail, refresh, previous, next,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(h);
            close(&weak, backend);
            return;
        }
        let mut held = None;
        let mut shown: Vec<oteryn_client_runtime::account_achievements::AccountAchievementRow> =
            Vec::new();
        let mut matching = 0;
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
            let search_value = text(search);
            let filter_value = match SendMessageW(filter, CB_GETCURSEL, 0, 0) {
                1 => AchievementFilter::Regular,
                2 => AchievementFilter::Secret,
                _ => AchievementFilter::All,
            };
            let sort_value = match SendMessageW(sort, CB_GETCURSEL, 0, 0) {
                1 => AchievementSort::Points,
                2 => AchievementSort::Grade,
                3 => AchievementSort::Earned,
                _ => AchievementSort::Name,
            };
            let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| shown.get(i));
            SetWindowTextW(
                detail,
                wide(&selected.map_or_else(
                    || "Select an achievement to read its description".into(),
                    |r| {
                        format!(
                            "{}\r\n{}\r\nEarned: {} (server Unix ms)",
                            r.name, r.description, r.earned_at
                        )
                    },
                ))
                .as_ptr(),
            );
            let mut m = lock(&shared);
            if !backend.valid(&m) {
                break;
            }
            let ready = backend.ready(&m);
            SetWindowTextW(h, wide(&backend.title(&m)).as_ptr());
            let p = backend.panel(&mut m);
            let mut intents = Vec::new();
            if p.catalog.search != search_value
                || p.catalog.filter != filter_value
                || p.catalog.sort != sort_value
            {
                p.catalog.page = 0;
            }
            p.catalog.search = search_value;
            p.catalog.filter = filter_value;
            p.catalog.sort = sort_value;
            for id in actions.0.drain(..) {
                match id {
                    2 => {
                        p.catalog.enabled = true;
                        intents.push(Intent::Refresh);
                    }
                    3 if !p.catalog.enabled => {
                        intents.push(Intent::Previous);
                    }
                    4 if !p.catalog.enabled => {
                        intents.push(Intent::Next);
                    }
                    3 => {
                        p.catalog.page = p.catalog.page.saturating_sub(1);
                    }
                    4 if p.catalog.page.saturating_add(1).saturating_mul(64) < matching => {
                        p.catalog.page += 1;
                    }
                    _ => {}
                }
            }
            let signature = (
                p.catalog.revision,
                p.catalog.filter,
                p.catalog.sort,
                p.catalog.search.clone(),
                p.catalog.page,
                p.pending,
                p.error,
                p.selected,
                p.catalog.collecting,
            );
            if held.as_ref() != Some(&signature) {
                let rows = if p.catalog.ready {
                    catalog_rows(&p.catalog)
                } else {
                    p.page
                        .as_ref()
                        .map_or_else(Vec::new, |page| page.rows.iter().collect())
                };
                matching = rows.len();
                shown = rows
                    .into_iter()
                    .skip(if p.catalog.ready {
                        p.catalog.page.saturating_mul(64)
                    } else {
                        0
                    })
                    .take(64)
                    .cloned()
                    .collect();
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &shown {
                    let label = format!(
                        "{} — Grade {} — {} points{}",
                        row.name,
                        row.grade,
                        row.points,
                        if row.secret { " — Secret" } else { "" }
                    );
                    SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                let status = if !p.selected {
                    "Feature unavailable in this session"
                } else if p.catalog.collecting {
                    "Loading the complete account list"
                } else {
                    p.error.unwrap_or("")
                };
                let points = if p.catalog.ready {
                    p.catalog.points
                } else {
                    p.page.as_ref().map_or(0, |page| page.total_points)
                };
                let earned = if p.catalog.ready {
                    p.catalog.rows.len()
                } else {
                    p.page.as_ref().map_or(0, |page| page.fact_count as usize)
                };
                let page = if p.catalog.ready {
                    (p.catalog.page as u64).saturating_add(1)
                } else {
                    p.page.as_ref().map_or(1, |page| u64::from(page.page) + 1)
                };
                let scope = if p.catalog.ready {
                    "matching"
                } else {
                    "here; load all for filters"
                };
                SetWindowTextW(header, wide(&format!("{points} points · {earned} earned · {matching} {scope} · Page {page}\r\n{status}")).as_ptr());
                held = Some(signature);
            }
            EnableWindow(
                refresh,
                i32::from(ready && p.selected && p.pending.is_none()),
            );
            EnableWindow(
                previous,
                i32::from(if p.catalog.ready {
                    p.catalog.page > 0
                } else {
                    !p.catalog.enabled && p.pending.is_none() && p.has_previous()
                }),
            );
            EnableWindow(
                next,
                i32::from(if p.catalog.ready {
                    (p.catalog.page + 1).saturating_mul(64) < matching
                } else {
                    !p.catalog.enabled && p.pending.is_none() && p.has_next()
                }),
            );
            for control in [search, filter, sort] {
                EnableWindow(control, i32::from(p.catalog.ready));
            }
            for intent in intents {
                if let Some(command) = backend.request(&mut m, intent)
                    && commands.send(command).is_err()
                {
                    backend.fail(&mut m);
                }
            }
            if let Some(command) = backend.next(&mut m)
                && commands.send(command).is_err()
            {
                backend.fail(&mut m);
            }
            drop(m);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak, backend);
    }
}
fn close(weak: &Shared, backend: Backend) {
    if let Some(m) = weak.upgrade() {
        let mut m = lock(&m);
        if backend.valid(&m) {
            backend.panel(&mut m).open = false;
        }
    }
}
unsafe fn ctl(h: HWND, c: &str, t: &str, id: usize, r: [i32; 4], s: u32) -> HWND {
    // SAFETY: the caller owns the live parent; Win32 copies the temporary strings.
    unsafe {
        CreateWindowExW(
            0,
            wide(c).as_ptr(),
            wide(t).as_ptr(),
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
    // SAFETY: this thread owns the live edit control and the bounded output buffer.
    unsafe {
        let n = GetWindowTextLengthW(h).max(0);
        let mut b = vec![0; n as usize + 1];
        let n = GetWindowTextW(h, b.as_mut_ptr(), n + 1).max(0) as usize;
        String::from_utf16_lossy(&b[..n])
    }
}
