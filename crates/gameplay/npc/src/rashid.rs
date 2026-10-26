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

/// Items whose ownership is changed by the Travelling Trader callbacks.  These are semantic
/// identities rather than client ids: the item owner resolves them against the admitted bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissionItem {
    DeerTrophy,
    HeavyPackage,
    ScarabCheese,
    FineVase,
    CrimsonSword,
    GoldfishBowl,
    IronOre,
}

/// Provenance retained with an item created or transformed by a prerequisite NPC callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemProvenance {
    pub purchased_at_unix_ms: Option<i64>,
    pub paid_gold: u16,
    pub forbidden_transport: bool,
    pub broken: bool,
    pub ores_spent: u8,
}

impl ItemProvenance {
    const PLAIN: Self = Self {
        purchased_at_unix_ms: None,
        paid_gold: 0,
        forbidden_transport: false,
        broken: false,
        ores_spent: 0,
    };
}

/// Durable values changed by a mission. The admitted character owner persists this value in the
/// same transaction as its quest transition; [`execute_mission`] deliberately uses copy/commit so
/// a refused callback cannot leave a partial item or money mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionLedger {
    pub completed: u8,
    pub gold: u64,
    pub items: std::collections::BTreeMap<MissionItem, ItemProvenance>,
    pub recognised_trader_achievement: bool,
}

/// A prerequisite NPC operation admitted by the item/money owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prerequisite {
    BuyScarabCheese { now_unix_ms: i64 },
    BuyFineVase,
    BreakFineVase,
    UseForbiddenCheeseTransport,
    ForgeCrimsonSword,
}

/// Apply a prerequisite callback atomically. Prices and input quantities are part of the native
/// callback, not facts supplied by the client.
pub fn apply_prerequisite(ledger: &mut MissionLedger, operation: Prerequisite) -> bool {
    let mut next = ledger.clone();
    let accepted = match operation {
        Prerequisite::BuyScarabCheese { now_unix_ms } if next.gold >= 100 => {
            next.gold -= 100;
            next.items.insert(
                MissionItem::ScarabCheese,
                ItemProvenance {
                    purchased_at_unix_ms: Some(now_unix_ms),
                    paid_gold: 100,
                    ..ItemProvenance::PLAIN
                },
            );
            true
        }
        Prerequisite::BuyFineVase if next.gold >= 1_000 => {
            next.gold -= 1_000;
            next.items.insert(
                MissionItem::FineVase,
                ItemProvenance {
                    paid_gold: 1_000,
                    ..ItemProvenance::PLAIN
                },
            );
            true
        }
        Prerequisite::BreakFineVase => {
            next.items
                .get_mut(&MissionItem::FineVase)
                .is_some_and(|item| {
                    item.broken = true;
                    true
                })
        }
        Prerequisite::UseForbiddenCheeseTransport => next
            .items
            .get_mut(&MissionItem::ScarabCheese)
            .is_some_and(|item| {
                item.forbidden_transport = true;
                true
            }),
        Prerequisite::ForgeCrimsonSword
            if next.gold >= 400 && next.items.remove(&MissionItem::IronOre).is_some() =>
        {
            next.gold -= 400;
            next.items.insert(
                MissionItem::CrimsonSword,
                ItemProvenance {
                    paid_gold: 400,
                    ores_spent: 3,
                    ..ItemProvenance::PLAIN
                },
            );
            true
        }
        _ => false,
    };
    if accepted {
        *ledger = next;
    }
    accepted
}

