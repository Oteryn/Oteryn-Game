//! Console filters and action-bar geometry apply through the persisted settings owner.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct PresentationControls {
    console: [HWND; 11],
    bar: [HWND; 14],
}
impl PresentationControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                tr!("Local speech"),
                tr!("Private messages and channels"),
                tr!("Guild messages"),
                tr!("Public channel messages"),
                tr!("Party messages"),
                tr!("Show message mode labels"),
                tr!("Show dropped-line notices"),
                tr!("Show server feedback"),
            ];
            let console = std::array::from_fn(|index| match index {
                0..=7 => control(
                    hwnd,
                    "BUTTON",
                    labels[index],
                    290 + index,
                    [10, 60 + index as i32 * 30, 710, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                8 => control(
                    hwnd,
                    "STATIC",
                    tr!("Recent received lines (1–1000)"),
                    0,
                    [10, 310, 500, 26],
                    0,
                ),
                9 => control(
                    hwnd,
                    "EDIT",
                    "1000",
                    298,
                    [560, 310, 90, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
                _ => control(
                    hwnd,
                    "BUTTON",
                    tr!("Apply console settings"),
                    280,
                    [10, 350, 710, 26],
                    0,
                ),
            });
            let labels = [
                tr!("Left position (pixels)"),
                tr!("Top position (pixels)"),
                tr!("Columns (1–4)"),
                tr!("Button width (24–160)"),
                tr!("Button height (24–160)"),
                tr!("Spacing (0–32)"),
            ];
            let bar = std::array::from_fn(|index| match index {
                0..=11 if index % 2 == 0 => control(
                    hwnd,
                    "STATIC",
                    labels[index / 2],
                    0,
                    [10, 60 + index as i32 / 2 * 40, 450, 26],
                    0,
                ),
                0..=11 => control(
                    hwnd,
                    "EDIT",
                    "0",
                    310 + index,
                    [500, 60 + index as i32 / 2 * 40, 150, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
                12 => control(
                    hwnd,
                    "BUTTON",
                    tr!("Show spell action bar"),
                    322,
                    [10, 310, 710, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                _ => control(
                    hwnd,
                    "BUTTON",
                    tr!("Apply action-bar layout"),
                    281,
                    [10, 350, 710, 26],
                    0,
                ),
            });
            if console.iter().chain(&bar).any(|handle| handle.is_null()) {
                return None;
            }
            Some(Self { console, bar })
        }
    }
    pub(crate) unsafe fn show(&self, page: SettingsPage) {
        for (handles, selected) in [
            (&self.console[..], SettingsPage::Console),
            (&self.bar[..], SettingsPage::ActionBarLayout),
        ] {
            for handle in handles {
                unsafe {
                    ShowWindow(*handle, if page == selected { SW_SHOW } else { SW_HIDE });
                }
            }
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            let settings = &preferences.console;
            for (handle, checked) in
                self.console[..8]
                    .iter()
                    .zip(settings.visible.into_iter().chain([
                        settings.mode_labels,
                        settings.dropped,
                        settings.feedback,
                    ]))
            {
                SendMessageW(*handle, BM_SETCHECK, usize::from(checked), 0);
            }
            SetWindowTextW(
                self.console[9],
                wide(&settings.line_limit.to_string()).as_ptr(),
            );
            let bar = &preferences.action_bar;
            for (index, value) in [
                bar.origin[0],
                bar.origin[1],
                u16::from(bar.columns),
                bar.size[0],
                bar.size[1],
                bar.gap,
            ]
            .into_iter()
            .enumerate()
            {
                SetWindowTextW(self.bar[index * 2 + 1], wide(&value.to_string()).as_ptr());
            }
            SendMessageW(self.bar[12], BM_SETCHECK, usize::from(bar.visible), 0);
        }
    }
    pub(crate) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        if !matches!(action, 280 | 281) {
            return false;
        }
        unsafe {
            let mut preferences = state.preferences.clone();
            if action == 280 {
                let Ok(limit) = text(self.console[9]).parse::<u16>() else {
                    state.feedback = tr!("Enter a console limit from 1 to 1000").into();
                    return true;
                };
                preferences.console.visible = std::array::from_fn(|index| {
                    SendMessageW(self.console[index], BM_GETCHECK, 0, 0) == 1
                });
                preferences.console.mode_labels =
                    SendMessageW(self.console[5], BM_GETCHECK, 0, 0) == 1;
                preferences.console.dropped = SendMessageW(self.console[6], BM_GETCHECK, 0, 0) == 1;
                preferences.console.feedback =
                    SendMessageW(self.console[7], BM_GETCHECK, 0, 0) == 1;
                preferences.console.line_limit = limit;
            } else {
                let mut values = [0_u16; 6];
                for (index, value) in values.iter_mut().enumerate() {
                    let Ok(parsed) = text(self.bar[index * 2 + 1]).parse() else {
                        state.feedback = tr!("Enter whole numbers for action-bar geometry").into();
                        return true;
                    };
                    *value = parsed;
                }
                let Ok(columns) = u8::try_from(values[2]) else {
                    state.feedback = tr!("Choose 1 to 4 columns").into();
                    return true;
                };
                preferences.action_bar.origin = [values[0], values[1]];
                preferences.action_bar.columns = columns;
                preferences.action_bar.size = [values[3], values[4]];
                preferences.action_bar.gap = values[5];
                preferences.action_bar.visible = SendMessageW(self.bar[12], BM_GETCHECK, 0, 0) == 1;
            }
            state.apply(preferences);
        }
        true
    }
}
