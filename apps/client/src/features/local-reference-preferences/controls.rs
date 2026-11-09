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

#[path = "shortcut_controls.rs"]
mod shortcut_controls;
use shortcut_controls::ShortcutControls;
#[path = "graphics_controls.rs"]
mod graphics_controls;
use graphics_controls::GraphicsControls;
#[path = "layout_controls.rs"]
mod layout_controls;
use layout_controls::LayoutControls;
#[path = "console_controls.rs"]
mod console_controls;
use console_controls::ConsoleControls;
#[path = "hud_controls.rs"]
mod hud_controls;
use hud_controls::HudControls;
#[path = "frame_controls.rs"]
mod frame_controls;
use frame_controls::FrameControls;
#[path = "battle_sound_controls.rs"]
mod battle_sound_controls;
use battle_sound_controls::BattleSoundControls;

#[path = "mixer_controls.rs"]
mod mixer_controls;
use mixer_controls::MixerControls;

#[path = "battle_filter_controls.rs"]
mod battle_filter_controls;
use battle_filter_controls::BattleFilterControls;
pub(super) struct Controls {
    pub(super) language: LanguageControls,
    pub(super) presentation: PresentationControls,
    pub(super) minimap: MinimapControls,
    pub(super) guide: HelpControls,
    pub(super) screenshots: ScreenshotControls,
    pub(super) gameplay: GameplayControls,
    pub(super) graphics: GraphicsControls,
    pub(super) shortcuts: ShortcutControls,
    pub(super) battle_filters: BattleFilterControls,
    pub(super) actor_hud: HudControls,
    pub(super) console: ConsoleControls,
    pub(super) layout: LayoutControls,
    pub(super) mixer: MixerControls,
    pub(super) stability: HWND,
    pub(super) battle_sounds: BattleSoundControls,
    pub(super) frames: FrameControls,
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
    pub(super) manual: HWND,
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

#[path = "dialog_construction.rs"]
mod dialog_construction;

pub(super) unsafe fn update_feedback(handle: HWND, state: &SettingsState) {
    let hint = if state.page == SettingsPage::CustomHotkeys {
        super::super::navigation_controls::page_help(state.page)
    } else {
        page_hint(state.page)
    };
    unsafe {
        SetWindowTextW(
            handle,
            wide(&format!("{hint}\r\n{}", state.feedback)).as_ptr(),
        );
    }
}
impl Controls {
    pub(super) unsafe fn read_page(
        mixer: &MixerControls,
        frames: &FrameControls,
        battle: &BattleSoundControls,
        battle_filters: &BattleFilterControls,
        page: SettingsPage,
        preferences: &mut oteryn_client::play::ReferencePreferences,
    ) -> Result<(), String> {
        unsafe {
            match page {
                SettingsPage::Battle => battle_filters.read(preferences),
                SettingsPage::Mixer => mixer.read(preferences),
                SettingsPage::Effects => frames.read(preferences),
                SettingsPage::Sound => battle.read(preferences),
                _ => Ok(()),
            }
        }
    }
}
