//! Construction and validation of the settings dialog controls.
use super::*;
#[path = "interface_controls.rs"]
mod interface_controls;
use interface_controls::InterfaceControls;
#[path = "arc_controls.rs"]
mod arc_controls;
use arc_controls::ArcControls;
#[path = "layout_controls.rs"]
mod layout_controls;
use layout_controls::LayoutControls;
#[path = "console_controls.rs"]
mod console_controls;
use console_controls::ConsoleControls;

pub(super) struct Controls {
    pub(super) console: ConsoleControls,
    pub(super) layout: LayoutControls,
    pub(super) arcs: ArcControls,
    pub(super) interface: InterfaceControls,
    pub(super) profile_name: HWND,
    pub(super) profile_save: HWND,
    pub(super) profile_load: HWND,
    pub(super) profile_delete: HWND,
    pub(super) pages: HWND,
    pub(super) help: HWND,
    pub(super) list: HWND,
    pub(super) chord: HWND,
    pub(super) rebind: HWND,
    pub(super) manual: HWND,
    pub(super) chat_on: HWND,
    pub(super) chat_off: HWND,
    pub(super) mouse_preset: HWND,
    pub(super) delay_label: HWND,
    pub(super) delay: HWND,
    pub(super) controls_apply: HWND,
    pub(super) rotation_label: HWND,
    pub(super) rotation: HWND,
    pub(super) label: HWND,
    pub(super) scale: HWND,
    pub(super) volume: HWND,
    pub(super) mute: HWND,
    pub(super) apply: HWND,
    pub(super) transfer_label: HWND,
    pub(super) transfer_path: HWND,
    pub(super) import: HWND,
    pub(super) export: HWND,
    pub(super) backup: HWND,
    pub(super) defaults: HWND,
    pub(super) hud_checks: Vec<HWND>,
    pub(super) feedback: HWND,
}

