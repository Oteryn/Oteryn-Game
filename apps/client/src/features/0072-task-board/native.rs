//! Native full weekly, delivery, bounty and shop controls.
#![allow(unsafe_code, reason = "Win32 controls remain on their creating thread")]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::task_board::{FullIntent, full_request, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[path = "native/tree.rs"]
mod tree;

const REFRESH: usize = 102;
const START: usize = 103;
const DELIVER: usize = 104;
const OPEN_BOUNTY: usize = 105;
const SELECT_BOUNTY: usize = 106;
const CLAIM: usize = 107;
const DIFFICULTY: usize = 108;
const REROLL: usize = 109;
const ELITE: usize = 110;
const PREFERRED: usize = 111;
const UNWANTED: usize = 112;
const UNLOCK: usize = 113;
const BUY: usize = 114;
const BUY_TALISMAN: usize = 115;
const UPGRADE_TALISMAN: usize = 116;
#[derive(Default)]
struct Actions(Vec<usize>);
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.task_board.open {
            return;
        }
        mailbox.task_board.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("task-board-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn run(
    weak: WeakMailbox,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Task Board — Ctrl+Shift+T")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            820,
            760,
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
        let text = control(
            hwnd,
            "EDIT",
            "",
            101,
            [10, 10, 470, 380],
            WS_BORDER | WS_VSCROLL | WS_HSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let upgrades = tree::create(hwnd);
        if upgrades.is_null() {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        control(hwnd, "STATIC", tr!("Difficulty"), 0, [10, 400, 100, 24], 0);
        let difficulty = control(
            hwnd,
            "COMBOBOX",
            "",
            201,
            [110, 400, 140, 160],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for label in [tr!("Beginner"), tr!("Adept"), tr!("Expert"), tr!("Master")] {
            SendMessageW(difficulty, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(difficulty, CB_SETCURSEL, 0, 0);
        control(
            hwnd,
            "STATIC",
            "Task row / list slot (1-based)",
            0,
            [270, 400, 260, 24],
            0,
        );
        let row = control(
            hwnd,
            "EDIT",
            "1",
            202,
            [540, 400, 55, 24],
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
        );
        control(
            hwnd,
            "STATIC",
            tr!("Preferred / unwanted creature"),
            0,
            [10, 435, 330, 24],
            0,
        );
        let race = control(
            hwnd,
            "COMBOBOX",
            "",
            203,
            [350, 435, 440, 240],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for (index, (id, label)) in [
            (REFRESH, tr!("Refresh")),
            (START, tr!("Start weekly")),
            (DELIVER, tr!("Deliver row")),
            (OPEN_BOUNTY, tr!("Open bounty")),
            (SELECT_BOUNTY, tr!("Select bounty row")),
            (CLAIM, tr!("Claim bounty")),
            (DIFFICULTY, tr!("Set bounty difficulty")),
            (REROLL, tr!("Reroll bounty (1 token)")),
            (ELITE, tr!("Elite")),
            (PREFERRED, tr!("Set preferred")),
            (UNWANTED, tr!("Set unwanted")),
            (UNLOCK, tr!("Unlock list slot")),
            (BUY, tr!("Buy shop row")),
            (BUY_TALISMAN, "Buy talisman (5000 gold)"),
            (UPGRADE_TALISMAN, "Upgrade selected path"),
        ]
        .into_iter()
        .enumerate()
        {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [
                    10 + (index % 4) as i32 * 195,
                    475 + (index / 4) as i32 * 38,
                    185,
                    30,
                ],
                WS_TABSTOP
                    | if id == ELITE {
                        BS_AUTOCHECKBOX as u32
                    } else {
                        0
                    },
            );
        }
        control(hwnd, "STATIC", tr!("Shop offer"), 0, [10, 635, 120, 24], 0);
        let shop = control(
            hwnd,
            "COMBOBOX",
            "",
            204,
            [140, 635, 650, 240],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        let mut shop_ids: Vec<u16> = Vec::new();
        let mut choices: Vec<String> = Vec::new();
        let mut held = None;
        loop {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                DestroyWindow(hwnd);
                break;
            };
            let mut mailbox = lock(&shared);
            if mailbox.quest_shutdown {
                break;
            }
            if commands.is_closed() {
                DestroyWindow(hwnd);
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                let selected = read_text(row)
                    .parse::<u16>()
                    .ok()
                    .and_then(|v| v.checked_sub(1));
                let slot = selected.and_then(|v| u8::try_from(v).ok());
                let level = u8::try_from(SendMessageW(difficulty, CB_GETCURSEL, 0, 0)).ok();
                let selected_race = usize::try_from(SendMessageW(race, CB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|index| choices.get(index));
                let intent = match id {
                    REFRESH => Some(FullIntent::Refresh),
                    START => level.map(FullIntent::StartWeekly),
                    DELIVER => slot.map(FullIntent::Deliver),
                    OPEN_BOUNTY => Some(FullIntent::OpenBounty),
                    SELECT_BOUNTY => slot.map(FullIntent::SelectBounty),
                    CLAIM => Some(FullIntent::ClaimBounty),
                    DIFFICULTY => level.map(FullIntent::BountyDifficulty),
                    REROLL => Some(FullIntent::Reroll),
                    ELITE => Some(FullIntent::Elite(
                        SendMessageW(GetDlgItem(hwnd, ELITE as i32), BM_GETCHECK, 0, 0) == 1,
                    )),
                    PREFERRED => slot
                        .zip(selected_race)
                        .map(|(v, key)| FullIntent::Preferred(v, key.to_owned())),
                    UNWANTED => slot
                        .zip(selected_race)
                        .map(|(v, key)| FullIntent::Unwanted(v, key.to_owned())),
                    UNLOCK => Some(FullIntent::UnlockList),
                    BUY => usize::try_from(SendMessageW(shop, CB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|row| shop_ids.get(row).copied())
                        .map(FullIntent::BuyShop),
                    _ => None,
                };
                if matches!(id, BUY_TALISMAN | UPGRADE_TALISMAN) {
                    use oteryn_client_ui::task_board::talisman;
                    let chosen = if id == BUY_TALISMAN {
                        Some(talisman::Intent::Buy)
                    } else {
                        tree::selected_path(upgrades).map(talisman::Intent::Upgrade)
                    };
                    if let Some(chosen) = chosen {
                        super::super::input::enqueue_talisman(&mut mailbox, &commands, chosen);
                    } else {
                        mailbox.task_board.feedback = Some("Select a talisman path in the tree");
                    }
                    continue;
                }
                let command = intent.and_then(|intent| match intent {
                    FullIntent::Elite(enabled) => {
                        oteryn_client_ui::task_board_weekly_results::set_elite(
                            &mut mailbox.task_board,
                            enabled,
                        )
                    }
                    intent => full_request(&mut mailbox.task_board, intent),
                });
                if let Some(command) = command
                    && commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .is_err()
                {
                    mailbox.task_board.pending = false;
                    mailbox.task_board.feedback = Some(tr!("The session has ended"));
                }
            }
            let next_choices = oteryn_client_ui::task_board::creature_choices(&mailbox.task_board);
            if choices != next_choices {
                let previous = usize::try_from(SendMessageW(race, CB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|index| choices.get(index))
                    .cloned();
                choices = next_choices;
                SendMessageW(race, CB_RESETCONTENT, 0, 0);
                for key in &choices {
                    let name = if key.is_empty() {
                        tr!("Clear selection").into()
                    } else {
                        oteryn_client_ui::task_board::creature_name(key)
                    };
                    SendMessageW(race, CB_ADDSTRING, 0, wide(&name).as_ptr() as isize);
                }
                let index = previous
                    .as_ref()
                    .and_then(|key| choices.iter().position(|value| value == key))
                    .unwrap_or(0);
                SendMessageW(race, CB_SETCURSEL, index, 0);
            }
            if !mailbox.task_board.pending
                && let Some(reply) = &mailbox.task_board.weekly
            {
                SendMessageW(
                    GetDlgItem(hwnd, ELITE as i32),
                    BM_SETCHECK,
                    usize::from(reply.bounty.elite),
                    0,
                );
            }
            let rendered = panel_text(&mailbox.task_board).replace('\n', "\r\n");
            if held.as_ref() != Some(&rendered) {
                tree::refresh_shop(shop, &mailbox.task_board, &mut shop_ids);
                tree::refresh(upgrades, &mailbox.task_board);
                SetWindowTextW(text, wide(&rendered).as_ptr());
                held = Some(rendered);
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).task_board.open = false;
    }
}

unsafe fn read_text(hwnd: HWND) -> String {
    let mut value = [0u16; 129];
    unsafe {
        GetWindowTextW(hwnd, value.as_mut_ptr(), value.len() as i32);
    }
    let len = value.iter().position(|v| *v == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..len])
}
