//! Stateless source descriptors; conditions live only in native occupied actor slots.
//! This is a conditional-visibility veto, not an alternate perception or protocol authority.
use super::*;
use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityEffect, ProjectV2AbilityKind,
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues, StatusKind,
};
use crate::foundation::{MovementPositionContext, WorldId};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pin {
    world: WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    movement: MovementPositionContext,
}
impl Pin {
    fn current(r: &ChannelRuntimeV1) -> Self {
        let p = r.content_pin();
        Self {
            world: p.world_id(),
            activation: p.activation_sequence(),
            server: p.server_artifact_digest(),
            client: p.client_artifact_digest(),
            frame: p.frame_binding_digest(),
            movement: r.pinned_movement_context(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InvisibilityError {
    InvalidNativeBinding,
    UnsupportedSource,
    ContentPinMismatch,
    StaleCreature,
    MissingPosition,
    MissingApplicationFacts,
    StackedCasterCell,
    MissingCombatPolicy,
    InvalidOccurrence,
    StaleClock,
    ReplayConflict,
    ConditionRefused,
    AllocationFailed,
    CandidateBudget,
}
/// Independently current tile Combat admission, supplied by the actual map/Combat owner.
/// No permissive production default: missing policy facts must refuse before condition mutation.
pub(crate) trait InvisibleTileCombatPolicy {
    fn current_tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        caster: ExactActorRef,
    ) -> Option<bool>;
}
fn current_position(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<(), InvisibilityError> {
    let p = runtime
        .read_actor_position(actor)
        .map_err(|_| InvisibilityError::MissingPosition)?;
    if p.context() != runtime.pinned_movement_context() {
        return Err(InvisibilityError::ContentPinMismatch);
    }
    Ok(())
}
/// Minted from exact current native profiles, never a network-supplied actor token.
#[derive(Debug, Clone)]
pub(crate) struct SelfInvisibleSource {
    pin: Pin,
    creature: Ref,
    ability: Ref,
    definition: ConditionDefinition,
    interval_ms: u64,
    chance_ppm: u32,
}
fn behavior<'a>(
    creature: &Ref,
    records: &[ProjectReferenceRecord],
    profiles: &'a [ProjectV2AuthoringProfile],
) -> Result<&'a crate::content::ProjectV2BehaviorAuthoring, InvisibilityError> {
    if creature.family != ProjectV2Family::Creature {
        return Err(InvisibilityError::InvalidNativeBinding);
    }
    let mut keys = std::collections::BTreeSet::new();
    if !profiles.iter().all(|p| keys.insert(&p.target)) {
        return Err(InvisibilityError::InvalidNativeBinding);
    }
    let mut matches=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..} if identity.family=="Creature"&&identity.key==creature.key&&identity.revision==creature.revision));
    let Some(ProjectReferenceRecord::Creature { behavior, .. }) = matches.next() else {
        return Err(InvisibilityError::InvalidNativeBinding);
    };
    if matches.next().is_some() {
        return Err(InvisibilityError::InvalidNativeBinding);
    }
    let reference = Ref {
        family: ProjectV2Family::Behavior,
        key: behavior.key.clone(),
        revision: behavior.revision.clone(),
    };
    let Some(Data::Behavior(b)) = profiles
        .iter()
        .find(|p| p.target == reference)
        .map(|p| &p.data)
    else {
        return Err(InvisibilityError::InvalidNativeBinding);
    };
    Ok(b)
}
impl SelfInvisibleSource {
    pub(crate) fn from_native(
        runtime: &ChannelRuntimeV1,
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        source_digest: [u8; 32],
    ) -> Result<Self, InvisibilityError> {
        if runtime.content_pin().server_artifact_digest() != source_digest {
            return Err(InvisibilityError::ContentPinMismatch);
        }
        let b = behavior(creature, records, profiles)?;
        let e = b
            .defenses
            .get(index)
            .ok_or(InvisibilityError::InvalidNativeBinding)?;
        let Some(Data::Ability(a)) = profiles
            .iter()
            .find(|p| p.target == e.ability)
            .map(|p| &p.data)
        else {
            return Err(InvisibilityError::InvalidNativeBinding);
        };
        let mut ar=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..} if identity.family=="Ability"&&identity.key==e.ability.key&&identity.revision==e.ability.revision));
        if ar.next().is_none()
            || ar.next().is_some()
            || e.ability.family != ProjectV2Family::Ability
            || e.interval_ms == 0
            || e.chance_ppm > 1_000_000
        {
            return Err(InvisibilityError::InvalidNativeBinding);
        }
        let d = a
            .details
            .as_deref()
            .ok_or(InvisibilityError::InvalidNativeBinding)?;
        if d.kind != ProjectV2AbilityKind::Spell
            || d.needs_target
            || d.needs_direction
            || d.range_tiles != 0
            || d.area.is_some()
            || d.chain.is_some()
            || d.encounter.is_some()
            || d.windup.is_some()
            || !d.variants.is_empty()
            || d.effects.len() != 1
        {
            return Err(InvisibilityError::UnsupportedSource);
        }
        let ProjectV2AbilityEffect::Inline(effect) = &d.effects[0] else {
            return Err(InvisibilityError::UnsupportedSource);
        };
        let definition =
            crate::creature_condition_content::lower_condition_definition(effect, 1, None)
                .map_err(|_| InvisibilityError::UnsupportedSource)?;
        if !matches!(
            definition.values(),
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                ..
            }
        ) {
            return Err(InvisibilityError::UnsupportedSource);
        }
        Ok(Self {
            pin: Pin::current(runtime),
            creature: creature.clone(),
            ability: e.ability.clone(),
            definition,
            interval_ms: e.interval_ms,
            chance_ppm: e.chance_ppm,
        })
    }
    /// Descriptor only; scheduling/chance must use the existing current owner cycle.
    pub(crate) fn schedule(&self) -> (u64, u32) {
        (self.interval_ms, self.chance_ppm)
    }
}
#[derive(Debug, Clone)]
pub(crate) struct CreatureVision {
    pin: Pin,
    creature: Ref,
    sense_invisible: bool,
}
impl CreatureVision {
    pub(crate) fn from_native(
        runtime: &ChannelRuntimeV1,
        creature: &Ref,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        source_digest: [u8; 32],
    ) -> Result<Self, InvisibilityError> {
        if runtime.content_pin().server_artifact_digest() != source_digest {
            return Err(InvisibilityError::ContentPinMismatch);
        }
        Ok(Self {
            pin: Pin::current(runtime),
            creature: creature.clone(),
            sense_invisible: behavior(creature, records, profiles)?
                .targeting
                .sense_invisible,
        })
    }
}

