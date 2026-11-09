//! Controls and layout for the searchable quest journal.
use super::*;
pub(super) struct Controls {
    pub(super) quests: HWND,
    pub(super) missions: HWND,
    pub(super) description: HWND,
    pub(super) refresh: HWND,
    pub(super) track: HWND,
    pub(super) list_search: HWND,
    pub(super) list_sort: HWND,
    pub(super) auto_add: HWND,
    pub(super) auto_remove: HWND,
    pub(super) filter: HWND,
    pub(super) search: HWND,
    pub(super) tracker: HWND,
}
pub(super) unsafe fn create(hwnd: HWND) -> Controls {
    // SAFETY: the native window thread owns the parent and every child control.
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
            tr!("Find number, status or known mission"),
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
            tr!("Quest list / Refresh"),
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
            tr!("Track quest"),
            TRACK,
            240,
            235,
            220,
            28,
            WS_TABSTOP,
        );
        let auto_add = control(
            hwnd,
            "BUTTON",
            tr!("Auto-add: off"),
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
            tr!("Auto-remove: off"),
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
            tr!("Tracker: all quests"),
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
            tr!("Search tracked missions"),
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
        Controls {
            quests,
            missions,
            description,
            refresh,
            track,
            list_search,
            list_sort,
            auto_add,
            auto_remove,
            filter,
            search,
            tracker,
        }
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Win32 control creation takes explicit layout bounds"
)]
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
) -> HWND {
    // SAFETY: parent is live on this thread; system class strings live for the call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
