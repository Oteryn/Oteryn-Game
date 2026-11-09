//! Construction and validation of the settings dialog controls.
use super::*;
#[path = "interface_controls.rs"]
mod interface_controls;
use interface_controls::InterfaceControls;
#[path = "arc_controls.rs"]
mod arc_controls;
use arc_controls::ArcControls;
#[path = "gameplay_controls.rs"]
mod gameplay_controls;
use gameplay_controls::GameplayControls;
#[path = "help_controls.rs"]
mod help_controls;
pub(super) use help_controls::{HelpControls, page_hint};
#[path = "screenshot_controls.rs"]
mod screenshot_controls;
use screenshot_controls::ScreenshotControls;
#[path = "minimap_controls.rs"]
mod minimap_controls;
use minimap_controls::MinimapControls;
#[path = "presentation_controls.rs"]
mod presentation_controls;
use presentation_controls::PresentationControls;
#[path = "localization_controls.rs"]
mod localization_controls;
use localization_controls::LanguageControls;

pub(super) struct Controls {
    pub(super) language: LanguageControls,
    pub(super) presentation: PresentationControls,
    pub(super) minimap: MinimapControls,
    pub(super) guide: HelpControls,
    pub(super) screenshots: ScreenshotControls,
    pub(super) gameplay: GameplayControls,
    pub(super) arcs: ArcControls,
    pub(super) interface: InterfaceControls,
    pub(super) profile_name: HWND,
    pub(super) profile_save: HWND,
    pub(super) profile_load: HWND,
    pub(super) profile_delete: HWND,
    pub(super) pages: HWND,
    pub(super) help: HWND,
    pub(super) list: HWND,
    pub(super) chord: HWND,
    pub(super) rebind: HWND,
    pub(super) chat_on: HWND,
    pub(super) chat_off: HWND,
    pub(super) mouse_preset: HWND,
    pub(super) delay_label: HWND,
    pub(super) delay: HWND,
    pub(super) controls_apply: HWND,
    pub(super) rotation_label: HWND,
    pub(super) rotation: HWND,
    pub(super) label: HWND,
    pub(super) scale: HWND,
    pub(super) volume: HWND,
    pub(super) mute: HWND,
    pub(super) apply: HWND,
    pub(super) transfer_label: HWND,
    pub(super) transfer_path: HWND,
    pub(super) import: HWND,
    pub(super) export: HWND,
    pub(super) backup: HWND,
    pub(super) defaults: HWND,
    pub(super) hud_checks: Vec<HWND>,
    pub(super) feedback: HWND,
}

#[path = "controls_localized_3.rs"]
mod controls_localized_3;

pub(super) unsafe fn update_feedback(handle: HWND, state: &SettingsState) {
    unsafe {
        SetWindowTextW(
            handle,
            wide(&format!("{}\r\n{}", page_hint(state.page), state.feedback)).as_ptr(),
        );
    }
}
