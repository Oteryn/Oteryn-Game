//! Settings controls on their owning native window thread.
use super::*;

pub(super) fn run(shared: Arc<Mutex<Shared>>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn Settings").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
            650,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            lock(&shared).open = false;
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
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
        let mouse_preset = control(
            hwnd,
            "BUTTON",
            "Swap primary / secondary mouse buttons",
            221,
            [10, 150, 360, 26],
            BS_AUTOCHECKBOX as u32,
        );
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
        let profiles = control(
            hwnd,
            "BUTTON",
            "Hotkey profiles...",
            120,
            [510, 145, 210, 26],
            0,
        );
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
        let help_page = control(
            hwnd,
            "BUTTON",
            "Help and reports...",
            121,
            [10, 310, 210, 30],
            0,
        );
        let controls_page = control(
            hwnd,
            "BUTTON",
            "Mouse and movement...",
            122,
            [250, 310, 240, 30],
            0,
        );
        let shortcuts_page = control(
            hwnd,
            "BUTTON",
            "Shortcut buttons...",
            123,
            [510, 310, 210, 30],
            0,
        );
        let minimap_page = control(
            hwnd,
            "BUTTON",
            "Minimap transfer...",
            124,
            [10, 350, 210, 30],
            0,
        );
        let frames_page = control(
            hwnd,
            "BUTTON",
            "Frame rate and indicators...",
            125,
            [250, 350, 280, 30],
            0,
        );
        let basic_page = control(
            hwnd,
            "BUTTON",
            "Basic options...",
            126,
            [550, 350, 170, 30],
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
                130 + index,
                [
                    10 + (index % 4) as i32 * 175,
                    400 + (index / 4) as i32 * 30,
                    170,
                    26,
                ],
                BS_AUTOCHECKBOX as u32,
            )
        })
        .collect::<Vec<_>>();
        let feedback = control(hwnd, "STATIC", "", 0, [10, 470, 710, 90], 0);
        if [
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
            profiles,
            help_page,
            controls_page,
            shortcuts_page,
            minimap_page,
            frames_page,
            basic_page,
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
            DestroyWindow(hwnd);
            lock(&shared).open = false;
            return;
        }
        let bindings = preference_bindings().unwrap_or_default();
        let mut held = None;
        let mut selected = -1;
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
            let mut state = lock(&shared);
            if state.shutdown {
                break;
            }
            let page_index = SendMessageW(pages, CB_GETCURSEL, 0, 0);
            if let Some(page) = usize::try_from(page_index)
                .ok()
                .and_then(|i| SettingsPage::ALL.get(i))
            {
                state.page = *page;
            }
            for (handles, page) in [
                (
                    &[
                        help,
                        list,
                        chord,
                        rebind,
                        mouse_preset,
                        delay_label,
                        delay,
                        controls_apply,
                        rotation_label,
                        rotation,
                    ][..],
                    SettingsPage::Gameplay,
                ),
                (&[label, scale][..], SettingsPage::Interface),
                (&[volume, mute][..], SettingsPage::Sound),
                (
                    &[
                        transfer_label,
                        transfer_path,
                        import,
                        export,
                        backup,
                        defaults,
                    ][..],
                    SettingsPage::Help,
                ),
            ] {
                for handle in handles {
                    ShowWindow(*handle, if state.page == page { SW_SHOW } else { SW_HIDE });
                }
            }
            for (index, handle) in hud_checks.iter().enumerate() {
                let page = if index < 3 {
                    SettingsPage::Interface
                } else {
                    SettingsPage::Graphics
                };
                ShowWindow(*handle, if state.page == page { SW_SHOW } else { SW_HIDE });
            }
            ShowWindow(
                apply,
                if matches!(
                    state.page,
                    SettingsPage::Interface | SettingsPage::Sound | SettingsPage::Graphics
                ) {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let index = SendMessageW(list, CB_GETCURSEL, 0, 0);
            if index != selected && index >= 0 {
                selected = index;
                if let Some(binding) = bindings.get(index as usize) {
                    SetWindowTextW(
                        chord,
                        wide(state.preferences.binding_chord(binding)).as_ptr(),
                    );
                }
            }
            for action in std::mem::take(&mut actions.0) {
                if (120..=126).contains(&action) {
                    drop(state);
                    subpages::open(action, Arc::clone(&shared), hwnd);
                    state = lock(&shared);
                    held = None;
                    continue;
                }
                if (110..=113).contains(&action) {
                    drop(state);
                    let confirmed = action == 111
                        || MessageBoxW(
                            hwnd,
                            wide("Replace your current settings? A backup will be saved first.")
                                .as_ptr(),
                            wide("Oteryn Settings").as_ptr(),
                            MB_YESNO | MB_ICONQUESTION,
                        ) == IDYES;
                    state = lock(&shared);
                    if !confirmed || state.shutdown {
                        continue;
                    }
                    let path = std::path::PathBuf::from(text(transfer_path));
                    let result = match action {
                        110 => state.import_from(&path),
                        111 => state.export_to(&path),
                        112 => state.restore_backup(),
                        _ => state.reset_defaults(),
                    };
                    state.feedback = match result {
                        Ok(()) if action == 111 => "Export saved. Existing files are never overwritten.".into(),
                        Ok(()) => "Settings replaced and saved. The previous settings are available through Restore backup.".into(),
                        Err(error) => format!("Settings transfer failed: {error}"),
                    };
                    continue;
                }
                let mut preferences = state.preferences.clone();
                if action == 223 {
                    match text(delay).parse::<u16>() {
                        Ok(value) => {
                            preferences.rotation_modifier =
                                text(rotation).trim().to_ascii_uppercase();
                            preferences.keyboard_delay_ms = value;
                            preferences.swap_mouse_buttons =
                                SendMessageW(mouse_preset, BM_GETCHECK, 0, 0) == 1;
                            state.apply(preferences);
                        }
                        Err(_) => {
                            state.feedback = "Enter a keyboard delay from 0 to 1000 ms".into()
                        }
                    }
                } else if action == 103 {
                    if let Some(binding) = bindings.get(selected.max(0) as usize) {
                        match preferences.rebind(binding.id, &text(chord)) {
                            Ok(()) => state.apply(preferences),
                            Err(error) => {
                                state.feedback = format!("{error}. Choose a different chord.")
                            }
                        }
                    }
                } else {
                    match (text(scale).parse::<u16>(), text(volume).parse::<u8>()) {
                        (Ok(scale), Ok(volume)) => {
                            preferences.ui_scale = scale;
                            preferences.audio.master_volume = volume;
                            preferences.audio.muted = SendMessageW(mute, BM_GETCHECK, 0, 0) == 1;
                            preferences.hud_bars =
                                SendMessageW(hud_checks[0], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_harmony =
                                SendMessageW(hud_checks[1], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_serene =
                                SendMessageW(hud_checks[2], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_lights =
                                SendMessageW(hud_checks[3], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_effects =
                                SendMessageW(hud_checks[4], BM_GETCHECK, 0, 0) == 1;
                            preferences.creature_bars =
                                SendMessageW(hud_checks[5], BM_GETCHECK, 0, 0) == 1;
                            preferences.health_circle =
                                SendMessageW(hud_checks[6], BM_GETCHECK, 0, 0) == 1;
                            state.apply(preferences);
                        }
                        _ => state.feedback = "Enter valid zoom and volume percentages".into(),
                    }
                }
            }
            if held.as_ref() != Some(&state.preferences) {
                SetWindowTextW(help, wide(&format!("Profile: {} / chat {} editor.\r\nProfiles configures both key slots and saved presets.", state.preferences.hotkey_profiles.selected, if state.preferences.hotkey_profiles.chat_on { "on" } else { "off" })).as_ptr());
                let selection = selected.max(0) as usize;
                SendMessageW(list, CB_RESETCONTENT, 0, 0);
                for binding in &bindings {
                    let label = format!(
                        "{}: {}",
                        binding.id,
                        state.preferences.binding_chord(binding)
                    );
                    SendMessageW(list, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                SendMessageW(list, CB_SETCURSEL, selection, 0);
                SetWindowTextW(
                    scale,
                    wide(&state.preferences.ui_scale.to_string()).as_ptr(),
                );
                SetWindowTextW(
                    volume,
                    wide(&state.preferences.audio.master_volume.to_string()).as_ptr(),
                );
                SendMessageW(
                    mute,
                    BM_SETCHECK,
                    usize::from(state.preferences.audio.muted),
                    0,
                );
                for (control, checked) in hud_checks.iter().zip([
                    state.preferences.hud_bars,
                    state.preferences.show_harmony,
                    state.preferences.show_serene,
                    state.preferences.show_lights,
                    state.preferences.show_effects,
                    state.preferences.creature_bars,
                    state.preferences.health_circle,
                ]) {
                    SendMessageW(*control, BM_SETCHECK, usize::from(checked), 0);
                }
                SendMessageW(
                    mouse_preset,
                    BM_SETCHECK,
                    usize::from(state.preferences.swap_mouse_buttons),
                    0,
                );
                SetWindowTextW(
                    delay,
                    wide(&state.preferences.keyboard_delay_ms.to_string()).as_ptr(),
                );
                SetWindowTextW(
                    rotation,
                    wide(&state.preferences.rotation_modifier).as_ptr(),
                );
                selected = -1;
                held = Some(state.preferences.clone());
            }
            let page_help = match state.page {
                SettingsPage::Graphics => {
                    "Renderer and window options use the supported native renderer. Additional controls are not available yet."
                }
                SettingsPage::Sound => {
                    "Master volume % (0–100). Changes apply to playback immediately."
                }
                _ => "",
            };
            SetWindowTextW(
                feedback,
                wide(&format!(
                    "{}\r\n{page_help}\r\n{}",
                    state.scope_label(),
                    state.feedback
                ))
                .as_ptr(),
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        lock(&shared).open = false;
    }
}
