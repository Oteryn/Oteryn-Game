//! Gameplay checkboxes remain on the settings window thread.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct GameplayControls([HWND; 3]);

impl GameplayControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        let handles = [
            "Show inspection actions in the context menu",
            "Switch chase off when walking manually",
            "Secondary click: loot nearby (off: context menu)",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, label)| unsafe {
            control(
                hwnd,
                "BUTTON",
                label,
                240 + index,
                [10, 260 + index as i32 * 32, 700, 26],
                BS_AUTOCHECKBOX as u32,
            )
        })
        .collect::<Vec<_>>();
        if handles.iter().any(|handle| handle.is_null()) {
            return None;
        }
        Some(Self([handles[0], handles[1], handles[2]]))
    }

    pub(crate) unsafe fn show(&self, visible: bool) {
        for handle in self.0 {
            unsafe {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        for (handle, checked) in self.0.into_iter().zip([
            preferences.gameplay.inspect_context_menu,
            preferences.gameplay.cancel_chase_on_walk,
            preferences.gameplay.secondary_click_loots_nearby,
        ]) {
            unsafe {
                SendMessageW(handle, BM_SETCHECK, usize::from(checked), 0);
            }
        }
    }

    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) {
        let values = self
            .0
            .map(|handle| unsafe { SendMessageW(handle, BM_GETCHECK, 0, 0) == 1 });
        preferences.gameplay.inspect_context_menu = values[0];
        preferences.gameplay.cancel_chase_on_walk = values[1];
        preferences.gameplay.secondary_click_loots_nearby = values[2];
    }
}
