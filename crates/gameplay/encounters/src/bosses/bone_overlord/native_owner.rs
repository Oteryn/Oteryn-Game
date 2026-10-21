use oteryn_gameplay_entities::carrier::encounter_access::{
    EncounterHealProjection, EncounterHooks,
};
use oteryn_gameplay_entities::carrier::{
    ChannelActorCarrier, ChannelRuntimeAssignmentBinding, NamespaceContinuityGuard, Slot,
};
use oteryn_gameplay_entities::*;
pub trait BoneOverlordCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn bind_bone_overlord_cage_phase(
        &self,
        cages: [ExactActorRef; 4],
        phylactery: ExactActorRef,
        encounter_fingerprint: &str,
        artifact_digest: [u8; 32],
    ) -> Result<BoneOverlordCagePhase, BonePhaseError>;
    #[allow(clippy::too_many_arguments)]
    fn commit_bone_overlord_projected_death(
        &mut self,
        state: &mut BoneOverlordCagePhase,
        actor: ExactActorRef,
    ) -> Result<BonePhaseTransition, BonePhaseError>;
}
// Source-specific approved authored Encounter state; NOT a Crystal Lua callback.
// Exact participant bindings are verified by the owning native Content loader.
const BONE_CAGE_KEYS: [&[u8]; 4] = [
    b"oteryn:creature.elyrax_s_soulcage",
    b"oteryn:creature.myzareth_s_soulcage",
    b"oteryn:creature.scarith_s_soulcage",
    b"oteryn:creature.zharvorin_s_soulcage",
];
const BONE_CASE_FINGERPRINT: &str =
    "cc3143372125a5bd4bff66bbd1dc0b3cf2989e6892e3135901f5083f2cd689ef";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneOverlordPhase {
    Soulcages,
    Phylactery,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonePhaseError {
    Carrier(CarrierError),
    InvalidSource,
    WrongParticipant,
    WrongPhase,
    ContentChanged,
}
impl From<CarrierError> for BonePhaseError {
    fn from(e: CarrierError) -> Self {
        Self::Carrier(e)
    }
}
/// Fixed-size one-instance owner state: five captured native generations, four-bit death mask.
/// Does not implement shared HP, targetability, spawns, quest entry/exit or rewards.
#[derive(Debug)]
pub struct BoneOverlordCagePhase {
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    dead_mask: u8,
    phase: BoneOverlordPhase,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonePhaseTransition {
    pub newly_committed: bool,
    pub cages_destroyed: u8,
    pub phase: BoneOverlordPhase,
}
impl BoneOverlordCarrierExt for ChannelRuntimeV1 {
    /// Native loader seam: registry has verified five canonical typed profiles and exact Crystal
    /// bindings. Fingerprint qualifies approved authored Encounter, not a donor callback claim.
    fn bind_bone_overlord_cage_phase(
        &self,
        cages: [ExactActorRef; 4],
        phylactery: ExactActorRef,
        encounter_fingerprint: &str,
        artifact_digest: [u8; 32],
    ) -> Result<BoneOverlordCagePhase, BonePhaseError> {
        if encounter_fingerprint != BONE_CASE_FINGERPRINT {
            return Err(BonePhaseError::InvalidSource);
        }
        if artifact_digest != self.content_pin().server_artifact_digest() {
            return Err(BonePhaseError::ContentChanged);
        }
        for (i, a) in cages.iter().enumerate() {
            if !self.matches_live_creature_identity(*a, BONE_CAGE_KEYS[i]) {
                return Err(BonePhaseError::WrongParticipant);
            }
            if cages[..i].contains(a) {
                return Err(BonePhaseError::WrongParticipant);
            }
        }
        if !self
            .matches_live_creature_identity(phylactery, b"oteryn:creature.bonelord_s_phylactery")
        {
            return Err(BonePhaseError::WrongParticipant);
        }
        Ok(BoneOverlordCagePhase {
            cages,
            phylactery,
            dead_mask: 0,
            phase: BoneOverlordPhase::Soulcages,
            world: self.content_pin().world_id(),
            channel: self.encounter_owner_scope().1,
            generation: self.encounter_owner_scope().2,
            activation: self.content_pin().activation_sequence(),
            server: self.content_pin().server_artifact_digest(),
            client: self.content_pin().client_artifact_digest(),
            frame: self.content_pin().frame_binding_digest(),
            map: self.content_pin().map_revision_digest(),
        })
    }
    /// Called after actual current-owner corpse projection, before despawn, under the same owner
    /// borrow. No caller-authored death boolean/key. Validation completes before phase write.
    fn commit_bone_overlord_projected_death(
        &mut self,
        state: &mut BoneOverlordCagePhase,
        actor: ExactActorRef,
    ) -> Result<BonePhaseTransition, BonePhaseError> {
        if state.world != self.content_pin().world_id()
            || state.channel != self.encounter_owner_scope().1
            || state.generation != self.encounter_owner_scope().2
            || state.activation != self.content_pin().activation_sequence()
            || state.server != self.content_pin().server_artifact_digest()
            || state.client != self.content_pin().client_artifact_digest()
            || state.frame != self.content_pin().frame_binding_digest()
            || state.map != self.content_pin().map_revision_digest()
        {
            return Err(BonePhaseError::ContentChanged);
        }
        let cage = state.cages.iter().position(|a| *a == actor);
        if cage.is_none() && actor != state.phylactery {
            return Err(BonePhaseError::WrongParticipant);
        }
        // validate_ref alone checks scope/index; explicitly reject a recycled actor generation.
        let idx = self.encounter_actor_index(actor)?;
        let expected = if let Some(i) = cage {
            BONE_CAGE_KEYS[i]
        } else {
            b"oteryn:creature.bonelord_s_phylactery"
        };
        match self.encounter_slot(idx) {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && target_identity.as_ref() == expected => {}
            _ => return Err(BonePhaseError::Carrier(CarrierError::StaleActorGeneration)),
        }
        self.borrow_combat_death().projected_death(actor)?;
        let mut newly = false;
        if let Some(i) = cage {
            if state.dead_mask & (1 << i) == 0 {
                if state.phase != BoneOverlordPhase::Soulcages {
                    return Err(BonePhaseError::WrongPhase);
                }
                state.dead_mask |= 1 << i;
                newly = true;
                if state.dead_mask == 15 {
                    state.phase = BoneOverlordPhase::Phylactery
                }
            }
        } else if state.phase != BoneOverlordPhase::Completed {
            if state.phase != BoneOverlordPhase::Phylactery {
                return Err(BonePhaseError::WrongPhase);
            }
            state.phase = BoneOverlordPhase::Completed;
            newly = true;
        }
        Ok(BonePhaseTransition {
            newly_committed: newly,
            cages_destroyed: state.dead_mask.count_ones() as u8,
            phase: state.phase,
        })
    }
}

// PROJECT approximation only: wiki shared-life semantics + Crystal per-cage120000 baseline.
// The physical creature slots are the only HP values; registration/phase state stores no pool HP.
#[derive(Debug)]
pub struct BoneSharedHpResult {
    pub transitions: [OwnerDamageResult; 4],
    pub approximation: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeBoneSharedGroup {
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    cage_deaths: [Option<CreatureDeathOccurrenceKey>; 4],
}
pub trait BoneSharedCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn commit_bone_shared_damage(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        target: ExactActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<BoneSharedHpResult, BonePhaseError>;
    fn bone_shared_owner_guard(
        &self,
        state: &BoneOverlordCagePhase,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<[usize; 4], BonePhaseError>;
    #[allow(clippy::too_many_arguments)]
    fn register_bone_shared_hp(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    ) -> Result<(), BonePhaseError>;
}

impl BoneSharedCarrierExt for ChannelRuntimeV1 {
    fn bone_shared_owner_guard(
        &self,
        state: &BoneOverlordCagePhase,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<[usize; 4], BonePhaseError> {
        let (carrier, continuity) = self.encounter_parts();
        bone_shared_owner_guard(
            carrier,
            continuity,
            self.content_pin(),
            self.binding(),
            self.encounter_owner_scope(),
            state,
            current,
            stamp,
        )
    }

    /// Called only after existing actual five-profile Bone registry qualified the phase instance.
    /// No health counter is stored: all shared health lives in the four physical creature slots.
    fn register_bone_shared_hp(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    ) -> Result<(), BonePhaseError> {
        let indexes = self.bone_shared_owner_guard(state, current, stamp)?;
        if indexes.iter().any(|i| {
            !matches!(
                self.encounter_slot(*i),
                Slot::CreatureOccupied { health: 1.., .. }
            )
        }) {
            return Err(BonePhaseError::WrongPhase);
        }
        self.with_encounter_hooks::<BoneEncounterHooks, _, BonePhaseError>(|runtime, hooks| {
        if hooks.groups
            .iter()
            .any(|g| g.cages == state.cages && g.phylactery == state.phylactery)
        {
            return Ok(());
        }
        if hooks.groups.iter().any(|g| {
            g.cages.iter().any(|a| state.cages.contains(a)) || g.phylactery == state.phylactery
        }) {
            return Err(BonePhaseError::WrongParticipant);
        }
        let keep: [bool; 16] = std::array::from_fn(|i| {
            hooks.groups.get(i).is_some_and(|g|g.cages.iter().chain(std::iter::once(&g.phylactery)).any(|a|runtime.encounter_actor_index(*a).ok().is_some_and(|index|matches!(runtime.encounter_slot(index),Slot::CreatureOccupied{generation,..}if *generation==a.0.actor_local_generation.0))))
        });
        let mut n = 0;
        hooks.groups.retain(|_| {
            let retain = keep[n];
            n += 1;
            retain
        });
        if hooks.groups.len() >= 16 {
            return Err(BonePhaseError::Carrier(CarrierError::CapacityExceeded));
        }
        hooks.groups
            .try_reserve(1)
            .map_err(|_| BonePhaseError::Carrier(CarrierError::AllocationFailed))?;
        hooks.groups.push(NativeBoneSharedGroup {
            cages: state.cages,
            phylactery: state.phylactery,
            world: state.world,
            channel: state.channel,
            generation: state.generation,
            activation: state.activation,
            server: state.server,
            client: state.client,
            frame: state.frame,
            map: state.map,
            cage_deaths: [None; 4],
        });
        Ok(())
        })
    }

    /// Same generic native receipt/contributor preparation as ordinary damage, staged for four
    /// actual actors before any physical HP write. One incoming occurrence, separate per-actor
    /// receipts; replay/changed-plan refusals retain ordinary native semantics.
    // Keep commit_bone_shared_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn commit_bone_shared_damage(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        target: ExactActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<BoneSharedHpResult, BonePhaseError> {
        let indexes = self.bone_shared_owner_guard(state, current, stamp)?;
        let target_index = state
            .cages
            .iter()
            .position(|a| *a == target)
            .ok_or(BonePhaseError::WrongParticipant)?;
        if command.target != BONE_CAGE_KEYS[target_index] {
            return Err(BonePhaseError::WrongParticipant);
        }
        let content = self.content_pin().clone();
        let binding = self.binding();
        let owner_scope = self.encounter_owner_scope();
        let (carrier, continuity) = self.encounter_parts_mut();
        let results = carrier.commit_encounter_damage_batch(
            continuity,
            state.cages,
            BONE_CAGE_KEYS,
            command,
            attacker,
            fail_before_write,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            None,
            |carrier, staged_indexes, results| {
                if *staged_indexes != indexes {
                    return Err(BonePhaseError::WrongParticipant);
                }
                if results.iter().any(|r| {
                    r.applied != results[0].applied
                        || r.health_before != results[0].health_before
                        || r.health_after != results[0].health_after
                }) {
                    return Err(BonePhaseError::InvalidSource);
                }
                if bone_shared_owner_guard(
                    carrier,
                    continuity,
                    &content,
                    binding,
                    owner_scope,
                    state,
                    current,
                    stamp,
                )? != indexes
                {
                    return Err(BonePhaseError::WrongParticipant);
                }
                Ok(())
            },
        )?;
        Ok(BoneSharedHpResult {
            transitions: results,
            approximation: "PROJECT_SHARED_POOL_FROM_CRYSTAL_CAGE_MAX_120000_NOT_GLOBAL",
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn bone_shared_owner_guard(
    carrier: &ChannelActorCarrier,
    continuity: &NamespaceContinuityGuard,
    content: &ChannelContentPin,
    binding: ChannelRuntimeAssignmentBinding,
    owner_scope: (WorldId, ChannelId, ScopeOwnershipGeneration),
    state: &BoneOverlordCagePhase,
    current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
) -> Result<[usize; 4], BonePhaseError> {
    let b = binding;
    let scope = oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
        b.world_id(),
        b.channel_id(),
    );
    if !current.is_current_for_scope(scope, b.scope_generation()) || !current.accepts_stamp(stamp) {
        return Err(BonePhaseError::Carrier(CarrierError::WrongScope));
    }
    if state.world != content.world_id()
        || state.channel != owner_scope.1
        || state.generation != owner_scope.2
        || state.activation != content.activation_sequence()
        || state.server != content.server_artifact_digest()
        || state.client != content.client_artifact_digest()
        || state.frame != content.frame_binding_digest()
        || state.map != content.map_revision_digest()
    {
        return Err(BonePhaseError::ContentChanged);
    }
    if state.phase != BoneOverlordPhase::Soulcages || state.dead_mask != 0 {
        return Err(BonePhaseError::WrongPhase);
    }
    let mut indexes = [0; 4];
    let mut hp = None;
    for (i, actor) in state.cages.iter().enumerate() {
        let index = carrier.encounter_actor_index(continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            target_identity,
            ..
        } = &carrier.encounter_slots()[index]
        else {
            return Err(BonePhaseError::WrongParticipant);
        };
        if target_identity.as_ref() != BONE_CAGE_KEYS[i]
            || carrier
                .encounter_native_summon_role(continuity, *actor)?
                .is_some()
            || *health < 0
            || *health > 120000
            || indexes[..i].contains(&index)
        {
            return Err(BonePhaseError::WrongParticipant);
        }
        if hp.is_some_and(|prior| prior != *health) {
            return Err(BonePhaseError::InvalidSource);
        }
        hp = Some(*health);
        indexes[i] = index;
    }
    Ok(indexes)
}

fn bone_shared_indexes(
    carrier: &ChannelActorCarrier,
    c: &NamespaceContinuityGuard,
    g: &NativeBoneSharedGroup,
) -> Result<[usize; 4], CarrierError> {
    if carrier.encounter_scope().0 != g.world
        || carrier.encounter_scope().1 != g.channel
        || carrier.encounter_scope().2 != g.generation
    {
        return Err(CarrierError::WrongScope);
    }
    let mut indexes = [0; 4];
    let mut hp = None;
    for i in 0..4 {
        let index = carrier.encounter_actor_index(c, g.cages[i].0)?;
        let Slot::CreatureOccupied {
            generation,
            health,
            target_identity,
            ..
        } = &carrier.encounter_slots()[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != g.cages[i].0.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if target_identity.as_ref() != BONE_CAGE_KEYS[i]
            || *health < 0
            || *health > 120000
            || indexes[..i].contains(&index)
            || carrier.encounter_is_native_summon(g.cages[i])
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if hp.is_some_and(|old| old != *health) {
            return Err(CarrierError::PlanConflict);
        }
        hp = Some(*health);
        indexes[i] = index;
    }
    Ok(indexes)
}

#[allow(clippy::too_many_arguments)]
fn commit_bone_shared_damage_inner(
    carrier: &mut ChannelActorCarrier,
    c: &NamespaceContinuityGuard,
    g: &NativeBoneSharedGroup,
    target: ActorRef,
    command: OwnerDamageCommand<'_>,
    attacker: Option<AttackerCommand>,
    fail: bool,
    max_sub_ordinal: Option<u16>,
    now_ms: Option<u64>,
) -> Result<OwnerDamageResult, CarrierError> {
    let indexes = bone_shared_indexes(carrier, c, g)?;
    let selected = g
        .cages
        .iter()
        .position(|a| a.0 == target)
        .ok_or(CarrierError::CreatureTargetMismatch)?;
    if command.target != BONE_CAGE_KEYS[selected] {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    let out = carrier.commit_encounter_damage_batch(
        c,
        g.cages,
        BONE_CAGE_KEYS,
        command,
        attacker,
        fail,
        max_sub_ordinal,
        now_ms,
        |carrier, staged_indexes, out| {
            if *staged_indexes != indexes {
                return Err(CarrierError::StaleActorGeneration);
            }
            if out.iter().any(|r| {
                r.applied != out[0].applied
                    || r.health_before != out[0].health_before
                    || r.health_after != out[0].health_after
            }) {
                return Err(CarrierError::PlanConflict);
            }
            if bone_shared_indexes(carrier, c, g)? != indexes {
                return Err(CarrierError::StaleActorGeneration);
            }
            Ok(())
        },
    )?;
    Ok(out[selected])
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BoneEncounterHooks {
    groups: Vec<NativeBoneSharedGroup>,
}
impl EncounterHooks for BoneEncounterHooks {
    fn clone_hooks(&self) -> Box<dyn EncounterHooks> {
        Box::new(self.clone())
    }
    fn eq_hooks(&self, other: &dyn EncounterHooks) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn contains(&self, actor: ExactActorRef) -> bool {
        self.groups
            .iter()
            .any(|g| g.phylactery == actor || g.cages.contains(&actor))
    }
    #[allow(clippy::too_many_arguments)]
    fn damage(
        &self,
        carrier: &mut ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        pin: Option<&ChannelContentPin>,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Option<Result<OwnerDamageResult, CarrierError>> {
        if !self.contains(ExactActorRef(actor_ref)) {
            return None;
        }
        Some((|| {
            if let Some(g) = self.groups.iter().find(|g| {
                g.phylactery == ExactActorRef(actor_ref)
                    || g.cages.contains(&ExactActorRef(actor_ref))
            }) {
                let pin = pin.ok_or(CarrierError::PositionContextMismatch)?;
                if pin.world_id() != g.world
                    || pin.activation_sequence() != g.activation
                    || pin.server_artifact_digest() != g.server
                    || pin.client_artifact_digest() != g.client
                    || pin.frame_binding_digest() != g.frame
                    || pin.map_revision_digest() != g.map
                {
                    return Err(CarrierError::PositionContextMismatch);
                }
                if carrier.encounter_scope().0 != g.world
                    || carrier.encounter_scope().1 != g.channel
                    || carrier.encounter_scope().2 != g.generation
                {
                    return Err(CarrierError::WrongScope);
                }
            }
            if let Some(g) = self
                .groups
                .iter()
                .find(|g| g.phylactery == ExactActorRef(actor_ref))
                && g.cage_deaths.iter().any(Option::is_none)
            {
                return Err(CarrierError::CreatureNotActionable);
            }
            if let Some(g) = self
                .groups
                .iter()
                .find(|g| g.cages.contains(&ExactActorRef(actor_ref)))
                .cloned()
            {
                return commit_bone_shared_damage_inner(
                    carrier,
                    continuity,
                    &g,
                    actor_ref,
                    command,
                    attacker,
                    fail_before_write,
                    max_sub_ordinal,
                    now_ms,
                );
            }
            carrier.commit_encounter_single_damage(
                continuity,
                actor_ref,
                command,
                attacker,
                fail_before_write,
                max_sub_ordinal,
                now_ms,
            )
        })())
    }
    fn projected_death(&mut self, actor: ExactActorRef, death: CreatureDeathOccurrenceKey) {
        for g in &mut self.groups {
            if let Some(i) = g.cages.iter().position(|a| *a == actor) {
                g.cage_deaths[i] = Some(death)
            }
        }
    }
    /// Normalize inside existing source-qualified native heal sink, before its preflight/publication.
    /// PROJECT coalescence: one source cast applies once per shared group; lowest source roster index
    /// in requested targets supplies its qualified draw. No invented spirit spell/radius/threshold.
    fn expand_heals(
        &self,
        runtime: &ChannelRuntimeV1,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError> {
        let mut expanded = Vec::new();
        expanded
            .try_reserve(64)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut map = Vec::new();
        map.try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (n, (actor, key, max, draw)) in requests.iter().enumerate() {
            if *max == 0 || *max > i64::MAX as u64 || *draw > i64::MAX as u64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            if requests[..n].iter().any(|r| r.0 == *actor) {
                return Err(CarrierError::PlanConflict);
            }
            if let Some(g) = self.groups.iter().find(|g| g.cages.contains(actor)) {
                if !bone_shared_pin_matches(runtime, g) {
                    return Err(CarrierError::PositionContextMismatch);
                }
                let indexes = bone_shared_indexes(
                    runtime.encounter_parts().0,
                    runtime.encounter_parts().1,
                    g,
                )?;
                let selected = g
                    .cages
                    .iter()
                    .position(|a| a == actor)
                    .ok_or(CarrierError::CreatureTargetMismatch)?;
                if key.as_bytes() != BONE_CAGE_KEYS[selected] || *max != 120000 {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                let chosen = g
                    .cages
                    .iter()
                    .find_map(|a| requests.iter().find(|r| r.0 == *a))
                    .ok_or(CarrierError::PlanConflict)?;
                if !expanded
                    .iter()
                    .any(|r: &(ExactActorRef, String, u64, u64)| g.cages.contains(&r.0))
                {
                    if expanded.len() + 4 > 64 {
                        return Err(CarrierError::AllocationFailed);
                    }
                    for i in 0..4 {
                        let Slot::CreatureOccupied { health, .. } =
                            runtime.encounter_slots()[indexes[i]]
                        else {
                            return Err(CarrierError::NotCreature);
                        };
                        if health <= 0 {
                            return Err(CarrierError::CreatureNotActionable);
                        }
                        expanded.push((
                            g.cages[i],
                            std::str::from_utf8(BONE_CAGE_KEYS[i])
                                .map_err(|_| CarrierError::InvalidCreatureTarget)?
                                .to_owned(),
                            120000,
                            chosen.3,
                        ));
                    }
                }
                map.push(
                    expanded
                        .iter()
                        .position(|r| r.0 == *actor)
                        .ok_or(CarrierError::PlanConflict)?,
                );
            } else {
                if expanded.len() >= 64 {
                    return Err(CarrierError::AllocationFailed);
                }
                map.push(expanded.len());
                expanded.push((*actor, key.clone(), *max, *draw));
            }
        }
        Ok((expanded, map))
    }
}

#[cfg(any(test, feature = "test-support"))]
pub trait BoneFixtureDamageExt {
    #[allow(clippy::too_many_arguments)]
    fn bone_shared_fixture_damage(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        damage: i64,
        occurrence: &str,
    ) -> Result<OwnerDamageResult, CarrierError>;
}
#[cfg(any(test, feature = "test-support"))]
impl BoneFixtureDamageExt for ChannelRuntimeV1 {
    fn bone_shared_fixture_damage(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        damage: i64,
        occurrence: &str,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let binding = format!("{occurrence}\0{key}\0{damage}");
        self.borrow_exact_actor_commit().commit_damage(
            actor,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: occurrence.as_bytes(),
                binding: binding.as_bytes(),
                damage,
            },
        )
    }
}

fn bone_shared_pin_matches(runtime: &ChannelRuntimeV1, g: &NativeBoneSharedGroup) -> bool {
    g.world == runtime.content_pin().world_id()
        && g.channel == runtime.encounter_owner_scope().1
        && g.generation == runtime.encounter_owner_scope().2
        && g.activation == runtime.content_pin().activation_sequence()
        && g.server == runtime.content_pin().server_artifact_digest()
        && g.client == runtime.content_pin().client_artifact_digest()
        && g.frame == runtime.content_pin().frame_binding_digest()
        && g.map == runtime.content_pin().map_revision_digest()
}

#[cfg(any(test, feature = "test-support"))]
#[path = "native_owner_tests.rs"]
mod tests;
#[cfg(any(test, feature = "test-support"))]
pub use tests::bone_shared_actual_owner_harness;
