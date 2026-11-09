//! Native spellbook list; every page and eligibility detail is supplied by the server.
#![allow(
    unsafe_code,
    reason = "HWNDs and boxed callback data remain on this dialog thread"
)]
#[path = "book_native_controls.rs"]
mod panel_controls;
use super::book::{GROUPS, SpellbookFilter, VOCATIONS};
use crate::play::{PlayLink, mailbox::lock};
use panel_controls::PanelControls;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Weak},
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
        if msg == WM_COMMAND && matches!(w & 0xffff, 1108..=1115) && w >> 16 == BN_CLICKED as usize
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
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(hwnd: HWND) -> String {
    let mut bytes = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, bytes.as_mut_ptr(), 128) };
    String::from_utf16_lossy(&bytes[..len.max(0) as usize])
}
type Mailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
pub(super) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.spellbook_details.native_open {
            return;
        }
        mailbox.spellbook_details.native_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("spellbook".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn close(weak: &Mailbox) {
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        mailbox.spellbook_details.open = false;
        mailbox.spellbook_details.native_open = false;
    }
}
fn run(
    weak: Mailbox,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn Spellbook").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1040,
            700,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let Some(PanelControls {
            labels,
            search,
            vocation,
            premium,
            group,
            level_label,
            min,
            max,
            apply,
            refresh,
            list,
            details,
            previous,
            next,
            archive,
            slot,
            assign,
            clear,
        }) = PanelControls::new(hwnd)
        else {
            return;
        };
        let mut archive_mode = false;
        if [
            archive,
            slot,
            assign,
            clear,
            labels,
            search,
            vocation,
            premium,
            group,
            level_label,
            min,
            max,
            apply,
            refresh,
            list,
            details,
            previous,
            next,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        for &label in VOCATIONS {
            SendMessageW(vocation, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        for label in ["Any premium status", "Free spells", "Premium required"] {
            SendMessageW(premium, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        for &label in GROUPS {
            SendMessageW(group, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let mut held_filter = None;
        let mut held_page = None;
        let mut held_text = String::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let state = lock(&shared).spellbook_details.clone();
            if !state.open || !state.selected {
                break;
            }
            if held_filter.as_ref() != Some(&state.filter) {
                SetWindowTextW(search, wide(&state.filter.search).as_ptr());
                SendMessageW(
                    vocation,
                    CB_SETCURSEL,
                    usize::from(state.filter.vocation),
                    0,
                );
                SendMessageW(premium, CB_SETCURSEL, usize::from(state.filter.premium), 0);
                SendMessageW(group, CB_SETCURSEL, usize::from(state.filter.group), 0);
                SetWindowTextW(min, wide(&state.filter.min_level.to_string()).as_ptr());
                SetWindowTextW(max, wide(&state.filter.max_level.to_string()).as_ptr());
                held_filter = Some(state.filter.clone());
                actions.0.clear();
            }
            for action in std::mem::take(&mut actions.0) {
                if action == 1113 {
                    archive_mode = !archive_mode;
                    SetWindowTextW(
                        hwnd,
                        wide(if archive_mode {
                            "Oteryn Magical Archive"
                        } else {
                            "Oteryn Spellbook"
                        })
                        .as_ptr(),
                    );
                    SetWindowTextW(
                        archive,
                        wide(if archive_mode {
                            "Back to Spellbook"
                        } else {
                            "Magical Archive"
                        })
                        .as_ptr(),
                    );
                    for control in [slot, assign, clear] {
                        ShowWindow(control, if archive_mode { SW_SHOW } else { SW_HIDE });
                    }
                    continue;
                }
                if matches!(action, 1114 | 1115) {
                    if archive_mode {
                        let selected_slot = usize::try_from(SendMessageW(slot, CB_GETCURSEL, 0, 0))
                            .unwrap_or(usize::MAX);
                        if action == 1114 {
                            let row = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                                .unwrap_or(usize::MAX);
                            if !super::archive::assign(&shared, row, selected_slot) {
                                lock(&shared).spellbook_details.feedback = "Select a spell from the confirmed current page before assigning.".into();
                            }
                        } else {
                            super::archive::clear(&shared, selected_slot);
                        }
                    }
                    continue;
                }
                let filter = match (text(min).parse::<u32>(), text(max).parse::<u32>()) {
                    (Ok(min_level), Ok(max_level)) => SpellbookFilter {
                        search: text(search),
                        vocation: u8::try_from(SendMessageW(vocation, CB_GETCURSEL, 0, 0))
                            .unwrap_or(0),
                        premium: u8::try_from(SendMessageW(premium, CB_GETCURSEL, 0, 0))
                            .unwrap_or(0),
                        group: u8::try_from(SendMessageW(group, CB_GETCURSEL, 0, 0)).unwrap_or(0),
                        min_level,
                        max_level,
                    },
                    _ => {
                        lock(&shared).spellbook_details.feedback =
                            "Enter valid minimum and maximum levels".into();
                        continue;
                    }
                };
                let page = if action == 1108 || filter != state.filter {
                    0
                } else {
                    let current = state.page.as_ref().map_or(0, |p| p.page);
                    match action {
                        1110 => current.saturating_sub(1),
                        1111 => current.saturating_add(1),
                        _ => current,
                    }
                };
                if !super::book::request(&commands, &shared, filter, page)
                    && state.pending.is_none()
                {
                    lock(&shared).spellbook_details.feedback="Invalid filters or page. Maximum level must be zero or at least the minimum.".into();
                }
            }
            let state = lock(&shared).spellbook_details.clone();
            if held_page.as_ref() != Some(&state.page) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                if let Some(page) = &state.page {
                    for entry in &page.entries {
                        let label = format!(
                            "{} · level {}{}",
                            entry.spell.name,
                            entry.spell.level,
                            if entry.spell.available {
                                " · eligible"
                            } else {
                                ""
                            }
                        );
                        SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                }
                SendMessageW(list, LB_SETCURSEL, 0, 0);
                held_page = Some(state.page.clone());
            }
            let row = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).unwrap_or(0);
            let mut detail_text = super::book_detail::text(&lock(&shared), row);
            if archive_mode {
                detail_text.push_str("\r\n\r\nSaved action bindings (resolved against the current server catalog):\r\n");
                detail_text.push_str(&state.archive.labels().join("\r\n"));
            }
            EnableWindow(
                assign,
                i32::from(
                    state.pending.is_none()
                        && state.confirmed_filter.as_ref() == Some(&state.filter)
                        && state
                            .page
                            .as_ref()
                            .is_some_and(|page| row < page.entries.len()),
                ),
            );
            if held_text != detail_text {
                SetWindowTextW(details, wide(&detail_text).as_ptr());
                held_text = detail_text;
            }
            let ready = state.pending.is_none();
            for button in [apply, refresh] {
                EnableWindow(button, i32::from(ready));
            }
            EnableWindow(
                previous,
                i32::from(ready && state.page.as_ref().is_some_and(|p| p.page > 0)),
            );
            EnableWindow(
                next,
                i32::from(
                    ready
                        && state
                            .page
                            .as_ref()
                            .is_some_and(|p| (usize::from(p.page) + 1) * 32 < usize::from(p.total)),
                ),
            );
            drop(shared);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
