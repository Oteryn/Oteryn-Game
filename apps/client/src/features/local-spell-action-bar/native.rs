//! Child control on the shell thread; its parent window owns the native lifetime.
// SAFETY: only this module calls Win32 for a child of winit's live window, on its thread.
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, MoveWindow, SW_HIDE, SW_SHOW, SetWindowTextW, ShowWindow,
    WS_CHILD, WS_VISIBLE,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

#[path = "../local-reference-preferences/arcs_native.rs"]
mod arcs;
#[path = "../local-reference-preferences/shortcut_strip.rs"]
mod shortcuts;
#[path = "slots_native.rs"]
mod slots;
pub(super) struct Hud {
    arcs: arcs::ArcHud,
    status: HWND,
    spells: slots::SpellRow,
    shortcuts: shortcuts::ShortcutStrip,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

impl Hud {
    pub(super) fn new(window: &Window) -> Option<Self> {
        let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // A STATIC control displays the spell status, HP/MP bars and numbers without taking
        // keyboard focus from gameplay. Win32 copies both strings during creation.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                8,
                8,
                900,
                40,
                handle.hwnd.get() as HWND,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return None;
        }
        let Some(shortcuts) = shortcuts::ShortcutStrip::new(handle.hwnd.get() as HWND) else {
            unsafe {
                DestroyWindow(hwnd);
            }
            return None;
        };
        let Some(arcs) = arcs::ArcHud::new(handle.hwnd.get() as HWND) else {
            unsafe {
                DestroyWindow(hwnd);
            }
            return None;
        };
        let Some(spells) = slots::SpellRow::new(handle.hwnd.get() as HWND) else {
            unsafe {
                DestroyWindow(hwnd);
            }
            return None;
        };
        Some(Self {
            spells,
            arcs,
            status: hwnd,
            shortcuts,
        })
    }

    pub(super) fn update(&self, text: Option<&str>, bounds: [u16; 4]) {
        unsafe {
            MoveWindow(
                self.status,
                i32::from(bounds[0]),
                i32::from(bounds[1]),
                i32::from(bounds[2]),
                i32::from(bounds[3]),
                1,
            );
            SetWindowTextW(self.status, wide(text.unwrap_or("")).as_ptr());
            ShowWindow(self.status, if text.is_some() { SW_SHOW } else { SW_HIDE });
        }
    }
}

impl Drop for Hud {
    fn drop(&mut self) {
        // DestroyWindow is also safe if the parent already destroyed its child.
        unsafe {
            DestroyWindow(self.status);
        }
    }
}

impl super::Application {
    pub(super) fn update_hud(&mut self) {
        if let (Some(play), Some(window)) = (&mut self.play, &self.window) {
            let audio = self.audio.status(&mut play.view);
            let movement = play.link.item_movement_status();
            let status = format!(
                "{movement} — {} — {audio} — Ctrl+arrows: Turn; Ctrl+F: Follow; Esc: Stop — {}",
                play.view.hud_status(),
                play.link.movement_feedback().unwrap_or("")
            );
            if let Some(hud) = &self.hud {
                hud.update(Some(&status), play.view.preferences().hud_bounds);
            }
            if let Some(hud) = &mut self.hud {
                hud.arcs.update(play.view.hud_arc_layout());
                let labels = play.link.spell_slot_labels();
                for slot in hud.spells.update(Some(&labels)) {
                    if let Some(slot) = u32::try_from(slot).ok().and_then(std::num::NonZeroU32::new)
                    {
                        play.link.cast_spell_slot(slot);
                    }
                }
                for id in hud
                    .shortcuts
                    .update(Some(&play.view.preferences().shortcuts))
                {
                    play.link.activate_shortcut(&id);
                }
            }
            if status != play.status {
                window.set_title(&format!(
                    "Oteryn — {status} — Ctrl+G: Guild — Ctrl+Shift+A: achievements"
                ));
                play.status = status;
            }
        }
        if self.play.is_none()
            && let Some(hud) = &mut self.hud
        {
            hud.spells.update(None);
            hud.arcs.update(None);
            hud.shortcuts.update(None);
            hud.update(None, [8, 8, 900, 40]);
        }
    }
}
