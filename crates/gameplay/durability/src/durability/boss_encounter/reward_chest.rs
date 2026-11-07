//! Personal reward entitlements owned by the existing boss settlement and item custody owners.
use super::super::item_mint::TypedDefinitionRef;
use crate::domain::CharacterId;
use crate::foundation::GameSessionId;
use oteryn_gameplay_encounter_rules::boss::RewardParticipation;

#[path = "reward_chest/settlement.rs"]
mod settlement;

/// One selected reward before it enters physical custody. Native Item policy determines its
/// stack shape at collection; reward rolls are frozen once, never repeated by a client command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardItem {
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardContributor {
    pub character_id: CharacterId,
    pub session_id: GameSessionId,
    pub lease_generation: u64,
    pub contribution: RewardParticipation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardChestPlan {
    pub character_id: CharacterId,
    pub session_id: GameSessionId,
    pub lease_generation: u64,
    pub contribution: RewardParticipation,
    pub items: Vec<BossRewardItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardDeath {
    pub death: [u8; 16],
    pub boss_key: String,
    pub minted_at_unix_ms: u64,
    /// Complete death-time census, including support-only and below-threshold participants.
    /// This is the denominator proof for both eligibility and the personalized loot factor.
    pub contributors: Vec<BossRewardContributor>,
    pub chests: Vec<BossRewardChestPlan>,
}

/// The existing reward-boss projection admits at most 1,000 selected Item entries.
pub const BOSS_REWARD_ITEMS_MAX: usize = 1_000;
/// The native combat owner retains at most sixteen Character damage shares per creature.
pub const BOSS_REWARD_PARTICIPANTS_MAX: usize = 16;
/// Incoming hits and healing can involve every member of the native fifty-character group.
pub const BOSS_REWARD_CONTRIBUTORS_MAX: usize = 50;
