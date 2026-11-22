use super::*;
/// The remembered-login rows; turning one off on Apply forgets this device.
pub(in super::super) const LOGIN_CHOICES: [&str; 3] = [
    "misc.stay_logged_in",
    "misc.remember_device",
    "misc.quick_login",
];
pub(super) const MAIN: [Row; 9] = [
    Row {
        id: "misc.character_sort_column",
        label: "Character List Sort Column",
        editor: Editor::Choice {
            values: &["Character", "Status", "World"],
            read: |p| usize::from(p.character_sort_column),
            write: |p, v| p.character_sort_column = v as u8,
        },
    },
    check!(
        "misc.character_sort_descending",
        "Reverse Character List Sort Order",
        character_sort_descending
    ),
    check!(
        "misc.confirm_store_purchase",
        "Confirm Store Purchases",
        confirm_store_purchase
    ),
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
pub(super) const GAMEPLAY: [Row; 6] = [
    Row {
        id: "gameplay.loot_click",
        label: "Quick Loot Controls",
        editor: Editor::Choice {
            values: &[
                "Context menu",
                "Right click",
                "Shift + right click",
                "Left click",
            ],
            read: |p| usize::from(p.gameplay.loot_click),
            write: |p, v| p.gameplay.loot_click = v as u8,
        },
    },
    check!(
        "gameplay.sort_recursive",
        "Include Nested Containers When Sorting",
        gameplay.sort_recursive
    ),
    check!(
        "gameplay.warn_recursive_sort",
        "Warn Before Sorting Recursively",
        gameplay.warn_recursive_sort
    ),
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
