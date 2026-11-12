//! Transport owner application of native creature condition descriptors.
use super::*;
use crate::foundation::{ApplicationFacts, ConditionSourceKind};
use crate::gameplay_transport::world_spatial;
use crate::monster_combat_lane::native_invisibility::*;

impl ChannelSpellStates {
    /// SRV6 owner difference from Crystal player.cpp:1383-1396: other players
    /// cannot observe or target an invisible player. The player's own view stays intact.
    pub(in crate::gameplay_transport) fn player_hidden_from(
        &self,
        runtime: &ChannelRuntimeV1,
        observer: ExactActorRef,
        target: ExactActorRef,
        now: SemanticTimeMicros,
    ) -> bool {
        if observer == target {
            return false;
        }
        let Some((_, session, state)) = self.actors.iter().find(|(actor, _, _)| *actor == target)
        else {
            return false;
        };
        if runtime.house_actor_state(target, *session).is_ok() {
            return state.owned_invisible_at(now.get()).unwrap_or(true)
                || runtime
                    .house_actor_conditions_at(target, *session, now.get())
                    .map_or(true, |conditions| conditions.invisible_at(now.get()));
        }
        self.get(runtime, target, *session).is_some_and(|state| {
            state.owned_invisible_at(now.get()).unwrap_or(true)
                || runtime
                    .actor_conditions_at(target, Some(*session), now.get())
                    .map_or(true, |conditions| conditions.invisible_at(now.get()))
        })
    }

    pub(in crate::gameplay_transport) fn player_ref_hidden_from(
        &self,
        runtime: &ChannelRuntimeV1,
        observer: ExactActorRef,
        target: world_spatial::EntityRef,
        now: SemanticTimeMicros,
    ) -> bool {
        self.actors.iter().any(|(actor, _, _)| {
            actor.placement_identity() == target.identity
                && actor.actor_local_generation() == target.generation
                && self.player_hidden_from(runtime, observer, *actor, now)
        })
    }

    /// The source cast remains scheduled/chance-qualified by the owning Defense dispatcher.
    /// This function stages canonical slot state only; caller retains the plan for replay.
    // Keep prepare_native_self_invisible ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_native_self_invisible(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        source: &SelfInvisibleSource,
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError> {
        if !exact_source_current(runtime, actor, source) {
            return Err(NativeConditionError::ContentChanged);
        }
        if facts.target_is_player
            || !runtime
                .is_sole_current_actor_on_cell(actor)
                .unwrap_or(false)
        {
            return Err(NativeConditionError::StaleActor);
        }
        // Source Combat self-defense bypasses the caster's condition immunity, and only its own.
        self.prepare_native_conditions(
            runtime,
            fence,
            stamp,
            actor,
            None,
            crate::foundation::ConditionSource {
                actor,
                session: None,
                kind: ConditionSourceKind::SelfUse,
            },
            std::slice::from_ref(&source.definition),
            &[],
            facts,
            policy,
        )
    }
    // Keep prepare_native_self_speed ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_native_self_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        source: &SelfSpeedSource,
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError> {
        if facts.base_speed != source.base_speed {
            return Err(NativeConditionError::StaleActor);
        }
        self.prepare_native_self_invisible(
            runtime,
            fence,
            stamp,
            actor,
            &source.inner,
            facts,
            policy,
        )
    }
    /// Visibility veto only. The owning AI still supplies its qualified hostility/geometric set
    /// and existing candidate budget; this never grants perception or a protocol subscription.
    pub fn native_creature_visible(
        &self,
        runtime: &ChannelRuntimeV1,
        observer: ExactActorRef,
        vision: &CreatureVision,
        target: ExactActorRef,
        now: SemanticTimeMicros,
    ) -> Result<bool, InvisibilityError> {
        if vision.pin != Pin::current(runtime) {
            return Err(InvisibilityError::ContentPinMismatch);
        }
        if !runtime
            .current_live_creature_identity(observer)
            .is_ok_and(|key| key == vision.creature.key.as_bytes())
        {
            return Err(InvisibilityError::StaleCreature);
        }
        current_position(runtime, observer)?;
        let invisible = self
            .native_actor_invisible(runtime, target, None, now)
            .ok_or(InvisibilityError::StaleCreature)?;
        Ok(!invisible || vision.sense_invisible)
    }
}