/// Admit and commit one weekday hand-in. `now_unix_ms` is owner time and is used to verify the
/// cheese purchase timestamp. Every removal, grant, achievement and quest cursor change becomes
/// visible together.
pub fn execute_mission(ledger: &mut MissionLedger, day: u8, now_unix_ms: i64) -> bool {
    let mut next = ledger.clone();
    let proof = match next.completed {
        0 if next.items.contains_key(&MissionItem::DeerTrophy) => MissionProof::DeerTrophy,
        1 if next.items.contains_key(&MissionItem::HeavyPackage) => MissionProof::HeavyPackage,
        2 => match next.items.get(&MissionItem::ScarabCheese) {
            Some(item) => MissionProof::ScarabCheese {
                age_seconds: item
                    .purchased_at_unix_ms
                    .and_then(|at| now_unix_ms.checked_sub(at))
                    .and_then(|age| u16::try_from(age / 1_000).ok())
                    .unwrap_or(u16::MAX),
                forbidden_transport: item.forbidden_transport,
            },
            None => return false,
        },
        3 => match next.items.get(&MissionItem::FineVase) {
            Some(item) => MissionProof::FineVase {
                broken: item.broken,
                paid_gold: item.paid_gold,
            },
            None => return false,
        },
        4 => match next.items.get(&MissionItem::CrimsonSword) {
            Some(item) => MissionProof::CrimsonSword {
                paid_gold: item.paid_gold,
                ores_spent: item.ores_spent,
            },
            None => return false,
        },
        5 if next.items.contains_key(&MissionItem::GoldfishBowl) => MissionProof::GoldfishBowl,
        6 => MissionProof::SundayRecognition,
        _ => return false,
    };
    if !accepts_mission(day, next.completed, proof) {
        return false;
    }
    let consumed = [
        MissionItem::DeerTrophy,
        MissionItem::HeavyPackage,
        MissionItem::ScarabCheese,
        MissionItem::FineVase,
        MissionItem::CrimsonSword,
        MissionItem::GoldfishBowl,
    ];
    if let Some(consumed) = consumed.get(usize::from(next.completed)) {
        next.items.remove(consumed);
    }
    match next.completed {
        0 => {
            next.items
                .insert(MissionItem::HeavyPackage, ItemProvenance::PLAIN);
        }
        1 => {}
        2 => {}
        3 => {}
        4 => {
            next.items
                .insert(MissionItem::GoldfishBowl, ItemProvenance::PLAIN);
        }
        6 => next.recognised_trader_achievement = true,
        _ => {}
    }
    next.completed += 1;
    *ledger = next;
    true
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
    fn admitted_callbacks_and_all_seven_hand_ins_are_atomic() {
        let mut ledger = MissionLedger {
            completed: 0,
            gold: 2_000,
            items: [(MissionItem::DeerTrophy, ItemProvenance::PLAIN)]
                .into_iter()
                .collect(),
            recognised_trader_achievement: false,
        };
        assert!(execute_mission(&mut ledger, 0, 1_000));
        assert!(execute_mission(&mut ledger, 1, 2_000));
        assert!(apply_prerequisite(
            &mut ledger,
            Prerequisite::BuyScarabCheese {
                now_unix_ms: 10_000
            }
        ));
        let before_late = ledger.clone();
        assert!(!execute_mission(&mut ledger, 2, 1_811_000));
        assert_eq!(ledger, before_late, "a refusal changes no durable value");
        assert!(execute_mission(&mut ledger, 2, 1_810_000));
        assert!(apply_prerequisite(&mut ledger, Prerequisite::BuyFineVase));
        assert!(execute_mission(&mut ledger, 3, 1_820_000));
        ledger
            .items
            .insert(MissionItem::IronOre, ItemProvenance::PLAIN);
        assert!(apply_prerequisite(
            &mut ledger,
            Prerequisite::ForgeCrimsonSword
        ));
        assert!(execute_mission(&mut ledger, 4, 1_830_000));
        assert!(execute_mission(&mut ledger, 5, 1_840_000));
        assert!(execute_mission(&mut ledger, 6, 1_850_000));
        assert_eq!(ledger.completed, 7);
        assert!(ledger.recognised_trader_achievement);
        assert_eq!(ledger.gold, 500);
        assert!(ledger.items.is_empty());
    }

    #[test]
    fn breakage_and_transport_provenance_refuse_without_partial_commit() {
        let base = MissionLedger {
            completed: 2,
            gold: 2_000,
            items: Default::default(),
            recognised_trader_achievement: false,
        };
        let mut transported = base.clone();
        assert!(apply_prerequisite(
            &mut transported,
            Prerequisite::BuyScarabCheese { now_unix_ms: 0 }
        ));
        assert!(apply_prerequisite(
            &mut transported,
            Prerequisite::UseForbiddenCheeseTransport
        ));
        let snapshot = transported.clone();
        assert!(!execute_mission(&mut transported, 2, 1_000));
        assert_eq!(transported, snapshot);

        let mut broken = MissionLedger {
            completed: 3,
            ..base
        };
        assert!(apply_prerequisite(&mut broken, Prerequisite::BuyFineVase));
        assert!(apply_prerequisite(&mut broken, Prerequisite::BreakFineVase));
        let snapshot = broken.clone();
        assert!(!execute_mission(&mut broken, 3, 1_000));
        assert_eq!(broken, snapshot);
    }

    #[test]
    fn rashid_identity_does_not_gate_other_traders() {
        assert!(is_rashid("canary:npc/rashid"));
        assert!(is_rashid("oteryn:npc.rashid"));
        assert!(!is_rashid("canary:npc/rashid_custom"));
        assert!(!is_rashid("canary:npc/nah'bob"));
    }
}
