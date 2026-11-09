use super::*;
#[path = "interface_controls.rs"]
mod interface_controls;
use interface_controls::InterfaceControls;
#[path = "arc_controls.rs"]
mod arc_controls;
use arc_controls::ArcControls;
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

pub(super) struct Controls {
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

impl Controls {
    pub(super) unsafe fn read_page(
        mixer: &MixerControls,
        frames: &FrameControls,
        battle: &BattleSoundControls,
        page: SettingsPage,
        preferences: &mut oteryn_client::play::ReferencePreferences,
    ) -> Result<(), String> {
        unsafe {
            match page {
                SettingsPage::Mixer => mixer.read(preferences),
                SettingsPage::Graphics => frames.read(preferences),
                SettingsPage::Sound => battle.read(preferences),
                _ => Ok(()),
            }
        }
    }
}

#[path = "controls_construct.rs"]
mod construct;
