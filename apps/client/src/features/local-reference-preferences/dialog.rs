//! Settings message pump and preference application on its owning thread.
use super::*;
#[path = "controls.rs"]
mod controls;
#[path = "resource_checks.rs"]
mod resource_checks;
use controls::Controls;
#[path = "custom_editor.rs"]
mod custom_editor;
#[path = "transfer.rs"]
mod transfer;
#[path = "transfer.rs"]
mod transfer;

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
        let Some(Controls {
            graphics,
            mut shortcuts,
            actor_hud,
            manual,
            console,
            layout,
            arcs,
            interface,
            custom_editor,
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
        }) = Controls::new(hwnd)
        else {
            DestroyWindow(hwnd);
            lock(&shared).open = false;
            return;
        };
        let navigation = navigation_controls::create(hwnd);
        let mut bindings = preference_bindings().unwrap_or_default();
        let mut binding_page = SettingsPage::Hotkeys;
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
            if let Ok(index) = usize::try_from(page_index) {
                state.select_page(index);
            }
            if binding_page != state.page {
                layout.place_apply(apply, state.page == SettingsPage::Status);
                binding_page = state.page;
                bindings = preference_bindings()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|binding| {
                        state.page != SettingsPage::BarHotkeys
                            || binding.id.starts_with("local-spell-action-bar-spell-")
                    })
                    .collect();
                held = None;
                selected = -1;
            }
            for handle in [help, list, chord, rebind] {
                ShowWindow(
                    handle,
                    if matches!(
                        state.page,
                        SettingsPage::Hotkeys
                            | SettingsPage::BarHotkeys
                            | SettingsPage::CustomHotkeys
                    ) {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
            shortcuts.show(state.page == SettingsPage::Shortcuts);
            graphics.show(state.page == SettingsPage::Graphics);
            for handle in &navigation {
                ShowWindow(*handle, SW_SHOW);
            }
            actor_hud.show(state.page == SettingsPage::ActorHud);
            console.show(state.page == SettingsPage::Console);
            layout.show(state.page == SettingsPage::Status);
            actor_hud.show(state.page == SettingsPage::ActorHud);
            console.show(state.page == SettingsPage::Console);
            layout.show(state.page == SettingsPage::Status);
            arcs.show(state.page == SettingsPage::Arcs);
            interface.show(state.page == SettingsPage::Interface);
            for (handles, page) in [
                (
                    &[
                        custom_editor,
                        profile_name,
                        profile_save,
                        profile_load,
                        profile_delete,
                    ][..],
                    SettingsPage::CustomHotkeys,
                ),
                (&[chat_on, chat_off, manual][..], SettingsPage::Hotkeys),
                (
                    &[
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
                    SettingsPage::Interface
                        | SettingsPage::Status
                        | SettingsPage::Sound
                        | SettingsPage::Graphics
                        | SettingsPage::Arcs
                        | SettingsPage::ActorHud
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
                if shortcuts.handle(action, &mut state) {
                    continue;
                }
                if (160..=166).contains(&action) {
                    drop(state);
                    subpages::open(action, Arc::clone(&shared), hwnd);
                    state = lock(&shared);
                    continue;
                }
                if (110..=113).contains(&action) {
                    drop(state);
                    transfer::apply(action, hwnd, &shared, &text(transfer_path));
                    state = lock(&shared);
                    continue;
                }
                let mut preferences = state.preferences.clone();
                if (231..=234).contains(&action) {
                    drop(state);
                    custom_editor::profile_action(action, hwnd, &shared, &text(profile_name));
                    state = lock(&shared);
                } else if matches!(action, 225 | 226) {
                    if let Err(error) = state.apply_chat_preset(action == 225) {
                        state.feedback = error.to_string();
                    }
                } else if action == 227 {
                    state.manual_hotkeys();
                } else if action == 223 {
                    match text(delay).parse::<u16>() {
                        Ok(value) => {
                            preferences.rotation_modifier =
                                text(rotation).trim().to_ascii_uppercase();
                            preferences.keyboard_delay_ms = value;
                            let preset = SendMessageW(mouse_preset, CB_GETCURSEL, 0, 0);
                            if usize::try_from(preset)
                                .ok()
                                .and_then(|index| preferences.set_mouse_preset(index).ok())
                                .is_none()
                            {
                                state.feedback = "Select a mouse preset".into();
                                continue;
                            }
                            state.apply(preferences);
                        }
                        Err(_) => {
                            state.feedback = "Enter a keyboard delay from 0 to 1000 ms".into()
                        }
                    }
                } else if (241..=244).contains(&action) {
                    if state.page == SettingsPage::Status
                        && layout.reorder(action, &mut preferences)
                    {
                        state.apply(preferences);
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
                            resource_checks::read(&hud_checks, &mut preferences);
                            let result = match state.page {
                                SettingsPage::Graphics => {
                                    graphics.read(&mut preferences);
                                    Ok(())
                                }
                                SettingsPage::Interface => interface.read(&mut preferences),
                                SettingsPage::Arcs => arcs.read(&mut preferences),
                                SettingsPage::Status => layout.read(&mut preferences),
                                SettingsPage::Console => console.read(&mut preferences),
                                SettingsPage::ActorHud => {
                                    actor_hud.read(&mut preferences);
                                    Ok(())
                                }
                                _ => Ok(()),
                            };
                            if let Err(error) = result {
                                state.feedback = error;
                                continue;
                            }
                            state.apply(preferences);
                        }
                        _ => state.feedback = "Enter valid zoom and volume percentages".into(),
                    }
                }
            }
            if held.as_ref() != Some(&state.preferences) {
                shortcuts.refresh(&state);
                graphics.refresh(&state.preferences);
                actor_hud.refresh(&state.preferences);
                console.refresh(&state.preferences);
                layout.refresh(&state.preferences);
                arcs.refresh(&state.preferences);
                interface.refresh(&state.preferences);
                let selection = selected.max(0) as usize;
                SendMessageW(list, CB_RESETCONTENT, 0, 0);
                for binding in &bindings {
                    let label = format!(
                        "{}: {}",
                        binding.label(),
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
                    CB_SETCURSEL,
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
            let page_help = navigation_controls::page_help(state.page);
            SetWindowTextW(
                feedback,
                wide(&format!("{page_help}\r\n{}", state.feedback)).as_ptr(),
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
#[path = "navigation_controls.rs"]
mod navigation_controls;
