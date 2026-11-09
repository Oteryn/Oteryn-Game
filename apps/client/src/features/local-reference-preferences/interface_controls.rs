//! Native controls for the supported interface presentation preferences.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct InterfaceControls {
    game_window: [HWND; 12],
    label: HWND,
    scale: HWND,
    cursor: HWND,
    names: [HWND; 3],
    tooltips: HWND,
    delay: HWND,
}

impl InterfaceControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let tooltips = control(
                hwnd,
                "BUTTON",
                "Tooltips (delay ms)",
                237,
                [330, 225, 220, 26],
                BS_AUTOCHECKBOX as u32,
            );
            let delay = control(
                hwnd,
                "EDIT",
                "500",
                238,
                [560, 225, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let game_window = [
                "Own spell effects",
                "Other spell effects",
                "Combat value text",
                "Loot result messages",
                "Target frame",
                "PvP frames",
                "Loot highlights",
                "Server notice banner",
                "Action bar labels",
                "Action bar cooldowns",
                "Three action-bar rows",
                "Insert newly learned spells",
            ]
            .map(|label| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    0,
                    [0, 0, 220, 26],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            for (index, handle) in game_window.iter().enumerate() {
                MoveWindow(
                    *handle,
                    10 + (index % 3) as i32 * 230,
                    305 + (index / 3) as i32 * 35,
                    220,
                    26,
                    1,
                );
            }
            if game_window.iter().any(|handle| handle.is_null()) {
                return None;
            }
            let names =
                [tr!("Own name"), tr!("Other player names"), "Creature names"].map(|label| {
                    control(
                        hwnd,
                        "BUTTON",
                        label,
                        0,
                        [0, 0, 220, 26],
                        BS_AUTOCHECKBOX as u32,
                    )
                });
            for (index, handle) in names.iter().enumerate() {
                MoveWindow(*handle, 10 + index as i32 * 230, 265, 220, 26, 1);
            }
            if names.iter().any(|handle| handle.is_null()) {
                return None;
            }
            let label = control(
                hwnd,
                "STATIC",
                "UI scale % (80–200; panels fit the screen)",
                0,
                [10, 185, 300, 26],
                0,
            );
            let scale = control(
                hwnd,
                "EDIT",
                "100",
                235,
                [380, 185, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let cursor = control(
                hwnd,
                "BUTTON",
                tr!("Crosshair cursor"),
                236,
                [10, 225, 300, 26],
                BS_AUTOCHECKBOX as u32,
            );
            (![label, scale, cursor, tooltips, delay]
                .iter()
                .any(|handle| handle.is_null()))
            .then_some(Self {
                game_window,
                label,
                scale,
                cursor,
                names,
                tooltips,
                delay,
            })
        }
    }

    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [
                self.label,
                self.scale,
                self.cursor,
                self.tooltips,
                self.delay,
            ]
            .into_iter()
            .chain(self.names)
            .chain(self.game_window)
            {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.tooltips_enabled = SendMessageW(self.tooltips, BM_GETCHECK, 0, 0) == 1;
            preferences.tooltip_delay_ms = text(self.delay)
                .parse()
                .map_err(|_| "Invalid tooltip delay".to_owned())?;
            let [
                own_spells,
                other_spells,
                value_text,
                loot_messages,
                target_frame,
                pvp_frame,
                loot_highlight,
                banner,
                bar_labels,
                bar_cooldowns,
                three_rows,
                auto_insert,
            ] = self
                .game_window
                .map(|handle| SendMessageW(handle, BM_GETCHECK, 0, 0) == 1);
            preferences.game_window.own_spells = own_spells;
            preferences.game_window.other_spells = other_spells;
            preferences.game_window.value_text = value_text;
            preferences.game_window.loot_messages = loot_messages;
            preferences.game_window.target_frame = target_frame;
            preferences.game_window.pvp_frame = pvp_frame;
            preferences.game_window.loot_highlight = loot_highlight;
            preferences.game_window.banner = banner;
            preferences.action_bar.labels = bar_labels;
            preferences.action_bar.cooldowns = bar_cooldowns;
            preferences.action_bar.three_rows = three_rows;
            preferences.action_bar.auto_insert = auto_insert;
            preferences.hud_names = self
                .names
                .map(|handle| SendMessageW(handle, BM_GETCHECK, 0, 0) == 1);
            preferences.interface_scale = text(self.scale)
                .parse()
                .map_err(|_| "Enter a UI scale from 80 to 200%".to_owned())?;
            preferences.cursor_crosshair = SendMessageW(self.cursor, BM_GETCHECK, 0, 0) == 1;
            Ok(())
        }
    }

    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            SendMessageW(
                self.tooltips,
                BM_SETCHECK,
                usize::from(preferences.tooltips_enabled),
                0,
            );
            SetWindowTextW(
                self.delay,
                wide(&preferences.tooltip_delay_ms.to_string()).as_ptr(),
            );
            let options = preferences.game_window;
            for (handle, enabled) in self.game_window.iter().zip([
                options.own_spells,
                options.other_spells,
                options.value_text,
                options.loot_messages,
                options.target_frame,
                options.pvp_frame,
                options.loot_highlight,
                options.banner,
                preferences.action_bar.labels,
                preferences.action_bar.cooldowns,
                preferences.action_bar.three_rows,
                preferences.action_bar.auto_insert,
            ]) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
            }
            for (handle, enabled) in self.names.iter().zip(preferences.hud_names) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
            }
            SetWindowTextW(
                self.scale,
                wide(&preferences.interface_scale.to_string()).as_ptr(),
            );
            SendMessageW(
                self.cursor,
                BM_SETCHECK,
                usize::from(preferences.cursor_crosshair),
                0,
            );
        }
    }
}
