//! Exact source-qualified tentacle callback geometry and item definitions.
// Exact pinned donor callback geometry/item IDs. Constructor proof and source hashes retained in lane.
// Validation only; source-qualified death registry remains the caller, current Map owner grants mutation.
pub const TENTACLE_SOURCE_MAP_ROWS: &[TentacleSourceMapRow] = &[
    (
        "oteryn:creature.tentacle",
        &[(33723, 31180, 7), (33724, 31180, 7), (33725, 31180, 7)],
        (33726, 31180, 7),
        35126,
        35119,
        35120,
    ),
    (
        "oteryn:creature.tentacle2",
        &[
            (33723, 31186, 7),
            (33723, 31187, 7),
            (33723, 31188, 7),
            (33723, 31189, 7),
        ],
        (33723, 31191, 7),
        35112,
        35109,
        35110,
    ),
    (
        "oteryn:creature.tentacle3",
        &[(33727, 31184, 7), (33727, 31185, 7)],
        (33727, 31186, 7),
        35112,
        35109,
        35110,
    ),
    (
        "oteryn:creature.tentacle4",
        &[
            (33731, 31180, 7),
            (33732, 31180, 7),
            (33733, 31180, 7),
            (33734, 31180, 7),
            (33735, 31180, 7),
        ],
        (33736, 31180, 7),
        35126,
        35119,
        35120,
    ),
    (
        "oteryn:creature.tentacle5",
        &[(33718, 31180, 7), (33718, 31179, 7), (33718, 31178, 7)],
        (33718, 31177, 7),
        35112,
        35510,
        35107,
    ),
    (
        "oteryn:creature.tentacle6",
        &[(33714, 31180, 6), (33714, 31179, 6), (33714, 31178, 6)],
        (33714, 31177, 6),
        35112,
        35510,
        35107,
    ),
    (
        "oteryn:creature.tentacle7",
        &[
            (33726, 31182, 6),
            (33726, 31181, 6),
            (33726, 31180, 6),
            (33726, 31179, 6),
            (33726, 31178, 6),
        ],
        (33726, 31177, 6),
        35112,
        35510,
        35107,
    ),
    (
        "oteryn:creature.tentacle8",
        &[
            (33718, 31187, 6),
            (33718, 31188, 6),
            (33718, 31189, 6),
            (33718, 31190, 6),
        ],
        (33718, 31191, 6),
        35112,
        35109,
        35110,
    ),
    (
        "oteryn:creature.tentacle9",
        &[(33714, 31185, 6), (33713, 31185, 6), (33712, 31185, 6)],
        (33711, 31185, 6),
        35126,
        35511,
        35122,
    ),
    (
        "oteryn:creature.tentacle10",
        &[(33716, 31181, 6), (33716, 31182, 6)],
        (33716, 31183, 6),
        35112,
        35109,
        35110,
    ),
];

pub type TentacleSourceMapRow = (
    &'static str,
    &'static [(i32, i32, i16)],
    (i32, i32, i16),
    u32,
    u32,
    u32,
);

/// Source item states that retain physical collision after a tentacle callback.
pub const TENTACLE_COLLIDING_ITEM_IDS: &[u32] = &[35112, 35126, 35510, 35511];
