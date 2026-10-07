//! Typed effects for the physical Channel owner's atomic spell transaction.
//! Candidate bounds are explicit; these do not widen the accepted two-effect Ability plan.

use super::{CharacterId, CommandRef, ExactActorRef, OwnerDamageResult};
use crate::foundation::{
    RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};

/// SPELL-BATCH-RL candidate, to be registered by the owning delivery contract.
pub(crate) const MAX_EFFECTS: usize = 256;
pub(crate) const MAX_BINDING_BYTES: usize = 131_072;
pub(crate) const MAX_COMMAND_RECEIPTS: usize = 16;

/// The actual player-vitals owner independently compares this anchor against its live state
/// before the physical owner commit. This immutable description alone grants no authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellAnchor {
    pub(crate) expected_revision: u64,
    pub(crate) next_revision: u64,
    pub(crate) paid_mana: u32,
    pub(crate) paid_soul: u32,
    pub(crate) cooldown_deadlines: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AvatarState {
    pub(crate) expires_ms: u64,
    pub(crate) outfit_look_type: u32,
    pub(crate) incoming_reduction_percent: u32,
    pub(crate) critical_chance_percent: u32,
    pub(crate) critical_extra_percentage_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManaShieldState {
    pub(crate) capacity: u32,
    pub(crate) expires_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonsterAiState {
    pub(crate) forced_distance: Option<(u32, u64)>,
    pub(crate) challenged_to: Option<(ExactActorRef, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OwnerCombatChange {
    PlayerConditions {
        expected: Box<super::super::condition::ConditionStore<String>>,
        next: Box<super::super::condition::ConditionStore<String>>,
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
pub(crate) struct CompanionConditionUpdate {
    pub(crate) expected: super::runtime_actor_companion::CompanionSnapshot,
    pub(crate) next: super::runtime_actor_companion::CompanionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerCombatEffect {
    pub(crate) target: ExactActorRef,
    /// Global within the original command, retained unchanged by a chain/delayed timer.
    pub(crate) sub_ordinal: u16,
    pub(crate) change: OwnerCombatChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerCombatBatch {
    pub(crate) caster: ExactActorRef,
    pub(crate) attacker: CharacterId,
    pub(crate) current_lease_generation: u64,
    pub(crate) command: CommandRef,
    pub(crate) occurrence: SpellOccurrenceBinding,
    /// Revision-qualified source/formula/legality description; the owner also encodes every
    /// typed field itself, so callers cannot hide an effect change behind an unchanged hash.
    pub(crate) binding: Vec<u8>,
    pub(crate) anchor: Option<SpellAnchor>,
    pub(crate) now_ms: u64,
    pub(crate) effects: Vec<OwnerCombatEffect>,
    pub(crate) deferred: Option<DeferredCommitAuthority>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectReceipt {
    pub(crate) target: ExactActorRef,
    pub(crate) sub_ordinal: u16,
    pub(crate) health: Option<OwnerDamageResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CombatBatchReceipt {
    pub(crate) applied: bool,
    pub(crate) caster: ExactActorRef,
    pub(crate) command: CommandRef,
    pub(crate) effects: Vec<EffectReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    Owner(crate::foundation::CarrierError),
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

impl From<crate::foundation::CarrierError> for Error {
    fn from(error: crate::foundation::CarrierError) -> Self {
        Self::Owner(error)
    }
}

/// A data-only occurrence binding usable by standalone Foundation consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellOccurrenceBinding {
    pub(crate) id: String,
    pub(crate) revisions: [String; 5],
}

/// Implemented by the audited actual player-vitals owner, never by a decoded input.
pub(crate) mod player_proof_seal {
    pub trait Sealed {}
}
pub(crate) trait PlayerBatchProof: player_proof_seal::Sealed {
    fn matches_batch(&self, batch: &OwnerCombatBatch) -> bool;
}

/// Immutable scheduler provenance, not current ownership. The actual timer owner's
/// successful drain is the sole production producer of this crate-private factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeferredCommitAuthority {
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
    now_ms: u64,
}
impl DeferredCommitAuthority {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_due(
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
            now_ms: batch.now_ms,
        })
    }
    pub(crate) fn matches(
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
            && self.now_ms == batch.now_ms
            && batch.anchor.is_none()
    }
    pub(crate) fn canonical_binding(&self) -> &[u8] {
        &self.parent_binding
    }
    pub(crate) fn phase_ordinal(&self) -> u16 {
        self.phase
    }
    pub(crate) fn admission_sequence(&self) -> u64 {
        self.admission_sequence
    }
}
