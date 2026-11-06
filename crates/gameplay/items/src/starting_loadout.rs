//! Dedicated starting-loadout input, adapted from Crystal 201189f871d23597b82caf88fd611454a2ddefb8
//! `data/scripts/creaturescripts/player/send_first_items.lua:1-101` (also Canary 47dfd51f).
//! Slot codes use the existing durable equipment ABI; zero means backpack contents.

/// (Tibia item id, quantity, durable equipment slot or zero for a backpack entry).
pub type StartingItem = (u32, u32, u8);

/// The source grants only unpromoted vocations and only at first login (:106-125).
pub fn items(vocation: &str) -> Option<&'static [StartingItem]> {
    Some(match vocation {
        "none" => &[(3003, 1, 0), (3457, 1, 0)],
        "sorcerer" => &[
            (3059, 1, 6),
            (3074, 1, 5),
            (7991, 1, 2),
            (7992, 1, 1),
            (3362, 1, 3),
            (3552, 1, 4),
            (3572, 1, 7),
            (3003, 1, 0),
            (5710, 1, 0),
            (268, 10, 0),
        ],
        "druid" => &[
            (3059, 1, 6),
            (3066, 1, 5),
            (7991, 1, 2),
            (7992, 1, 1),
            (3362, 1, 3),
            (3552, 1, 4),
            (3572, 1, 7),
            (3003, 1, 0),
            (5710, 1, 0),
            (268, 10, 0),
        ],
        "paladin" => &[
            (3425, 1, 6),
            (3277, 1, 5),
            (3571, 1, 2),
            (8095, 1, 3),
            (3552, 1, 4),
            (3572, 1, 7),
            (3374, 1, 1),
            (3003, 1, 0),
            (5710, 1, 0),
            (266, 10, 0),
            (3350, 1, 0),
            (3447, 50, 0),
        ],
        "knight" => &[
            (3425, 1, 6),
            (7773, 1, 5),
            (3359, 1, 2),
            (3354, 1, 1),
            (3372, 1, 3),
            (3552, 1, 4),
            (3572, 1, 7),
            (7774, 1, 0),
            (3327, 1, 0),
            (3003, 1, 0),
            (5710, 1, 0),
            (266, 10, 0),
        ],
        "monk" => &[
            (50171, 1, 5),
            (3359, 1, 2),
            (3354, 1, 1),
            (3372, 1, 3),
            (3552, 1, 4),
            (3572, 1, 7),
            (3425, 1, 0),
            (3003, 1, 0),
            (5710, 1, 0),
            (266, 10, 0),
        ],
        _ => return None,
    })
}

/// Crystal `data-global/scripts/actions/dawnport/vocation_door.lua:1-22`.
/// No Monk door is authored; the existing Oressa route also redirects Monk elsewhere.
pub fn departure(unique: u16) -> Option<(&'static str, (i32, i32, i16))> {
    Some(match unique {
        22001 => ("sorcerer", (32054, 31883, 6)),
        22002 => ("druid", (32073, 31883, 6)),
        22003 => ("paladin", (32059, 31883, 6)),
        22004 => ("knight", (32068, 31883, 6)),
        _ => return None,
    })
}

/// Crystal `data-global/lib/others/dawnport.lua:145-171`: all carried quantities,
/// including equipped items and items inside containers, are removed before teleporting.
pub const PROHIBITED: &[u32] = &[
    3355, 3562, 3559, 3552, 21348, 21350, 21400, 3350, 3267, 3412, 35562, 21470, 266, 268, 7876,
    21352, 21351,
];
