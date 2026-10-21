//! Boundary implemented by the Channel spell-state owner under the same runtime lock.
use super::native_conditions::{
    NativeConditionAdmission, NativeConditionApplication, NativeConditionError,
};
use super::native_invisibility::{SelfInvisibleSource, SelfSpeedSource};
use super::native_ticks::NativeOwnedPlayerTickReceipt;
use crate::foundation::{
    ApplicationFacts, ChannelRuntimeV1, ConditionDefinition, ConditionSource, ConditionType,
    ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use oteryn_simulation_determinism::DecisionOccurrenceId;

#[allow(clippy::too_many_arguments)]
pub trait MonsterCombatStates:
    crate::player_lethal::PlayerLethalVitals + crate::creature_area_heal::AreaHealStates
{
    fn prepare_native_conditions(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        target: ExactActorRef,
        session: Option<GameSessionId>,
        source: ConditionSource,
        definitions: &[ConditionDefinition],
        immunities: &[ConditionType],
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError>;
    fn prepare_native_self_invisible(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        source: &SelfInvisibleSource,
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError>;
    fn prepare_native_self_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        source: &SelfSpeedSource,
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError>;
    fn commit_native_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        application: &NativeConditionApplication,
        policy: &mut impl NativeConditionAdmission,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
    ) -> Result<bool, NativeConditionError>;
    fn tick_source_player_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        _occurrence: DecisionOccurrenceId,
        facts: oteryn_gameplay_foundation::foundation::condition::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<NativeOwnedPlayerTickReceipt>;
    fn tick_source_creature_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        facts: crate::foundation::TickFacts,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<crate::foundation::CreaturePeriodicReceipt, crate::foundation::CarrierError>;
    fn commit_self_heal_and_paralysis_removal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        ledger: &mut crate::foundation::CreatureSelfHealLedger,
        registration: &crate::foundation::CreatureSelfHealRegistration,
        actor: ExactActorRef,
        sequence: u64,
        draw: i64,
        now: u64,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<crate::foundation::OwnerDamageResult, crate::foundation::CarrierError>;
}
