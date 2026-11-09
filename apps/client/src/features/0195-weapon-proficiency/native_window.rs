// HWND operations stay on the creating thread, following the daily reward panel.
#![allow(unsafe_code)]
use super::*;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 32
                && (action_clicked(id, note)
                    || (matches!(id, 201..=204 | 206) && note == CBN_SELCHANGE as usize))
            {
                (*actions).0.push(id);
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
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.weapon_proficiency.open {
            return;
        }
        m.weapon_proficiency.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("0195-weapon-proficiency-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}
fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Weapon proficiency").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            685,
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
        let fields = [
            ("Weapon", ""),
            ("Perk group", ""),
            ("Perk", ""),
            ("Modification slot", ""),
            ("Committed shaping state", "Refresh needed"),
            ("Reshape offer", ""),
        ];
        let mut edits = [null_mut(); 6];
        for (i, (label, value)) in fields.iter().enumerate() {
            let y = 10 + i as i32 * 32;
            control(hwnd, "STATIC", label, 301 + i, [10, y, 235, 25], 0);
            edits[i] = control(
                hwnd,
                if i == 4 { "STATIC" } else { "COMBOBOX" },
                value,
                201 + i,
                [250, y, 440, if i == 4 { 25 } else { 240 }],
                WS_BORDER
                    | WS_TABSTOP
                    | if i == 4 {
                        0
                    } else {
                        CBS_DROPDOWNLIST as u32 | WS_VSCROLL
                    },
            );
        }
        let clear = control(
            hwnd,
            "BUTTON",
            "Clear selected perk",
            210,
            [10, 205, 250, 25],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        for (i, label) in [
            "Select perk",
            "Modify",
            "Rank up",
            "Orb rank",
            "Reshape offer",
            "Choose reshape",
            "Decline offer",
            "Clear modification",
            "Refresh catalogue",
            "Next catalogue",
            "Previous catalogue",
        ]
        .iter()
        .enumerate()
        {
            control(
                hwnd,
                "BUTTON",
                label,
                101 + i,
                [
                    10 + (i % 4) as i32 * 175,
                    240 + (i / 4) as i32 * 35,
                    165,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        for (id, label) in [
            (112, "Refresh weapons"),
            (113, "Previous page"),
            (114, "Next page"),
            (116, "Refresh shaping"),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [
                    10 + (if id == 116 { 3 } else { id as i32 - 112 }) * 175,
                    345,
                    165,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        let output = control(
            hwnd,
            "EDIT",
            "",
            400,
            [10, 390, 690, 240],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let mut message: MSG = std::mem::zeroed();
        let mut held: Option<WeaponProficiencyPanel> = None;
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
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            let panel = &mut mailbox.weapon_proficiency;
            if !panel.open || commands.is_closed() {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if matches!(action, 201..=204 | 206) {
                    let field = action - 201;
                    if let Ok(row) = usize::try_from(SendMessageW(edits[field], CB_GETCURSEL, 0, 0))
                    {
                        choose(panel, field, row);
                    }
                    continue;
                }
                if let Some(command) =
                    visual_submit(panel, action, SendMessageW(clear, BM_GETCHECK, 0, 0) == 1)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    panel.pending = false;
                    panel.feedback = Some("The session has ended");
                }
            }
            if !panel.pending
                && panel.weapon == 0
                && panel
                    .catalogue
                    .as_ref()
                    .is_some_and(|page| !page.entries.is_empty())
            {
                panel.select_weapon(0);
            }
            if !panel.pending
                && panel.state_refresh_needed
                && let Some(command) = request(panel, Intent::State)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some("The session has ended");
            }
            if held.as_ref() != Some(panel) {
                let weapons = panel
                    .catalogue
                    .as_ref()
                    .map(|page| {
                        page.entries
                            .iter()
                            .map(|entry| format!("{} — {} XP", entry.name, entry.experience))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selected_weapon = panel
                    .catalogue
                    .as_ref()
                    .and_then(|page| {
                        page.entries
                            .iter()
                            .position(|entry| entry.definition == panel.weapon)
                    })
                    .unwrap_or(0);
                fill(edits[0], &weapons, selected_weapon);
                let groups = panel
                    .weapon_entry()
                    .map(|entry| {
                        (0..entry.levels.len())
                            .map(|i| format!("Perk group {}", i + 1))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                fill(edits[1], &groups, usize::from(panel.level));
                let perks = panel
                    .weapon_entry()
                    .and_then(|entry| entry.levels.get(usize::from(panel.level)))
                    .map(|level| level.perks.clone())
                    .unwrap_or_default();
                fill(edits[2], &perks, usize::from(panel.perk));
                let slots = (0..2)
                    .map(|i| {
                        panel
                            .shaping
                            .as_ref()
                            .and_then(|state| state.slots[i].as_ref())
                            .map_or_else(
                                || format!("Slot {} — empty", i + 1),
                                |slot| {
                                    format!(
                                        "Slot {} — rank {} — {}",
                                        i + 1,
                                        slot.rank,
                                        slot.current
                                    )
                                },
                            )
                    })
                    .collect::<Vec<_>>();
                fill(edits[3], &slots, usize::from(panel.slot).wrapping_sub(1));
                SetWindowTextW(
                    edits[4],
                    wide(if panel.shaping.is_some() {
                        "Current server observation"
                    } else {
                        "Refresh needed / unavailable"
                    })
                    .as_ptr(),
                );
                let offers = panel
                    .shaping
                    .as_ref()
                    .and_then(|state| state.slots.get(usize::from(panel.slot).wrapping_sub(1)))
                    .and_then(Option::as_ref)
                    .map(|slot| slot.offers.clone())
                    .unwrap_or_default();
                fill(edits[5], &offers, usize::from(panel.shaping_choice));
                for control in edits.into_iter().chain([clear]) {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        control,
                        i32::from(!panel.pending && panel.selected),
                    );
                }
                SetWindowTextW(output, wide(&display(panel)).as_ptr());
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(25));
        }
        DestroyWindow(hwnd);
        close(&weak);
        drop(actions);
    }
}
fn control(parent: HWND, class: &str, label: &str, id: usize, rect: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
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
fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).weapon_proficiency.open = false;
    }
}

fn fill(hwnd: HWND, values: &[String], selected: usize) {
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
        for value in values {
            SendMessageW(hwnd, CB_ADDSTRING, 0, wide(value).as_ptr() as isize);
        }
        if !values.is_empty() {
            SendMessageW(hwnd, CB_SETCURSEL, selected.min(values.len() - 1), 0);
        }
    }
}
