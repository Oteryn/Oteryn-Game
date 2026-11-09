//! The bar buttons feed the existing rebinding and spell-slot input path.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(super) struct Buttons([HWND; 4]);
impl Buttons {
    pub(super) unsafe fn new(parent: HWND) -> Option<Self> {
        unsafe {
            let handles = std::array::from_fn(|slot| {
                control(parent, "BUTTON", "", 25300 + slot, [0, 0, 64, 30], 0)
            });
            if handles.iter().any(|handle| handle.is_null()) {
                for handle in handles {
                    if !handle.is_null() {
                        DestroyWindow(handle);
                    }
                }
                return None;
            }
            Some(Self(handles))
        }
    }
    pub(super) unsafe fn show(
        &self,
        preferences: &ReferencePreferences,
        viewport: [u32; 2],
        playing: bool,
    ) {
        let bounds = preferences.action_bar.rects(viewport);
        let bindings = preference_bindings().unwrap_or_default();
        for (slot, (handle, [x, y, width, height])) in self.0.iter().zip(bounds).enumerate() {
            let id = format!("local-spell-action-bar-spell-{}", slot + 1);
            let label = bindings
                .iter()
                .find(|binding| binding.id == id)
                .map_or_else(
                    || (slot + 1).to_string(),
                    |binding| preferences.binding_chord(binding).to_owned(),
                );
            unsafe {
                MoveWindow(*handle, x, y, width, height, 1);
                SetWindowTextW(*handle, wide(&label).as_ptr());
                ShowWindow(
                    *handle,
                    if playing && preferences.action_bar.visible {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
    }
}
impl Drop for Buttons {
    fn drop(&mut self) {
        for handle in self.0 {
            unsafe {
                DestroyWindow(handle);
            }
        }
    }
}
