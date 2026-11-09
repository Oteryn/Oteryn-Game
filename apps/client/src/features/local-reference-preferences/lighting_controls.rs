//! Percent controls share the durable preference owner used by scene restoration.
use super::*;

pub(super) struct LightingControls([HWND; 9]);
impl LightingControls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                tr!("Minimum ambient brightness %"),
                tr!("Lower-floor dimming %"),
                tr!("Indoor dimming %"),
                tr!("Cloud dimming %"),
            ];
            let handles = std::array::from_fn(|index| match index {
                8 => control(
                    hwnd,
                    "BUTTON",
                    tr!("Apply lighting"),
                    283,
                    [330, 285, 380, 26],
                    0,
                ),
                index if index % 2 == 0 => control(
                    hwnd,
                    "STATIC",
                    labels[index / 2],
                    0,
                    [330, 105 + index as i32 / 2 * 40, 280, 26],
                    0,
                ),
                index => control(
                    hwnd,
                    "EDIT",
                    "0",
                    330 + index,
                    [640, 105 + index as i32 / 2 * 40, 70, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
            });
            (!handles.iter().any(|h| h.is_null())).then_some(Self(handles))
        }
    }
    pub(super) unsafe fn show(&self, page: SettingsPage) {
        for handle in self.0 {
            unsafe {
                ShowWindow(
                    handle,
                    if page == SettingsPage::Graphics {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
    }
    pub(super) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        let graphics = preferences.graphics;
        for (index, value) in [
            graphics.ambient,
            graphics.separator,
            graphics.indoor,
            graphics.cloud,
        ]
        .into_iter()
        .enumerate()
        {
            unsafe {
                SetWindowTextW(self.0[index * 2 + 1], wide(&value.to_string()).as_ptr());
            }
        }
    }
    pub(super) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        if action != 283 {
            return false;
        }
        let mut values = [0_u8; 4];
        for (index, value) in values.iter_mut().enumerate() {
            let Ok(parsed) = unsafe { text(self.0[index * 2 + 1]) }.parse::<u8>() else {
                state.feedback = tr!("Enter lighting percentages from 0 to 100").into();
                return true;
            };
            if parsed > 100 {
                state.feedback = tr!("Enter lighting percentages from 0 to 100").into();
                return true;
            }
            *value = parsed;
        }
        let mut preferences = state.preferences.clone();
        preferences.graphics.ambient = values[0];
        preferences.graphics.separator = values[1];
        preferences.graphics.indoor = values[2];
        preferences.graphics.cloud = values[3];
        state.apply(preferences);
        true
    }
}
