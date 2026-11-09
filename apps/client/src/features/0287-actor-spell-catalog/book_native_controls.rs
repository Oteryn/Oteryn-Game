//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) labels: HWND,
    pub(super) search: HWND,
    pub(super) vocation: HWND,
    pub(super) premium: HWND,
    pub(super) group: HWND,
    pub(super) level_label: HWND,
    pub(super) min: HWND,
    pub(super) max: HWND,
    pub(super) apply: HWND,
    pub(super) refresh: HWND,
    pub(super) list: HWND,
    pub(super) details: HWND,
    pub(super) previous: HWND,
    pub(super) next: HWND,
    pub(super) archive: HWND,
    pub(super) slot: HWND,
    pub(super) assign: HWND,
    pub(super) clear: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = control(
                hwnd,
                "STATIC",
                "Search name or words                         Vocation                              Premium                         Cooldown group",
                0,
                [16, 12, 990, 24],
                0,
            );
            let search = control(
                hwnd,
                "EDIT",
                "",
                1101,
                [16, 40, 280, 28],
                WS_BORDER | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(search, EM_SETLIMITTEXT, 64, 0);
            let vocation = control(
                hwnd,
                "COMBOBOX",
                "",
                1102,
                [310, 40, 230, 280],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let premium = control(
                hwnd,
                "COMBOBOX",
                "",
                1103,
                [555, 40, 200, 160],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let group = control(
                hwnd,
                "COMBOBOX",
                "",
                1104,
                [770, 40, 230, 300],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let level_label = control(
                hwnd,
                "STATIC",
                "Level from / to (0 means unlimited)",
                0,
                [16, 86, 290, 24],
                0,
            );
            let min = control(
                hwnd,
                "EDIT",
                "0",
                1105,
                [310, 82, 100, 28],
                WS_BORDER | ES_NUMBER as u32,
            );
            let max = control(
                hwnd,
                "EDIT",
                "0",
                1106,
                [425, 82, 100, 28],
                WS_BORDER | ES_NUMBER as u32,
            );
            let apply = control(hwnd, "BUTTON", "Apply filters", 1108, [555, 82, 200, 30], 0);
            let refresh = control(hwnd, "BUTTON", "Refresh page", 1109, [770, 82, 230, 30], 0);
            let list = control(
                hwnd,
                "LISTBOX",
                "",
                1107,
                [16, 130, 465, 455],
                LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let details = control(
                hwnd,
                "EDIT",
                "",
                1112,
                [500, 130, 500, 455],
                WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            );
            let previous = control(hwnd, "BUTTON", "Previous page", 1110, [16, 600, 220, 30], 0);
            let next = control(hwnd, "BUTTON", "Next page", 1111, [250, 600, 230, 30], 0);
            let archive = control(
                hwnd,
                "BUTTON",
                "Magical Archive",
                1113,
                [500, 600, 190, 30],
                0,
            );
            let slot = control(
                hwnd,
                "COMBOBOX",
                "",
                1116,
                [700, 600, 95, 180],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for index in 1..=oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
                SendMessageW(
                    slot,
                    CB_ADDSTRING,
                    0,
                    wide(&format!("Slot {index}")).as_ptr() as isize,
                );
            }
            SendMessageW(slot, CB_SETCURSEL, 0, 0);
            let assign = control(hwnd, "BUTTON", "Assign", 1114, [805, 600, 90, 30], 0);
            let clear = control(hwnd, "BUTTON", "Clear", 1115, [905, 600, 90, 30], 0);
            for control in [slot, assign, clear] {
                ShowWindow(control, SW_HIDE);
            }

            Some(Self {
                labels,
                search,
                vocation,
                premium,
                group,
                level_label,
                min,
                max,
                apply,
                refresh,
                list,
                details,
                previous,
                next,
                archive,
                slot,
                assign,
                clear,
            })
        }
    }
}
