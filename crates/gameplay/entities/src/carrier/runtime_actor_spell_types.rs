//! Typed effects for the physical Channel owner's atomic spell transaction.
//! Candidate bounds are explicit; these do not widen the accepted two-effect Ability plan.

use super::{CharacterId, CommandRef, ExactActorRef, OwnerDamageResult};
use oteryn_gameplay_foundation::foundation::{
    RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};

/// SPELL-BATCH-RL candidate, to be registered by the owning delivery contract.
pub const MAX_EFFECTS: usize = 256;
pub const MAX_BINDING_BYTES: usize = 131_072;
pub const MAX_COMMAND_RECEIPTS: usize = 16;

/// The actual player-vitals owner independently compares this anchor against its live state
/// before the physical owner commit. This immutable description alone grants no authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAnchor {
    pub expected_revision: u64,
    pub next_revision: u64,
    pub paid_mana: u32,
    pub paid_soul: u32,
    pub cooldown_deadlines: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarState {
    pub expires_ms: u64,
    pub outfit_look_type: u32,
    pub incoming_reduction_percent: u32,
    pub critical_chance_percent: u32,
    pub critical_extra_percentage_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaShieldState {
    pub capacity: u32,
    pub expires_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterAiState {
    pub forced_distance: Option<(u32, u64)>,
    pub challenged_to: Option<(ExactActorRef, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerCombatChange {
    PlayerConditions {
        expected: Box<oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>>,
        next: Box<oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>>,
    },
    CompanionConditions(Box<CompanionConditionUpdate>),
    CompanionMaster(Box<super::runtime_actor_companion::PreparedCompanionAssignment>),
    Damage {
        target_atom: String,
        magnitude: i64,
    },
    Heal {
        target_atom: String,
        magnitude: i64,
    },
    /// Source damage healing is committed before residual damage, as one
    /// retained source occurrence. Damage may be zero after immunity.
    DamageWithHealing {
        target_atom: String,
        damage: i64,
        healing: i64,
    },
    Avatar(AvatarState),
    ManaShield(ManaShieldState),
    /// Correlated mana payment is staged by the actual player-vitals owner in the same group.
    ConsumeManaShield {
        expected_capacity: u32,
        amount: u32,
    },
    MonsterAi(MonsterAiState),
    DispelParalysis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionConditionUpdate {
    pub expected: super::runtime_actor_companion::CompanionSnapshot,
    pub next: super::runtime_actor_companion::CompanionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCombatEffect {
    pub target: ExactActorRef,
    /// Global within the original command, retained unchanged by a chain/delayed timer.
    pub sub_ordinal: u16,
    pub change: OwnerCombatChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCombatBatch {
    pub caster: ExactActorRef,
    pub attacker: CharacterId,
    pub current_lease_generation: u64,
    pub command: CommandRef,
    pub occurrence: SpellOccurrenceBinding,
    /// Revision-qualified source/formula/legality description; the owner also encodes every
    /// typed field itself, so callers cannot hide an effect change behind an unchanged hash.
    pub binding: Vec<u8>,
    pub anchor: Option<SpellAnchor>,
    /// Original native semantic time, captured before millisecond conversion.
    /// Missing precision cannot qualify shared-experience activity.
    pub now_us: Option<oteryn_simulation_determinism::SemanticTimeMicros>,
    pub now_ms: u64,
    pub effects: Vec<OwnerCombatEffect>,
    pub deferred: Option<DeferredCommitAuthority>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectReceipt {
    pub target: ExactActorRef,
    pub sub_ordinal: u16,
    pub health: Option<OwnerDamageResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatBatchReceipt {
    pub applied: bool,
    pub caster: ExactActorRef,
    pub command: CommandRef,
    pub effects: Vec<EffectReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Owner(oteryn_gameplay_foundation::foundation::CarrierError),
    InvalidBatch,
    InvalidAnchor,
    InvalidMagnitude,
    InvalidCondition,
    TooManyEffects,
    BindingTooLarge,
    CommandConflict,
    SupersededSession,
    StaleCommand,
    SnapshotChanged,
    PlayerVitalsOwnerRequired,
    AllocationFailed,
}

impl From<oteryn_gameplay_foundation::foundation::CarrierError> for Error {
    fn from(error: oteryn_gameplay_foundation::foundation::CarrierError) -> Self {
        Self::Owner(error)
    }
}

/// A data-only occurrence binding usable by standalone Foundation consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellOccurrenceBinding {
    pub id: String,
    pub revisions: [String; 5],
}

impl From<oteryn_gameplay_foundation::foundation::ability_data::AbilityOccurrence>
    for SpellOccurrenceBinding
{
    fn from(
        value: oteryn_gameplay_foundation::foundation::ability_data::AbilityOccurrence,
    ) -> Self {
        let r = value.revisions();
        Self {
            id: value.id().as_str().to_owned(),
            revisions: [
                r.ruleset().to_owned(),
                r.content().to_owned(),
                r.world_policy().to_owned(),
                r.formula().to_owned(),
                r.simulation().to_owned(),
            ],
        }
    }
}

/// Implemented by the audited actual player-vitals owner, never by a decoded input.
pub mod player_proof_seal {
    pub trait Sealed {}
}
pub trait PlayerBatchProof: player_proof_seal::Sealed {
    fn matches_batch(&self, batch: &OwnerCombatBatch) -> bool;
    /// Actual per-effect HP transitions staged by the player-vitals owner.
    fn health_receipt(&self, target: ExactActorRef, sub_ordinal: u16) -> Option<OwnerDamageResult>;
}

/// Immutable scheduler provenance, not current ownership. The actual timer owner's
/// successful drain is the sole production producer of this crate-private factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredCommitAuthority {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    work_stamp: RuntimeWorkStamp,
    phase: u16,
    admission_sequence: u64,
    caster: ExactActorRef,
    attacker: CharacterId,
    lease_generation: u64,
    command: CommandRef,
    occurrence: SpellOccurrenceBinding,
    parent_binding: Vec<u8>,
    binding: Vec<u8>,
    effects: Vec<OwnerCombatEffect>,
    now_us: Option<oteryn_simulation_determinism::SemanticTimeMicros>,
    now_ms: u64,
}
impl DeferredCommitAuthority {
    #[allow(clippy::too_many_arguments)]
    pub fn from_due(
        fence: &ScopeRuntimeFence,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        stamp: RuntimeWorkStamp,
        phase: u16,
        sequence: u64,
        parent_binding: Vec<u8>,
        batch: &OwnerCombatBatch,
    ) -> Result<Self, Error> {
        if !fence.is_current_for_scope(scope, generation)
            || !fence.accepts_stamp(stamp)
            || batch.anchor.is_some()
            || sequence == 0
            || parent_binding.is_empty()
            || parent_binding.len() > MAX_BINDING_BYTES
        {
            return Err(Error::InvalidBatch);
        }
        Ok(Self {
            scope,
            generation,
            work_stamp: stamp,
            phase,
            admission_sequence: sequence,
            caster: batch.caster,
            attacker: batch.attacker,
            lease_generation: batch.current_lease_generation,
            command: batch.command,
            occurrence: batch.occurrence.clone(),
            parent_binding,
            binding: batch.binding.clone(),
            effects: batch.effects.clone(),
            now_us: batch.now_us,
            now_ms: batch.now_ms,
        })
    }
    pub fn matches(
        &self,
        batch: &OwnerCombatBatch,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> bool {
        self.scope == scope
            && self.generation == generation
            && self.work_stamp.generation() == generation
            && self.caster == batch.caster
            && self.attacker == batch.attacker
            && self.lease_generation == batch.current_lease_generation
            && self.command == batch.command
            && self.occurrence == batch.occurrence
            && self.binding == batch.binding
            && self.effects == batch.effects
            && self.now_us == batch.now_us
            && self.now_ms == batch.now_ms
            && batch.anchor.is_none()
    }
    pub fn canonical_binding(&self) -> &[u8] {
        &self.parent_binding
    }
    pub fn phase_ordinal(&self) -> u16 {
        self.phase
    }
    pub fn admission_sequence(&self) -> u64 {
        self.admission_sequence
    }
}
