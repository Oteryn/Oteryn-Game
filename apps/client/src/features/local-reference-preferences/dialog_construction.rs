use super::*;
impl Controls {
    pub(in super::super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let miscellaneous = MiscellaneousControls::new(hwnd)?;
            let graphics = GraphicsControls::new(hwnd)?;
            let shortcuts = ShortcutControls::new(hwnd)?;
            let battle_filters = BattleFilterControls::new(hwnd)?;
            let mixer = MixerControls::new(hwnd)?;
            let stability = control(
                hwnd,
                "BUTTON",
                "Optimise connection stability (next login): disable TCP buffering",
                0,
                [10, 265, 710, 26],
                BS_AUTOCHECKBOX as u32,
            );
            let battle_sounds = BattleSoundControls::new(hwnd)?;
            let frames = FrameControls::new(hwnd)?;
            let actor_hud = HudControls::new(hwnd)?;
            let resources = ResourceControls::new(hwnd)?;
            let console = ConsoleControls::new(hwnd)?;
            let layout = LayoutControls::new(hwnd)?;
            let arcs = ArcControls::new(hwnd)?;
            let gameplay = GameplayControls::new(hwnd)?;
            let screenshots = ScreenshotControls::new(hwnd)?;
            let guide = HelpControls::new(hwnd)?;
            let minimap = MinimapControls::new(hwnd)?;
            let presentation = PresentationControls::new(hwnd)?;
            let language = LanguageControls::new(hwnd)?;
            let interface = InterfaceControls::new(hwnd)?;
            let custom_editor = control(
                hwnd,
                "BUTTON",
                "New / edit object, text or passive action...",
                234,
                [10, 235, 710, 30],
                0,
            );
            let profile_name = control(
                hwnd,
                "EDIT",
                "default",
                230,
                [10, 185, 300, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            let profile_save = control(
                hwnd,
                "BUTTON",
                tr!("Save profile"),
                231,
                [330, 185, 120, 26],
                0,
            );
            let profile_load = control(
                hwnd,
                "BUTTON",
                tr!("Load profile"),
                232,
                [460, 185, 120, 26],
                0,
            );
            let profile_delete = control(
                hwnd,
                "BUTTON",
                tr!("Delete profile"),
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
                SendMessageW(
                    pages,
                    CB_ADDSTRING,
                    0,
                    wide(page.localized_label(oteryn_client_ui::localization::current())).as_ptr()
                        as isize,
                );
            }
            SendMessageW(pages, CB_SETCURSEL, 0, 0);
            let help = control(
                hwnd,
                "STATIC",
                tr!(
                    "Key bindings: select an action, enter e.g. ALT+H, then Rebind.\nMouse buttons follow the selected preset. Conflicts include all registered actions."
                ),
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
                tr!("Chat-on preset (arrows)"),
                225,
                [10, 150, 300, 30],
                0,
            );
            let chat_off = control(
                hwnd,
                "BUTTON",
                tr!("Chat-off preset (WASD)"),
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
                tr!("Keyboard repeat delay ms (0–1000)"),
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
                tr!("Apply controls"),
                223,
                [500, 185, 180, 26],
                0,
            );
            let rotation_label = control(
                hwnd,
                "STATIC",
                tr!("Turn modifier (CTRL / ALT / SHIFT)"),
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
                tr!("World zoom % (80–180, fitted to window)"),
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
                tr!("Mute"),
                107,
                [250, 145, 100, 26],
                BS_AUTOCHECKBOX as u32,
            );
            let apply = control(hwnd, "BUTTON", tr!("Apply"), 106, [380, 145, 110, 26], 0);
            let transfer_label = control(
                hwnd,
                "STATIC",
                tr!("Settings transfer file (JSON). Import/defaults save a recoverable backup."),
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
            let import = control(hwnd, "BUTTON", tr!("Import..."), 110, [10, 265, 150, 30], 0);
            let export = control(
                hwnd,
                "BUTTON",
                tr!("Export new file"),
                111,
                [175, 265, 160, 30],
                0,
            );
            let backup = control(
                hwnd,
                "BUTTON",
                tr!("Restore backup..."),
                112,
                [350, 265, 170, 30],
                0,
            );
            let defaults = control(
                hwnd,
                "BUTTON",
                tr!("Restore defaults..."),
                113,
                [535, 265, 185, 30],
                0,
            );
            let hud_checks = [
                tr!("HP / MP bars"),
                tr!("Harmony"),
                tr!("Serene"),
                tr!("Light effects"),
                tr!("Combat effects"),
                tr!("Creature health / icons"),
                tr!("HP / MP circle"),
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
                miscellaneous,
                custom_editor,
                battle_filters,
                mixer,
                stability,
                battle_sounds,
                frames,
                language,
                presentation,
                minimap,
                guide,
                screenshots,
                gameplay,
                graphics,
                shortcuts,
                actor_hud,
                resources,
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
