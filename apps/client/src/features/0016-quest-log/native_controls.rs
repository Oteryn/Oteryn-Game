//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) quests: HWND,
    pub(super) list_search: HWND,
    pub(super) list_sort: HWND,
    pub(super) missions: HWND,
    pub(super) description: HWND,
    pub(super) track: HWND,
    pub(super) auto_add: HWND,
    pub(super) auto_remove: HWND,
    pub(super) filter: HWND,
    pub(super) search: HWND,
    pub(super) tracker: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(
        hwnd: HWND,
        weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    ) -> Option<Self> {
        unsafe {
            let quests = control(
                hwnd,
                "LISTBOX",
                "",
                QUESTS,
                10,
                94,
                220,
                136,
                LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER,
            );
            control(
                hwnd,
                "STATIC",
                "Find number, status or known mission",
                112,
                10,
                10,
                220,
                20,
                0,
            );
            let list_search = control(
                hwnd,
                "EDIT",
                "",
                113,
                10,
                32,
                220,
                24,
                WS_BORDER | WS_TABSTOP,
            );
            SendMessageW(
                list_search,
                windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
                128,
                0,
            );
            let list_sort = control(
                hwnd,
                "COMBOBOX",
                "",
                114,
                10,
                62,
                220,
                200,
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
            );
            for label in QUEST_SORT_LABELS {
                SendMessageW(list_sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let missions = control(
                hwnd,
                "LISTBOX",
                "",
                MISSIONS,
                240,
                10,
                490,
                220,
                LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER,
            );
            let description = control(
                hwnd,
                "EDIT",
                "",
                105,
                10,
                270,
                720,
                260,
                ES_MULTILINE as u32
                    | ES_READONLY as u32
                    | ES_AUTOVSCROLL as u32
                    | WS_VSCROLL
                    | WS_BORDER
                    | WS_TABSTOP,
            );
            let refresh = control(
                hwnd,
                "BUTTON",
                "Quest list / Refresh",
                REFRESH,
                10,
                235,
                220,
                28,
                WS_TABSTOP,
            );
            let track = control(
                hwnd,
                "BUTTON",
                "Track quest",
                TRACK,
                240,
                235,
                220,
                28,
                WS_TABSTOP,
            );
            if [
                quests,
                missions,
                description,
                refresh,
                track,
                list_search,
                list_sort,
            ]
            .iter()
            .any(|h| h.is_null())
            {
                DestroyWindow(hwnd);
                close(&weak);
                return None;
            }
            let auto_add = control(
                hwnd,
                "BUTTON",
                "Auto-add: off",
                AUTO_ADD,
                750,
                10,
                290,
                28,
                WS_TABSTOP,
            );
            let auto_remove = control(
                hwnd,
                "BUTTON",
                "Auto-remove: off",
                AUTO_REMOVE,
                750,
                45,
                290,
                28,
                WS_TABSTOP,
            );
            let filter = control(
                hwnd,
                "BUTTON",
                "Tracker: all quests",
                FILTER,
                750,
                80,
                290,
                28,
                WS_TABSTOP,
            );
            control(
                hwnd,
                "STATIC",
                "Search tracked missions",
                109,
                750,
                115,
                290,
                22,
                0,
            );
            let search = control(
                hwnd,
                "EDIT",
                "",
                110,
                750,
                140,
                290,
                28,
                WS_BORDER | WS_TABSTOP,
            );
            SendMessageW(
                search,
                windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
                128,
                0,
            );
            let tracker = control(
                hwnd,
                "EDIT",
                "",
                111,
                750,
                180,
                290,
                350,
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_BORDER,
            );

            Some(Self {
                quests,
                list_search,
                list_sort,
                missions,
                description,
                track,
                auto_add,
                auto_remove,
                filter,
                search,
                tracker,
            })
        }
    }
}
