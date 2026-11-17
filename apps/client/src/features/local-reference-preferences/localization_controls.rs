//! Persisted language selection; a restart applies the choice to every native window.
use super::*;
use oteryn_client::play::ReferencePreferences;
use oteryn_client_ui::localization::Language;
pub(crate) struct LanguageControls([HWND; 4]);
impl LanguageControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let handles = [
                control(hwnd, "STATIC", tr!("Language"), 0, [10, 60, 710, 26], 0),
                control(
                    hwnd,
                    "COMBOBOX",
                    "",
                    283,
                    [10, 100, 710, 200],
                    CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                ),
                control(
                    hwnd,
                    "STATIC",
                    tr!("Restart the client to apply this language to all windows."),
                    0,
                    [10, 145, 710, 70],
                    0,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Save language"),
                    282,
                    [10, 235, 710, 30],
                    0,
                ),
            ];
            if handles.iter().any(|handle| handle.is_null()) {
                return None;
            }
            for language in Language::ALL {
                SendMessageW(
                    handles[1],
                    CB_ADDSTRING,
                    0,
                    wide(language.name()).as_ptr() as isize,
                );
            }
            Some(Self(handles))
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        for handle in self.0 {
            unsafe {
                ShowWindow(handle, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            SendMessageW(self.0[1], CB_SETCURSEL, preferences.language() as usize, 0);
        }
    }
    pub(crate) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        if action != 282 {
            return false;
        }
        let index = unsafe { SendMessageW(self.0[1], CB_GETCURSEL, 0, 0) };
        if let Some(language) = usize::try_from(index)
            .ok()
            .and_then(|index| Language::ALL.get(index))
        {
            let mut preferences = state.preferences.clone();
            preferences.set_language(*language);
            state.apply(preferences);
        }
        true
    }
}
