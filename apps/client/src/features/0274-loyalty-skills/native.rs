//! Native skill controls stay on their creating thread; only mailbox values cross threads.
#![allow(
    unsafe_code,
    reason = "Win32 controls and subclass data are owned by this UI thread"
)]
#[path = "native_controls.rs"]
mod panel_controls;
use super::layout::{FAMILIES, SkillsNumbers};
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use panel_controls::PanelControls;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::{
            ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETPOS,
            PBM_SETRANGE32,
        },
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::*,
    },
};

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = w >> 16;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && ((matches!(id, 200 | 204) && notification == CBN_SELCHANGE as usize)
                    || (matches!(id, 1 | 201..=203 | 205) && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
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
            id as _,
            null_mut(),
            null(),
        )
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.loyalty_skills.open {
            return;
        }
        mailbox.loyalty_skills.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let fallback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("skills-widget".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn run(
    weak: Weak<Mutex<Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    // SAFETY: HWNDs and the Actions allocation remain on this thread until destruction.
    unsafe {
        let common = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        };
        InitCommonControlsEx(&common);
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Skills — Ctrl+K")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            920,
            780,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let Some(PanelControls {
            header,
            level,
            labels,
            bars,
            family,
            visible,
            numbers,
            progress,
            refresh,
        }) = PanelControls::new(hwnd, &weak)
        else {
            return;
        };
        let mut held_widget = None;
        let mut held_layout = None;
        let mut next_refresh = Instant::now() + Duration::from_secs(1);
        loop {
            let mut message: MSG = std::mem::zeroed();
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
            let mut mailbox = lock(&shared);
            if mailbox.quest_shutdown {
                break;
            }
            let manual = actions.0.contains(&1);
            if manual
                || (Instant::now() >= next_refresh
                    && IsWindowVisible(hwnd) != 0
                    && IsIconic(hwnd) == 0)
            {
                let full_refresh = manual || !mailbox.skill_progress.selected;
                drop(mailbox);
                let link = PlayLink {
                    owns_session: false,
                    mailbox: Arc::clone(&shared),
                    commands: commands.clone(),
                };
                if full_refresh {
                    link.skills_widget_request();
                } else {
                    link.skill_progress_request();
                }
                mailbox = lock(&shared);
                next_refresh = Instant::now() + Duration::from_secs(1);
            }
            for id in std::mem::take(&mut actions.0) {
                if id == 1 {
                    continue;
                }
                let state = &mut mailbox.skills_layout;
                if id == 200 {
                    state.active = SendMessageW(family, CB_GETCURSEL, 0, 0).clamp(0, 7) as u8;
                    held_layout = None;
                    continue;
                }
                let mut candidate = state.preferences.clone();
                match id {
                    201 => {
                        let show = SendMessageW(visible, BM_GETCHECK, 0, 0) == 1;
                        if show && !candidate.order.contains(&state.active) {
                            candidate.order.push(state.active);
                        }
                        if !show {
                            candidate.order.retain(|id| *id != state.active);
                        }
                    }
                    202 | 203 => {
                        if let Some(index) =
                            candidate.order.iter().position(|id| *id == state.active)
                        {
                            if id == 202 && index > 0 {
                                candidate.order.swap(index, index - 1);
                            }
                            if id == 203 && index + 1 < candidate.order.len() {
                                candidate.order.swap(index, index + 1);
                            }
                        }
                    }
                    204 => {
                        candidate.numbers = match SendMessageW(numbers, CB_GETCURSEL, 0, 0) {
                            1 => SkillsNumbers::Trained,
                            2 => SkillsNumbers::Loyalty,
                            3 => SkillsNumbers::Current,
                            _ => SkillsNumbers::All,
                        }
                    }
                    205 => candidate.progress = SendMessageW(progress, BM_GETCHECK, 0, 0) == 1,
                    _ => continue,
                }
                if candidate.valid() && candidate != state.preferences {
                    state.preferences = candidate;
                    state.dirty = true;
                    state.edit = crate::play::feature_0017_attack::view::next_edit();
                }
            }
            let layout = &mailbox.skills_layout;
            let widget = layout
                .preferences
                .project(&mailbox.skill_progress, &mailbox.loyalty_skills);
            if held_widget.as_ref() != Some(&widget) || held_layout.as_ref() != Some(layout) {
                SetWindowTextW(header, wide(&widget.header).as_ptr());
                ShowWindow(
                    level,
                    if layout.preferences.progress && widget.level_progress.is_some() {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
                if let Some(value) = widget.level_progress {
                    SendMessageW(level, PBM_SETPOS, usize::from(value), 0);
                }
                for i in 0..8 {
                    let row = widget.rows.get(i);
                    ShowWindow(labels[i], if row.is_some() { SW_SHOW } else { SW_HIDE });
                    ShowWindow(
                        bars[i],
                        if row.is_some_and(|r| layout.preferences.progress && r.progress.is_some())
                        {
                            SW_SHOW
                        } else {
                            SW_HIDE
                        },
                    );
                    if let Some(row) = row {
                        SetWindowTextW(labels[i], wide(&row.text(&layout.preferences)).as_ptr());
                        if let Some(value) = row.progress {
                            SendMessageW(bars[i], PBM_SETPOS, usize::from(value), 0);
                        }
                    }
                }
                SendMessageW(family, CB_SETCURSEL, usize::from(layout.active), 0);
                SendMessageW(
                    visible,
                    BM_SETCHECK,
                    usize::from(layout.preferences.order.contains(&layout.active)),
                    0,
                );
                SendMessageW(
                    numbers,
                    CB_SETCURSEL,
                    layout.preferences.numbers as usize,
                    0,
                );
                SendMessageW(
                    progress,
                    BM_SETCHECK,
                    usize::from(layout.preferences.progress),
                    0,
                );
                EnableWindow(
                    refresh,
                    i32::from(
                        (mailbox.loyalty_skills.selected || mailbox.skill_progress.selected)
                            && !mailbox.loyalty_skills.pending
                            && !mailbox.skill_progress.pending,
                    ),
                );
                held_widget = Some(widget);
                held_layout = Some(layout.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).loyalty_skills.open = false;
    }
}
