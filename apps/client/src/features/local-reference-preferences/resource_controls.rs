//! Independent resource/condition presentation controls on the Interface settings page.
use super::*;
use oteryn_client::play::ResourceHudOptions;
pub(crate) struct ResourceControls([HWND; 5]);
impl ResourceControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let mut handles = [null_mut(); 5];
            for (index, label) in [
                "HP / MP values",
                "Soul",
                "Stamina",
                "Conditions",
                "Rest icons",
            ]
            .into_iter()
            .enumerate()
            {
                handles[index] = control(
                    hwnd,
                    "BUTTON",
                    label,
                    260 + index,
                    [
                        10 + (index % 3) as i32 * 230,
                        350 + (index / 3) as i32 * 30,
                        220,
                        26,
                    ],
                    BS_AUTOCHECKBOX as u32,
                );
            }
            handles
                .iter()
                .all(|handle| !handle.is_null())
                .then_some(Self(handles))
        }
    }
    pub(crate) unsafe fn apply(&self, options: &mut ResourceHudOptions) {
        unsafe {
            let values = self
                .0
                .map(|handle| SendMessageW(handle, BM_GETCHECK, 0, 0) == 1);
            *options = ResourceHudOptions {
                values: values[0],
                soul: values[1],
                stamina: values[2],
                conditions: values[3],
                rest: values[4],
            };
        }
    }
    pub(crate) unsafe fn sync(&self, options: &ResourceHudOptions) {
        unsafe {
            for (handle, checked) in self.0.iter().zip([
                options.values,
                options.soul,
                options.stamina,
                options.conditions,
                options.rest,
            ]) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(checked), 0);
            }
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.0 {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
}
