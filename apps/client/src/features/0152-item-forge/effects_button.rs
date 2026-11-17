#![allow(
    unsafe_code,
    reason = "Native controls remain on their creating thread"
)]
use super::*;
use oteryn_client_ui::item_forge::ItemForgePanel;
pub(super) unsafe fn create(parent: HWND) -> HWND {
    unsafe {
        control(
            parent,
            "BUTTON",
            "Selected item tier effects",
            118,
            [250, 344, 260, 30],
            WS_TABSTOP,
        )
    }
}
pub(super) unsafe fn enable(hwnd: HWND, p: &ItemForgePanel, editable: bool) {
    unsafe {
        EnableWindow(
            hwnd,
            i32::from(
                editable
                    && p.preview.selected
                    && p.preview.item(p.first_item).is_some_and(|i| !i.container),
            ),
        );
    }
}
