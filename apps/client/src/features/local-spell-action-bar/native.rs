//! Docked HUD child windows are created and updated exclusively on the shell thread.
#![allow(
    unsafe_code,
    reason = "Win32 HUD controls stay on their creating UI thread"
)]
#[path = "native/events.rs"]
mod events;
#[path = "native/panes.rs"]
mod panes;
use events::Actions;
use oteryn_client::play::{HudAction, HudPanel};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::Controls::{ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx},
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

#[path = "../local-reference-preferences/arcs_native.rs"]
mod arcs;
#[path = "../local-reference-preferences/shortcut_strip.rs"]
mod shortcuts;
#[path = "slots_native.rs"]
mod slots;
pub(super) struct Hud {
    arcs: arcs::ArcHud,
    spells: slots::SpellRow,
    shortcuts: shortcuts::ShortcutStrip,
    panes: Vec<panes::Pane>,
    actions: Actions,
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
impl Hud {
    pub(super) fn new(window: &Window) -> Option<Self> {
        let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        let common = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        };
        if unsafe { InitCommonControlsEx(&common) } == 0 {
            return None;
        }
        let actions = Actions::default();
        let panes = HudPanel::ALL
            .into_iter()
            .map(|id| panes::Pane::new(handle.hwnd.get() as HWND, id, actions.clone()))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            panes,
            actions,
            arcs: arcs::ArcHud::new(handle.hwnd.get() as HWND)?,
            spells: slots::SpellRow::new(handle.hwnd.get() as HWND)?,
            shortcuts: shortcuts::ShortcutStrip::new(handle.hwnd.get() as HWND)?,
        })
    }
    pub(super) fn take_actions(&self) -> Vec<HudAction> {
        std::mem::take(&mut *self.actions.borrow_mut())
    }
    pub(super) fn hide(&self) {
        for pane in &self.panes {
            pane.hide();
        }
    }
}
impl super::Application {
    pub(super) fn update_hud(&mut self) {
        let menu = self.settings.take_hud_toggle();
        if let (Some(play), Some(window), Some(hud)) = (&mut self.play, &self.window, &mut self.hud)
        {
            let size = window.inner_size();
            let root =
                [size.width, size.height].map(|value| u16::try_from(value).unwrap_or(u16::MAX));
            let mut actions = hud.take_actions();
            if let Some(id) = menu {
                actions.push(HudAction::Visible(
                    id,
                    !play
                        .view
                        .preferences()
                        .effective_hud_layout()
                        .pane(id)
                        .visible,
                ));
            }
            for action in actions {
                match play.view.hud_action(action, root) {
                    Ok(()) => self.settings.capture(&mut play.view),
                    Err(error) => eprintln!("Could not apply HUD layout: {error}"),
                }
            }
            let audio = self.audio.status(&mut play.view);
            let movement = play.link.item_movement_status();
            let status = format!(
                "{movement} — {} — {audio} — Ctrl+arrows: Turn; Ctrl+F: Follow; Esc: Stop — {}",
                play.view.hud_status(),
                play.link.movement_feedback().unwrap_or("")
            );
            let texts = [
                status.clone(),
                play.view.combat_status(),
                play.view.hud_conditions_text(),
                play.view.hud_monk_text(),
            ];
            let layout = play.view.preferences().effective_hud_layout();
            self.settings.sync_hud_menu(&layout);
            hud.arcs.update(play.view.hud_arc_layout());
            let labels = play.link.spell_slot_labels();
            for slot in hud.spells.update(Some(&labels)) {
                if let Some(slot) = u32::try_from(slot).ok().and_then(std::num::NonZeroU32::new) {
                    play.link.cast_spell_slot(slot);
                }
            }
            for id in hud
                .shortcuts
                .update(Some(&play.view.preferences().shortcuts))
            {
                play.link.activate_shortcut(&id);
            }
            for (index, pane) in hud.panes.iter().enumerate() {
                let id = HudPanel::ALL[index];
                let rect =
                    if id == HudPanel::Resources && !play.view.preferences().status_panel_visible {
                        None
                    } else {
                        layout.rect(id, root, play.view.preferences().interface_scale)
                    };
                pane.update(
                    &texts[index],
                    rect,
                    play.view.actor_vitals(),
                    play.view.preferences().hud_bars,
                );
            }
            if status != play.status {
                window.set_title(&format!(
                    "Oteryn — {status} — Ctrl+G: Guild — Ctrl+Shift+A: achievements"
                ));
                play.status = status;
            }
        } else if let Some(hud) = &mut self.hud {
            hud.hide();
            hud.spells.update(None);
            hud.arcs.update(None);
            hud.shortcuts.update(None);
        }
    }
}
