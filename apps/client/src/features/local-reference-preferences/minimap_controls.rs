//! Portable minimap controls are distinct from settings-file transfer.
use super::*;
pub(crate) struct MinimapControls([HWND; 5]);
impl MinimapControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let handles = [
                control(
                    hwnd,
                    "STATIC",
                    tr!("Portable minimap file"),
                    0,
                    [10, 60, 710, 26],
                    0,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "",
                    272,
                    [10, 100, 710, 26],
                    WS_BORDER | ES_AUTOHSCROLL as u32,
                ),
                control(
                    hwnd,
                    "STATIC",
                    tr!(
                        "Import replaces this world's exploration and markers. Export first to keep a copy.\r\nFiles must match the current world and content generation. Join a world before transferring."
                    ),
                    0,
                    [10, 145, 710, 80],
                    0,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Import and replace"),
                    270,
                    [10, 245, 340, 30],
                    0,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Export new file"),
                    271,
                    [370, 245, 350, 30],
                    0,
                ),
            ];
            SendMessageW(handles[1], EM_SETLIMITTEXT, 240, 0);
            (!handles.iter().any(|h| h.is_null())).then_some(Self(handles))
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        for handle in self.0 {
            unsafe {
                ShowWindow(handle, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        if !matches!(action, 270 | 271) {
            return false;
        }
        state.request_minimap_transfer(
            action == 270,
            std::path::PathBuf::from(unsafe { text(self.0[1]) }),
        );
        true
    }
}
