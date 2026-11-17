//! Crystal ff7ede59 data-global/lib/quests/svargrond_arena.lua and
//! data-global/startup/tables/chest.lua:2307-2419; Canary 99902524 agrees.
pub const FEES: [u64; 3] = [1_000, 5_000, 10_000];
pub const PIT_SECONDS: u32 = 600;
pub const CENTERS: [(i32, i32); 10] = [
    (32210, 31098),
    (32196, 31098),
    (32182, 31098),
    (32168, 31098),
    (32175, 31084),
    (32189, 31084),
    (32203, 31084),
    (32196, 31070),
    (32182, 31070),
    (32189, 31057),
];
pub const KICK: (i32, i32, i16) = (32251, 31098, 6);
pub const REWARD_ROOM: (i32, i32, i16) = (32222, 31080, 6);
pub const OPPONENTS: [[&str; 10]; 3] = [
    [
        "frostfur",
        "bloodpaw",
        "bovinus",
        "achad",
        "colerian_the_barbarian",
        "the_hairy_one",
        "axeitus_headbanger",
        "rocky",
        "cursed_gladiator",
        "orcus_the_cruel",
    ],
    [
        "avalanche",
        "kreebosh_the_exile",
        "the_dark_dancer",
        "the_hag",
        "slim",
        "grimgor_guteater",
        "drasilla",
        "spirit_of_earth",
        "spirit_of_water",
        "spirit_of_fire",
    ],
    [
        "webster",
        "darakan_the_executioner",
        "norgle_glacierbeard",
        "the_pit_lord",
        "svoren_the_mad",
        "the_masked_marauder",
        "gnorre_chyllson",
        "fallen_mooh_tah_master_ghar",
        "deathbringer",
        "the_obliverator",
    ],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reward {
    pub root: u16,
    pub contents: Vec<(u16, u32)>,
}

pub fn reward(rank: u8, category: u8, choice: u8) -> Option<Reward> {
    if !(1..=3).contains(&rank)
        || !(1..=4).contains(&category)
        || !(1..=if category == 3 { 3 } else { 1 }).contains(&choice)
    {
        return None;
    }
    let (root, contents): (u16, &[(u16, u32)]) = match (rank, category) {
        (1, 1) => (
            2856,
            &[(2995, 1), (6570, 1), (6574, 1), (6569, 10), (7377, 1)],
        ),
        (2, 1) => (2856, &[(6569, 10), (7375, 1), (6574, 1), (7183, 1)]),
        (3, 1) => (2856, &[(6569, 10), (6574, 1), (7372, 1), (5080, 1)]),
        (1, 2) => (7342, &[(7364, 100), (7365, 100)]),
        // A donor stack of 200 is delivered as two legal stacks of 100.
        (2, 2) => (
            7342,
            &[
                (7365, 100),
                (3450, 100),
                (3450, 100),
                (11605, 1),
                (11615, 1),
            ],
        ),
        (3, 2) => (
            7342,
            &[(11603, 1), (11609, 1), (7443, 1), (7440, 1), (6528, 100)],
        ),
        (1, 3) => ([7392, 7380, 7406][usize::from(choice - 1)], &[]),
        (2, 3) => ([7415, 7389, 7384][usize::from(choice - 1)], &[]),
        (3, 3) => ([7429, 7434, 7390][usize::from(choice - 1)], &[]),
        (1, 4) => (5807, &[]),
        (2, 4) => (5806, &[]),
        (3, 4) => (5805, &[]),
        _ => return None,
    };
    Some(Reward {
        root,
        contents: contents.to_vec(),
    })
}

pub fn claim_key(rank: u8, category: u8) -> String {
    format!("oteryn:reward.svargrond-arena.{rank}.{category}")
}

pub const ACHIEVEMENTS: [&str; 3] = [
    "oteryn:achievement/greenhorn",
    "oteryn:achievement/scrapper",
    "oteryn:achievement/warlord_of_svargrond",
];
