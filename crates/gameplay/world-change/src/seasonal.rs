//! Seasonal windows evaluated on the committed Berlin server-save date.
use crate::owner::SeasonalWindow;
use sha2::{Digest, Sha256};

pub const KINGSDAY: &str = "kingsday";
pub const LAST_CREEP: &str = "last_creep_standing";
pub const SPRING: &str = "spring_into_life";
pub const TINDER_BOX: &str = "tinder_box";
pub const KEYS: [&str; 4] = [KINGSDAY, LAST_CREEP, SPRING, TINDER_BOX];

// Crystal 201189f8, data-global/scripts/quests/tinder_box_quest_chyllfroest/
// movements_prohibit_entry.lua and actions_reward.lua include May 1.
const TINDER_WINDOW: SeasonalWindow = SeasonalWindow {
    start: (4, 1),
    end: (5, 2),
};
// Tibia Fandom archive, Spring_into_Life and Spring_into_Life/Spoiler.
const SPRING_WINDOW: SeasonalWindow = SeasonalWindow {
    start: (4, 16),
    end: (4, 23),
};
// W15 of OTERYN_GAME_EVENTS_WORLD1_DECISION_2026-10-15.md.
const KINGSDAY_PERMILLE: u128 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub key: String,
    pub id: [u8; 32],
}

pub fn at_save(world: &[u8; 16], epoch: u64, year: i32, month: u8, day: u8) -> Vec<Occurrence> {
    let mut result = Vec::new();
    let mut open = |key: &str, start: &[u8]| {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.seasonal.occurrence.v1\0");
        hash.update(world);
        hash.update(key.as_bytes());
        hash.update([0]);
        hash.update(start);
        result.push(Occurrence {
            key: key.to_owned(),
            id: hash.finalize().into(),
        });
    };

    let kingsday_draw = crate::daily::draw_for_epoch(world, epoch, KINGSDAY);
    if (u128::from(kingsday_draw) * 1000) >> 64 < KINGSDAY_PERMILLE {
        open(KINGSDAY, &epoch.to_be_bytes());
    }
    // Fandom Last_Creep_Standing/Dates excludes March, September and October.
    // The spoiler specifies five days and thirty four-hour rounds.
    if !matches!(month, 3 | 9 | 10) && (20..25).contains(&day) {
        open(
            LAST_CREEP,
            &[year.to_be_bytes().as_slice(), &[month]].concat(),
        );
    }
    if SPRING_WINDOW.contains(month, day) {
        open(SPRING, &year.to_be_bytes());
    }
    if TINDER_WINDOW.contains(month, day) {
        open(TINDER_BOX, &year.to_be_bytes());
    }
    result
}
