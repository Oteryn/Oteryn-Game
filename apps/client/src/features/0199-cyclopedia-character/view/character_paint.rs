//! Render confirmed Character facts in the existing thread-owned controls.
#![allow(unsafe_code)]
use super::controls::wide;
use oteryn_client_runtime::cyclopedia_character::CharacterTab;
use oteryn_client_ui::cyclopedia_character::{
    CombatRow, CyclopediaCharacterPanel, HistoryRow, ProfileRow,
};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Controls::{PBM_SETPOS, PBM_SETRANGE32},
        WindowsAndMessaging::*,
    },
};
#[derive(Default)]
pub(super) struct Tables {
    history: Vec<HistoryRow>,
    combat: Vec<CombatRow>,
    store: String,
    tab: Option<CharacterTab>,
    profile: Vec<ProfileRow>,
}
pub(super) unsafe fn paint(
    panel: &CyclopediaCharacterPanel,
    hwnd: HWND,
    handles: [HWND; 6],
    cards: &[HWND],
    state: &mut Tables,
) {
    let [output, stamina_bar, training_bar, heading, detail, history] = handles;
    unsafe {
        let badges = panel.command_kind == 387;
        let table = !badges && panel.history_tab == Some(panel.tab);
        let pvp =
            panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::RecentPvpKills;
        for id in 114..=116 {
            ShowWindow(GetDlgItem(hwnd, id), if pvp { SW_SHOW } else { SW_HIDE });
        }
        let overview = !badges
            && panel.general.is_some()
            && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats;
        let combat = !badges && panel.combat.as_ref().is_some_and(|s| s.tab() == panel.tab);
        let store = !badges
            && panel.store.is_some()
            && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::StoreSummary;
        let misc =
            !badges && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc;
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
                for (handle, (value, max)) in [stamina_bar, training_bar].into_iter().zip(bars) {
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
        if state.tab != Some(panel.tab) {
            SendMessageW(history, LB_RESETCONTENT, 0, 0);
            state.history.clear();
            state.combat.clear();
            state.store.clear();
            state.tab = Some(panel.tab);
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
            if state.history != rows {
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
                state.history = rows;
            }
            let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
            let description = selected
                .and_then(|i| state.history.get(i))
                .map_or("Select a row to read its complete description", |r| {
                    r.description.as_str()
                });
            SetWindowTextW(detail, wide(description).as_ptr());
        }
        if combat {
            SetWindowTextW(heading, wide("Statistic                    Value                    Source — select a row for details").as_ptr());
            let rows = oteryn_client_ui::cyclopedia_character::combat_rows(panel);
            if state.combat != rows {
                SendMessageW(history, LB_RESETCONTENT, 0, 0);
                for row in &rows {
                    let label = format!("{}\t{}\t{}", row.label, row.value, row.source);
                    SendMessageW(history, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                state.combat = rows;
            }
            let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
            let description = selected.and_then(|i| state.combat.get(i)).map_or_else(
                || "Select a statistic to inspect its contributions".into(),
                |row| oteryn_client_ui::cyclopedia_character::combat_detail(panel, row.key),
            );
            SetWindowTextW(detail, wide(&description).as_ptr());
        }
        if store {
            let blessings = oteryn_client_ui::cyclopedia_character::store_blessings(panel);
            if state.store != blessings {
                SendMessageW(history, LB_RESETCONTENT, 0, 0);
                for blessing in blessings.lines() {
                    SendMessageW(history, LB_ADDSTRING, 0, wide(blessing).as_ptr() as isize);
                }
                state.store = blessings;
            }
        }
        super::profile_table::paint(
            panel,
            [history, heading, detail, output],
            &cards,
            [stamina_bar, training_bar],
            &mut state.profile,
        );
    }
}
