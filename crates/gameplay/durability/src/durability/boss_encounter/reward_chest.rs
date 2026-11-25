//! Personal reward entitlements owned by the existing boss settlement and item custody owners.
use super::super::item_mint::TypedDefinitionRef;
use crate::domain::CharacterId;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};
use oteryn_gameplay_encounter_rules::boss::RewardParticipation;

#[path = "reward_chest/settlement.rs"]
mod settlement;

/// One selected reward before it enters physical custody. Native Item policy determines its
/// stack shape at collection; reward rolls are frozen once, never repeated by a client command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardItem {
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    /// One-based ordinal of the root container in this character's chest.
    pub parent_ordinal: Option<u16>,
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

/// Stable native boss occurrence shared by chest and quest settlement.
pub fn scoped_death_id(
    scope: RuntimeScopeRefV1,
    generation: u64,
    actor: u32,
    actor_generation: u64,
) -> [u8; 16] {
    let (kind, scope_id) = match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => (1_u8, *channel_id.as_bytes()),
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2_u8, instance_id),
    };
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:personal-boss-death:v1\0")
        .chain_update(scope.world_id().as_bytes())
        .chain_update([kind])
        .chain_update(scope_id)
        .chain_update(generation.to_be_bytes())
        .chain_update(actor.to_be_bytes())
        .chain_update(actor_generation.to_be_bytes())
        .finalize();
    let mut result = [0; 16];
    result.copy_from_slice(&digest[..16]);
    result
}

/// Frozen canonical death with optional admitted private rewards. A withheld
/// private table never invents a chest or prevents the donor's quest flag.
#[derive(Clone, Debug)]
pub struct BossDeathAtomicPlan {
    pub death: [u8; 16],
    pub boss_key: String,
    pub minted_at_unix_ms: u64,
    pub rewards: Option<BossRewardDeath>,
}
impl From<BossRewardDeath> for BossDeathAtomicPlan {
    fn from(rewards: BossRewardDeath) -> Self {
        Self {
            death: rewards.death,
            boss_key: rewards.boss_key.clone(),
            minted_at_unix_ms: rewards.minted_at_unix_ms,
            rewards: Some(rewards),
        }
    }
}
