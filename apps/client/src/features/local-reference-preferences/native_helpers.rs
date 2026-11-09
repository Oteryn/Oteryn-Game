use super::*;
pub(super) unsafe fn set_checked(handle: HWND, checked: bool) {
    unsafe {
        SendMessageW(handle, BM_SETCHECK, usize::from(checked), 0);
    }
}

pub(super) unsafe fn settings_window() -> HWND {
    unsafe {
        CreateWindowExW(
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
        )
    }
}

pub(super) unsafe fn refresh_hud_checks(
    hud_checks: &[HWND],
    preferences: &oteryn_client::play::ReferencePreferences,
) {
    unsafe {
        for (control, checked) in hud_checks.iter().zip([
            preferences.hud_bars,
            preferences.show_harmony,
            preferences.show_serene,
            preferences.show_lights,
            preferences.show_effects,
            preferences.creature_bars,
            preferences.health_circle,
        ]) {
            SendMessageW(*control, BM_SETCHECK, usize::from(checked), 0);
        }
    }
}

pub(super) unsafe fn refresh_scale_volume(
    scale: HWND,
    volume: HWND,
    preferences: &oteryn_client::play::ReferencePreferences,
) {
    unsafe {
        SetWindowTextW(scale, wide(&preferences.ui_scale.to_string()).as_ptr());
        SetWindowTextW(
            volume,
            wide(&preferences.audio.master_volume.to_string()).as_ptr(),
        );
    }
}

pub(super) unsafe fn refresh_input_controls(
    mouse: HWND,
    delay: HWND,
    rotation: HWND,
    preferences: &oteryn_client::play::ReferencePreferences,
) {
    unsafe {
        SendMessageW(
            mouse,
            CB_SETCURSEL,
            usize::from(preferences.swap_mouse_buttons),
            0,
        );
        SetWindowTextW(
            delay,
            wide(&preferences.keyboard_delay_ms.to_string()).as_ptr(),
        );
        SetWindowTextW(rotation, wide(&preferences.rotation_modifier).as_ptr());
    }
}
