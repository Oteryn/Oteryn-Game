//! Synthetic centi-oz weights for the explicitly named native PostgreSQL fixtures.
//! Production must never use this catalogue.

pub const GENERATION_DIGEST: [u8; 32] = [0xD1; 32];

/// Exact (family, production key, revision, weight) fixture records; all others are unknown.
pub const FIXTURE_WEIGHTS: &[(&str, &str, &str, u32)] = &[
    ("Item", "fixture:b3.backpack", "definition-r1", 1),
    ("Item", "fixture:b3.coin", "definition-r1", 1),
    ("Item", "fixture:b3.stone", "definition-r1", 1),
    ("Item", "fixture:b3.bag", "definition-r1", 1),
    ("Item", "fixture:equip.sword", "definition-r1", 1),
    ("Item", "fixture:equip.armor", "definition-r1", 1),
    ("Item", "fixture:equip.heavy-armor", "definition-r1", 1),
    ("Item", "fixture:equip.bag", "definition-r1", 1),
    ("Item", "fixture:equip.timed-helmet", "definition-r1", 1),
    ("Item", "fixture:npc.sword", "definition-r1", 1),
    ("Item", "oteryn:item.tibia.i3031", "definition-r1", 10),
    ("Item", "oteryn:item.tibia.i3035", "definition-r1", 10),
    ("Item", "oteryn:item.tibia.i3043", "definition-r1", 10),
    ("Item", "oteryn:item.tibia.i3607", "definition-r1", 400),
    ("Item", "fixture:house.item", "definition-r1", 1),
    ("Item", "oteryn:item.tibia.i3577", "definition-r1", 1),
    ("Item", "oteryn:item.tibia.i3264", "definition-r1", 1),
    ("Item", "fixture:ground.root", "definition-r1", 1),
    ("ItemType", "fixture:corpse-loot", "rev-1", 1),
    ("Item", "fixture:capacity.exact", "definition-r1", 155999),
    ("Item", "fixture:capacity.over", "definition-r1", 156000),
    ("Item", "fixture:capacity.level51", "definition-r1", 157998),
];
