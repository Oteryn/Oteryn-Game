//! Position, bounds and compact style for the existing native status panel.
use super::*;
use oteryn_client::play::ReferencePreferences;
#[path = "status_order_controls.rs"]
mod status_order_controls;
use status_order_controls::OrderControls;

pub(crate) struct LayoutControls {
    order: OrderControls,
    visible: HWND,
    label: HWND,
    bounds: [HWND; 4],
    compact: HWND,
}
impl LayoutControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let order = OrderControls::new(hwnd)?;
            let visible = control(
                hwnd,
                "BUTTON",
                "Show status panel",
                0,
                [330, 185, 250, 26],
                BS_AUTOCHECKBOX as u32,
            );
            let label = control(
                hwnd,
                "STATIC",
                "Status panel X / Y / width / height (client pixels)",
                0,
                [10, 110, 700, 26],
                0,
            );
            let bounds = [0, 1, 2, 3].map(|index| {
                control(
                    hwnd,
                    "EDIT",
                    "0",
                    0,
                    [10 + index * 120, 145, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                )
            });
            let compact = control(
                hwnd,
                "BUTTON",
                "Compact vitals style",
                0,
                [10, 185, 300, 26],
                BS_AUTOCHECKBOX as u32,
            );
            (![label, compact, visible]
                .iter()
                .chain(bounds.iter())
                .any(|handle| handle.is_null()))
            .then_some(Self {
                order,
                visible,
                label,
                bounds,
                compact,
            })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            self.order.show(visible);
            for handle in [self.label, self.compact, self.visible]
                .into_iter()
                .chain(self.bounds)
            {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            for (value, handle) in preferences.hud_bounds.iter_mut().zip(self.bounds) {
                *value = text(handle)
                    .parse()
                    .map_err(|_| "Invalid status panel bounds".to_owned())?;
            }
            preferences.status_panel_visible = SendMessageW(self.visible, BM_GETCHECK, 0, 0) == 1;
            preferences.compact_status = SendMessageW(self.compact, BM_GETCHECK, 0, 0) == 1;
            Ok(())
        }
    }
    pub(crate) unsafe fn place_apply(&self, apply: HWND, active: bool) {
        let [x, y] = if active { [610, 185] } else { [380, 145] };
        unsafe {
            MoveWindow(apply, x, y, 110, 26, 1);
        }
    }
    pub(crate) unsafe fn reorder(
        &self,
        action: usize,
        preferences: &mut ReferencePreferences,
    ) -> bool {
        unsafe { self.order.reorder(action, preferences) }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            self.order.refresh(preferences);
            SendMessageW(
                self.visible,
                BM_SETCHECK,
                usize::from(preferences.status_panel_visible),
                0,
            );
            for (handle, value) in self.bounds.iter().zip(preferences.hud_bounds) {
                SetWindowTextW(*handle, wide(&value.to_string()).as_ptr());
            }
            SendMessageW(
                self.compact,
                BM_SETCHECK,
                usize::from(preferences.compact_status),
                0,
            );
        }
    }
}
