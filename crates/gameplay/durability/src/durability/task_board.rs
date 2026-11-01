//! P04/P05 extend the quest revision/receipt owner with generated Task Board state.
//! Balances and talisman upgrades remain in the existing quest tracks. The JSON row
//! stores only the generated board and bounty selection; it is never a second wallet.

use std::collections::BTreeMap;
use std::sync::Arc;

use oteryn_gameplay_quests::quest::task_board::full::{
    TaskBoardAction, TaskBoardBalances, TaskBoardCreature, TaskBoardData,
    TaskBoardRefusal as RuleRefusal,
};
use serde::{Deserialize, Serialize};

use super::quest_state::{CommittedQuestTransition, QuestCause};

mod availability;
mod catalogue;
mod custody;
mod entitlements;
mod expansion;
mod fee;
mod mint;
mod progression;
mod state;
mod talisman;
mod writer;
pub use entitlements::read_promotion_points_in_transaction;
pub use talisman::{
    BountyTalismanSnapshot, read_talisman_in_item_transaction, read_talisman_in_transaction,
};

/// Immutable, bounded server content resolved from the active native generation.
#[derive(Debug, Clone)]
pub struct TaskBoardCatalogue {
    pub creatures: Vec<TaskBoardCreature>,
    /// The same Character content pin used by the existing quest owner.
    pub gameplay_content_revision: String,
    pub progression: Option<oteryn_gameplay_content::content::character_progression_content::CharacterProgressionContent>,
    /// Native appearance mapping revision, kept separate from the gameplay pin.
    pub content_revision: String,
    pub items: BTreeMap<u16, TaskBoardItem>,
    pub cosmetics: Option<oteryn_gameplay_content::content::native_gameplay::NativeCosmeticsView>,
}

#[derive(Debug, Clone)]
pub struct TaskBoardItem {
    pub key: String,
    pub revision: String,
    pub stackable: bool,
    /// Existing Item owner's complete admitted facts; absent for nonportable or
    /// otherwise unqualified reward outputs. Delivery matching remains separate.
    pub mint: Option<super::item_transfer::ItemDefinitionFacts>,
}

/// Snapshot after a command. A receipt retains these exact bytes for command replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardView {
    pub data: TaskBoardData,
    pub balances: TaskBoardBalances,
    pub tracks: BTreeMap<String, i64>,
    pub level: u32,
    /// Monday's day ordinal, derived only from the committed World reset owner.
    pub period: i64,
    pub database_now: i64,
    pub reroll_tokens: u64,
    /// Informative source-width snapshot for unfinished delivery rows. This is
    /// frozen with the receipt; internal kill receipts do not project item counts.
    #[serde(default)]
    pub delivery_available: BTreeMap<u16, u32>,
}

#[derive(Debug, Clone)]
pub struct TaskBoardCommand {
    pub cause: QuestCause,
    pub action: TaskBoardAction,
    /// Original bounded command bytes, not a semantic re-encoding. This distinguishes
    /// legacy/full query representations sharing command 89 and a CommandRef.
    pub payload_binding: Vec<u8>,
    /// Qualified current coin/backpack facts from the existing item-content owner.
    /// They are not client input and do not replace the original wire-byte binding.
    pub fee_change: Option<super::item_fee_burn::FeeChangeFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskBoardRefusal {
    Rule(RuleRefusal),
    InsufficientItems,
    InsufficientGold,
    UnavailableItem,
    CapacityExceeded,
    UnsupportedEntitlement,
}

#[derive(Debug, Clone)]
pub struct TaskBoardCommit {
    pub view: TaskBoardView,
    pub refusal: Option<TaskBoardRefusal>,
    pub transition: Option<CommittedQuestTransition>,
    pub replayed: bool,
}

/// State is constrained by the donor's maximum nine kill/nine delivery slots and
/// bounded five-entry lists; this byte cap also rejects malformed persisted JSON.
pub const MAX_TASK_BOARD_STATE_BYTES: usize = 65_536;
pub const MAX_TASK_BOARD_PAYLOAD_BYTES: usize = 1_024;
pub const SOULSEALS: &str = "oteryn:taskboard/weekly/soulseals";
pub const BOARD_REVISION: &str = "oteryn:taskboard/weekly/board-revision";
