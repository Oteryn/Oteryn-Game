//! Source combat trait bridge into the existing native DEATH-2 player vitals owner.
//! Only ChannelSpellStates commits these receipts; attack consumers retain their own
//! bounded occurrence ordering ledger. A receipt is provenance, never current authority.
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    ScopeOwnershipGeneration, WorldId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerLethalReceipt {
    actor: ExactActorRef,
    session: GameSessionId,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    occurrence: [u8; 16],
    revision: u64,
    position: MovementLocalPosition,
}
impl PlayerLethalReceipt {
    /// Metadata from the SAME committed native DEATH-2 hit. No alternate mint/queue.
    /// Caller is the sole ChannelSpellStates bridge; this receipt is never authority.
    pub(crate) fn from_native_commit(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        revision: u64,
        occurrence: [u8; 16],
        position: MovementLocalPosition,
    ) -> Self {
        let binding = runtime.binding();
        Self {
            actor,
            session,
            world: binding.world_id(),
            channel: binding.channel_id(),
            generation: binding.scope_generation(),
            occurrence,
            revision,
            position,
        }
    }
    pub(crate) const fn actor(self) -> ExactActorRef {
        self.actor
    }
    pub(crate) const fn session(self) -> GameSessionId {
        self.session
    }
    pub(crate) const fn world(self) -> WorldId {
        self.world
    }
    pub(crate) const fn channel(self) -> ChannelId {
        self.channel
    }
    pub(crate) const fn generation(self) -> ScopeOwnershipGeneration {
        self.generation
    }
    pub(crate) const fn occurrence_bytes(self) -> [u8; 16] {
        self.occurrence
    }
    pub(crate) const fn revision(self) -> u64 {
        self.revision
    }
    pub(crate) const fn position(self) -> MovementLocalPosition {
        self.position
    }
    /// Independently current carrier facts; never reconstruct current authority from receipt.
    pub(crate) fn current(self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        binding.world_id() == self.world
            && binding.channel_id() == self.channel
            && binding.scope_generation() == self.generation
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerDamageReceipt {
    pub(crate) applied: u32,
    pub(crate) health_after: u32,
    pub(crate) vitals_revision: u64,
    pub(crate) death: Option<PlayerLethalReceipt>,
}
/// An MP-only owner commit; carries no HP mutation or player-death authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerManaDrainReceipt {
    pub(crate) applied: u32,
    pub(crate) mana_before: u32,
    pub(crate) mana_after: u32,
    pub(crate) vitals_revision: u64,
}
/// Native transient source heal, no mana/death/cooldown/condition mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerHealReceipt {
    pub(crate) applied: u32,
    pub(crate) health_before: u32,
    pub(crate) health_after: u32,
    pub(crate) max_health: u32,
    pub(crate) vitals_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceAppearanceReceipt {
    pub(crate) actor: ExactActorRef,
    pub(crate) applied: bool,
}
pub(crate) struct SourceAppearanceTarget<'a> {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: Option<GameSessionId>,
    pub(crate) facts: crate::foundation::ApplicationFacts<'a>,
    pub(crate) immunities: Vec<crate::foundation::ConditionType>,
}
pub(crate) trait PlayerLethalVitals {
    /// Source cast owns occurrence/replay and independently current tile/Combat admission.
    /// The actual Player and physical Creature stores are staged together or all refuse.
    fn apply_source_appearance_batch(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
        _source: ExactActorRef,
        _definitions: &[crate::foundation::ConditionDefinition],
        _targets: &[SourceAppearanceTarget<'_>],
    ) -> Option<Vec<SourceAppearanceReceipt>> {
        None
    }

    /// Independently current actor/session HP view for composed source preflight.
    /// No caller-supplied maximum or HP field becomes current authority.
    fn source_player_health(
        &self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
    ) -> Option<(u32, u32, u64)> {
        None
    }

    /// Private source-qualified caller seam. Unsupported owners refuse, never synthesize HP.
    fn apply_source_player_heal(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _magnitude: u32,
        _occurrence: &str,
        _now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerHealReceipt> {
        None
    }

    /// Secondary-only source-qualified area dispel. Caller owns cast replay/Combat admission.
    /// Unsupported owners refuse; this creates no HP/MP receipt.
    // Keep remove_attack_invisibility ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn remove_attack_invisibility(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _source: ExactActorRef,
        _now: u64,
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        false
    }

    fn apply_attack_mana_drain(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _magnitude: u32,
        _occurrence: &str,
        _now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerManaDrainReceipt> {
        None
    }

    /// An admitted secondary-only effect, with no primary HP receipt or synthetic hit.
    /// Owners without a condition store explicitly refuse.
    // Keep apply_attack_conditions ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_attack_conditions(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _occurrence: &str,
        _source: ExactActorRef,
        _definitions: &[crate::foundation::ConditionDefinition],
        _facts: &crate::foundation::ApplicationFacts<'_>,
        _immunities: &[crate::foundation::ConditionType],
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        false
    }

    /// Secondary definitions require independently current admission facts. Owners which do
    /// not implement atomic conditions must refuse before committing primary damage.
    // Keep apply_composite_attack_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_composite_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        _source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        _facts: &crate::foundation::ApplicationFacts<'_>,
        _immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let binding = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return None;
        }

        if definitions.is_empty() {
            self.apply_attack_damage(
                runtime,
                target,
                session,
                magnitude,
                occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(_facts.now),
            )
        } else {
            None
        }
    }

    fn apply_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerDamageReceipt>;
}
