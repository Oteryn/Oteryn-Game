// HWND operations stay on the creating thread, following the daily reward panel.
#![allow(unsafe_code)]
use super::controls::{control, read_text, wide};
use super::*;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::{
    Controls::{
        ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETPOS, PBM_SETRANGE32,
    },
    Input::KeyboardAndMouse::EnableWindow,
    WindowsAndMessaging::*,
};
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
                && matches!(id, 101..=117 | 140 | 145..=148 | 150)
                && note == BN_CLICKED as usize
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.cyclopedia_character.open {
            return;
        }
        m.cyclopedia_character.open = true;
    }
    link.cyclopedia_character_request(Intent::Tab);
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("0199-cyclopedia-character-panel".into())
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
            wide("Character cyclopedia").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            740,
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
        let character_widgets::Widgets {
            badge_id,
            edits,
            output,
            heading,
            history,
            detail,
            achievement_browser,
            status_label,
            cards,
            stamina_bar,
            training_bar,
        } = character_widgets::create(hwnd);
        let mut held_history = Vec::new();
        let mut held_combat = Vec::new();
        let mut held_store = String::new();
        let mut painted_tab = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.cyclopedia_character_shutdown {
                break;
            }
            let mut analytics = false;
            let mut browse_achievements = false;
            for action in std::mem::take(&mut actions.0) {
                if action == 150 {
                    if let Ok(id) = read_text(badge_id).parse::<u8>()
                        && mailbox
                            .cyclopedia_character
                            .badges
                            .as_ref()
                            .is_some_and(|ids| ids.contains(&id))
                        && !mailbox.cyclopedia_character.hidden_badges.remove(&id)
                    {
                        mailbox.cyclopedia_character.hidden_badges.insert(id);
                    }
                    continue;
                }
                if action == 117 {
                    analytics = true;
                    continue;
                }
                if action == 140 {
                    browse_achievements = true;
                    continue;
                }
                let values = edits.map(read_text);
                if let Some(command) = submit(
                    &mut mailbox.cyclopedia_character,
                    action,
                    values.each_ref().map(String::as_str),
                ) && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    mailbox.cyclopedia_character.pending = false;
                    mailbox.cyclopedia_character.feedback = Some("The session has ended");
                }
                SetWindowTextW(
                    edits[0],
                    wide(&mailbox.cyclopedia_character.offset.to_string()).as_ptr(),
                );
            }
            SetWindowTextW(
                output,
                wide(&display(&mailbox.cyclopedia_character)).as_ptr(),
            );
            let p = &mut mailbox.cyclopedia_character;
            if !p.pending
                && p.misc_resources_due
                && let Some(command) = request(p, Intent::MiscResources)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                p.pending = false;
                p.feedback = Some("The session has ended");
            }
            let panel = &mailbox.cyclopedia_character;
            EnableWindow(
                achievement_browser,
                i32::from(panel.selected && !panel.pending),
            );
            ShowWindow(
                achievement_browser,
                if panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::Achievements
                {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let status = if panel.pending {
                "Waiting for the server"
            } else {
                panel.feedback.unwrap_or("")
            };
            SetWindowTextW(status_label, wide(status).as_ptr());
            let badges = panel.command_kind == 387;
            let table = !badges && panel.history_tab == Some(panel.tab);
            let pvp = panel.tab
                == oteryn_client_runtime::cyclopedia_character::CharacterTab::RecentPvpKills;
            for id in 114..=116 {
                ShowWindow(GetDlgItem(hwnd, id), if pvp { SW_SHOW } else { SW_HIDE });
            }
            let overview = !badges
                && panel.general.is_some()
                && panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats;
            let combat = !badges && panel.combat.as_ref().is_some_and(|s| s.tab() == panel.tab);
            let store = !badges
                && panel.store.is_some()
                && panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::StoreSummary;
            let misc = !badges
                && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc;
            super::misc_layout::layout(&cards, [stamina_bar, training_bar], heading, detail, misc);
            let card_rows = if misc {
                oteryn_client_ui::cyclopedia_character::misc_cards(panel)
            } else if store {
                oteryn_client_ui::cyclopedia_character::store_cards(panel)
            } else {
                oteryn_client_ui::cyclopedia_character::overview_cards(panel)
            };
            ShowWindow(
                output,
                if table || overview || combat || store {
                    SW_HIDE
                } else {
                    SW_SHOW
                },
            );
            for (i, handle) in cards.iter().enumerate() {
                ShowWindow(
                    *handle,
                    if (overview || store || misc) && i < card_rows.len() {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
            for handle in [stamina_bar, training_bar] {
                ShowWindow(
                    handle,
                    if overview || (misc && panel.misc_general.is_some()) {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
            if overview || store || misc {
                for (handle, (label, value)) in cards.iter().zip(card_rows) {
                    SetWindowTextW(*handle, wide(&format!("{label}\r\n{value}")).as_ptr());
                }
                let bars = if misc {
                    oteryn_client_ui::cyclopedia_character::misc_bars(panel)
                } else {
                    oteryn_client_ui::cyclopedia_character::overview_bars(panel)
                };
                if let Some(bars) = bars {
                    for (handle, (value, max)) in [stamina_bar, training_bar].into_iter().zip(bars)
                    {
                        SendMessageW(handle, PBM_SETRANGE32, 0, max as isize);
                        SendMessageW(handle, PBM_SETPOS, usize::from(value), 0);
                    }
                }
            }
            for handle in [heading, detail] {
                ShowWindow(handle, if table || combat { SW_SHOW } else { SW_HIDE });
            }
            ShowWindow(
                history,
                if table || combat || store {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            SetWindowPos(
                history,
                null_mut(),
                10,
                if store { 415 } else { 380 },
                if misc { 400 } else { 690 },
                if store { 180 } else { 155 },
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            if painted_tab != Some(panel.tab) {
                SendMessageW(history, LB_RESETCONTENT, 0, 0);
                held_history.clear();
                held_combat.clear();
                held_store.clear();
                painted_tab = Some(panel.tab);
            }
            if table {
                let note = if panel.pending {
                    "Loading"
                } else {
                    panel.feedback.unwrap_or("")
                };
                use oteryn_client_runtime::cyclopedia_character::KillStatus;
                let filter = match panel.kill_filter {
                    None => "All results",
                    Some(KillStatus::GuildWar) => "Guild war",
                    Some(KillStatus::Unjustified) => "Unjustified",
                };
                let columns =
                    format!("Time (server Unix ms)     Result     Description | {filter} | {note}");
                SetWindowTextW(heading, wide(&columns).as_ptr());
                let rows: Vec<_> = oteryn_client_ui::cyclopedia_character::history_rows(panel)
                    .into_iter()
                    .cloned()
                    .collect();
                if held_history != rows {
                    SendMessageW(history, LB_RESETCONTENT, 0, 0);
                    for row in &rows {
                        let kind = match row.status {
                            Some(KillStatus::Unjustified) => "Unjustified",
                            Some(KillStatus::GuildWar) => "Guild war",
                            None => "Death",
                        };
                        let label = format!("{}\t{}\t{}", row.at, kind, row.description);
                        SendMessageW(history, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    held_history = rows;
                }
                let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
                let description = selected
                    .and_then(|i| held_history.get(i))
                    .map_or("Select a row to read its complete description", |r| {
                        r.description.as_str()
                    });
                SetWindowTextW(detail, wide(description).as_ptr());
            }
            if combat {
                SetWindowTextW(heading, wide("Statistic                    Value                    Source — select a row for details").as_ptr());
                let rows = oteryn_client_ui::cyclopedia_character::combat_rows(panel);
                if held_combat != rows {
                    SendMessageW(history, LB_RESETCONTENT, 0, 0);
                    for row in &rows {
                        let label = format!("{}\t{}\t{}", row.label, row.value, row.source);
                        SendMessageW(history, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    held_combat = rows;
                }
                let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
                let description = selected.and_then(|i| held_combat.get(i)).map_or_else(
                    || "Select a statistic to inspect its contributions".into(),
                    |row| oteryn_client_ui::cyclopedia_character::combat_detail(panel, row.key),
                );
                SetWindowTextW(detail, wide(&description).as_ptr());
            }
            if store {
                let blessings = oteryn_client_ui::cyclopedia_character::store_blessings(panel);
                if held_store != blessings {
                    SendMessageW(history, LB_RESETCONTENT, 0, 0);
                    for blessing in blessings.lines() {
                        SendMessageW(history, LB_ADDSTRING, 0, wide(blessing).as_ptr() as isize);
                    }
                    held_store = blessings;
                }
            }
            drop(mailbox);
            if analytics {
                crate::play::feature_0010_analyser::view::open_shared(&shared, &commands);
            }
            if browse_achievements {
                crate::play::feature_local_account_achievements::view::open_character(
                    weak.clone(),
                    commands.clone(),
                );
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        DestroyWindow(hwnd);
        close(&weak);
        drop(actions);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).cyclopedia_character.open = false;
    }
}

#[path = "character_widgets.rs"]
mod character_widgets;
