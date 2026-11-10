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
                    || (id == 207 && note == EN_CHANGE as usize)
                    || (matches!(id, 201..=204 | 206 | 208..=209 | 211)
                        && note == CBN_SELCHANGE as usize))
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
            wide(tr!("Weapon proficiency")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            850,
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
        control(
            hwnd,
            "STATIC",
            tr!("All weapons — filter by name"),
            307,
            [10, 10, 320, 25],
            0,
        );
        let search = control(
            hwnd,
            "EDIT",
            "",
            207,
            [10, 40, 320, 25],
            WS_BORDER | WS_TABSTOP,
        );
        let filter = control(
            hwnd,
            "COMBOBOX",
            "",
            208,
            [10, 75, 180, 200],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        fill(
            filter,
            &[
                tr!("All weapons"),
                tr!("In progress"),
                tr!("Mastered"),
                tr!("Untrained"),
            ]
            .map(str::to_owned),
            0,
        );
        let sort = control(
            hwnd,
            "COMBOBOX",
            "",
            209,
            [200, 75, 130, 160],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        fill(sort, &[tr!("Name"), tr!("Progress")].map(str::to_owned), 0);
        let tree = control(
            hwnd,
            "LISTBOX",
            "",
            211,
            [350, 410, 690, 190],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
        );
        let slot_frame = control(
            hwnd,
            "BUTTON",
            tr!("Modification slot"),
            308,
            [340, 70, 710, 46],
            BS_GROUPBOX as u32,
        );
        let fields = [
            (tr!("Weapon"), ""),
            (tr!("Perk group"), ""),
            (tr!("Perk"), ""),
            (tr!("Modification slot"), ""),
            (tr!("Committed shaping state"), tr!("Refresh needed")),
            (tr!("Reshape offer"), ""),
        ];
        let mut edits = [null_mut(); 6];
        for (i, (label, value)) in fields.iter().enumerate() {
            if i == 0 {
                edits[i] = control(
                    hwnd,
                    "LISTBOX",
                    "",
                    201,
                    [10, 115, 320, 480],
                    WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
                );
                continue;
            }
            let y = 10 + (i - 1) as i32 * 36;
            control(hwnd, "STATIC", label, 301 + i, [350, y, 235, 25], 0);
            edits[i] = control(
                hwnd,
                if i == 4 { "STATIC" } else { "COMBOBOX" },
                value,
                201 + i,
                [590, y, 440, if i == 4 { 25 } else { 240 }],
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
            tr!("Clear selected perk"),
            210,
            [350, 205, 250, 25],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        for (i, label) in [
            tr!("Select perk"),
            tr!("Modify"),
            tr!("Rank up"),
            tr!("Orb rank"),
            tr!("Reshape offer"),
            tr!("Choose reshape"),
            tr!("Decline offer"),
            tr!("Clear modification"),
            tr!("Refresh catalogue"),
            tr!("Next catalogue"),
            tr!("Previous catalogue"),
        ]
        .iter()
        .enumerate()
        .take(9)
        {
            control(
                hwnd,
                "BUTTON",
                label,
                101 + i,
                [
                    350 + (i % 4) as i32 * 175,
                    240 + (i / 4) as i32 * 35,
                    165,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        for (id, label) in [
            (112, tr!("Refresh weapons")),
            (113, tr!("Previous page")),
            (114, tr!("Next page")),
            (116, tr!("Refresh shaping")),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [
                    350 + (if id == 116 { 3 } else { id as i32 - 112 }) * 175,
                    345,
                    165,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        for (id, label, x) in [
            (117, tr!("Confirm Clear"), 350),
            (118, tr!("Cancel Clear"), 525),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 380, 165, 28], WS_TABSTOP);
        }
        let output = control(
            hwnd,
            "EDIT",
            "",
            400,
            [10, 615, 1040, 185],
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
                if action == 207 {
                    let mut text = vec![0u16; (GetWindowTextLengthW(search) + 1).max(1) as usize];
                    GetWindowTextW(search, text.as_mut_ptr(), text.len() as i32);
                    panel.search = String::from_utf16_lossy(&text[..text.len() - 1]);
                    continue;
                }
                if matches!(action, 208..=209) {
                    let row = SendMessageW(
                        if action == 208 { filter } else { sort },
                        CB_GETCURSEL,
                        0,
                        0,
                    );
                    if action == 208 {
                        panel.progress_filter = row as u8;
                    } else {
                        panel.sort_progress = row == 1;
                    }
                    continue;
                }
                if action == 211 {
                    if let Ok(row) = usize::try_from(SendMessageW(tree, LB_GETCURSEL, 0, 0)) {
                        choose(panel, 6, row);
                    }
                    continue;
                }
                if matches!(action, 201..=204 | 206) {
                    let field = action - 201;
                    if let Ok(row) = usize::try_from(SendMessageW(
                        edits[field],
                        if field == 0 {
                            LB_GETCURSEL
                        } else {
                            CB_GETCURSEL
                        },
                        0,
                        0,
                    )) {
                        choose(panel, field, row);
                    }
                    continue;
                }
                if let Some(command) =
                    visual_submit(panel, action, SendMessageW(clear, BM_GETCHECK, 0, 0) == 1)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    panel.pending = false;
                    panel.feedback = Some(tr!("The session has ended"));
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
            if let Some(command) =
                oteryn_client_ui::weapon_proficiency::tick(panel, std::time::Instant::now())
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some(tr!("The session has ended"));
            }
            if held.as_ref() != Some(panel) {
                let rows = panel.visible_weapons();
                let weapons = rows
                    .iter()
                    .filter_map(|row| panel.catalogue.as_ref()?.entries.get(*row))
                    .map(|entry| {
                        lfmt!(
                            "{} — {}% — {} XP",
                            entry.name,
                            oteryn_client_ui::weapon_proficiency::progress_percent(entry),
                            entry.experience
                        )
                    })
                    .collect::<Vec<_>>();
                let selected_weapon = rows.iter().position(|row| {
                    panel
                        .catalogue
                        .as_ref()
                        .is_some_and(|page| page.entries[*row].definition == panel.weapon)
                });
                fill_list(edits[0], &weapons, selected_weapon);
                let tree_rows = panel.perk_tree_rows();
                let tree_selection = tree_rows.iter().position(|(_, choice)| {
                    *choice == Some((usize::from(panel.level), usize::from(panel.perk)))
                });
                fill_list(
                    tree,
                    &tree_rows
                        .into_iter()
                        .map(|(text, _)| text)
                        .collect::<Vec<_>>(),
                    tree_selection,
                );
                let slot_title = panel
                    .shaping
                    .as_ref()
                    .and_then(|state| state.slots.get(usize::from(panel.slot).wrapping_sub(1)))
                    .and_then(Option::as_ref)
                    .map_or_else(
                        || lfmt!("Slot {} — empty", panel.slot),
                        |slot| lfmt!("Modified slot {} — rank {}", panel.slot, slot.rank),
                    );
                SetWindowTextW(slot_frame, wide(&slot_title).as_ptr());
                let modified = panel
                    .shaping
                    .as_ref()
                    .and_then(|state| state.slots.get(usize::from(panel.slot).wrapping_sub(1)))
                    .is_some_and(Option::is_some);
                ShowWindow(slot_frame, if modified { SW_SHOW } else { SW_HIDE });
                for id in [117, 118] {
                    ShowWindow(
                        GetDlgItem(hwnd, id),
                        if panel.clear_confirmation.is_some() {
                            SW_SHOW
                        } else {
                            SW_HIDE
                        },
                    );
                }
                let groups = panel
                    .weapon_entry()
                    .map(|entry| {
                        (0..entry.levels.len())
                            .map(|i| lfmt!("Perk group {}", i + 1))
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
                                || lfmt!("Slot {} — empty", i + 1),
                                |slot| {
                                    lfmt!("Slot {} — rank {} — {}", i + 1, slot.rank, slot.current)
                                },
                            )
                    })
                    .collect::<Vec<_>>();
                fill(edits[3], &slots, usize::from(panel.slot).wrapping_sub(1));
                SetWindowTextW(
                    edits[4],
                    wide(if panel.shaping.is_some() {
                        tr!("Current server observation")
                    } else {
                        tr!("Refresh needed / unavailable")
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

fn fill_list(hwnd: HWND, values: &[String], selected: Option<usize>) {
    unsafe {
        SendMessageW(hwnd, LB_RESETCONTENT, 0, 0);
        for value in values {
            SendMessageW(hwnd, LB_ADDSTRING, 0, wide(value).as_ptr() as isize);
        }
        if let Some(row) = selected {
            SendMessageW(hwnd, LB_SETCURSEL, row, 0);
        }
    }
}
