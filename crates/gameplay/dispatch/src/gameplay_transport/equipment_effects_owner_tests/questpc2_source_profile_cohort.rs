//! Existing source admissions absent from collector main78294c081.
//! No profile under test supplies this independently enumerated key cohort.
pub(super) const IDS: &[u16] = &[
    // source_gate_item_profiles.py
    1626, 1627, 1634, 1635, 1948, 2572, 2573, 2574, 2575, 2576, 2577, 2578, 2579, 2580, 5111, 5112,
    21935, // native_goal_corpse_items.py
    4013, 4025, 4027, 4041, 4042, 4047, 4048, 4128, 4147, 4160, 4161, 5978, 5981, 6011,
    // native_goal_loot_items.py
    3061, 3563, 3593, 3595, 3597, 3661, 23986, 44158, // questpc2_rotten_loot_items.py
    43501, 43502, 43503, 43504, // key-door-item-profiles.json
    1631, 5102, 5103, 6264, 6265, // party-rite-item-profiles.json
    1841, 2114, 2153, 3656, // tool-item-profiles.json
    1791, // quest-chest-item-profiles.json
    1777, 2436, 2438, 2473, 2474,
    // Selected Purple Tome and Nightmare Teddy source claims
    2822, 2823, 2848, 21982,
];

// Main floor/tool/reward cohorts count the first five; c77d436d8 QREADY
// also owns Purple Tome 2822/2823/2848 and Nightmare Teddy 21982.
pub(super) const SHARED_WITH_MAIN: &[u16] =
    &[1791, 1948, 2822, 2823, 2848, 3593, 5102, 5103, 21982];
