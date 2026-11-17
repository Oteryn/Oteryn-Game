use super::*;

pub(super) unsafe fn read(
    checks: &[HWND],
    preferences: &mut oteryn_client::play::ReferencePreferences,
) {
    unsafe {
        preferences.hud_bars = SendMessageW(checks[0], BM_GETCHECK, 0, 0) == 1;
        preferences.show_harmony = SendMessageW(checks[1], BM_GETCHECK, 0, 0) == 1;
        preferences.show_serene = SendMessageW(checks[2], BM_GETCHECK, 0, 0) == 1;
        preferences.show_lights = SendMessageW(checks[3], BM_GETCHECK, 0, 0) == 1;
        preferences.show_effects = SendMessageW(checks[4], BM_GETCHECK, 0, 0) == 1;
        preferences.creature_bars = SendMessageW(checks[5], BM_GETCHECK, 0, 0) == 1;
        preferences.health_circle = SendMessageW(checks[6], BM_GETCHECK, 0, 0) == 1;
    }
}