impl Controls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let console = ConsoleControls::new(hwnd)?;
            let layout = LayoutControls::new(hwnd)?;
            let arcs = ArcControls::new(hwnd)?;
            let interface = InterfaceControls::new(hwnd)?;
            let profile_name = control(
                hwnd,
                "EDIT",
                "default",
                230,
                [10, 185, 300, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            let profile_save = control(hwnd, "BUTTON", "Save profile", 231, [330, 185, 120, 26], 0);
            let profile_load = control(hwnd, "BUTTON", "Load profile", 232, [460, 185, 120, 26], 0);
            let profile_delete = control(
                hwnd,
                "BUTTON",
                "Delete profile",
                233,
                [590, 185, 120, 26],
                0,
            );
            let pages = control(
                hwnd,
                "COMBOBOX",
                "",
                220,
                [10, 10, 720, 240],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for page in SettingsPage::ALL {
                SendMessageW(pages, CB_ADDSTRING, 0, wide(page.label()).as_ptr() as isize);
            }
            SendMessageW(pages, CB_SETCURSEL, 0, 0);
            let help = control(
                hwnd,
                "STATIC",
                "Key bindings: select an action, enter e.g. ALT+H, then Rebind.\nMouse buttons follow the selected preset. Conflicts include all registered actions.",
                0,
                [10, 50, 720, 45],
                0,
            );
            let list = control(
                hwnd,
                "COMBOBOX",
                "",
                101,
                [10, 100, 440, 280],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let chord = control(
                hwnd,
                "EDIT",
                "",
                102,
                [460, 100, 140, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            let rebind = control(hwnd, "BUTTON", "Rebind", 103, [610, 100, 110, 26], 0);
            let manual = control(
                hwnd,
                "BUTTON",
                "Manual hotkeys (no automatic chat presets)",
                227,
                [10, 190, 630, 30],
                0,
            );
            let chat_on = control(
                hwnd,
                "BUTTON",
                "Chat-on preset (arrows)",
                225,
                [10, 150, 300, 30],
                0,
            );
            let chat_off = control(
                hwnd,
                "BUTTON",
                "Chat-off preset (WASD)",
                226,
                [330, 150, 300, 30],
                0,
            );
            let mouse_preset = control(
                hwnd,
                "COMBOBOX",
                "",
                221,
                [10, 150, 360, 150],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for label in oteryn_client::play::ReferencePreferences::MOUSE_PRESETS {
                SendMessageW(mouse_preset, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let delay_label = control(
                hwnd,
                "STATIC",
                "Keyboard repeat delay ms (0–1000)",
                0,
                [10, 185, 360, 26],
                0,
            );
            let delay = control(
                hwnd,
                "EDIT",
                "",
                222,
                [380, 185, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let controls_apply = control(
                hwnd,
                "BUTTON",
                "Apply controls",
                223,
                [500, 185, 180, 26],
                0,
            );
            let rotation_label = control(
                hwnd,
                "STATIC",
                "Turn modifier (CTRL / ALT / SHIFT)",
                0,
                [10, 225, 360, 26],
                0,
            );
            let rotation = control(
                hwnd,
                "EDIT",
                "",
                224,
                [380, 225, 100, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            let label = control(
                hwnd,
                "STATIC",
                "World zoom % (80–180, fitted to window)",
                0,
                [10, 110, 700, 25],
                0,
            );
            let scale = control(
                hwnd,
                "EDIT",
                "",
                104,
                [10, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let volume = control(
                hwnd,
                "EDIT",
                "",
                105,
                [130, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let mute = control(
                hwnd,
                "BUTTON",
                "Mute",
                107,
                [250, 145, 100, 26],
                BS_AUTOCHECKBOX as u32,
            );
            let apply = control(hwnd, "BUTTON", "Apply", 106, [380, 145, 110, 26], 0);
            let transfer_label = control(
                hwnd,
                "STATIC",
                "Settings transfer file (JSON). Import/defaults save a recoverable backup.",
                0,
                [10, 195, 720, 25],
                0,
            );
            let transfer_path = control(
                hwnd,
                "EDIT",
                "",
                114,
                [10, 225, 710, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(transfer_path, EM_SETLIMITTEXT, 240, 0);
            let import = control(hwnd, "BUTTON", "Import...", 110, [10, 265, 150, 30], 0);
            let export = control(
                hwnd,
                "BUTTON",
                "Export new file",
                111,
                [175, 265, 160, 30],
                0,
            );
            let backup = control(
                hwnd,
                "BUTTON",
                "Restore backup...",
                112,
                [350, 265, 170, 30],
                0,
            );
            let defaults = control(
                hwnd,
                "BUTTON",
                "Restore defaults...",
                113,
                [535, 265, 185, 30],
                0,
            );
            let hud_checks = [
                "HP / MP bars",
                "Harmony",
                "Serene",
                "Light effects",
                "Combat effects",
                "Creature health / icons",
                "HP / MP circle",
            ]
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    120 + index,
                    [
                        10 + (index % 4) as i32 * 175,
                        310 + (index / 4) as i32 * 30,
                        170,
                        26,
                    ],
                    BS_AUTOCHECKBOX as u32,
                )
            })
            .collect::<Vec<_>>();
            let feedback = control(hwnd, "STATIC", "", 0, [10, 380, 710, 90], 0);
            if [
                manual,
                profile_name,
                profile_save,
                profile_load,
                profile_delete,
                chat_on,
                chat_off,
                rotation_label,
                rotation,
                mouse_preset,
                delay_label,
                delay,
                controls_apply,
                pages,
                help,
                list,
                chord,
                rebind,
                label,
                scale,
                volume,
                mute,
                apply,
                feedback,
                transfer_label,
                transfer_path,
                import,
                export,
                backup,
                defaults,
            ]
            .iter()
            .chain(hud_checks.iter())
            .any(|h| h.is_null())
            {
                return None;
            }
            Some(Self {
                manual,
                console,
                layout,
                arcs,
                interface,
                profile_name,
                profile_save,
                profile_load,
                profile_delete,
                pages,
                help,
                list,
                chord,
                rebind,
                chat_on,
                chat_off,
                mouse_preset,
                delay_label,
                delay,
                controls_apply,
                rotation_label,
                rotation,
                label,
                scale,
                volume,
                mute,
                apply,
                transfer_label,
                transfer_path,
                import,
                export,
                backup,
                defaults,
                hud_checks,
                feedback,
            })
        }
    }
}
