//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) search: HWND,
    pub(super) filter: HWND,
    pub(super) sort: HWND,
    pub(super) header: HWND,
    pub(super) list: HWND,
    pub(super) detail: HWND,
    pub(super) refresh: HWND,
    pub(super) previous: HWND,
    pub(super) next: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(h: HWND, weak: &Shared, backend: Backend) -> Option<Self> {
        unsafe {
            let search = ctl(
                h,
                "EDIT",
                "",
                10,
                [10, 10, 330, 26],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
            let filter = ctl(
                h,
                "COMBOBOX",
                "",
                11,
                [350, 10, 180, 150],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for label in [tr!("All achievements"), tr!("Regular"), tr!("Secret")] {
                SendMessageW(filter, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            SendMessageW(filter, CB_SETCURSEL, 0, 0);
            let sort = ctl(
                h,
                "COMBOBOX",
                "",
                12,
                [540, 10, 230, 150],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for label in [
                tr!("Name"),
                tr!("Points"),
                tr!("Grade"),
                tr!("Recently earned"),
            ] {
                SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            SendMessageW(sort, CB_SETCURSEL, 0, 0);
            let header = ctl(h, "STATIC", "", 13, [10, 48, 760, 45], 0);
            let list = ctl(
                h,
                "LISTBOX",
                "",
                14,
                [10, 100, 760, 275],
                WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
            );
            let detail = ctl(
                h,
                "EDIT",
                "",
                15,
                [10, 385, 760, 140],
                WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            );
            let refresh = ctl(
                h,
                "BUTTON",
                tr!("Load / refresh account"),
                2,
                [10, 540, 240, 32],
                0,
            );
            let previous = ctl(h, "BUTTON", tr!("Previous page"), 3, [270, 540, 240, 32], 0);
            let next = ctl(h, "BUTTON", tr!("Next page"), 4, [530, 540, 240, 32], 0);
            if [
                search, filter, sort, header, list, detail, refresh, previous, next,
            ]
            .iter()
            .any(|h| h.is_null())
            {
                DestroyWindow(h);
                close(&weak, backend);
                return None;
            }

            Some(Self {
                search,
                filter,
                sort,
                header,
                list,
                detail,
                refresh,
                previous,
                next,
            })
        }
    }
}
