//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) entries: HWND,
    pub(super) groups: HWND,
    pub(super) consents: HWND,
    pub(super) text: HWND,
    pub(super) icon: HWND,
    pub(super) notify: HWND,
    pub(super) status: HWND,
    pub(super) friends: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let list = LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP;
            control(
                hwnd,
                "STATIC",
                "VIP entries                 Groups                      Consent invitations",
                200,
                [10, 5, 620, 20],
                0,
            );
            let entries = control(hwnd, "LISTBOX", "", 101, [10, 30, 205, 160], list);
            let groups = control(
                hwnd,
                "LISTBOX",
                "",
                102,
                [225, 30, 180, 160],
                list | LBS_EXTENDEDSEL as u32,
            );
            let consents = control(hwnd, "LISTBOX", "", 103, [415, 30, 210, 160], list);
            control(
                hwnd,
                "STATIC",
                "Character / group name or description",
                201,
                [10, 195, 300, 20],
                0,
            );
            let text = control(
                hwnd,
                "EDIT",
                "",
                104,
                [10, 220, 300, 25],
                WS_BORDER | WS_TABSTOP,
            );
            control(
                hwnd,
                "STATIC",
                "Icon (0–19); select groups above to assign",
                202,
                [320, 195, 300, 20],
                0,
            );
            let icon = control(
                hwnd,
                "EDIT",
                "0",
                105,
                [320, 220, 50, 25],
                WS_BORDER | WS_TABSTOP,
            );
            let notify = control(
                hwnd,
                "BUTTON",
                "Notify",
                107,
                [545, 220, 80, 25],
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            );
            for (index, label) in [
                "Add",
                "Remove",
                "Save edit",
                "Add group",
                "Rename group",
                "Delete group",
                "Accept",
                "Reject",
                "Revoke",
                "Withdraw",
                "Show status",
                "Hide status",
                "Refresh",
                "Next page",
            ]
            .iter()
            .enumerate()
            {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    110 + index,
                    [
                        10 + (index % 5) as i32 * 123,
                        260 + (index / 5) as i32 * 35,
                        120,
                        30,
                    ],
                    WS_TABSTOP,
                );
            }
            let status = control(hwnd, "STATIC", "", 203, [10, 370, 615, 75], 0);
            for (id, title, x) in [
                (124, "All contacts", 10),
                (125, "Friends", 220),
                (126, "Invitations", 430),
            ] {
                control(hwnd, "BUTTON", title, id, [x, 455, 195, 28], WS_TABSTOP);
            }
            let friends = control(
                hwnd,
                "EDIT",
                "",
                204,
                [10, 490, 615, 140],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_BORDER,
            );

            Some(Self {
                entries,
                groups,
                consents,
                text,
                icon,
                notify,
                status,
                friends,
            })
        }
    }
}
