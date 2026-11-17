//! Current item labels and native combo population for the Forge window.
use super::wide;
use oteryn_client_ui::item_forge::ItemForgePanel;
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) fn choices(panel: &ItemForgePanel, containers: bool) -> Vec<(u64, String)> {
    if panel.preview.selected {
        panel
            .preview
            .items
            .iter()
            .filter(|i| i.container == containers)
            .map(|i| {
                (
                    i.handle.get(),
                    format!(
                        "{} — class {}, tier {} / {}",
                        i.name, i.classification, i.tier, i.maximum_tier
                    ),
                )
            })
            .collect()
    } else {
        panel
            .preview
            .live
            .iter()
            .enumerate()
            .map(|(index, i)| {
                (
                    i.handle.get(),
                    format!(
                        "Carried item {} — name unavailable on this server",
                        index + 1
                    ),
                )
            })
            .collect()
    }
}
pub(super) unsafe fn populate(hwnd: HWND, items: &[(u64, String)], chosen: u64) {
    // SAFETY: all combo handles are local to this window thread.
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
        let mut selected = usize::MAX;
        for (index, (handle, label)) in items.iter().enumerate() {
            SendMessageW(hwnd, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            if *handle == chosen {
                selected = index;
            }
        }
        SendMessageW(hwnd, CB_SETCURSEL, selected, 0);
    }
}
