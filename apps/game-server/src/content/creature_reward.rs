//! ARCH-KILL-REWARD-LOGOUT-1 §1.1: the immutable reward rows of one active generation, keyed
//! by the creature definition reference the runtime uses for its creature policies. A creature
//! without a settleable row fails closed: its kill settles nothing and logs one
//! `kill_reward_refused` line with the reason.

use std::collections::HashMap;
use std::sync::Arc;

use oteryn_simulation_determinism::ExactI64;

use crate::combat::{LootDefinitionRef, LootTableDefinition};
use crate::domain::bestiary::BestiaryRace;

/// What one creature's kill settles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureRewardRow {
    pub(crate) xp_amount: ExactI64,
    pub(crate) corpse_item: LootDefinitionRef,
    pub(crate) loot_table_ref: LootDefinitionRef,
    pub(crate) loot_table: LootTableDefinition,
    pub(crate) race: Option<BestiaryRace>,
}

/// Why a creature's kill settles nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoSettlementReason {
    /// The pin carries no `creature_loot` row for the creature.
    NoLootBinding,
    /// The bound loot reference has no pinned table.
    #[cfg_attr(not(test), allow(dead_code))] // Built by the reward table builder (part B).
    LootTableMissing,
}

impl NoSettlementReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NoLootBinding => "no_loot_binding",
            Self::LootTableMissing => "loot_table_missing",
        }
    }
}

/// The reward rows of one active generation. A creature with no entry is `no_loot_binding`.
#[derive(Debug, Clone, Default)]
pub(crate) struct CreatureRewardTable {
    rows: HashMap<String, Result<Arc<CreatureRewardRow>, NoSettlementReason>>,
}

impl CreatureRewardTable {
    #[cfg(test)]
    pub(crate) fn insert(
        &mut self,
        creature: impl Into<String>,
        row: Result<CreatureRewardRow, NoSettlementReason>,
    ) {
        self.rows.insert(creature.into(), row.map(Arc::new));
    }

    pub(crate) fn row(&self, creature: &str) -> Result<Arc<CreatureRewardRow>, NoSettlementReason> {
        self.rows
            .get(creature)
            .cloned()
            .unwrap_or(Err(NoSettlementReason::NoLootBinding))
    }
}
