//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) headings: Vec<HWND>,
    pub(super) lists: Vec<HWND>,
    pub(super) text: HWND,
    pub(super) active: HWND,
    pub(super) title: HWND,
    pub(super) search: HWND,
    pub(super) maximum: HWND,
    pub(super) players: HWND,
    pub(super) creatures: HWND,
    pub(super) sort: HWND,
    pub(super) controls: Vec<HWND>,
}
impl PanelControls {
    pub(super) unsafe fn new(hwnd: HWND, weak: &Weak<Mutex<Mailbox>>) -> Option<Self> {
        unsafe {
            let headings: Vec<_> = (0..4)
                .map(|i| control(hwnd, "STATIC", "", 0, (10 + i * 180, 10, 170, 25), 0))
                .collect();
            let lists: Vec<_> = (0..4)
                .map(|i| {
                    control(
                        hwnd,
                        "LISTBOX",
                        "",
                        200 + i as usize,
                        (10 + i * 180, 40, 170, 260),
                        LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_HSCROLL,
                    )
                })
                .collect();
            let stop = control(
                hwnd,
                "BUTTON",
                tr!("Stop attack"),
                101,
                (10, 320, 130, 30),
                0,
            );
            for &list in &lists {
                SendMessageW(list, LB_SETHORIZONTALEXTENT, 2048, 0);
            }
            let offensive = control(
                hwnd,
                "BUTTON",
                tr!("Offensive"),
                102,
                (150, 320, 110, 30),
                0,
            );
            let balanced = control(hwnd, "BUTTON", tr!("Balanced"), 103, (270, 320, 110, 30), 0);
            let defensive = control(
                hwnd,
                "BUTTON",
                tr!("Defensive"),
                104,
                (390, 320, 110, 30),
                0,
            );
            let stand = control(hwnd, "BUTTON", tr!("Stand"), 107, (510, 320, 100, 30), 0);
            let chase = control(hwnd, "BUTTON", tr!("Chase"), 108, (620, 320, 100, 30), 0);
            let text = control(hwnd, "STATIC", "", 109, (10, 360, 720, 60), 0);
            let editor_label = control(
                hwnd,
                "STATIC",
                tr!("List / name / name filter / maximum HP %"),
                0,
                (10, 425, 720, 25),
                0,
            );
            let active = control(
                hwnd,
                "COMBOBOX",
                "",
                300,
                (10, 455, 170, 150),
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let title = control(hwnd, "EDIT", "", 301, (190, 455, 170, 26), WS_BORDER);
            let search = control(hwnd, "EDIT", "", 302, (370, 455, 170, 26), WS_BORDER);
            let maximum = control(
                hwnd,
                "EDIT",
                "100",
                306,
                (550, 455, 170, 26),
                WS_BORDER | ES_NUMBER as u32,
            );
            let players = control(
                hwnd,
                "BUTTON",
                tr!("Players"),
                303,
                (10, 495, 160, 26),
                BS_AUTOCHECKBOX as u32,
            );
            let creatures = control(
                hwnd,
                "BUTTON",
                tr!("Creatures"),
                304,
                (190, 495, 160, 26),
                BS_AUTOCHECKBOX as u32,
            );
            let sort = control(
                hwnd,
                "COMBOBOX",
                "",
                305,
                (370, 495, 350, 150),
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for label in [
                tr!("Name"),
                tr!("Health: low first"),
                tr!("Health: high first"),
                tr!("Identity"),
            ] {
                SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let apply = control(hwnd, "BUTTON", tr!("Save list"), 307, (10, 540, 220, 30), 0);
            let add = control(hwnd, "BUTTON", tr!("Add list"), 308, (250, 540, 220, 30), 0);
            let remove = control(
                hwnd,
                "BUTTON",
                tr!("Remove list"),
                309,
                (490, 540, 230, 30),
                0,
            );
            let controls: Vec<_> = lists
                .iter()
                .copied()
                .chain([stop, offensive, balanced, defensive, stand, chase])
                .collect();
            if controls
                .iter()
                .chain(headings.iter())
                .chain(
                    [
                        text,
                        editor_label,
                        active,
                        title,
                        search,
                        maximum,
                        players,
                        creatures,
                        sort,
                        apply,
                        add,
                        remove,
                    ]
                    .iter(),
                )
                .any(|h| h.is_null())
            {
                DestroyWindow(hwnd);
                close(&weak);
                return None;
            }

            Some(Self {
                headings,
                lists,
                text,
                active,
                title,
                search,
                maximum,
                players,
                creatures,
                sort,
                controls,
            })
        }
    }
}