/// Source Defense-phase position Combat speed condition; preserves declared range even though
/// castSpell(caster) uses caster position rather than that attack target range.
#[derive(Debug, Clone)]
pub(crate) struct SelfSpeedSource {
    inner: SelfInvisibleSource,
    base_speed: u16,
}
impl SelfSpeedSource {
    pub(crate) fn from_native(
        runtime: &ChannelRuntimeV1,
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        source_digest: [u8; 32],
    ) -> Result<Self, InvisibilityError> {
        use InvisibilityError as E;
        if runtime.content_pin().server_artifact_digest() != source_digest {
            return Err(E::ContentPinMismatch);
        }
        let b = behavior(creature, records, profiles)?;
        let e = b.defenses.get(index).ok_or(E::InvalidNativeBinding)?;
        let Some(Data::Creature(c)) = profiles
            .iter()
            .find(|p| p.target == *creature)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidNativeBinding);
        };
        let base_speed = u16::try_from(c.speed.ok_or(E::InvalidNativeBinding)?)
            .map_err(|_| E::UnsupportedSource)?;
        let Some(Data::Ability(a)) = profiles
            .iter()
            .find(|p| p.target == e.ability)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidNativeBinding);
        };
        let mut ar=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.family=="Ability"&&identity.key==e.ability.key&&identity.revision==e.ability.revision));
        if ar.next().is_none()
            || ar.next().is_some()
            || e.ability.family != ProjectV2Family::Ability
            || e.interval_ms == 0
            || e.chance_ppm > 1_000_000
        {
            return Err(E::InvalidNativeBinding);
        }
        let d = a.details.as_deref().ok_or(E::InvalidNativeBinding)?;
        if d.kind != ProjectV2AbilityKind::Spell
            || d.needs_target
            || d.needs_direction
            || d.area.is_some()
            || d.chain.is_some()
            || d.encounter.is_some()
            || d.windup.is_some()
            || !d.variants.is_empty()
            || d.effects.len() != 1
        {
            return Err(E::UnsupportedSource);
        }
        let ProjectV2AbilityEffect::Inline(effect) = &d.effects[0] else {
            return Err(E::UnsupportedSource);
        };
        let crate::content::ProjectV2InlineEffectOperation::Condition { condition, .. } =
            &effect.operation
        else {
            return Err(E::UnsupportedSource);
        };
        if !matches!(condition.condition_type.as_str(), "haste" | "paralyze") {
            return Err(E::UnsupportedSource);
        }
        let reference = condition
            .speed_formula
            .as_ref()
            .ok_or(E::InvalidNativeBinding)?;
        let Some(Data::Formula(formula)) = profiles
            .iter()
            .find(|p| p.target == *reference)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidNativeBinding);
        };
        let mut fr=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Formula{identity,..}if identity.family=="Formula"&&identity.key==reference.key&&identity.revision==reference.revision));
        if fr.next().is_none() || fr.next().is_some() {
            return Err(E::InvalidNativeBinding);
        }
        let definition = crate::creature_condition_content::lower_condition_definition(
            effect,
            1,
            Some((reference, formula)),
        )
        .map_err(|_| E::UnsupportedSource)?;
        if !matches!(
            definition.values(),
            ConditionValues::Speed { .. } | ConditionValues::RationalSpeed { .. }
        ) {
            return Err(E::UnsupportedSource);
        }
        Ok(Self {
            inner: SelfInvisibleSource {
                pin: Pin::current(runtime),
                creature: creature.clone(),
                ability: e.ability.clone(),
                definition,
                interval_ms: e.interval_ms,
                chance_ppm: e.chance_ppm,
            },
            base_speed,
        })
    }
    /// Scheduling/chance is consumed exactly once by existing ProfileScheduleState Defense.
    pub(crate) fn schedule(&self) -> (u64, u32) {
        self.inner.schedule()
    }
    pub(crate) fn base_speed(&self) -> u16 {
        self.base_speed
    }
}

fn exact_source_current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    source: &SelfInvisibleSource,
) -> bool {
    source.pin == Pin::current(runtime)
        && runtime
            .current_live_creature_identity(actor)
            .is_ok_and(|key| key == source.creature.key.as_bytes())
        && runtime
            .read_actor_position(actor)
            .is_ok_and(|p| p.context() == runtime.pinned_movement_context())
}
impl crate::movement::speed::CurrentCreatureSpeedReader for SelfSpeedSource {
    fn current_native_base_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<u16> {
        exact_source_current(runtime, actor, &self.inner).then_some(self.base_speed)
    }
}
impl ChannelSpellStates {
    /// The source cast remains scheduled/chance-qualified by the owning Defense dispatcher.
    /// This function stages canonical slot state only; caller retains the plan for replay.
    // Keep prepare_native_self_invisible ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_native_self_invisible(
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
    pub(crate) fn prepare_native_self_speed(
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
    pub(crate) fn native_creature_visible(
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
