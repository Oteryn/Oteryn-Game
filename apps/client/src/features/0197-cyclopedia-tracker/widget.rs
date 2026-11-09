//! Dedicated Bestiary and Bosstiary widgets. HWNDs and action storage belong to its window thread.
#![allow(unsafe_code)]
use super::{ctl, text, w};
use crate::combat_input::PlayCommand;
use crate::play::mailbox::{Mailbox, lock};
use oteryn_client_ui::cyclopedia_tracker::{Intent, TrackerSort, tracker_rows, tracker_selection};
use std::ptr::{null, null_mut};
use std::sync::{Mutex, Weak};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};

type WeakMailbox = Weak<Mutex<Mailbox>>;
#[derive(Default)]
pub(super) struct Actions(pub Vec<usize>);

pub(super) unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: the creating thread retains Actions until after DestroyWindow.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 32 {
                (*actions).0.push(wp & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

pub(super) fn open(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>, boss: bool) {
    let Some(shared) = weak.upgrade() else { return };
    {
        let mut m = lock(&shared);
        if if boss {
            m.cyclopedia_tracker.bosstiary_widget_open
        } else {
            m.cyclopedia_tracker.bestiary_widget_open
        } {
            return;
        }
        if boss {
            m.cyclopedia_tracker.bosstiary_widget_open = true;
        } else {
            m.cyclopedia_tracker.bestiary_widget_open = true;
        }
    }
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("bestiary-widget".into())
        .spawn(move || run(weak, commands, boss))
        .is_err()
        && let Some(shared) = failed.upgrade()
    {
        if boss {
            lock(&shared).cyclopedia_tracker.bosstiary_widget_open = false;
        } else {
            lock(&shared).cyclopedia_tracker.bestiary_widget_open = false;
        }
    }
}

fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>, boss: bool) {
    // SAFETY: all handles, pointers and message dispatch remain on this thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            w("STATIC").as_ptr(),
            w(if boss {
                tr!("Bosstiary tracker")
            } else {
                tr!("Bestiary tracker")
            })
            .as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            730,
            520,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, boss);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        ctl(
            h,
            "STATIC",
            tr!("Search tracked creatures"),
            0,
            [10, 8, 230, 20],
            0,
        );
        let search = ctl(
            h,
            "EDIT",
            "",
            201,
            [10, 30, 300, 25],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let sort = ctl(
            h,
            "COMBOBOX",
            "",
            202,
            [320, 30, 175, 160],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in [tr!("Name"), tr!("Most kills"), tr!("Nearest completion")] {
            SendMessageW(sort, CB_ADDSTRING, 0, w(label).as_ptr() as isize);
        }
        SendMessageW(sort, CB_SETCURSEL, 0, 0);
        let hide = ctl(
            h,
            "BUTTON",
            tr!("Hide completed"),
            203,
            [505, 30, 180, 25],
            BS_AUTOCHECKBOX as u32,
        );
        let list = ctl(
            h,
            "LISTBOX",
            "",
            204,
            [10, 65, 680, 260],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        ctl(
            h,
            "STATIC",
            tr!("Creature key to add"),
            0,
            [10, 335, 250, 20],
            0,
        );
        let key = ctl(
            h,
            "EDIT",
            "oteryn:creature.",
            205,
            [10, 360, 350, 25],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(key, EM_SETLIMITTEXT, 128, 0);
        ctl(h, "BUTTON", tr!("Add"), 206, [370, 360, 85, 28], 0);
        ctl(
            h,
            "BUTTON",
            tr!("Remove selected"),
            207,
            [465, 360, 140, 28],
            0,
        );
        ctl(h, "BUTTON", tr!("Refresh"), 208, [610, 360, 80, 28], 0);
        let status = ctl(h, "STATIC", "", 209, [10, 400, 680, 60], 0);
        if [search, sort, hide, list, key, status]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(h);
            close(&weak, boss);
            return;
        }
        if let Some(shared) = weak.upgrade() {
            let initial = lock(&shared).cyclopedia_tracker.clone();
            SetWindowTextW(
                search,
                w(if boss {
                    &initial.boss_search
                } else {
                    &initial.search
                })
                .as_ptr(),
            );
            SendMessageW(
                sort,
                CB_SETCURSEL,
                match if boss {
                    initial.boss_sort
                } else {
                    initial.sort
                } {
                    TrackerSort::Name => 0,
                    TrackerSort::Kills => 1,
                    TrackerSort::Remaining => 2,
                },
                0,
            );
            SendMessageW(
                hide,
                BM_SETCHECK,
                usize::from(if boss {
                    initial.boss_hide_completed
                } else {
                    initial.hide_completed
                }),
                0,
            );
        }
        let mut held = None;
        let mut initial = true;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let search_text = text(search);
            let chosen_sort = match SendMessageW(sort, CB_GETCURSEL, 0, 0) {
                1 => TrackerSort::Kills,
                2 => TrackerSort::Remaining,
                _ => TrackerSort::Name,
            };
            let hide_completed = SendMessageW(hide, BM_GETCHECK, 0, 0) != 0;
            let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
            let key_text = text(key);
            let selected_key = held.as_ref().and_then(|old| {
                selected.and_then(|i| {
                    tracker_rows(old, boss)
                        .get(i)
                        .map(|e| e.creature_key.clone())
                })
            });
            let panel = {
                let mut m = lock(&shared);
                let p = &mut m.cyclopedia_tracker;
                if boss {
                    p.boss_search = search_text;
                    p.boss_sort = chosen_sort;
                    p.boss_hide_completed = hide_completed;
                } else {
                    p.search = search_text;
                    p.sort = chosen_sort;
                    p.hide_completed = hide_completed;
                }
                if initial && !p.pending {
                    actions.0.push(208);
                    initial = false;
                }
                for action in actions.0.drain(..) {
                    if m.cyclopedia_tracker.pending {
                        continue;
                    }
                    let intent = match action {
                        206 => {
                            m.cyclopedia_tracker.creature_key.clone_from(&key_text);
                            Intent::Add
                        }
                        207 => {
                            let Some(key) = &selected_key else { continue };
                            m.cyclopedia_tracker.creature_key.clone_from(key);
                            Intent::Remove
                        }
                        208 => Intent::Read,
                        _ => continue,
                    };
                    m.cyclopedia_tracker.boss = boss;
                    super::super::super::input::enqueue(&mut m, &commands, intent);
                }
                m.cyclopedia_tracker.clone()
            };
            if held.as_ref() != Some(&panel) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in tracker_rows(&panel, boss) {
                    let goal = row.thresholds[2];
                    let percent = if goal == 0 {
                        0
                    } else {
                        (u64::from(row.kill_count) * 100 / u64::from(goal)).min(100)
                    };
                    let label = format!(
                        "{}   {} / {} kills ({}%)   stages {} / {} / {}{}",
                        row.creature_key,
                        row.kill_count,
                        goal,
                        percent,
                        row.thresholds[0],
                        row.thresholds[1],
                        goal,
                        if row.completed {
                            tr!(" — Complete")
                        } else {
                            ""
                        }
                    );
                    SendMessageW(list, LB_ADDSTRING, 0, w(&label).as_ptr() as isize);
                }
                if let Some(i) = selected_key
                    .as_deref()
                    .and_then(|key| tracker_selection(&panel, boss, key))
                {
                    SendMessageW(list, LB_SETCURSEL, i, 0);
                }
                let message = if !panel.selected {
                    tr!("Feature unavailable in this session")
                } else if panel.pending {
                    tr!("Waiting for the server")
                } else {
                    panel.feedback.unwrap_or(tr!(
                        "Select a tracked creature to remove it. Progress comes from the server."
                    ))
                };
                SetWindowTextW(status, w(message).as_ptr());
                held = Some(panel);
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak, boss);
    }
}
fn close(weak: &WeakMailbox, boss: bool) {
    if let Some(shared) = weak.upgrade() {
        if boss {
            lock(&shared).cyclopedia_tracker.bosstiary_widget_open = false;
        } else {
            lock(&shared).cyclopedia_tracker.bestiary_widget_open = false;
        }
    }
}
