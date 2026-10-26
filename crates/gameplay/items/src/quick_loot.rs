//! QUICKLOOT-1 policy rules shared by the command and durable item owners.
//!
//! This module deliberately contains no custody mutation.  It produces a bounded, deterministic
//! plan which the existing item-transfer owner can commit while retaining its capacity, corpse
//! exclusivity and session-generation checks.

use std::collections::{BTreeMap, BTreeSet};

/// Global's maximum number of nearby corpses considered by one manual Loot All command.
pub const MAX_NEARBY_CORPSES: usize = 30;
/// Top-damage/party exclusivity after death.
pub const EXCLUSIVE_LOOT_MILLIS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LootCategory {
    Gold,
    CreatureProduct,
    Equipment,
    Consumable,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualLootMode {
    /// Loot the explicitly selected corpse only.
    Click,
    /// Loot eligible corpses on the actor's tile and its eight neighbours.
    AllNearby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipient<K> {
    Container(K),
    GoldPouch(K),
    MainContainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootItem<K> {
    pub key: K,
    pub category: LootCategory,
    pub is_coin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corpse<K> {
    pub key: K,
    pub distance: u8,
    pub age_millis: u64,
    pub top_damage_or_party: bool,
    pub items: Vec<LootItem<K>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootSettings<K> {
    /// Premium category assignments. Free accounts ignore this map.
    pub category_targets: BTreeMap<LootCategory, Recipient<K>>,
    pub unassigned_target: Recipient<K>,
    pub fallback_to_main: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedTransfer<K> {
    pub corpse: K,
    pub item: K,
    pub recipient: Recipient<K>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TooFar,
    NotOwner,
    NotSelected,
    RecipientRejectsItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skipped<K> {
    pub corpse: K,
    pub item: Option<K>,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualLootPlan<K> {
    pub accepted: Vec<PlannedTransfer<K>>,
    pub skipped: Vec<Skipped<K>>,
}

/// Builds the stable transfer order for a manual command. This is intentionally only admission:
/// each accepted transfer must still pass the existing transactional custody/capacity owner.
pub fn plan_manual_loot<K: Copy + Ord>(
    mode: ManualLootMode,
    clicked: Option<K>,
    premium: bool,
    settings: &LootSettings<K>,
    corpses: &[Corpse<K>],
) -> ManualLootPlan<K> {
    let mut accepted = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = BTreeSet::new();
    for corpse in corpses.iter().take(MAX_NEARBY_CORPSES) {
        if !seen.insert(corpse.key) {
            continue;
        }
        let selected = match mode {
            ManualLootMode::Click => clicked == Some(corpse.key),
            ManualLootMode::AllNearby => true,
        };
        if !selected {
            skipped.push(Skipped {
                corpse: corpse.key,
                item: None,
                reason: SkipReason::NotSelected,
            });
            continue;
        }
        if corpse.distance > 1 {
            skipped.push(Skipped {
                corpse: corpse.key,
                item: None,
                reason: SkipReason::TooFar,
            });
            continue;
        }
        if corpse.age_millis < EXCLUSIVE_LOOT_MILLIS && !corpse.top_damage_or_party {
            skipped.push(Skipped {
                corpse: corpse.key,
                item: None,
                reason: SkipReason::NotOwner,
            });
            continue;
        }
        for item in &corpse.items {
            let recipient = if premium {
                settings
                    .category_targets
                    .get(&item.category)
                    .copied()
                    .unwrap_or(settings.unassigned_target)
            } else {
                settings.unassigned_target
            };
            if matches!(recipient, Recipient::GoldPouch(_)) && !item.is_coin {
                if settings.fallback_to_main {
                    accepted.push(PlannedTransfer {
                        corpse: corpse.key,
                        item: item.key,
                        recipient: Recipient::MainContainer,
                    });
                } else {
                    skipped.push(Skipped {
                        corpse: corpse.key,
                        item: Some(item.key),
                        reason: SkipReason::RecipientRejectsItem,
                    });
                }
            } else {
                accepted.push(PlannedTransfer {
                    corpse: corpse.key,
                    item: item.key,
                    recipient,
                });
            }
        }
    }
    ManualLootPlan { accepted, skipped }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuiverSlot {
    Extra,
    Shield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ammunition<K, T> {
    pub key: K,
    pub kind: T,
    pub quantity: u32,
}

/// Selects the first compatible non-empty stack in container order. Incompatible and empty
/// stacks never mask a later compatible stack.
pub fn select_quiver_ammunition<K: Copy, T: Copy + Eq>(
    required: &T,
    entries: &[Ammunition<K, T>],
) -> Option<Ammunition<K, T>> {
    entries
        .iter()
        .copied()
        .find(|ammo| ammo.quantity > 0 && &ammo.kind == required)
}

/// The post-shot quantity used by the atomic ranged commit. Returning `None` means no shot may
/// be admitted; callers must consume this exact selected item in the same transaction as the hit.
pub fn consume_one<K: Copy, T: Copy>(ammo: Ammunition<K, T>) -> Option<(K, u32)> {
    ammo.quantity.checked_sub(1).map(|left| (ammo.key, left))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> LootSettings<u8> {
        LootSettings {
            category_targets: BTreeMap::from([(LootCategory::Gold, Recipient::GoldPouch(8))]),
            unassigned_target: Recipient::Container(7),
            fallback_to_main: false,
        }
    }

    #[test]
    fn nearby_is_bounded_and_exclusive_rights_are_kept() {
        let corpses = (0..32)
            .map(|key| Corpse {
                key,
                distance: 1,
                age_millis: 9_999,
                top_damage_or_party: key != 0,
                items: vec![LootItem {
                    key,
                    category: LootCategory::Other,
                    is_coin: false,
                }],
            })
            .collect::<Vec<_>>();
        let plan = plan_manual_loot(
            ManualLootMode::AllNearby,
            None,
            false,
            &settings(),
            &corpses,
        );
        assert_eq!(plan.accepted.len(), 29);
        assert_eq!(plan.skipped[0].reason, SkipReason::NotOwner);
        assert!(plan.accepted.iter().all(|entry| entry.corpse < 30));
    }

    #[test]
    fn category_sorting_is_premium_and_gold_pouch_is_coin_only() {
        let corpse = Corpse {
            key: 1,
            distance: 0,
            age_millis: EXCLUSIVE_LOOT_MILLIS,
            top_damage_or_party: false,
            items: vec![
                LootItem {
                    key: 2,
                    category: LootCategory::Gold,
                    is_coin: true,
                },
                LootItem {
                    key: 3,
                    category: LootCategory::Gold,
                    is_coin: false,
                },
            ],
        };
        let premium = plan_manual_loot(
            ManualLootMode::AllNearby,
            None,
            true,
            &settings(),
            std::slice::from_ref(&corpse),
        );
        assert_eq!(premium.accepted[0].recipient, Recipient::GoldPouch(8));
        assert_eq!(premium.skipped[0].reason, SkipReason::RecipientRejectsItem);
        let free = plan_manual_loot(
            ManualLootMode::AllNearby,
            None,
            false,
            &settings(),
            &[corpse],
        );
        assert_eq!(free.accepted.len(), 2);
        assert!(
            free.accepted
                .iter()
                .all(|entry| entry.recipient == Recipient::Container(7))
        );
    }

    #[test]
    fn quiver_skips_empty_and_incompatible_stacks_and_consumes_atomically() {
        let entries = [
            Ammunition {
                key: 1,
                kind: "arrow",
                quantity: 0,
            },
            Ammunition {
                key: 2,
                kind: "bolt",
                quantity: 50,
            },
            Ammunition {
                key: 3,
                kind: "arrow",
                quantity: 2,
            },
        ];
        assert_eq!(
            select_quiver_ammunition(&"arrow", &entries),
            Some(Ammunition {
                key: 3,
                kind: "arrow",
                quantity: 2
            })
        );
        assert_eq!(
            consume_one(Ammunition {
                key: 3,
                kind: "arrow",
                quantity: 2
            }),
            Some((3, 1))
        );
        assert_eq!(
            consume_one(Ammunition {
                key: 4,
                kind: "arrow",
                quantity: 0
            }),
            None
        );
    }
}
