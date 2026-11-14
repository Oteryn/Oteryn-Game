use super::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
pub(crate) struct MiscellaneousControls {
    checks: [HWND; 5],
    buttons: [HWND; 3],
    web: [HWND; 2],
    hint: HWND,
}
impl MiscellaneousControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                "Stay logged in during this client session",
                "Remember this device after restart (OS credential vault)",
                "Quick login: use the last available character",
                "Confirm game logout",
                "Confirm closing Oteryn",
            ];
            let checks = std::array::from_fn(|i| {
                control(
                    hwnd,
                    "BUTTON",
                    labels[i],
                    9101 + i,
                    [10, 100 + i as i32 * 35, 710, 28],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            let buttons = std::array::from_fn(|i| {
                control(
                    hwnd,
                    "BUTTON",
                    [
                        "Apply session and confirmation settings",
                        "Reset confirmations",
                        "Sign out and forget this device",
                    ][i],
                    9106 + i,
                    [10, 290 + i as i32 * 35, 710, 28],
                    0,
                )
            });
            let web = std::array::from_fn(|i| {
                control(
                    hwnd,
                    "BUTTON",
                    [
                        "Support tickets (opens the browser)",
                        "Cookie policy (opens the browser)",
                    ][i],
                    9109 + i,
                    [10, 395 + i as i32 * 35, 710, 28],
                    0,
                )
            });
            let hint = control(
                hwnd,
                "STATIC",
                "Remembered login requires explicit opt-in and the Platform feature switch. No password or OAuth refresh token is saved. Closing the client preserves a remembered family; Sign out forgets it.",
                0,
                [10, 55, 710, 42],
                0,
            );
            if checks
                .iter()
                .chain(&buttons)
                .chain(&web)
                .any(|h| h.is_null())
                || hint.is_null()
            {
                None
            } else {
                Some(Self {
                    checks,
                    buttons,
                    web,
                    hint,
                })
            }
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for h in self
                .checks
                .iter()
                .chain(&self.buttons)
                .chain(&self.web)
                .chain(std::iter::once(&self.hint))
            {
                ShowWindow(*h, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn refresh(&self, p: &oteryn_client::play::ReferencePreferences) {
        unsafe {
            let m = p.miscellaneous;
            for (h, value) in self.checks.iter().zip([
                m.stay_logged_in,
                m.remember_device,
                m.quick_login,
                m.confirm_logout,
                m.confirm_exit,
            ]) {
                set_checked(*h, value);
            }
            let enabled =
                std::env::var("OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED").as_deref() == Ok("true");
            EnableWindow(self.checks[0], i32::from(enabled));
            EnableWindow(self.checks[2], i32::from(enabled));
            EnableWindow(self.checks[1], i32::from(enabled && cfg!(windows)));
        }
    }
    pub(crate) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        unsafe {
            match action {
                9106 => {
                    let mut preferences = state.preferences.clone();
                    let values: [bool; 5] = std::array::from_fn(|i| {
                        SendMessageW(self.checks[i], BM_GETCHECK, 0, 0) == 1
                    });
                    preferences.miscellaneous = oteryn_client::play::MiscellaneousPreferences {
                        stay_logged_in: values[0] || values[1],
                        remember_device: values[1],
                        quick_login: values[2],
                        confirm_logout: values[3],
                        confirm_exit: values[4],
                    };
                    let revoke = state.preferences.miscellaneous.stay_logged_in
                        && !preferences.miscellaneous.stay_logged_in
                        || state.preferences.miscellaneous.remember_device
                            && !preferences.miscellaneous.remember_device;
                    state.apply(preferences);
                    if revoke {
                        state.request_native_sign_out();
                    }
                    true
                }
                9107 => {
                    let mut p = state.preferences.clone();
                    p.miscellaneous.confirm_exit = true;
                    p.miscellaneous.confirm_logout = true;
                    state.apply(p);
                    true
                }
                9108 => {
                    let mut p = state.preferences.clone();
                    p.miscellaneous.remember_device = false;
                    state.apply(p);
                    state.request_native_sign_out();
                    state
                        .feedback
                        .push_str(" Signing out and clearing the OS credential vault.");
                    true
                }
                9109 => {
                    state.request_web_link(oteryn_client::web_links::WebLink::SupportTickets);
                    true
                }
                9110 => {
                    state.request_web_link(oteryn_client::web_links::WebLink::CookiePolicy);
                    true
                }
                _ => false,
            }
        }
    }
}
