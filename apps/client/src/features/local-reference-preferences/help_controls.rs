//! Local guide navigation and report launch controls.
use super::*;
use oteryn_client::play::help::{Report, Topic};

pub(crate) struct HelpControls {
    handles: [HWND; 4],
    displayed: String,
}
impl HelpControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let handles = [
                control(
                    hwnd,
                    "COMBOBOX",
                    "",
                    262,
                    [10, 60, 710, 160],
                    CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "",
                    263,
                    [10, 105, 710, 220],
                    WS_BORDER
                        | WS_VSCROLL
                        | ES_MULTILINE as u32
                        | ES_READONLY as u32
                        | ES_AUTOVSCROLL as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Report a client problem"),
                    260,
                    [10, 340, 340, 30],
                    0,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Report player behavior"),
                    261,
                    [370, 340, 350, 30],
                    0,
                ),
            ];
            if handles.iter().any(|h| h.is_null()) {
                return None;
            }
            for topic in Topic::ALL {
                SendMessageW(
                    handles[0],
                    CB_ADDSTRING,
                    0,
                    wide(topic.label()).as_ptr() as isize,
                );
            }
            SendMessageW(handles[0], CB_SETCURSEL, 0, 0);
            Some(Self {
                handles,
                displayed: String::new(),
            })
        }
    }
    pub(crate) unsafe fn update(&mut self, state: &SettingsState) {
        unsafe {
            let visible = state.page == SettingsPage::HelpGuide;
            for handle in self.handles {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
            if !visible {
                return;
            }
            let index = SendMessageW(self.handles[0], CB_GETCURSEL, 0, 0).max(0) as usize;
            let Some(topic) = Topic::ALL.get(index) else {
                return;
            };
            let text = state.help_text(*topic);
            if text != self.displayed {
                SetWindowTextW(self.handles[1], wide(&text).as_ptr());
                self.displayed = text;
            }
        }
    }
    pub(crate) fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        let report = match action {
            260 => Report::Bug,
            261 => Report::Player,
            _ => return false,
        };
        state.request_help_report(report);
        true
    }
}

pub(crate) fn page_hint(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::CustomHotkeys => {
            "Profile names use letters, digits, spaces, _ or -. Up to 16 saved profiles. Object, text and passive action assignment is not available yet."
        }
        SettingsPage::BarHotkeys => {
            tr!(
                "Choose a shortcut for each spell slot. Configure its screen position and grid under Interface / action bar layout."
            )
        }
        SettingsPage::Graphics => {
            tr!(
                "Light and combat effect visibility apply to the live world. HUD presentation is configured under Interface."
            )
        }
        SettingsPage::Sound => {
            tr!("Master volume % (0–100). Changes apply to playback immediately.")
        }
        _ => "",
    }
}
