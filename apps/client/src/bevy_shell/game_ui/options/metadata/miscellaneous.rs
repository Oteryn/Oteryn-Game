use super::*;
/// The remembered-login rows; turning one off on Apply forgets this device.
pub(in super::super) const LOGIN_CHOICES: [&str; 3] = [
    "misc.stay_logged_in",
    "misc.remember_device",
    "misc.quick_login",
];
pub(super) const MAIN: [Row; 6] = [
    // A remembered device keeps the session, so remembering implies staying logged in.
    Row {
        id: "misc.stay_logged_in",
        label: "Stay logged in during this client session",
        editor: Editor::Check {
            read: |p| p.miscellaneous.stay_logged_in,
            write: |p, v| p.miscellaneous.stay_logged_in = v || p.miscellaneous.remember_device,
        },
    },
    Row {
        id: "misc.remember_device",
        label: "Remember this device after restart (OS credential vault)",
        editor: Editor::Check {
            read: |p| p.miscellaneous.remember_device,
            write: |p, v| {
                p.miscellaneous.remember_device = v;
                p.miscellaneous.stay_logged_in |= v;
            },
        },
    },
    check!(
        "misc.quick_login",
        "Quick login: use the last available character",
        miscellaneous.quick_login
    ),
    check!(
        "misc.logout",
        "Ask Before Logging Out",
        miscellaneous.confirm_logout
    ),
    check!(
        "misc.exit",
        "Ask Before Exiting",
        miscellaneous.confirm_exit
    ),
    check!(
        "misc.connection",
        "Optimise Connection Stability",
        optimise_connection
    ),
];
pub(super) const GAMEPLAY: [Row; 3] = [
    check!(
        "gameplay.inspect",
        "Show Inspection Actions in Context Menu",
        gameplay.inspect_context_menu
    ),
    check!(
        "gameplay.chase",
        "Stop Chasing When Walking Manually",
        gameplay.cancel_chase_on_walk
    ),
    check!(
        "gameplay.secondary_loot",
        "Secondary Click Loots Nearby",
        gameplay.secondary_click_loots_nearby
    ),
];
