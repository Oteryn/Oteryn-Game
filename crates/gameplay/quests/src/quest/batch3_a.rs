//! Test-only authoring harness for the batch-three A source programs.
//!
//! No function here is a production NPC adapter or durable owner. The parent
//! module mounts this harness only under cfg(test), per coordinator ANSWER:
//! https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589900671
//!
//! Per-quest modules are discovered at build time. An isolated quest commit
//! therefore needs no later quest's JSON or Rust files.

use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleInput, NpcRuleTrigger};
use std::collections::BTreeMap;

pub fn input<'a>(
    npc: &'a str,
    tracks: &'a BTreeMap<String, i64>,
    inventory: &'a NpcInventorySnapshot,
    outfits: &'a BTreeMap<u16, u8>,
) -> NpcRuleInput<'a> {
    NpcRuleInput {
        npc,
        trigger: NpcRuleTrigger::Say,
        message: "a",
        topic: 2,
        tracks,
        inventory,
        now_epoch_seconds: 1_785_542_400,
        calendar_month: 8,
        level: 100,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits,
    }
}

include!(concat!(env!("OUT_DIR"), "/quest_batch3_a_modules.rs"));
