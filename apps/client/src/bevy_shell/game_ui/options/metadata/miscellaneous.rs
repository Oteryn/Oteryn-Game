use super::*;
pub(super) const MAIN: [Row; 2] = [
    check!(
        "misc.logout",
        "Ask Before Logging Out",
        miscellaneous.confirm_logout
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
