//! Native performance overlay, owned by the game window thread.
use super::*;
impl Settings {
    pub(in crate::windows_shell) fn performance(&self) -> oteryn_client::play::PerformanceSettings {
        self.frame_preferences().into()
    }
    pub(in crate::windows_shell) fn performance_indicator(&self, value: Option<&str>) {
        unsafe {
            if !self.performance_label.is_null() {
                let mut rect: RECT = std::mem::zeroed();
                if GetClientRect(self.parent, &mut rect) != 0 {
                    MoveWindow(
                        self.performance_label,
                        (rect.right - 310).max(8),
                        50,
                        300,
                        26,
                        1,
                    );
                }
                SetWindowTextW(self.performance_label, wide(value.unwrap_or("")).as_ptr());
                ShowWindow(
                    self.performance_label,
                    if value.is_some() { SW_SHOW } else { SW_HIDE },
                );
            }
        }
    }
}
