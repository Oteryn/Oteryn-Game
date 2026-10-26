//! Global Rashid schedule and Travelling Trader gates.
//!
//! Weekdays are derived from the accepted server-save boundary by the world owner.  This module
//! deliberately accepts that day ordinal rather than consulting the host clock.

/// Durable quest track which unlocks Rashid's shop after Sunday's recognition.
pub const MISSION_07_TRACK: &str =
    "canary:quest-progress/quest/u8_1/the_travelling_trader/mission07";

/// Rashid's city for a server-save day (`0` is Monday, `6` is Sunday).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RashidCity {
    Svargrond,
    LibertyBay,
    PortHope,
    Ankrahmun,
    Darashia,
    Edron,
    Carlin,
}

/// Native Global position belonging to one city.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RashidPlacement {
    pub city: RashidCity,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// Resolve the native placement from the accepted world-reset epoch. Epoch zero is the
/// catalogue's Monday boundary; subsequent accepted server saves advance the weekday. This is
/// deliberately independent of process start time and the host calendar.
#[must_use]
pub const fn placement_after_server_save(epoch: u64) -> RashidPlacement {
    match epoch % 7 {
        0 => RashidPlacement {
            city: RashidCity::Svargrond,
            x: 32_207,
            y: 31_155,
            floor: 7,
        },
        1 => RashidPlacement {
            city: RashidCity::LibertyBay,
            x: 32_300,
            y: 32_837,
            floor: 7,
        },
        2 => RashidPlacement {
            city: RashidCity::PortHope,
            x: 32_577,
            y: 32_753,
            floor: 7,
        },
        3 => RashidPlacement {
            city: RashidCity::Ankrahmun,
            x: 33_066,
            y: 32_879,
            floor: 6,
        },
        4 => RashidPlacement {
            city: RashidCity::Darashia,
            x: 33_235,
            y: 32_483,
            floor: 7,
        },
        5 => RashidPlacement {
            city: RashidCity::Edron,
            x: 33_166,
            y: 31_810,
            floor: 6,
        },
        _ => RashidPlacement {
            city: RashidCity::Carlin,
            x: 32_328,
            y: 31_782,
            floor: 6,
        },
    }
}

/// Facts owned by the item/travel callbacks and consumed when a mission is handed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionProof {
    DeerTrophy,
    HeavyPackage,
    ScarabCheese {
        age_seconds: u16,
        forbidden_transport: bool,
    },
    FineVase {
        broken: bool,
        paid_gold: u16,
    },
    CrimsonSword {
        paid_gold: u16,
        ores_spent: u8,
    },
    GoldfishBowl,
    SundayRecognition,
}

/// Mission order and exact Global acceptance constraints. The caller remains responsible for
/// atomically consuming the named item and committing the corresponding quest transition.
#[must_use]
pub const fn accepts_mission(day_since_monday: u8, completed: u8, proof: MissionProof) -> bool {
    match (day_since_monday, completed, proof) {
        (0, 0, MissionProof::DeerTrophy) | (1, 1, MissionProof::HeavyPackage) => true,
        (
            2,
            2,
            MissionProof::ScarabCheese {
                age_seconds,
                forbidden_transport,
            },
        ) => age_seconds <= 30 * 60 && !forbidden_transport,
        (
            3,
            3,
            MissionProof::FineVase {
                broken: false,
                paid_gold: 1_000,
            },
        ) => true,
        (
            4,
            4,
            MissionProof::CrimsonSword {
                paid_gold,
                ores_spent: 3,
            },
        ) => paid_gold <= 400,
        (5, 5, MissionProof::GoldfishBowl) | (6, 6, MissionProof::SundayRecognition) => true,
        _ => false,
    }
}

/// Resolve placement from the accepted server-save day, never the host's local weekday.
#[must_use]
pub const fn city_after_server_save(day_since_monday: u8) -> Option<RashidCity> {
    match day_since_monday {
        0 => Some(RashidCity::Svargrond),
        1 => Some(RashidCity::LibertyBay),
        2 => Some(RashidCity::PortHope),
        3 => Some(RashidCity::Ankrahmun),
        4 => Some(RashidCity::Darashia),
        5 => Some(RashidCity::Edron),
        6 => Some(RashidCity::Carlin),
        _ => None,
    }
}

/// Global's final trade gate. Missing/unloaded quest state fails closed.
#[must_use]
pub fn trade_unlocked(track_value: Option<i64>) -> bool {
    track_value == Some(1)
}

/// Both donor spellings identify the same NPC service owner.
#[must_use]
pub fn is_rashid(npc_key: &str) -> bool {
    npc_key.rsplit(['/', '.']).next() == Some("rashid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_weekday_order_uses_server_save_ordinal() {
        assert_eq!(city_after_server_save(0), Some(RashidCity::Svargrond));
        assert_eq!(city_after_server_save(1), Some(RashidCity::LibertyBay));
        assert_eq!(city_after_server_save(2), Some(RashidCity::PortHope));
        assert_eq!(city_after_server_save(3), Some(RashidCity::Ankrahmun));
        assert_eq!(city_after_server_save(4), Some(RashidCity::Darashia));
        assert_eq!(city_after_server_save(5), Some(RashidCity::Edron));
        assert_eq!(city_after_server_save(6), Some(RashidCity::Carlin));
        assert_eq!(city_after_server_save(7), None);
        assert_eq!(placement_after_server_save(0).city, RashidCity::Svargrond);
        assert_eq!(placement_after_server_save(6).city, RashidCity::Carlin);
        assert_eq!(
            placement_after_server_save(7),
            placement_after_server_save(0)
        );
    }

    #[test]
    fn mission_sequence_enforces_provenance_and_weekday() {
        assert!(accepts_mission(0, 0, MissionProof::DeerTrophy));
        assert!(accepts_mission(
            2,
            2,
            MissionProof::ScarabCheese {
                age_seconds: 1_800,
                forbidden_transport: false
            }
        ));
        assert!(!accepts_mission(
            2,
            2,
            MissionProof::ScarabCheese {
                age_seconds: 1_801,
                forbidden_transport: false
            }
        ));
        assert!(!accepts_mission(
            2,
            2,
            MissionProof::ScarabCheese {
                age_seconds: 10,
                forbidden_transport: true
            }
        ));
        assert!(!accepts_mission(
            3,
            3,
            MissionProof::FineVase {
                broken: true,
                paid_gold: 1_000
            }
        ));
        assert!(accepts_mission(
            4,
            4,
            MissionProof::CrimsonSword {
                paid_gold: 400,
                ores_spent: 3
            }
        ));
        assert!(!accepts_mission(
            4,
            4,
            MissionProof::CrimsonSword {
                paid_gold: 401,
                ores_spent: 3
            }
        ));
        assert!(accepts_mission(6, 6, MissionProof::SundayRecognition));
        assert!(!accepts_mission(5, 6, MissionProof::SundayRecognition));
    }

    #[test]
    fn only_sunday_recognition_unlocks_trade() {
        assert!(!trade_unlocked(None));
        assert!(!trade_unlocked(Some(0)));
        assert!(trade_unlocked(Some(1)));
        assert!(!trade_unlocked(Some(2)));
    }

    #[test]
    fn rashid_identity_does_not_gate_other_traders() {
        assert!(is_rashid("canary:npc/rashid"));
        assert!(is_rashid("oteryn:npc.rashid"));
        assert!(!is_rashid("canary:npc/rashid_custom"));
        assert!(!is_rashid("canary:npc/nah'bob"));
    }
}
