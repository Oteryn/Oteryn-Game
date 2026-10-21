//! Local native-combat assembly. No grant provider, background loop or invented world facts.
use crate::ability::RevisionSet;
use crate::ai_think::{
    ThinkOccurrence,
    profile_schedule::{
        AttackTarget, MonsterSummonFacts, ProfileScheduleInput, ProfileScheduleState,
        ScheduleError, ScheduleList,
    },
};
#[cfg(test)]
use crate::bosses::bone_overlord::native_owner::BoneFixtureDamageExt;
use crate::bosses::bone_overlord::native_owner::BoneOverlordCarrierExt;
use crate::content::ProjectV2Draft;
use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityAuthoring, ProjectV2AbilityKind,
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data, ProjectV2BehaviorAuthoring,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_area_heal::{
    AreaHealError, AreaHealOutcome, AreaHealOwner, AreaHealPolicy, AreaHealSource,
    CreatureHealCatalog,
};
use crate::creature_auto_attack::{AttackError, AutoAttackOwner, MeleeSource, SwingOutcome};
use crate::creature_chain_attack::{ChainMapFacts, ChainOwner};
use crate::creature_damage_spell::{
    ComposedOutcome, ComposedOwner, ComposedSource, ComposedWorldReader, CreatureCombatFacts,
    DamageSpellOwner, SpellOutcome, SpellSource, SpellWorldReader,
};
use crate::creature_defense_presentation::DefensePresentationSource;
use crate::foundation::ApplicationFacts;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use crate::foundation::{
    CarrierError, CreatureSelfHealLedger, CreatureSelfHealRegistration, OwnerDamageResult,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::monster_owner_cycle::{CycleError, MonsterThinkLane};
use crate::self_heal_qualification::QualifiedCreatureSelfHeal;
use crate::source_threshold_heal::{
    ThresholdHealError, ThresholdHealOutcome, ThresholdHealOwner, ThresholdHealSource,
};
use crate::spell::chain::TilePosition;
use native_invisibility::{
    InvisibilityError, InvisibleTileCombatPolicy, SelfInvisibleSource, SelfSpeedSource,
};
use native_ticks::NativeOwnedPlayerTickReceipt;
#[cfg(test)]
use oteryn_gameplay_encounters::bosses::crystal_deaths::native_fixtures::CrystalRouterFixtureExt;
pub use oteryn_gameplay_monster_ai::monster_combat_lane::{
    MonsterCombatReader, MonsterCombatStates, RegularCreatureMovementFacts, SelfConditionFacts,
    SummonFollowError, SummonFollowFacts, TrustedMonsterConditionTickInputs, native_conditions,
    native_invisibility, native_ticks,
};
use oteryn_simulation_determinism::GameplayDecisionRoot;
use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[derive(Clone)]
pub struct NativeHeal {
    index: usize,
    ability: Ref,
    source: QualifiedCreatureSelfHeal,
    registration: CreatureSelfHealRegistration,
}
#[derive(Debug)]
// Presentation is returned by value from source-qualified owner dispatch; boxing the result would add a heap allocation after committed owner state. Preserve the prepublication allocation boundary.
#[allow(clippy::large_enum_variant)]
pub enum DefenseOutcome {
    SelfHeal(OwnerDamageResult),
    ThresholdHeal(ThresholdHealOutcome),
    Icicle(crate::creature_icicle::IcicleOutcome),
    AreaHeal(AreaHealOutcome),
    Invisible(bool),
    Presentation(crate::creature_damage_spell::SourcePresentationEvent),
    Speed(bool),
    Appearance(bool),
}
#[derive(Debug)]
pub enum DefenseError {
    Carrier(CarrierError),
    Invisible(InvisibilityError),
    Speed(InvisibilityError),
    InvalidDraw,
    NativeCondition(native_conditions::NativeConditionError),
    Presentation(AttackError),
    AreaHeal(AreaHealError),
    ThresholdHeal(ThresholdHealError),
    Icicle(crate::creature_icicle::IcicleError),
}

struct ChainSpellReader<'a, 'b, R: MonsterCombatReader>(&'a std::cell::RefCell<&'b mut R>);
struct ChainMapReader<'a, 'b, R: MonsterCombatReader>(&'a std::cell::RefCell<&'b mut R>);
impl<R: MonsterCombatReader> SpellWorldReader for ChainSpellReader<'_, '_, R> {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        s: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        self.0.borrow_mut().current_players(r, s)
    }
    fn current_facing(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<crate::creature_attack_geometry::Facing> {
        self.0.borrow_mut().current_facing(r, a, s)
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        x: i32,
        y: i32,
        z: i16,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.borrow_mut().tile_allowed(r, a, x, y, z, s)
    }
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        i: ExactActorRef,
        t: ExactActorRef,
        p: GameSessionId,
        e: Option<&str>,
        s: RuntimeWorkStamp,
    ) -> Option<crate::creature_damage_spell::SpellCombatFacts> {
        self.0.borrow_mut().combat(r, i, t, p, e, s)
    }
}
impl<R: MonsterCombatReader> ChainMapFacts for ChainMapReader<'_, '_, R> {
    fn sight(
        &mut self,
        r: &ChannelRuntimeV1,
        f: TilePosition,
        t: TilePosition,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.borrow_mut().chain_sight(r, f, t, s)
    }
    fn path(
        &mut self,
        r: &ChannelRuntimeV1,
        f: TilePosition,
        t: TilePosition,
        s: RuntimeWorkStamp,
    ) -> Option<Vec<TilePosition>> {
        self.0.borrow_mut().chain_path(r, f, t, s)
    }
}

struct SummonMapReader<'a, R: MonsterCombatReader>(&'a mut R);
impl<R: MonsterCombatReader> crate::monster_summon::SummonLocationPolicy
    for SummonMapReader<'_, R>
{
    fn current_target_reachable(
        &mut self,
        r: &ChannelRuntimeV1,
        p: ExactActorRef,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.summon_target_reachable(r, p, t, s)
    }
    fn current_cell_admits(
        &mut self,
        r: &ChannelRuntimeV1,
        p: ExactActorRef,
        c: &Ref,
        a: crate::foundation::MovementLocalPosition,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.summon_cell_admits(r, p, c, a, s)
    }
}
struct AreaHealMapReader<'a, R: MonsterCombatReader> {
    inner: &'a mut R,
    ability: &'a Ref,
}
impl<R: MonsterCombatReader> AreaHealPolicy for AreaHealMapReader<'_, R> {
    fn facing(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<crate::creature_attack_geometry::Facing> {
        self.inner.current_facing(r, a, s)
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        x: i32,
        y: i32,
        z: i16,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.tile_allowed(r, a, x, y, z, s)
    }
    fn target_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner
            .area_heal_target_allowed(r, a, t, self.ability, s)
    }
    fn non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.area_heal_non_player_side(r, t, s)
    }
    fn masterless(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.area_heal_masterless(r, t, s)
    }
    fn is_top_creature(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.area_heal_top_creature(r, t, s)
    }
}
include!("../time_guardian/source_time_profile_bundle.rs");
pub struct NativeActor {
    pub seven_sources: Vec<crate::source_encounter_seven::SevenSource>,
    pub time_profiles: Option<std::sync::Arc<BTreeMap<String, TimeProfileBundle>>>,
    pub callback_spawn_sources: Vec<crate::source_callback_spawn::SourceCallbackSpawn>,
    callback_casts: Vec<crate::source_callback_cast::SourceCallbackCast>,
    smelly_source: Option<crate::smelly_encounter_dispatch::SourceSmelly>,
    movement_cadence: Option<crate::movement::speed::CreatureMovementCadence>,
    pub native_speed: Option<crate::movement::speed::NativeCreatureSpeed>,
    move_pacer: crate::movement::pacing::CreatureStepPacer,
    condition_casts: Vec<NativeDefenseConditionMemo>,
    pub actor: ExactActorRef,
    content: [u8; 32],
    pub behavior: ProjectV2BehaviorAuthoring,
    pub abilities: BTreeMap<Ref, ProjectV2AbilityAuthoring>,
    pub schedule: ProfileScheduleState,
    pub melee: Vec<MeleeSource>,
    pub spells: Vec<(usize, SpellSource)>,
    defense_variants: Vec<SpellSource>,
    appearance_defenses: Vec<(usize, SpellSource)>,
    composed: Vec<(usize, ComposedSource)>,
    pub unqualified: Vec<(usize, AttackError)>,
    pub heals: Vec<NativeHeal>,
    area_heals: Vec<(usize, AreaHealSource)>,
    threshold_heals: Vec<ThresholdHealSource>,
    icicle: Option<crate::creature_icicle::IcicleSource>,
    pub invisible: Vec<(usize, SelfInvisibleSource)>,
    pub speed: Vec<(usize, SelfSpeedSource)>,
    presentations: Vec<(usize, DefensePresentationSource)>,
    pub defense_summons: Vec<crate::monster_summon::DefenseSummonSource>,
    pub summons: Option<crate::monster_summon::NativeSummonCatalog>,
    pub selected_target: Option<(ExactActorRef, GameSessionId)>,
    pub creature: Ref,
}
#[derive(Debug)]
pub enum CombatLaneError {
    Cycle(CycleError),
    Attack(AttackError),
    DuplicateActor,
}
#[derive(Debug)]
pub struct ThinkCombatResult {
    pub seven_map_pending: Vec<(
        Ref,
        Result<crate::source_encounter_seven::SourceMapWork, AttackError>,
    )>,
    pub time_phases: Vec<(
        Ref,
        Result<crate::source_encounter_seven::TimePhaseOutcome, AttackError>,
    )>,
    pub callback_spawns: Vec<(Ref, Result<bool, AttackError>)>,
    /// Real existing AI proposals; physical SQL owner must return a COMMIT receipt.
    pub source_items: Vec<(
        Ref,
        Result<
            crate::creature_source_items::ScheduledCreatureSourceItems,
            crate::durability::spell_item_transaction::SpellItemError,
        >,
    )>,
    pub callback_casts: Vec<(Ref, Result<bool, AttackError>)>,
    pub smelly_scheduled: Vec<(Ref, Result<bool, AttackError>)>,
    pub occurrence: ThinkOccurrence,
    pub selection: Result<(), AttackError>,
    pub schedule: Result<(), ScheduleError>,
    pub spells: Vec<(Ref, Result<SpellOutcome, AttackError>)>,
    pub defense_variants: Vec<(Ref, Result<SpellOutcome, AttackError>)>,
    pub composed: Vec<(Ref, Result<ComposedOutcome, AttackError>)>,
    pub chains: Vec<(Ref, Result<u64, AttackError>)>,
    /// Explicit source entries not executed by the qualified consumer; never silently global-parity.
    pub unqualified: Vec<(usize, AttackError)>,
    pub deferred_defence: Vec<Ref>,
    pub defenses: Vec<(Ref, Result<DefenseOutcome, DefenseError>)>,
    pub defense_summons: Vec<(
        Ref,
        Result<crate::monster_summon::DefenseSummonBatch, crate::monster_summon::SummonError>,
    )>,
    pub deferred_summons: usize,
    pub follow:
        Option<Result<Option<crate::foundation::NativeSummonMovementOutcome>, SummonFollowError>>,
    pub summons: Vec<(
        Ref,
        Result<crate::monster_summon::SummonReceipt, crate::monster_summon::SummonError>,
    )>,
}
#[derive(Debug)]
pub struct CombatPulse {
    pub callback_speech_delivery:
        Option<Result<usize, crate::weak_spot_speech::WeakSpotSpeechError>>,
    pub callback_speech_errors: Vec<(ExactActorRef, AttackError)>,
    pub time_returns: Vec<Result<crate::source_encounter_seven::TimePhaseOutcome, AttackError>>,
    pub callback_removals: Vec<(ExactActorRef, Result<(), crate::foundation::CarrierError>)>,
    pub callback_spawn_pulses: Result<
        Vec<Result<crate::source_callback_spawn::CallbackSpawnReceipt, AttackError>>,
        AttackError,
    >,
    pub creature_conditions: Vec<(
        ExactActorRef,
        Result<crate::foundation::CreaturePeriodicReceipt, CarrierError>,
    )>,
    pub callback_pulses: crate::source_callback_cast::CallbackPulseBatch,
    pub smelly_pulses: crate::smelly_encounter_dispatch::SmellyPulseResults,
    pub movements: Vec<(
        ExactActorRef,
        Result<
            Option<crate::foundation::MovementPositionSnapshot>,
            crate::movement::speed::CreatureCadenceError,
        >,
    )>,
    pub child_registration: Vec<ChildRegistrationReceipt>,
    pub thinks: Vec<ThinkCombatResult>,
    pub melee: Vec<(Ref, Result<SwingOutcome, AttackError>)>,
    pub chain_steps:
        crate::creature_chain_attack::ChainPulseResults<crate::creature_damage_spell::SpellOutcome>,
    pub windup_casts: crate::creature_damage_spell::WindupCastResults,
    pub conditions: Result<Vec<(ExactActorRef, Option<NativeOwnedPlayerTickReceipt>)>, AttackError>,
}
pub struct MonsterCombatLane {
    callback_speech: crate::weak_spot_speech::WeakSpotSpeechMailbox,
    pub time_guardian_owner: crate::source_encounter_seven::TimeGuardianOwner,
    callback_spawn_owner: Option<crate::source_callback_spawn::CallbackSpawnOwner>,
    callback_casts: Option<crate::source_callback_cast::CallbackCastOwner>,
    native_appearance_content:
        Option<std::sync::Arc<crate::content::native_gameplay::NativeGameplayState>>,
    smelly: crate::smelly_encounter_dispatch::SmellyEncounterOwner,
    bone_encounters: Vec<BoneCombatInstance>,
    crystal_deaths: Option<crate::crystal_death_composition::CrystalDeathOwner>,
    timer: MonsterThinkLane,
    pub actors: Vec<NativeActor>,
    melee: AutoAttackOwner,
    spells: DamageSpellOwner,
    composed: ComposedOwner,
    chains: ChainOwner<SpellSource>,
    last_time: Option<SemanticTimeMicros>,
    heals: CreatureSelfHealLedger,
    area_catalog: Option<CreatureHealCatalog>,
    area_owner: AreaHealOwner,
    threshold_heal_owner: ThresholdHealOwner,
    icicle_owner: crate::creature_icicle::IcicleOwner,
    defense_summon_owner: crate::monster_summon::DefenseSummonOwner,
    summons: crate::monster_summon::NativeSummonOwner,
    pub child_closure: Option<std::sync::Arc<NativeChildClosure>>,
}
impl MonsterCombatLane {
    pub fn new(
        runtime: &ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
    ) -> Result<Self, CombatLaneError> {
        Ok(Self {
            time_guardian_owner: crate::source_encounter_seven::TimeGuardianOwner::new(runtime)
                .map_err(CombatLaneError::Attack)?,
            callback_speech: Default::default(),
            callback_spawn_owner: None,
            callback_casts: None,
            native_appearance_content: None,
            smelly: crate::smelly_encounter_dispatch::SmellyEncounterOwner::default(),
            timer: MonsterThinkLane::new(runtime, current).map_err(CombatLaneError::Cycle)?,
            actors: Vec::new(),
            melee: AutoAttackOwner::default(),
            spells: DamageSpellOwner::default(),
            composed: ComposedOwner::default(),
            chains: ChainOwner::new(runtime, current).map_err(CombatLaneError::Attack)?,
            last_time: None,
            heals: CreatureSelfHealLedger::default(),
            bone_encounters: Vec::new(),
            area_catalog: None,
            area_owner: AreaHealOwner::default(),
            threshold_heal_owner: ThresholdHealOwner::default(),
            icicle_owner: crate::creature_icicle::IcicleOwner::default(),
            defense_summon_owner: crate::monster_summon::DefenseSummonOwner::default(),
            summons: crate::monster_summon::NativeSummonOwner::default(),
            child_closure: None,
            crystal_deaths: None,
        })
    }
    /// Records/profiles are provided by the current native loader owner. Digest equality is an
    /// expected binding check, not a fabricated proof that arbitrary records came from an artifact.
    // Keep register_native source/owner ABI explicit: exact actor/source binding, occurrence, immutable definition and separately owned runtime/policy facts must not be conflated.
    #[allow(clippy::too_many_arguments)]
    fn register_native_definitions(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        creature: &Ref,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        loader_digest: [u8; 32],
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CombatLaneError> {
        let binding = runtime.binding();
        if !current.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) {
            return Err(CombatLaneError::Cycle(CycleError::StaleOwner));
        }
        if creature.family != ProjectV2Family::Creature {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        }
        if loader_digest != runtime.content_pin().server_artifact_digest() {
            return Err(CombatLaneError::Attack(AttackError::ContentChanged));
        }
        if !runtime.matches_live_creature_identity(actor, creature.key.as_bytes()) {
            return Err(CombatLaneError::Attack(AttackError::StaleIssuer));
        }
        match crate::monster_summon::current_source_creature_role(runtime, actor) {
            Ok(
                crate::monster_summon::CurrentSourceCreatureRole::Ordinary
                | crate::monster_summon::CurrentSourceCreatureRole::IntrinsicMonsterSummon(_),
            ) => {}
            _ => return Err(CombatLaneError::Attack(AttackError::InvalidSource)),
        }
        if self.actors.iter().any(|a| a.actor == actor) {
            return Err(CombatLaneError::DuplicateActor);
        }
        let record=records.iter().find(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..} if identity.key==creature.key&&identity.revision==creature.revision)).ok_or(CombatLaneError::Attack(AttackError::InvalidSource))?;
        let ProjectReferenceRecord::Creature { behavior, .. } = record else {
            unreachable!()
        };
        let bref = Ref {
            family: ProjectV2Family::Behavior,
            key: behavior.key.clone(),
            revision: behavior.revision.clone(),
        };
        let mut map = BTreeMap::new();
        for p in profiles {
            if map.insert(p.target.clone(), &p.data).is_some() {
                return Err(CombatLaneError::Attack(AttackError::InvalidSource));
            }
        }
        let Some(Data::Behavior(behavior)) = map.get(&bref).copied() else {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        };
        if behavior.attacks.len() > 16 || behavior.defenses.len() > 8 {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        }
        let abilities = profiles
            .iter()
            .filter_map(|p| match &p.data {
                Data::Ability(a) => Some((p.target.clone(), a.clone())),
                _ => None,
            })
            .collect();
        let mut melee = Vec::new();
        let mut spells = Vec::new();
        let mut unqualified = Vec::new();
        for (index, entry) in behavior.attacks.iter().enumerate() {
            let Some(Data::Ability(a)) = map.get(&entry.ability).copied() else {
                return Err(CombatLaneError::Attack(AttackError::InvalidSource));
            };
            if a.details
                .as_ref()
                .is_some_and(|d| d.kind == ProjectV2AbilityKind::Melee)
            {
                match MeleeSource::from_native(creature, index, records, profiles, loader_digest) {
                    Ok(s) => melee.push(s),
                    Err(e) => unqualified.push((index, e)),
                }
            } else {
                match SpellSource::from_native(creature, index, records, profiles, loader_digest) {
                    Ok(s) => spells.push((index, s)),
                    Err(e) => unqualified.push((index, e)),
                }
            }
        }
        self.actors
            .retain(|a| runtime.contains_live_creature(a.actor));
        self.actors
            .try_reserve(1)
            .map_err(|_| CombatLaneError::Attack(AttackError::LedgerFull))?;
        let occurrence = self
            .timer
            .schedule(runtime, current, actor, due)
            .map_err(CombatLaneError::Cycle)?;
        self.actors.push(NativeActor {
            seven_sources: Vec::new(),
            time_profiles: None,
            callback_spawn_sources: Vec::new(),
            callback_casts: Vec::new(),
            smelly_source: None,
            native_speed: crate::movement::speed::NativeCreatureSpeed::from_native(
                runtime,
                creature,
                records,
                profiles,
                loader_digest,
            ),
            move_pacer: crate::movement::pacing::CreatureStepPacer::default(),
            condition_casts: Vec::new(),
            actor,
            content: loader_digest,
            movement_cadence: crate::movement::speed::CreatureMovementCadence::bind(runtime, actor),
            behavior: behavior.clone(),
            abilities,
            schedule: ProfileScheduleState::new(actor),
            melee,
            spells,
            defense_variants: behavior
                .defenses
                .iter()
                .enumerate()
                .filter_map(|(i, _)| {
                    SpellSource::from_native_defense(creature, i, records, profiles, loader_digest)
                        .ok()
                })
                .collect(),
            appearance_defenses: behavior
                .defenses
                .iter()
                .enumerate()
                .filter_map(|(i, _)| {
                    SpellSource::from_native_appearance_defense(
                        creature,
                        i,
                        records,
                        profiles,
                        loader_digest,
                    )
                    .ok()
                    .map(|s| (i, s))
                })
                .collect(),
            composed: Vec::new(),
            unqualified,
            heals: Vec::new(),
            area_heals: Vec::new(),
            threshold_heals: Vec::new(),
            icicle: None,
            defense_summons: Vec::new(),
            summons: None,
            selected_target: None,
            creature: creature.clone(),
            presentations: behavior
                .defenses
                .iter()
                .enumerate()
                .filter_map(|(i, _)| {
                    DefensePresentationSource::from_native(
                        creature,
                        i,
                        records,
                        profiles,
                        loader_digest,
                    )
                    .ok()
                    .map(|source| (i, source))
                })
                .collect(),
            speed: behavior
                .defenses
                .iter()
                .enumerate()
                .filter_map(|(i, _)| {
                    SelfSpeedSource::from_native(
                        runtime,
                        creature,
                        i,
                        records,
                        profiles,
                        loader_digest,
                    )
                    .ok()
                    .map(|source| (i, source))
                })
                .collect(),
            invisible: behavior
                .defenses
                .iter()
                .enumerate()
                .filter_map(|(i, _)| {
                    SelfInvisibleSource::from_native(
                        runtime,
                        creature,
                        i,
                        records,
                        profiles,
                        loader_digest,
                    )
                    .ok()
                    .map(|s| (i, s))
                })
                .collect(),
        });
        Ok(occurrence)
    }
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::too_many_arguments)]
    pub fn register_native(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        creature: &Ref,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        loader_digest: [u8; 32],
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CombatLaneError> {
        self.register_native_definitions(
            runtime,
            current,
            actor,
            creature,
            records,
            profiles,
            loader_digest,
            due,
        )
    }
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        creature: &Ref,
        draft: &ProjectV2Draft,
        loader_digest: [u8; 32],
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CombatLaneError> {
        self.register_project_definitions(
            runtime,
            current,
            actor,
            creature,
            draft,
            loader_digest,
            due,
        )
    }
    /// The loaded artifact supplies definitions only; registration retains the existing
    /// current scope, physical actor identity and exact typed project admission checks.
    // Keep independently current runtime/fence, loaded content, source/proposal and owner facts explicit at this native composition boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn register_project_with_native(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        creature: &Ref,
        draft: &ProjectV2Draft,
        native: std::sync::Arc<crate::content::native_gameplay::NativeGameplayState>,
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CombatLaneError> {
        let digest = native.source_digest();
        if digest != runtime.content_pin().server_artifact_digest()
            || self
                .native_appearance_content
                .as_ref()
                .is_some_and(|old| old.source_digest() != digest)
        {
            return Err(CombatLaneError::Attack(AttackError::ContentChanged));
        }
        if !native.qualifies_current_project_definitions(digest, draft) {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        }
        let receipt = self
            .register_project_definitions(runtime, current, actor, creature, draft, digest, due)?;
        self.native_appearance_content = Some(native);
        Ok(receipt)
    }
    /// Private definition registrar; production admission first checks the independently
    /// loader-pinned definition closure, rather than trusting a supplied artifact digest.
    // Keep register_project source/owner ABI explicit: exact actor/source binding, occurrence, immutable definition and separately owned runtime/policy facts must not be conflated.
    #[allow(clippy::too_many_arguments)]
    fn register_project_definitions(
        &mut self,
        runtime: &ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        actor: ExactActorRef,
        creature: &Ref,
        draft: &ProjectV2Draft,
        loader_digest: [u8; 32],
        due: SemanticTimeMicros,
    ) -> Result<ThinkOccurrence, CombatLaneError> {
        if !crate::monster_summon::native_draft_world_matches(runtime, draft) {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        }
        if self.child_closure.is_none() {
            self.child_closure = Some(std::sync::Arc::new(NativeChildClosure::qualify(
                runtime,
                draft,
                loader_digest,
            )?));
        }
        let mut heals = Vec::new();
        if self.area_catalog.is_none() {
            self.area_catalog = Some(
                CreatureHealCatalog::from_native(draft, loader_digest)
                    .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?,
            );
        }
        let mut area_heals = Vec::new();
        let record = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == creature.key && identity.revision == creature.revision => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or(CombatLaneError::Attack(AttackError::InvalidSource))?;
        let behavior = draft
            .state
            .authoring_profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b)
                    if p.target.key == record.key && p.target.revision == record.revision =>
                {
                    Some(b)
                }
                _ => None,
            })
            .ok_or(CombatLaneError::Attack(AttackError::InvalidSource))?;
        for (index, entry) in behavior.defenses.iter().enumerate() {
            if let Ok(source) = AreaHealSource::from_native(draft, creature, index, loader_digest) {
                area_heals.push((index, source));
            }
            if let Ok(source) =
                QualifiedCreatureSelfHeal::qualify(draft, &creature.key, &entry.ability.key)
            {
                if source.source_schedule() != (entry.interval_ms, entry.chance_ppm) {
                    return Err(CombatLaneError::Attack(AttackError::InvalidSource));
                }
                let registration = source
                    .register(runtime)
                    .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?;
                heals.push(NativeHeal {
                    index,
                    ability: entry.ability.clone(),
                    source,
                    registration,
                });
            }
        }
        let mut composed = Vec::new();
        for (i, _) in behavior.attacks.iter().enumerate() {
            match ComposedSource::from_native(draft, creature, i, loader_digest) {
                Ok(source) => composed.push((i, source)),
                Err(AttackError::UnsupportedShape) => {}
                Err(e) => return Err(CombatLaneError::Attack(e)),
            }
        }
        let defense_summons = behavior
            .defenses
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                crate::monster_summon::DefenseSummonSource::qualify(
                    runtime,
                    creature,
                    draft,
                    i,
                    loader_digest,
                )
                .ok()
            })
            .collect();
        let summon_catalog = if behavior.summons.is_some() {
            Some(
                crate::monster_summon::NativeSummonCatalog::from_project(
                    runtime,
                    creature,
                    draft,
                    loader_digest,
                )
                .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?,
            )
        } else {
            None
        };
        let mut seven_sources = Vec::new();
        for (list, entries) in [
            (ScheduleList::Attack, &behavior.attacks),
            (ScheduleList::Defence, &behavior.defenses),
        ] {
            for index in 0..entries.len() {
                if let Some(source) = crate::source_encounter_seven::SevenSource::qualify(
                    draft,
                    creature,
                    index,
                    list,
                    loader_digest,
                )
                .map_err(CombatLaneError::Attack)?
                {
                    seven_sources.push(source);
                }
            }
        }
        let time_profiles = if let Some(source) = seven_sources.iter().find(|s| {
            matches!(
                s.kind(),
                crate::source_encounter_seven::SevenKind::TimeGuardian
                    | crate::source_encounter_seven::SevenKind::TimeGuardiann
            )
        }) {
            let mut phases = BTreeMap::new();
            for key in [
                "oteryn:creature.the_time_guardian",
                "oteryn:creature.the_blazing_time_guardian",
                "oteryn:creature.the_freezing_time_guardian",
            ] {
                phases.insert(
                    key.to_owned(),
                    TimeProfileBundle::prepare(runtime, source, key, draft)
                        .map_err(CombatLaneError::Attack)?,
                );
            }
            Some(std::sync::Arc::new(phases))
        } else {
            None
        };
        let mut callback_spawn_sources = Vec::new();
        for (list, entries) in [
            (ScheduleList::Attack, &behavior.attacks),
            (ScheduleList::Defence, &behavior.defenses),
        ] {
            for index in 0..entries.len() {
                if let Some(source) = crate::source_callback_spawn::SourceCallbackSpawn::qualify(
                    draft,
                    creature,
                    index,
                    list,
                    loader_digest,
                )
                .map_err(CombatLaneError::Attack)?
                {
                    callback_spawn_sources.push(source);
                }
            }
        }
        let mut callback_casts = Vec::new();
        for index in 0..behavior.attacks.len() {
            if let Some(source) = crate::source_callback_cast::SourceCallbackCast::qualify(
                draft,
                creature,
                index,
                loader_digest,
            )
            .map_err(CombatLaneError::Attack)?
            {
                callback_casts.push(source)
            }
        }
        let smelly_source = if creature.key == "oteryn:creature.smelly_cheese" {
            Some(
                crate::smelly_encounter_dispatch::SourceSmelly::qualify(
                    draft,
                    creature,
                    loader_digest,
                )
                .map_err(CombatLaneError::Attack)?,
            )
        } else {
            None
        };
        let occurrence = self.register_native_definitions(
            runtime,
            current,
            actor,
            creature,
            &draft.core.records,
            &draft.state.authoring_profiles,
            loader_digest,
            due,
        )?;
        let Some(registered) = self.actors.iter_mut().find(|a| a.actor == actor) else {
            return Err(CombatLaneError::Attack(AttackError::InvalidSource));
        };
        registered
            .unqualified
            .retain(|(i, _)| !composed.iter().any(|(entry, _)| i == entry));
        registered.seven_sources = seven_sources;
        registered.time_profiles = time_profiles;
        registered.callback_spawn_sources = callback_spawn_sources;
        registered.callback_casts = callback_casts;
        registered.smelly_source = smelly_source;
        registered.composed = composed;
        registered.heals = heals;
        registered.area_heals = area_heals;
        registered.icicle =
            crate::creature_icicle::IcicleSource::qualify(draft, creature, 0, loader_digest).ok();
        registered.threshold_heals = behavior
            .defenses
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                ThresholdHealSource::from_native(draft, creature, i, loader_digest).ok()
            })
            .collect();
        registered.defense_summons = defense_summons;
        registered.summons = summon_catalog;
        Ok(occurrence)
    }
    /// The caller holds runtime→states and supplies independently current scope authority and
    /// owner clock. Due think work and the global attack pulse get separate accepted ordinals.
    pub fn run<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        reader: &mut R,
        revisions: RevisionSet,
    ) -> Result<CombatPulse, CombatLaneError> {
        oteryn_gameplay_monster_ai::monster_combat_lane::run_encounters(
            self, runtime, states, current, clock, reader, revisions,
        )
    }
    fn run_encounter_turn<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        reader: &mut R,
        revisions: RevisionSet,
    ) -> Result<CombatPulse, CombatLaneError> {
        let now = clock.now();
        if self.last_time.is_some_and(|t| now < t) {
            return Err(CombatLaneError::Attack(AttackError::InvalidPlan));
        }
        let child_closure = self.child_closure.clone();
        let native_appearance_content = self.native_appearance_content.clone();
        let mut new_children = Vec::new();
        let time_guardian_owner = &mut self.time_guardian_owner;
        let actors = &mut self.actors;
        let melee = &mut self.melee;
        let spells = &mut self.spells;
        let composed = &mut self.composed;
        let chains = &mut self.chains;
        let heals = &mut self.heals;
        let area_catalog = &self.area_catalog;
        let area_owner = &mut self.area_owner;
        let threshold_heal_owner = &mut self.threshold_heal_owner;
        let icicle_owner = &mut self.icicle_owner;
        let defense_summon_owner = &mut self.defense_summon_owner;
        let summons = &mut self.summons;
        if self.callback_casts.is_none() {
            self.callback_casts = Some(
                crate::source_callback_cast::CallbackCastOwner::new(runtime, current)
                    .map_err(CombatLaneError::Attack)?,
            );
        }
        let callback_casts = self
            .callback_casts
            .as_mut()
            .ok_or(CombatLaneError::Attack(AttackError::InvalidPlan))?;
        if self.callback_spawn_owner.is_none() {
            self.callback_spawn_owner = Some(
                crate::source_callback_spawn::CallbackSpawnOwner::new(runtime, current)
                    .map_err(CombatLaneError::Attack)?,
            );
        }
        let callback_spawn_owner = self
            .callback_spawn_owner
            .as_mut()
            .ok_or(CombatLaneError::Attack(AttackError::InvalidPlan))?;
        let smelly = &mut self.smelly;
        let thinks = self
            .timer
            .run_due(
                runtime,
                states,
                current,
                clock,
                |runtime, states, fence, stamp, occurrence, due| {
                    let mut result = ThinkCombatResult {
                        seven_map_pending: Vec::new(),
                        time_phases: Vec::new(),
                        source_items: Vec::new(),
                        callback_casts: Vec::new(),
                        callback_spawns:Vec::new(),
                        smelly_scheduled: Vec::new(),
                        occurrence,
                        selection: Ok(()),
                        schedule: Ok(()),
                        spells: Vec::new(),
                        defense_variants: Vec::new(),
                        composed: Vec::new(),
                        chains: Vec::new(),
                        unqualified: Vec::new(),
                        deferred_defence: Vec::new(),
                        defenses: Vec::new(),
                        defense_summons: Vec::new(),
                        deferred_summons: 0,
                        follow: None,
                        summons: Vec::new(),
                    };
                    let role=match crate::monster_summon::current_source_creature_role(runtime,occurrence.actor) {
                        Ok(role @ (crate::monster_summon::CurrentSourceCreatureRole::Ordinary |
                            crate::monster_summon::CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)))=>role,
                        _=>{result.selection=Err(AttackError::InvalidSource);return result},
                    };
                    let master=role.intrinsic_master();
                    let inherited = master.and_then(|m| {
                        actors
                            .iter()
                            .find(|a| a.actor == m && runtime.contains_live_creature(m))
                            .and_then(|a| a.selected_target)
                    });
                    let Some(a) = actors.iter_mut().find(|a| a.actor == occurrence.actor) else {
                        result.selection = Err(AttackError::InvalidSource);
                        return result;
                    };
                    result.unqualified = a.unqualified.clone();
                    if a.content != runtime.content_pin().server_artifact_digest() {
                        melee.clear_target(a.actor);
                        result.selection = Err(AttackError::ContentChanged);
                        return result;
                    }
                    let selected = if master.is_some() {
                        Some(inherited.filter(|(target, session)| {
                            reader.summon_inherited_target_eligible(
                                runtime,
                                a.actor,
                                *target,
                                *session,
                                stamp,
                                &a.behavior,
                            ) == Some(true)
                        }))
                    } else {
                        reader.selected_player(runtime, stamp, occurrence, &a.behavior)
                    };
                    a.selected_target = None;
                    let target = match selected {
                        Some(Some((target, session))) => {
                            if !a.behavior.targeting.hostile || !a.behavior.targeting.can_target {
                                melee.clear_target(a.actor);
                                None
                            } else if runtime.player_control_facts(target, session).is_err() {
                                melee.clear_target(a.actor);
                                result.selection = Err(AttackError::StaleTarget);
                                return result;
                            } else {
                                if !a.melee.is_empty()
                                    && let Err(e) = melee.accept_ai_target(
                                        runtime, fence, stamp, occurrence, target, session,
                                        &a.melee, due,
                                    ) {
                                        result.selection = Err(e);
                                        return result;
                                    }
                                let distance = runtime
                                    .read_actor_position(a.actor)
                                    .ok()
                                    .zip(runtime.read_actor_position(target).ok())
                                    .and_then(|(i, t)| {
                                        if i.position().floor == t.position().floor {
                                            u16::try_from(
                                                (i64::from(i.position().x)
                                                    - i64::from(t.position().x))
                                                .abs()
                                                .max(
                                                    (i64::from(i.position().y)
                                                        - i64::from(t.position().y))
                                                    .abs(),
                                                ),
                                            )
                                            .ok()
                                        } else {
                                            None
                                        }
                                    });
                                Some(AttackTarget {
                                    actor: target,
                                    same_floor_distance: distance,
                                })
                            }
                        }
                        Some(None) => {
                            melee.clear_target(a.actor);
                            None
                        }
                        None => {
                            melee.clear_target(a.actor);
                            result.selection = Err(AttackError::MissingCombatFacts);
                            None
                        }
                    };
                    a.selected_target = if target.is_some() {
                        selected.flatten()
                    } else {
                        None
                    };
                    if let Some(parent) = master
                        && target.is_none() && a.behavior.movement.can_walk {
                            result.follow = Some(
                                match reader.summon_follow_movement(
                                    runtime,
                                    a.actor,
                                    parent,
                                    &a.creature,
                                    stamp,
                                ) {
                                    Some(facts) => commit_paced_summon_follow(
                                        runtime, fence, stamp, a.actor, parent, facts,
                                        a.native_speed.as_ref(), &mut a.move_pacer, due,
                                    ),
                                    None => Err(SummonFollowError::MissingMap),
                                },
                            );
                        }
                    // Source timers are delivered even without a fresh chance/admitted cast.
                    for source in &a.threshold_heals {
                        match threshold_heal_owner.tick(
                            runtime,
                            fence,
                            stamp,
                            source,
                            a.actor,
                            due.get(),
                        ) {
                            Ok(Some(receipt)) => result.defenses.push((
                                source.ability().clone(),
                                Ok(DefenseOutcome::ThresholdHeal(receipt)),
                            )),
                            Ok(None) => {}
                            Err(e) => result.defenses.push((
                                source.ability().clone(),
                                Err(DefenseError::ThresholdHeal(e)),
                            )),
                        }
                    }
                    let root = GameplayDecisionRoot::from_bytes(a.content);
                    let facts = if let Some(catalog) = &a.summons {
                        let path_target = target.as_ref().map(|t| t.actor).filter(|t| {
                            reader.summon_target_reachable(runtime, a.actor, *t, stamp)
                                == Some(true)
                        });
                        Some(catalog.facts(runtime, occurrence, path_target))
                    } else if master.is_some() {
                        a.behavior
                            .summons
                            .as_ref()
                            .map(|entries| MonsterSummonFacts {
                                occurrence,
                                is_summon: true,
                                target_with_path: None,
                                total_count: runtime.native_summon_count(a.actor, None),
                                entry_counts: entries
                                    .entries
                                    .iter()
                                    .map(|entry| {
                                        crate::ai_think::profile_schedule::ObservedSummonCount {
                                            creature: entry.creature.clone(),
                                            count: runtime.native_summon_count(
                                                a.actor,
                                                Some(&entry.creature.key),
                                            ),
                                        }
                                    })
                                    .collect(),
                            })
                    } else {
                        reader.summon_facts(runtime, stamp, occurrence, &a.behavior)
                    };
                    let plan = a.schedule.prepare_with_summons(
                        ProfileScheduleInput {
                            occurrence,
                            behavior: &a.behavior,
                            abilities: &a.abilities,
                            target,
                            revisions: &revisions,
                            root: &root,
                        },
                        facts.as_ref(),
                    );
                    let plan = match plan {
                        Ok(p) => p,
                        Err(e) => {
                            result.schedule = Err(e);
                            return result;
                        }
                    };
                    if let Some(catalog) = &a.summons {
                        for proposal in &plan.summon_proposals {
                            if child_closure.as_ref().is_none_or(|c| {
                                c.content != a.content
                                    || !c.children.iter().any(|(r, _)| *r == proposal.creature)
                            }) {
                                result.summons.push((
                                    proposal.creature.clone(),
                                    Err(crate::monster_summon::SummonError::InvalidSource),
                                ));
                                continue;
                            }
                            let outcome = summons.execute(
                                runtime,
                                fence,
                                stamp,
                                catalog,
                                proposal,
                                &mut SummonMapReader(reader),
                            );
                            if let Ok(child) = &outcome
                                && child.newly_created {
                                    new_children.push((
                                        child.child,
                                        proposal.creature.clone(),
                                        due,
                                        stamp,
                                    ));
                                }
                            result.summons.push((proposal.creature.clone(), outcome));
                        }
                    } else {
                        result.deferred_summons = plan.summon_proposals.len();
                    }
                    for proposal in plan.proposals {
                        if let Some(source)=a.seven_sources.iter().find(|s|s.ability()==proposal.ability).cloned() {
                            if matches!(source.kind(),crate::source_encounter_seven::SevenKind::TimeGuardian|crate::source_encounter_seven::SevenKind::TimeGuardiann) {
                                let hit=a.dispatch_time_phase(runtime,fence,stamp,time_guardian_owner,&source,&proposal,due);
                                result.time_phases.push((proposal.ability,hit));
                            } else {
                                // Descriptive pending work ONLY: no quest-coordinate relocation grant exists.
                                result.seven_map_pending.push((proposal.ability.clone(),source.prepare_map_work(runtime,fence,stamp,&proposal)));
                            }
                            continue;
                        }

                        if let Some(source)=a.callback_spawn_sources.iter().find(|s|s.ability()==&proposal.ability){let admitted=callback_spawn_owner.schedule(runtime,fence,stamp,source,&proposal,due);result.callback_spawns.push((proposal.ability,admitted));continue;}
                        if let Some(native)=native_appearance_content.as_deref().filter(|n| n.source_item_ability(&a.creature,&proposal.ability).is_some()) {
                            let item=crate::creature_source_items::ScheduledCreatureSourceItems::prepare(runtime,fence,stamp,native,&a.creature,&proposal);
                            result.source_items.push((proposal.ability.clone(),item));
                            continue;
                        }
                        if proposal.list != ScheduleList::Attack {
                            if let Some(source)=a.icicle.as_ref().filter(|s|s.ability==proposal.ability&&s.index==proposal.entry_index) {
                                let hit=icicle_owner.execute(runtime,fence,stamp,source,&proposal,melee,
                                    &mut IcicleMapReader{inner:reader,ability:&source.ability},due);
                                if hit.as_ref().is_ok_and(|h|h.cleared_target){a.selected_target=None;}
                                result.defenses.push((proposal.ability,hit.map(DefenseOutcome::Icicle).map_err(DefenseError::Icicle)));
                                continue;
                            }
                            if let Some(source) = a
                                .defense_variants
                                .iter()
                                .find(|s| s.ability == proposal.ability)
                            {
                                let hit = spells.execute_with_native(
                                    runtime, fence, stamp, states, source, &proposal, reader, due,native_appearance_content.as_deref(),
                                );
                                result.defense_variants.push((proposal.ability, hit));
                                continue;
                            }
                            if let Some(source) = a
                                .defense_summons
                                .iter()
                                .find(|s| s.index == proposal.entry_index)
                            {
                                let hit = if child_closure.as_ref().is_some_and(|c| {
                                    c.content == a.content
                                        && c.children.iter().any(|(r, _)| r == source.child())
                                }) {
                                    defense_summon_owner.execute(
                                        runtime,
                                        fence,
                                        stamp,
                                        source,
                                        occurrence,
                                        &proposal,
                                        &mut SummonMapReader(reader),
                                    )
                                } else {
                                    Err(crate::monster_summon::SummonError::InvalidSource)
                                };
                                if let Ok(batch) = &hit {
                                    for child in &batch.children {
                                        if child.newly_created {
                                            new_children.push((
                                                child.child,
                                                source.child().clone(),
                                                due,
                                                stamp,
                                            ));
                                        }
                                    }
                                }
                                result.defense_summons.push((proposal.ability, hit));
                                continue;
                            }
                            let hash = Sha256::new()
                                .chain_update(b"oteryn:native-self-defense:v1")
                                .chain_update(proposal.occurrence.id().as_str().as_bytes())
                                .finalize();
                            let mut id = [0; 16];
                            id.copy_from_slice(&hash[..16]);
                            let id = DecisionOccurrenceId::from_bytes(id);
                            if let Some(source) = a
                                .threshold_heals
                                .iter()
                                .find(|s| s.ability() == &proposal.ability)
                            {
                                let hit = threshold_heal_owner
                                    .cast(runtime, fence, stamp, source, &proposal, due.get())
                                    .map(DefenseOutcome::ThresholdHeal)
                                    .map_err(DefenseError::ThresholdHeal);
                                result.defenses.push((proposal.ability, hit));
                            } else if let Some(source) = a.heals.iter().find(|s| {
                                s.index == proposal.entry_index && s.ability == proposal.ability
                            }) {
                                let (minimum, maximum) = source.source.magnitude_range();
                                let hit = deterministic_decision_u64(
                                    &root,
                                    id,
                                    "self_heal_magnitude",
                                    proposal.entry_index as u64,
                                )
                                .map_err(|_| DefenseError::InvalidDraw)
                                .and_then(|draw| {
                                    let draw = crate::spell::uniform_draw(
                                        draw,
                                        minimum as i64,
                                        maximum as i64,
                                    );
                                    (if source.source.removes_paralysis() {
                                        states.commit_self_heal_and_paralysis_removal(
                                            runtime,
                                            heals,
                                            &source.registration,
                                            a.actor,
                                            occurrence.sequence,
                                            draw,
                                            due.get(), fence, stamp,
                                        )
                                    } else {
                                        runtime.commit_creature_self_heal(
                                            heals,
                                            &source.registration,
                                            a.actor,
                                            occurrence.sequence,
                                            draw,
                                        )
                                    })
                                    .map(DefenseOutcome::SelfHeal)
                                    .map_err(DefenseError::Carrier)
                                });
                                result.defenses.push((proposal.ability, hit));
                            } else if let Some((_, source)) = a
                                .area_heals
                                .iter()
                                .find(|(i, _)| *i == proposal.entry_index)
                            {
                                let hit = match area_catalog.as_ref() {
                                    Some(catalog) => {
                                        let mut policy = AreaHealMapReader {
                                            inner: reader,
                                            ability: &proposal.ability,
                                        };
                                        area_owner
                                            .execute(
                                                runtime,
                                                fence,
                                                stamp,
                                                catalog,
                                                source,
                                                &proposal,
                                                &mut policy,
                                                states,
                                                due.get(),
                                            )
                                            .map(DefenseOutcome::AreaHeal)
                                            .map_err(DefenseError::AreaHeal)
                                    }
                                    None => {
                                        Err(DefenseError::AreaHeal(AreaHealError::InvalidSource))
                                    }
                                };
                                result.defenses.push((proposal.ability, hit));
                            } else if let Some((_,source))=a.appearance_defenses.iter().find(|(i,_)| *i==proposal.entry_index) {
                                let hit=spells.execute_with_native(runtime,fence,stamp,states,source,&proposal,reader,due,native_appearance_content.as_deref());
                                result.defense_variants.push((proposal.ability,hit));
                            } else if let Some((_, source)) =
                                a.invisible.iter().find(|(i, _)| *i == proposal.entry_index)
                            {
                                let config = reader.self_condition_facts(runtime, a.actor, stamp);
                                let facts = config.map(|c| ApplicationFacts {
                                    now: due.get(),
                                    base_speed: c.base_speed,
                                    mana_shield_capacity: c.mana_shield_capacity,
                                    target_reentry_protected: c.target_reentry_protected,
                                    source_reentry_protected: c.source_reentry_protected,
                                    target_is_player: false,
                                    decision_root: &root,
                                    occurrence: id,
                                });
                                let hit = match facts {
                                    Some(facts) => commit_native_defense_condition(
                                        runtime, states, fence, stamp, a.actor,
                                        proposal.entry_index, occurrence.sequence,
                                        NativeSelfConditionSource::Invisible(source), facts,
                                        &mut a.condition_casts, reader,
                                    ).map(DefenseOutcome::Invisible),
                                    None => Err(DefenseError::Invisible(InvisibilityError::MissingApplicationFacts)),
                                };
                                result.defenses.push((proposal.ability, hit));
                            } else if let Some((_, source)) =
                                a.speed.iter().find(|(i, _)| *i == proposal.entry_index)
                            {
                                let config = reader.self_condition_facts(runtime, a.actor, stamp);
                                let facts = config.map(|c| ApplicationFacts {
                                    now: due.get(),
                                    base_speed: source.base_speed(),
                                    mana_shield_capacity: c.mana_shield_capacity,
                                    target_reentry_protected: c.target_reentry_protected,
                                    source_reentry_protected: c.source_reentry_protected,
                                    target_is_player: false,
                                    decision_root: &root,
                                    occurrence: id,
                                });
                                let hit = match facts {
                                    Some(facts) => commit_native_defense_condition(
                                        runtime, states, fence, stamp, a.actor,
                                        proposal.entry_index, occurrence.sequence,
                                        NativeSelfConditionSource::Speed(source), facts,
                                        &mut a.condition_casts, reader,
                                    ).map(DefenseOutcome::Speed),
                                    None => Err(DefenseError::Speed(InvisibilityError::MissingApplicationFacts)),
                                };
                                result.defenses.push((proposal.ability, hit));
                            } else if let Some((_, source)) = a
                                .presentations
                                .iter()
                                .find(|(i, _)| *i == proposal.entry_index)
                            {
                                let event = source
                                    .produce(
                                        runtime,
                                        fence,
                                        stamp,
                                        &proposal,
                                        &mut crate::creature_damage_spell::SpellPresentationReader(reader),
                                    )
                                    .map(DefenseOutcome::Presentation)
                                    .map_err(DefenseError::Presentation);
                                result.defenses.push((proposal.ability, event));
                            } else {
                                result.deferred_defence.push(proposal.ability)
                            }
                            continue;
                        }
                        if let Some(source)=a.callback_casts.iter().find(|s|s.parent()==&proposal.ability){
                            let hit=callback_casts.schedule(runtime,fence,stamp,source,&proposal,due);
                            result.callback_casts.push((proposal.ability,hit));continue;
                        }
                        if let Some(source) = a.smelly_source.as_ref().filter(|s| s.ability() == &proposal.ability) {
                            let hit = smelly.schedule(runtime, fence, stamp, source, &proposal, due);
                            result.smelly_scheduled.push((proposal.ability, hit));
                            continue;
                        }
                        if let Some((_, source)) =
                            a.composed.iter().find(|(i, _)| *i == proposal.entry_index)
                        {
                            let hit = match area_catalog.as_ref() {
                                Some(catalog) => composed.execute(
                                    runtime,
                                    fence,
                                    stamp,
                                    states,
                                    catalog,
                                    source,
                                    &proposal,
                                    &mut ComposedReader(reader),
                                    due,
                                ),
                                None => Err(AttackError::InvalidSource),
                            };
                            result.composed.push((proposal.ability, hit));
                            continue;
                        }
                        if let Some((_, source)) = a.spells.iter().find(|(i, source)| {
                            *i == proposal.entry_index && source.chain_details().is_some()
                        }) {
                            let shared = std::cell::RefCell::new(&mut *reader);
                            let mut world = ChainSpellReader(&shared);
                            let mut map = ChainMapReader(&shared);
                            let hit = chains.schedule(
                                runtime, fence, stamp, source, &proposal, &mut world, &mut map, due,
                            );
                            result.chains.push((proposal.ability, hit));
                            continue;
                        }
                        let hit = match a.spells.iter().find(|(i, _)| *i == proposal.entry_index) {
                            Some((_, source)) => spells.execute_with_native(
                                runtime, fence, stamp, states, source, &proposal, reader, due,native_appearance_content.as_deref(),
                            ),
                            None => Err(AttackError::UnsupportedShape),
                        };
                        result.spells.push((proposal.ability, hit));
                    }
                    result
                },
            )
            .map_err(CombatLaneError::Cycle)?;
        let mut child_registration =
            self.register_pending_children(runtime, current, new_children, child_closure.as_ref());
        self.actors
            .retain(|a| runtime.contains_live_creature(a.actor));
        let return_ordinal = current
            .accept_input(runtime.binding().scope_generation())
            .map_err(|e| CombatLaneError::Cycle(CycleError::Ordinal(e)))?;
        let return_stamp = current.stamp(return_ordinal);
        let due_returns = self
            .time_guardian_owner
            .due_returns(runtime, current, clock)
            .map_err(CombatLaneError::Attack)?;
        let mut time_returns = Vec::new();
        time_returns
            .try_reserve(due_returns.len())
            .map_err(|_| CombatLaneError::Attack(AttackError::LedgerFull))?;
        for due in due_returns {
            let result = (|| {
                let prepared = due?;
                let actor =
                    crate::source_encounter_seven::TimeGuardianOwner::returned_actor(&prepared);
                let native = self
                    .actors
                    .iter_mut()
                    .find(|a| a.actor == actor)
                    .ok_or(AttackError::StaleIssuer)?;
                native.dispatch_time_return(
                    runtime,
                    current,
                    return_stamp,
                    &mut self.time_guardian_owner,
                    prepared,
                )
            })();
            time_returns.push(result);
        }
        let sources = self
            .actors
            .iter()
            .flat_map(|a| a.melee.iter().cloned().map(move |s| (a.actor, s)))
            .collect::<Vec<_>>();
        let generation = runtime.binding().scope_generation();
        let ordinal = current
            .accept_input(generation)
            .map_err(|e| CombatLaneError::Cycle(CycleError::Ordinal(e)))?;
        let stamp = current.stamp(ordinal);
        runtime
            .retire_native_summons(current, stamp)
            .map_err(|_| CombatLaneError::Attack(AttackError::InvalidPlan))?;
        let hits = self
            .melee
            .dispatch_due(
                runtime, current, stamp, states, &sources, reader, revisions, now,
            )
            .map_err(CombatLaneError::Attack)?;
        // Independent 50ms source work runs even when no new think is due. The queue drains
        // sealed one-use pulses; every hop rereads current map/Combat/session/fence facts.
        let callback_spawn_pulses = match self.callback_spawn_owner.as_mut() {
            Some(owner) => owner.drain(
                runtime,
                current,
                stamp,
                &mut CallbackSpawnMapReader { inner: reader },
                clock,
            ),
            None => Ok(Vec::new()),
        };
        if let Ok(batches) = &callback_spawn_pulses {
            child_registration.extend(self.register_callback_spawn_children(
                runtime,
                current,
                stamp,
                batches,
                child_closure.as_ref(),
                now,
            ));
        }
        let callback_pulses = match self.callback_casts.as_mut() {
            Some(owner) => {
                owner.drain(runtime, current, stamp, states, &self.spells, reader, clock)
            }
            None => Ok(Vec::new()),
        };
        let callback_removals = self
            .callback_casts
            .as_mut()
            .map(|owner| owner.take_source_removals())
            .unwrap_or_default();
        // Missing independently current player census retains bounded source warnings.
        let callback_speech_delivery = reader.current_players(runtime, stamp).map(|listeners| {
            let mut lines = self
                .callback_casts
                .as_mut()
                .map(|o| o.take_source_speech())
                .unwrap_or_default();
            if let Some(owner) = self.callback_spawn_owner.as_mut() {
                lines.extend(owner.take_source_speech());
            }
            let mut delivered = 0;
            for line in lines {
                delivered += self
                    .callback_speech
                    .publish_callback(runtime, &line, &listeners)?;
            }
            Ok(delivered)
        });
        let callback_speech_errors = self
            .callback_spawn_owner
            .as_mut()
            .map(|o| o.take_speech_errors())
            .unwrap_or_default();
        let smelly_pulses = self
            .smelly
            .drain(runtime, current, stamp, states, reader, clock);
        let windup_casts = self
            .spells
            .run_windup_due(runtime, current, states, reader, clock);
        let chain_steps = {
            let shared = std::cell::RefCell::new(&mut *reader);
            let mut world = ChainSpellReader(&shared);
            let mut map = ChainMapReader(&shared);
            self.chains.run_due(
                runtime,
                current,
                states,
                &self.spells,
                &mut world,
                &mut map,
                clock,
            )
        };
        // One target-owned pass per current player, independent of any creature/think roster.
        // Retained caster provenance may be dead; actual condition owner handles that distinction.
        let players = reader.current_players(runtime, stamp);
        let conditions = match players {
            Some(players) if players.len() <= 256 => {
                let mut keys = std::collections::BTreeSet::new();
                if players
                    .iter()
                    .any(|(a, _)| !keys.insert(a.placement_identity()))
                {
                    Err(AttackError::InvalidPlan)
                } else {
                    Ok(players
                        .into_iter()
                        .map(|(actor, session)| {
                            let facts = reader
                                .source_player_condition_tick_facts(runtime, actor, session, stamp);
                            let result = facts.and_then(|facts| {
                                let hash = Sha256::new()
                                    .chain_update(b"oteryn:aggregate-native-condition-tick:v1")
                                    .chain_update(
                                        format!("{:?}:{:?}:{}", actor, session, now.get())
                                            .as_bytes(),
                                    )
                                    .finalize();
                                let mut id = [0; 16];
                                id.copy_from_slice(&hash[..16]);
                                states.tick_source_player_conditions(
                                    runtime,
                                    actor,
                                    session,
                                    DecisionOccurrenceId::from_bytes(id),
                                    facts,
                                    now,
                                )
                            });
                            (actor, result)
                        })
                        .collect())
                }
            }
            _ => Err(AttackError::MissingCombatFacts),
        };
        // Every loaded physical source actor has one condition-owner pass,
        // independently of Think/attack schedules. Missing spatial facts refuse.
        let creature_conditions = self
            .actors
            .iter()
            .map(|a| {
                let result = if a.content != runtime.content_pin().server_artifact_digest() {
                    Err(CarrierError::ContentPinWorldMismatch)
                } else if let Some(facts) =
                    reader.source_creature_condition_tick_facts(runtime, a.actor, stamp)
                {
                    states.tick_source_creature_conditions(runtime, a.actor, facts, now)
                } else {
                    Err(CarrierError::PlanConflict)
                };
                (a.actor, result)
            })
            .collect();
        // One current owner pass independent of Think interval. Native speed conditions may
        // expire here without a scheduled defense; this reads only the canonical slot store.
        let movements = self.run_regular_creature_movement(runtime, current, stamp, reader, now);
        self.last_time = Some(now);
        Ok(CombatPulse {
            callback_speech_delivery,
            callback_speech_errors,
            time_returns,
            callback_removals,
            callback_spawn_pulses,
            creature_conditions,
            callback_pulses,
            smelly_pulses,
            movements,
            child_registration,
            thinks,
            melee: hits,
            chain_steps,
            windup_casts,
            conditions,
        })
    }
}

// Additive MonsterCombatLane API; one retained owner under existing runtime->states lock.
impl MonsterCombatLane {
    pub fn bind_crystal_death_owner(
        &mut self,
        project: &crate::content::WorldProject,
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Result<(), crate::crystal_death_composition::CrystalDeathCompositionError> {
        if self.crystal_deaths.is_some() {
            return Err(
                crate::crystal_death_composition::CrystalDeathCompositionError::Source(
                    "Crystal death owner already bound",
                ),
            );
        }
        self.crystal_deaths = Some(crate::crystal_death_composition::CrystalDeathOwner::bind(
            project, runtime, native,
        )?);
        Ok(())
    }
    pub fn project_crystal_creature_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
    ) -> Result<
        crate::crystal_death_composition::CrystalProjectedDeath,
        crate::crystal_death_composition::CrystalDeathCompositionError,
    > {
        let owner = self.crystal_deaths.as_mut().ok_or(
            crate::crystal_death_composition::CrystalDeathCompositionError::Source(
                "actual project Crystal death owner not bound",
            ),
        )?;
        owner.project_and_dispatch(runtime, current, stamp, actor, listeners, server_rum_draw)
    }
    pub fn drain_source_callback_speech(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<Vec<u8>>, crate::weak_spot_speech::WeakSpotSpeechError> {
        self.callback_speech.drain(runtime, actor, session)
    }
    pub fn clear_source_callback_speech_session(&mut self, session: GameSessionId) {
        self.callback_speech.clear_session(session)
    }
    pub fn drain_crystal_speech(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<Vec<u8>>, crate::weak_spot_speech::WeakSpotSpeechError> {
        match self.crystal_deaths.as_mut() {
            Some(owner) => owner.drain_speech(runtime, actor, session),
            None => Ok(Vec::new()),
        }
    }
    pub fn clear_crystal_speech_session(&mut self, session: GameSessionId) {
        if let Some(owner) = self.crystal_deaths.as_mut() {
            owner.clear_session(session)
        }
    }
}

// Immutable union of exact source child execution dependencies, retained once per Channel lane.
// No Draft clone, alternate factory or artifact-digest membership inference.
pub struct NativeChildClosure {
    callback_children: Vec<(
        Ref,
        Vec<crate::source_callback_cast::SourceCallbackCast>,
        Vec<crate::source_callback_spawn::SourceCallbackSpawn>,
    )>,
    records: Vec<ProjectReferenceRecord>,
    profiles: Vec<ProjectV2AuthoringProfile>,
    pub children: Vec<(Ref, Vec<NativeHeal>)>,
    content: [u8; 32],
}
impl NativeChildClosure {
    fn qualify(
        runtime: &ChannelRuntimeV1,
        draft: &ProjectV2Draft,
        content: [u8; 32],
    ) -> Result<Self, CombatLaneError> {
        if content != runtime.content_pin().server_artifact_digest() {
            return Err(CombatLaneError::Attack(AttackError::ContentChanged));
        }
        let mut children = draft
            .state
            .authoring_profiles
            .iter()
            .filter_map(|p| match &p.data {
                Data::Behavior(b) => b.summons.as_ref(),
                _ => None,
            })
            .flat_map(|s| s.entries.iter().map(|e| e.creature.clone()))
            .collect::<std::collections::BTreeSet<_>>();
        for p in &draft.state.authoring_profiles {
            if let Data::Behavior(b) = &p.data {
                for entry in &b.defenses {
                    if let Some(Data::Ability(a)) = draft
                        .state
                        .authoring_profiles
                        .iter()
                        .find(|p| p.target == entry.ability)
                        .map(|p| &p.data)
                        && let Some(d) = &a.details
                    {
                        for effect in &d.effects {
                            if let crate::content::ProjectV2AbilityEffect::Inline(e) = effect
                                    && let crate::content::ProjectV2InlineEffectOperation::SummonCreature{creatures,..}=&e.operation{children.extend(creatures.iter().cloned());}
                        }
                    }
                }
            }
        }

        children.extend(
            crate::source_callback_spawn::qualified_native_children(draft)
                .map_err(CombatLaneError::Attack)?,
        );
        let mut needed = children.clone();
        let mut queue = children.iter().cloned().collect::<Vec<_>>();
        let mut core = std::collections::BTreeMap::new();
        for (index, r) in draft.core.records.iter().enumerate() {
            let value = serde_json::to_value(r)
                .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?;
            if let Ok(key) = serde_json::from_value::<Ref>(value["identity"].clone()) {
                core.insert(key, (index, value));
            }
        }
        let mut profiles = std::collections::BTreeMap::new();
        for (index, p) in draft.state.authoring_profiles.iter().enumerate() {
            if profiles.insert(p.target.clone(), index).is_some() {
                return Err(CombatLaneError::Attack(AttackError::InvalidSource));
            }
        }
        fn refs(
            v: &serde_json::Value,
            children: &std::collections::BTreeSet<Ref>,
            out: &mut Vec<Ref>,
        ) {
            match v {
                serde_json::Value::Object(o) => {
                    if o.len() == 3
                        && o.contains_key("family")
                        && o.contains_key("key")
                        && o.contains_key("revision")
                        && let Ok(r) = serde_json::from_value::<Ref>(v.clone())
                        && (matches!(
                            r.family,
                            ProjectV2Family::Ability
                                | ProjectV2Family::Effect
                                | ProjectV2Family::Formula
                                | ProjectV2Family::Behavior
                                | ProjectV2Family::Presentation
                        ) || (r.family == ProjectV2Family::Creature && children.contains(&r)))
                    {
                        out.push(r)
                    }
                    for value in o.values() {
                        refs(value, children, out)
                    }
                }
                serde_json::Value::Array(a) => {
                    for value in a {
                        refs(value, children, out)
                    }
                }
                _ => {}
            }
        }
        let mut cursor = 0;
        while cursor < queue.len() {
            if queue.len() > 10000 {
                return Err(CombatLaneError::Attack(AttackError::LedgerFull));
            }
            let key = queue[cursor].clone();
            cursor += 1;
            let mut next = Vec::new();
            if let Some((_, value)) = core.get(&key) {
                refs(value, &children, &mut next)
            }
            if let Some(index) = profiles.get(&key) {
                let value = serde_json::to_value(&draft.state.authoring_profiles[*index])
                    .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?;
                refs(&value, &children, &mut next)
            }
            for r in next {
                if needed.insert(r.clone()) {
                    queue.push(r)
                }
            }
        }
        let records = core
            .iter()
            .filter(|(key, _)| needed.contains(*key))
            .map(|(_, (index, _))| draft.core.records[*index].clone())
            .collect();
        let selected_profiles = profiles
            .iter()
            .filter(|(key, _)| needed.contains(*key))
            .map(|(_, index)| draft.state.authoring_profiles[*index].clone())
            .collect();
        let mut qualified = Vec::new();
        let mut callback_children = Vec::new();
        for creature in children {
            if !draft.state.source_identity_bindings.iter().any(|b| {
                b.target == creature
                    && b.disposition == crate::content::ProjectV2SourceIdentityDisposition::Exact
                    && draft
                        .state
                        .sources
                        .iter()
                        .any(|s| s.key == b.source_key && s.revision == b.source_revision)
            }) {
                continue;
            }
            let Some(behavior) = draft.core.records.iter().find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == creature.key && identity.revision == creature.revision => {
                    Some(behavior)
                }
                _ => None,
            }) else {
                continue;
            };
            let Some(Data::Behavior(b)) = draft
                .state
                .authoring_profiles
                .iter()
                .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
                .map(|p| &p.data)
            else {
                continue;
            };
            let mut heals = Vec::new();
            for (index, e) in b.defenses.iter().enumerate() {
                if let Ok(source) =
                    QualifiedCreatureSelfHeal::qualify(draft, &creature.key, &e.ability.key)
                {
                    if source.source_schedule() != (e.interval_ms, e.chance_ppm) {
                        return Err(CombatLaneError::Attack(AttackError::InvalidSource));
                    }
                    let registration = source
                        .register(runtime)
                        .map_err(|_| CombatLaneError::Attack(AttackError::InvalidSource))?;
                    heals.push(NativeHeal {
                        index,
                        ability: e.ability.clone(),
                        source,
                        registration,
                    })
                }
            }
            let mut casts = Vec::new();
            let mut spawns = Vec::new();
            for (list, entries) in [
                (ScheduleList::Attack, &b.attacks),
                (ScheduleList::Defence, &b.defenses),
            ] {
                for index in 0..entries.len() {
                    if list == ScheduleList::Attack
                        && let Some(source) =
                            crate::source_callback_cast::SourceCallbackCast::qualify(
                                draft, &creature, index, content,
                            )
                            .map_err(CombatLaneError::Attack)?
                    {
                        casts.push(source);
                    }
                    if let Some(source) =
                        crate::source_callback_spawn::SourceCallbackSpawn::qualify(
                            draft, &creature, index, list, content,
                        )
                        .map_err(CombatLaneError::Attack)?
                    {
                        spawns.push(source);
                    }
                }
            }
            callback_children.push((creature.clone(), casts, spawns));
            qualified.push((creature, heals));
        }
        Ok(Self {
            records,
            profiles: selected_profiles,
            callback_children,
            children: qualified,
            content,
        })
    }
}

fn commit_summon_follow(
    runtime: &mut ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    child: ExactActorRef,
    parent: ExactActorRef,
    facts: SummonFollowFacts<'_>,
) -> Result<Option<crate::foundation::NativeSummonMovementOutcome>, SummonFollowError> {
    let b = runtime.binding();
    if !fence.is_current_for_scope(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    ) || !fence.accepts_stamp(stamp)
    {
        return Err(SummonFollowError::StaleOwner);
    }
    if runtime
        .native_summon_role(child)
        .map_err(SummonFollowError::Carrier)?
        != Some(parent)
        || !runtime.contains_live_creature(parent)
        || !runtime.contains_live_creature(child)
    {
        return Err(SummonFollowError::StaleRelation);
    }
    let expected = runtime
        .read_actor_position(child)
        .map_err(SummonFollowError::Carrier)?;
    let at = expected.position();
    let owner = runtime
        .read_actor_position(parent)
        .map_err(SummonFollowError::Carrier)?
        .position();
    let distance = |p: crate::foundation::MovementLocalPosition| {
        (i64::from(p.x) - i64::from(owner.x))
            .abs()
            .max((i64::from(p.y) - i64::from(owner.y)).abs())
    };
    if at.floor != owner.floor {
        return Err(SummonFollowError::MissingMap);
    }
    if distance(at) <= 2 {
        return Ok(None);
    }
    if facts.tile_admits != Some(true) {
        return Err(SummonFollowError::MissingMap);
    }
    let (dx, dy) = match facts.direction {
        crate::movement::CardinalStep::North => (0, -1),
        crate::movement::CardinalStep::East => (1, 0),
        crate::movement::CardinalStep::South => (0, 1),
        crate::movement::CardinalStep::West => (-1, 0),
    };
    let next = crate::foundation::MovementLocalPosition {
        x: at
            .x
            .checked_add(dx)
            .ok_or(SummonFollowError::WrongDirection)?,
        y: at
            .y
            .checked_add(dy)
            .ok_or(SummonFollowError::WrongDirection)?,
        floor: at.floor,
    };
    if (i64::from(next.x) - i64::from(owner.x)).abs()
        + (i64::from(next.y) - i64::from(owner.y)).abs()
        >= (i64::from(at.x) - i64::from(owner.x)).abs()
            + (i64::from(at.y) - i64::from(owner.y)).abs()
    {
        return Err(SummonFollowError::WrongDirection);
    }
    if !runtime.native_summon_cell_free(next) {
        return Err(SummonFollowError::MissingMap);
    }
    crate::movement::step_native_summon_cardinal(
        runtime,
        fence,
        stamp,
        child,
        expected,
        &facts.selection,
        facts.index,
        facts.direction,
    )
    .map(Some)
    .map_err(SummonFollowError::Movement)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod native_summon_follow_tests {
    use super::*;
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
        EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    #[test]
    fn native_summon_follow_executes_existing_movement_kernel_and_unknown_tile_never_moves() {
        let (mut r, mut f, parent, _, catalog, p) =
            crate::monster_summon::summon_execution_fixture_for_world({
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = 1;
                crate::foundation::WorldId::decode(&id).expect("monster_combat_lane.rs:native_summon_follow_tests:4881: qualified fixture operation must succeed")
            });
        let ordinal = f.accept_input(r.binding().scope_generation()).expect("monster_combat_lane.rs:native_summon_follow_tests:4883: qualified fixture operation must succeed");
        let stamp = f.stamp(ordinal);
        // This explicitly synthetic engineering cell has no Reference activation claim.
        let mut map = FollowSpawnMap;
        let child = crate::monster_summon::NativeSummonOwner::default()
            .execute(&mut r, &f, stamp, &catalog, &p, &mut map)
            .expect("monster_combat_lane.rs:native_summon_follow_tests:4889: qualified fixture operation must succeed")
            .child;
        for _ in 0..5 {
            let before = r.read_actor_position(parent).expect("monster_combat_lane.rs:native_summon_follow_tests:4892: qualified fixture operation must succeed");
            let mut next = before.position();
            next.x += 1;
            r.borrow_movement_position()
                .commit_cardinal(before, next)
                .expect("monster_combat_lane.rs:native_summon_follow_tests:4897: qualified fixture operation must succeed");
        }
        let before = r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_follow_tests:4899: qualified fixture operation must succeed");
        let mut next = before.position();
        next.x += 1;
        let scope = EngineeringStaticCellScope {
            world_id: r.binding().world_id(),
            coordinate_frame: CoordinateFrameRef::new("summon-follow-engineering-frame").expect("monster_combat_lane.rs:native_summon_follow_tests:4904: qualified fixture operation must succeed"),
            map_revision: MapRevisionRef::new("summon-follow-engineering-map").expect("monster_combat_lane.rs:native_summon_follow_tests:4905: qualified fixture operation must succeed"),
            generation_digest: [7; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new(
                    "lock",
                    "summon-follow-engineering-lock",
                )
                .expect("monster_combat_lane.rs:native_summon_follow_tests:4912: qualified fixture operation must succeed"),
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:movement").expect("monster_combat_lane.rs:native_summon_follow_tests:4914: qualified fixture operation must succeed"),
                    ProductionAtom::new("revision", "summon-follow-engineering-r1").expect("monster_combat_lane.rs:native_summon_follow_tests:4915: qualified fixture operation must succeed"),
                    Sha256HexDigest::new(&"a".repeat(64)).expect("monster_combat_lane.rs:native_summon_follow_tests:4916: qualified fixture operation must succeed"),
                )],
            },
        };
        let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell: LogicalCell {
                x: next.x,
                y: next.y,
                z: i32::from(next.floor),
            },
            collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
        }])
        .expect("monster_combat_lane.rs:native_summon_follow_tests:4929: qualified fixture operation must succeed");
        let facts = |tile_admits| SummonFollowFacts {
            direction: crate::movement::CardinalStep::East,
            selection: crate::movement::MovementEngineeringSelection {
                owner_context: before.context(),
                content_scope: &scope,
            },
            index: &index,
            ground: Some(&crate::movement::speed::EngineeringGroundSpeed),
            tile_admits,
        };
        assert!(matches!(
            commit_summon_follow(&mut r, &f, stamp, child, parent, facts(None)),
            Err(SummonFollowError::MissingMap)
        ));
        assert_eq!(r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_follow_tests:4944: qualified fixture operation must succeed"), before);
        let outcome = commit_summon_follow(&mut r, &f, stamp, child, parent, facts(Some(true)))
            .expect("monster_combat_lane.rs:native_summon_follow_tests:4946: qualified fixture operation must succeed")
            .expect("monster_combat_lane.rs:native_summon_follow_tests:4947: qualified fixture operation must succeed");
        let moved = match outcome {
            crate::foundation::NativeSummonMovementOutcome::Moved(moved) => Some(moved),
            _ => None,
        }
        .expect("qualified toward-master step must keep source child alive");
        assert_eq!(moved.position(), next);
        assert_eq!(r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_follow_tests:4952: qualified fixture operation must succeed"), moved);
        f.apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect("monster_combat_lane.rs:native_summon_follow_tests:4953: qualified fixture operation must succeed"))
            .expect("monster_combat_lane.rs:native_summon_follow_tests:4954: qualified fixture operation must succeed");
        assert!(matches!(
            commit_summon_follow(&mut r, &f, stamp, child, parent, facts(Some(true))),
            Err(SummonFollowError::StaleOwner)
        ));
        assert_eq!(r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_follow_tests:4959: qualified fixture operation must succeed"), moved);
    }
    pub(super) struct FollowSpawnMap;
    impl crate::monster_summon::SummonLocationPolicy for FollowSpawnMap {
        fn current_target_reachable(
            &mut self,
            r: &ChannelRuntimeV1,
            p: ExactActorRef,
            t: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.read_actor_position(p).ok()?;
            r.read_actor_position(t).ok()?;
            Some(true)
        }
        fn current_cell_admits(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &Ref,
            _: crate::foundation::MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            Some(true)
        }
    }
}
#[derive(Debug)]
pub struct ChildRegistrationReceipt {
    pub actor: ExactActorRef,
    pub creature: Ref,
    /// None is a verified already-registered exact binding, never a manufactured occurrence.
    pub result: Result<Option<ThinkOccurrence>, CombatLaneError>,
    pub cleanup: Option<Result<(), CarrierError>>,
}
impl MonsterCombatLane {
    fn register_pending_children(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        pending: Vec<(ExactActorRef, Ref, SemanticTimeMicros, RuntimeWorkStamp)>,
        closure: Option<&std::sync::Arc<NativeChildClosure>>,
    ) -> Vec<ChildRegistrationReceipt> {
        let mut receipts = Vec::new();
        for (actor, creature, due, stamp) in pending {
            let b = runtime.binding();
            let current_job = current.is_current_for_scope(
                crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                b.scope_generation(),
            ) && current.accepts_stamp(stamp);
            let result = if !current_job {
                Err(CombatLaneError::Cycle(CycleError::StaleOwner))
            } else if self.actors.iter().any(|a| {
                a.actor == actor
                    && a.creature == creature
                    && closure.is_some_and(|c| a.content == c.content)
                    && runtime.matches_live_creature_identity(actor, creature.key.as_bytes())
            }) {
                Ok(None)
            } else if let Some(c) = closure {
                let registered = self.register_native_definitions(
                    runtime,
                    current,
                    actor,
                    &creature,
                    &c.records,
                    &c.profiles,
                    c.content,
                    due,
                );
                if registered.is_ok()
                    && let Some(a) = self.actors.iter_mut().find(|a| a.actor == actor)
                {
                    a.heals = c
                        .children
                        .iter()
                        .find(|(r, _)| *r == creature)
                        .map(|(_, h)| h.clone())
                        .unwrap_or_default();
                }
                registered.map(Some)
            } else {
                Err(CombatLaneError::Attack(AttackError::InvalidSource))
            };
            let cleanup = if result.is_err() {
                Some(runtime.retire_native_summon_registration_failure(current, stamp, actor))
            } else {
                None
            };
            receipts.push(ChildRegistrationReceipt {
                actor,
                creature,
                result,
                cleanup,
            });
        }
        receipts
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod native_summon_typed_outward_test {
    use super::*;
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
        EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    #[test]
    fn qualified_outward_child_move_returns_typed_removal_not_stale_position() {
        let (mut r, mut f, parent, _, catalog, p) =
            crate::monster_summon::summon_execution_fixture_for_world({
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = 1;
                crate::foundation::WorldId::decode(&id).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5077: qualified fixture operation must succeed")
            });
        let ordinal = f.accept_input(r.binding().scope_generation()).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5079: qualified fixture operation must succeed");
        let stamp = f.stamp(ordinal);
        let child = crate::monster_summon::NativeSummonOwner::default()
            .execute(
                &mut r,
                &f,
                stamp,
                &catalog,
                &p,
                &mut native_summon_follow_tests::FollowSpawnMap,
            )
            .expect("monster_combat_lane.rs:native_summon_typed_outward_test:5090: qualified fixture operation must succeed")
            .child;
        while r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5092: qualified fixture operation must succeed").position().x < 130 {
            let at = r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5093: qualified fixture operation must succeed");
            let mut next = at.position();
            next.x += 1;
            r.borrow_movement_position()
                .commit_cardinal(at, next)
                .expect("monster_combat_lane.rs:native_summon_typed_outward_test:5098: qualified fixture operation must succeed");
        }
        let before = r.read_actor_position(child).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5100: qualified fixture operation must succeed");
        let mut next = before.position();
        next.x += 1;
        let scope = EngineeringStaticCellScope {
            world_id: r.binding().world_id(),
            coordinate_frame: CoordinateFrameRef::new("summon-removal-engineering-frame").expect("monster_combat_lane.rs:native_summon_typed_outward_test:5105: qualified fixture operation must succeed"),
            map_revision: MapRevisionRef::new("summon-removal-engineering-map").expect("monster_combat_lane.rs:native_summon_typed_outward_test:5106: qualified fixture operation must succeed"),
            generation_digest: [7; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new(
                    "lock",
                    "summon-removal-engineering-lock",
                )
                .expect("monster_combat_lane.rs:native_summon_typed_outward_test:5113: qualified fixture operation must succeed"),
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:movement").expect("monster_combat_lane.rs:native_summon_typed_outward_test:5115: qualified fixture operation must succeed"),
                    ProductionAtom::new("revision", "summon-removal-engineering-r1").expect("monster_combat_lane.rs:native_summon_typed_outward_test:5116: qualified fixture operation must succeed"),
                    Sha256HexDigest::new(&"a".repeat(64)).expect("monster_combat_lane.rs:native_summon_typed_outward_test:5117: qualified fixture operation must succeed"),
                )],
            },
        };
        let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell: LogicalCell {
                x: next.x,
                y: next.y,
                z: i32::from(next.floor),
            },
            collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
        }])
        .expect("monster_combat_lane.rs:native_summon_typed_outward_test:5130: qualified fixture operation must succeed");
        let selection = crate::movement::MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let result = crate::movement::step_native_summon_cardinal(
            &mut r,
            &f,
            stamp,
            child,
            before,
            &selection,
            &index,
            crate::movement::CardinalStep::East,
        )
        .expect("monster_combat_lane.rs:native_summon_typed_outward_test:5145: qualified fixture operation must succeed");
        assert_eq!(
            result,
            crate::foundation::NativeSummonMovementOutcome::Removed(child)
        );
        assert_eq!(
            r.native_summon_role(child),
            Err(CarrierError::StaleActorGeneration)
        );
        assert_eq!(
            r.project_native_summon_death(child),
            Err(CarrierError::StaleActorGeneration)
        );
        assert_eq!(r.native_summon_count(parent, None), 0);
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod pending_summon_failure_regression {
    use super::*;
    use crate::content::*;
    #[test]
    #[ignore = "requires retained source project fixture; run explicitly locally"]
    fn two_real_children_first_registration_failure_keeps_later_child_and_receipts() {
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let path = retained_native_capture_path.as_path();
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_400_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 29,
            max_reimport_states: 404,
        };
        let project = capture_world_project(
            path.parent().expect("monster_combat_lane.rs:pending_summon_failure_regression:5187: qualified fixture operation must succeed"),
            path.file_name().expect("monster_combat_lane.rs:pending_summon_failure_regression:5188: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_combat_lane.rs:pending_summon_failure_regression:5193: qualified fixture operation must succeed");
        let draft = project.migrate_to_v2();
        // Explicit bounded physical fixture casts; actual child execution definitions are retained
        // from this captured native source. The deliberately wrong first job uses existing Rat,
        // not an invented Creature definition or altered source data.
        let (mut r, mut f, parent, _, catalog, mut proposal) =
            crate::monster_summon::summon_execution_fixture_for_world(
                project.lower_reference_source().expect("monster_combat_lane.rs:pending_summon_failure_regression:5202: qualified fixture operation must succeed").world_id,
            );
        let ordinal = f.accept_input(r.binding().scope_generation()).expect("monster_combat_lane.rs:pending_summon_failure_regression:5204: qualified fixture operation must succeed");
        let stamp = f.stamp(ordinal);
        let mut map = native_summon_follow_tests::FollowSpawnMap;
        let mut owner = crate::monster_summon::NativeSummonOwner::default();
        let first = owner
            .execute(&mut r, &f, stamp, &catalog, &proposal, &mut map)
            .expect("monster_combat_lane.rs:pending_summon_failure_regression:5209: qualified fixture operation must succeed")
            .child;
        proposal.occurrence.sequence = 1;
        let second = owner
            .execute(&mut r, &f, stamp, &catalog, &proposal, &mut map)
            .expect("monster_combat_lane.rs:pending_summon_failure_regression:5214: qualified fixture operation must succeed")
            .child;
        assert_ne!(first, second);
        let closure =
            std::sync::Arc::new(NativeChildClosure::qualify(&r, &draft, [1; 32]).expect("monster_combat_lane.rs:pending_summon_failure_regression:5219: qualified fixture operation must succeed"));
        let mut lane = MonsterCombatLane::new(&r, &f).expect("monster_combat_lane.rs:pending_summon_failure_regression:5220: qualified fixture operation must succeed");
        let wrong = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.rat".into(),
            revision: "definition-r1".into(),
        };
        let due = SemanticTimeMicros::from_micros(0);
        let receipts = lane.register_pending_children(
            &mut r,
            &mut f,
            vec![
                (first, wrong, due, stamp),
                (second, proposal.creature.clone(), due, stamp),
            ],
            Some(&closure),
        );
        assert_eq!(receipts.len(), 2);
        assert!(receipts[0].result.is_err());
        assert_eq!(receipts[0].cleanup, Some(Ok(())));
        assert!(!r.contains_live_creature(first));
        assert_eq!(
            receipts[1]
                .result
                .as_ref()
                .expect("monster_combat_lane.rs:pending_summon_failure_regression:5243: qualified fixture operation must succeed")
                .as_ref()
                .expect("monster_combat_lane.rs:pending_summon_failure_regression:5245: qualified fixture operation must succeed")
                .sequence,
            0
        );
        assert!(receipts[1].cleanup.is_none());
        assert!(r.contains_live_creature(second));
        assert!(lane.actors.iter().any(|a| a.actor == second));
        assert_eq!(r.native_summon_count(parent, None), 1);
        let replay = lane.register_pending_children(
            &mut r,
            &mut f,
            vec![(second, proposal.creature, due, stamp)],
            Some(&closure),
        );
        assert!(replay[0].result.as_ref().expect("monster_combat_lane.rs:pending_summon_failure_regression:5260: qualified fixture operation must succeed").is_none());
        assert!(replay[0].cleanup.is_none());
        assert!(r.contains_live_creature(second));
        assert_eq!(lane.actors.iter().filter(|a| a.actor == second).count(), 1);
    }
}

struct BoneCombatInstance {
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    state: crate::foundation::BoneOverlordCagePhase,
}
pub struct MonsterDeathWithBone {
    pub projected: crate::crystal_death_composition::CrystalProjectedDeath,
    pub bone:
        Vec<Result<crate::foundation::BonePhaseTransition, crate::foundation::BonePhaseError>>,
}
impl MonsterCombatLane {
    /// Only the native encounter admission caller supplies explicit actor/placement associations.
    /// Same-name global census does not identify an encounter instance or grant membership.
    // Keep admit_bone_shared_encounter ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_bone_shared_encounter(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        project: &crate::content::WorldProject,
        native: &crate::content::native_gameplay::NativeGameplayState,
        map: &crate::content::CanonicalReferencePlayableContent,
        map_fence: &crate::world_runtime::ScopeContentGenerationFence,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        placements: [&crate::content::PlacementKey; 5],
        cages: [ExactActorRef; 4],
        phylactery: ExactActorRef,
    ) -> Result<(), crate::foundation::BonePhaseError> {
        use crate::foundation::{BonePhaseError, CarrierError};
        if self
            .bone_encounters
            .iter()
            .any(|i| i.phylactery == phylactery || i.cages.iter().any(|a| cages.contains(a)))
        {
            return Err(BonePhaseError::WrongParticipant);
        }
        self.bone_encounters.retain(|i| {
            i.cages
                .iter()
                .any(|a| runtime.read_actor_position(*a).is_ok())
                || runtime.read_actor_position(i.phylactery).is_ok()
        });
        if self.bone_encounters.len() >= 16 {
            return Err(BonePhaseError::Carrier(CarrierError::CapacityExceeded));
        }
        self.bone_encounters
            .try_reserve(1)
            .map_err(|_| BonePhaseError::Carrier(CarrierError::AllocationFailed))?;
        let state = crate::bone_phase_registry::register_bone_phase_shared(
            runtime, project, native, map, map_fence, current, stamp, placements, cages, phylactery,
        )?;
        self.bone_encounters.push(BoneCombatInstance {
            cages,
            phylactery,
            state,
        });
        Ok(())
    }
    /// Actual projection/callback turn plus native Bone tracker before actor removal. Callers handle
    /// explicit phase/callback errors; no authored phase counter is substituted for a sealed death.
    pub fn project_monster_death_with_bone(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
    ) -> Result<MonsterDeathWithBone, crate::crystal_death_composition::CrystalDeathCompositionError>
    {
        let mut bone = Vec::new();
        bone.try_reserve(self.bone_encounters.len()).map_err(|_| {
            crate::crystal_death_composition::CrystalDeathCompositionError::Owner(
                crate::foundation::CarrierError::AllocationFailed,
            )
        })?;
        let projected = self.project_crystal_creature_death(
            runtime,
            current,
            stamp,
            actor,
            listeners,
            server_rum_draw,
        )?;
        for i in &mut self.bone_encounters {
            if i.phylactery == actor || i.cages.contains(&actor) {
                bone.push(runtime.commit_bone_overlord_projected_death(&mut i.state, actor));
            }
        }
        Ok(MonsterDeathWithBone { projected, bone })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod bone_map_admission_actual_tests {
    use super::*;
    use crate::content::*;
    use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
    use crate::world_runtime::{ReferenceContentGeneration, ScopeContentGenerationFence};
    use std::collections::BTreeMap;
    #[test]
    #[ignore = "Retained actual native eleven-document source capture; isolated PROJECT positions, not donor map admission"]
    fn actual_source_map_association_admits_native_shared_hp_and_phase() {
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let p = retained_native_capture_path.as_path();
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96000000,
            max_total_bytes: 160000000,
            max_json_depth: 24,
            max_decoded_fields: 2400000,
            max_string_bytes: 43000000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 29,
            max_reimport_states: 404,
        };
        let project = capture_world_project(
            p.parent().expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5388: qualified fixture operation must succeed"),
            p.file_name().expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5389: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5394: qualified fixture operation must succeed");
        let mut map = link_reference_playable(project.lower_reference_source().expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5397: qualified fixture operation must succeed")).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5397: qualified fixture operation must succeed");
        let native =
            crate::content::native_gameplay::retained_callback_native_fixture(map.world_id);
        let mut runtime = crate::foundation::crystal_death_router_fixture_with_artifact(
            map.world_id,
            native.source_digest(),
        );
        let keys = [
            "oteryn:creature.elyrax_s_soulcage",
            "oteryn:creature.myzareth_s_soulcage",
            "oteryn:creature.scarith_s_soulcage",
            "oteryn:creature.zharvorin_s_soulcage",
            "oteryn:creature.bonelord_s_phylactery",
        ];
        let actors: Vec<_> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                runtime.crystal_router_fixture_actor(
                    k,
                    if i < 4 { 120000 } else { 50000 },
                    MovementLocalPosition {
                        x: 100 + i as i32,
                        y: 100,
                        floor: 7,
                    },
                )
            })
            .collect();
        let placement_keys: Vec<_> = (0..5)
            .map(|i| {
                PlacementKey::new(&format!("oteryn:reference.bone_project_placement_{i}")).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5423: qualified fixture operation must succeed")
            })
            .collect();
        let evidence = EvidenceBindingRef::new(
            ProductionAtom::new("fixture evidence", "manifest-r0").expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5427: qualified fixture operation must succeed"),
            ProductionKey::new("oteryn:reference.bone_unpromoted_map").expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5428: qualified fixture operation must succeed"),
            EvidenceDisposition::Unknown,
        );
        for i in 0..5 {
            let definition = map
                .definitions
                .iter()
                .find(|d| {
                    d.definition.family() == DefinitionFamily::Creature
                        && d.definition.key().as_str() == keys[i]
                })
                .expect("actual native catalog member")
                .definition
                .clone();
            let footprint = FootprintRelation::Qualified {
                members: vec![FootprintCell {
                    dx: 0,
                    dy: 0,
                    dz: 0,
                }],
                evidence: evidence.clone(),
            };
            map.placements.push(PlacementRef {
                key: placement_keys[i].clone(),
                map_revision: MapRevisionRef::new("bone-isolated-map-r1").expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5452: qualified fixture operation must succeed"),
                definition,
                address: SpatialAddress {
                    world_id: map.world_id,
                    coordinate_frame: map.coordinate_frame.clone(),
                    cell: LogicalCell {
                        x: 100 + i as i32,
                        y: 100,
                        z: 7,
                    },
                    evidence: evidence.clone(),
                },
                presentation_footprint: footprint.clone(),
                collision_footprint: footprint,
                local_object_initial_state: None,
                local_object_state_attributes: BTreeMap::new(),
                local_object_revert_after_ms: BTreeMap::new(),
                local_object_event_transitions: BTreeMap::new(),
            });
        }
        let b = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        let generation = b.scope_generation();
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(scope, generation).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5475: qualified fixture operation must succeed");
        let map_fence = ScopeContentGenerationFence::for_test(
            scope,
            generation,
            ReferenceContentGeneration::from_content(&map).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5479: qualified fixture operation must succeed"),
        );
        let mut lane = MonsterCombatLane::new(&runtime, &fence).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5481: qualified fixture operation must succeed");
        lane.bind_crystal_death_owner(&project, &runtime, &native).expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5482: qualified fixture operation must succeed");
        let cages = [actors[0], actors[1], actors[2], actors[3]];
        let phyl = actors[4];
        let ps = [
            &placement_keys[0],
            &placement_keys[1],
            &placement_keys[2],
            &placement_keys[3],
            &placement_keys[4],
        ];
        let mut bad = map.clone();
        bad.placements.last_mut().expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5493: qualified fixture operation must succeed").address.cell.x += 1;
        assert!(
            lane.admit_bone_shared_encounter(
                &mut runtime,
                &project,
                &native,
                &bad,
                &map_fence,
                &fence,
                stamp,
                ps,
                cages,
                phyl
            )
            .is_err()
        );
        assert!(lane.bone_encounters.is_empty());
        let mut wrong = ps;
        wrong[1] = wrong[0];
        assert!(
            lane.admit_bone_shared_encounter(
                &mut runtime,
                &project,
                &native,
                &map,
                &map_fence,
                &fence,
                stamp,
                wrong,
                cages,
                phyl
            )
            .is_err()
        );
        assert!(lane.bone_encounters.is_empty());
        lane.admit_bone_shared_encounter(
            &mut runtime,
            &project,
                &native,
            &map,
            &map_fence,
            &fence,
            stamp,
            ps,
            cages,
            phyl,
        )
        .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5535: qualified fixture operation must succeed");
        assert_eq!(lane.bone_encounters.len(), 1);
        assert!(
            lane.admit_bone_shared_encounter(
                &mut runtime,
                &project,
                &native,
                &map,
                &map_fence,
                &fence,
                stamp,
                ps,
                cages,
                phyl
            )
            .is_err()
        );
        assert!(
            runtime
                .bone_shared_fixture_damage(phyl, keys[4], 1, "blocked-before-cages")
                .is_err()
        );
        assert_eq!(runtime.crystal_router_fixture_health(phyl), 50000);
        runtime
            .bone_shared_fixture_damage(cages[0], keys[0], 120000, "same-real-owner-cast")
            .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5560: qualified fixture operation must succeed");
        for a in cages {
            assert_eq!(runtime.crystal_router_fixture_health(a), 0);
        }
        for a in cages {
            let death = lane
                .project_monster_death_with_bone(&mut runtime, &fence, stamp, a, &[], None)
                .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5567: qualified fixture operation must succeed");
            assert_eq!(death.bone.len(), 1);
            assert!(death.bone[0].is_ok());
        }
        runtime
            .bone_shared_fixture_damage(phyl, keys[4], 50000, "unlocked-phylactery")
            .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5573: qualified fixture operation must succeed");
        let death = lane
            .project_monster_death_with_bone(&mut runtime, &fence, stamp, phyl, &[], None)
            .expect("monster_combat_lane.rs:bone_map_admission_actual_tests:5576: qualified fixture operation must succeed");
        assert_eq!(death.bone.len(), 1);
        assert!(death.bone[0].is_ok());
    }
}

struct ComposedReader<'a, R>(&'a mut R);
impl<R: MonsterCombatReader> SpellWorldReader for ComposedReader<'_, R> {
    fn native_gameplay<'a>(
        &'a self,
        r: &ChannelRuntimeV1,
    ) -> Option<&'a crate::content::native_gameplay::NativeGameplayState> {
        self.0.native_gameplay(r)
    }
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        s: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        self.0.current_players(r, s)
    }
    fn current_facing(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<crate::creature_attack_geometry::Facing> {
        self.0.current_facing(r, a, s)
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        x: i32,
        y: i32,
        z: i16,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.tile_allowed(r, a, x, y, z, s)
    }
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        t: ExactActorRef,
        g: GameSessionId,
        e: Option<&str>,
        s: RuntimeWorkStamp,
    ) -> Option<crate::creature_damage_spell::SpellCombatFacts> {
        self.0.combat(r, a, t, g, e, s)
    }
}
impl<R: MonsterCombatReader> ComposedWorldReader for ComposedReader<'_, R> {
    fn creature_combat(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        t: ExactActorRef,
        e: Option<&str>,
        s: RuntimeWorkStamp,
    ) -> Option<CreatureCombatFacts> {
        self.0.composed_creature_combat(r, a, t, e, s)
    }
    fn callback_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        t: ExactActorRef,
        b: &Ref,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.composed_callback_allowed(r, a, t, b, s)
    }
    fn non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.area_heal_non_player_side(r, t, s)
    }
    fn top_creature(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.0.area_heal_top_creature(r, t, s)
    }
}

// Source-specific canonical condition publication for the two qualified self-defense forms.
// Retains native immutable plans for replay; never introduces another condition store.
struct NativeDefenseConditionMemo {
    index: usize,
    sequence: u64,
    binding: [u8; 32],
    decision_root: GameplayDecisionRoot,
    application: native_conditions::NativeConditionApplication,
}
enum NativeSelfConditionSource<'a> {
    #[allow(
        dead_code,
        reason = "retained source-qualified metadata; shipping activation remains separate"
    )]
    Appearance(&'a [crate::foundation::ConditionDefinition]),
    Invisible(&'a SelfInvisibleSource),
    Speed(&'a SelfSpeedSource),
}
struct NativeSelfTilePolicy<'a, T>(&'a mut T);
impl<T: InvisibleTileCombatPolicy> native_conditions::NativeConditionAdmission
    for NativeSelfTilePolicy<'_, T>
{
    fn current_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        source: crate::foundation::ConditionSource,
    ) -> Option<bool> {
        if source.actor != target || source.session.is_some() {
            return Some(false);
        }
        self.0.current_tile_allowed(runtime, target)
    }
}
// Keep commit_native_defense_condition ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
fn commit_native_defense_condition(
    runtime: &mut ChannelRuntimeV1,
    states: &impl MonsterCombatStates,
    current: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    actor: ExactActorRef,
    index: usize,
    sequence: u64,
    source: NativeSelfConditionSource<'_>,
    facts: ApplicationFacts<'_>,
    memos: &mut Vec<NativeDefenseConditionMemo>,
    reader: &mut impl InvisibleTileCombatPolicy,
) -> Result<bool, DefenseError> {
    use native_conditions::NativeConditionError;
    let b = runtime.binding();
    if !current.is_current_for_scope(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    ) || !current.accepts_stamp(stamp)
    {
        return Err(DefenseError::NativeCondition(
            NativeConditionError::StaleOwner,
        ));
    }
    if !runtime.contains_live_creature(actor) {
        return Err(DefenseError::NativeCondition(
            NativeConditionError::StaleActor,
        ));
    }
    // Explicit public application facts only. The opaque root stays opaque and is
    // retained by exact value for replay, so changing RNG provenance cannot replay.
    let binding: [u8; 32] = Sha256::digest(
        format!(
            "{:?}",
            (
                index,
                facts.now,
                facts.base_speed,
                facts.mana_shield_capacity,
                facts.target_reentry_protected,
                facts.source_reentry_protected,
                facts.target_is_player,
                facts.occurrence
            )
        )
        .as_bytes(),
    )
    .into();
    let decision_root = facts.decision_root.clone();
    let old = memos.iter().position(|m| m.index == index);
    if let Some(i) = old {
        let memo = &memos[i];
        if sequence < memo.sequence {
            return Err(DefenseError::Invisible(InvisibilityError::ReplayConflict));
        }
        if sequence == memo.sequence {
            if binding != memo.binding || decision_root != memo.decision_root {
                return Err(DefenseError::Invisible(InvisibilityError::ReplayConflict));
            }
            let _retained_original = memo.application.native_plan();
            return Ok(false);
        }
    }
    if old.is_none() {
        if memos.len() >= 8 {
            return Err(DefenseError::Invisible(InvisibilityError::AllocationFailed));
        }
        memos
            .try_reserve(1)
            .map_err(|_| DefenseError::Invisible(InvisibilityError::AllocationFailed))?;
    }
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(facts.now);
    let mut policy = NativeSelfTilePolicy(reader);
    let application = match source {
        NativeSelfConditionSource::Appearance(definitions) => states.prepare_native_conditions(
            runtime,
            current,
            stamp,
            actor,
            None,
            crate::foundation::ConditionSource {
                actor,
                session: None,
                kind: crate::foundation::ConditionSourceKind::Creature,
            },
            definitions,
            &[],
            facts,
            &mut policy,
        ),
        NativeSelfConditionSource::Invisible(source) => states.prepare_native_self_invisible(
            runtime,
            current,
            stamp,
            actor,
            source,
            facts,
            &mut policy,
        ),
        NativeSelfConditionSource::Speed(source) => states.prepare_native_self_speed(
            runtime,
            current,
            stamp,
            actor,
            source,
            facts,
            &mut policy,
        ),
    }
    .map_err(DefenseError::NativeCondition)?;
    let applied = states
        .commit_native_conditions(runtime, current, stamp, &application, &mut policy, now)
        .map_err(DefenseError::NativeCondition)?;
    let memo = NativeDefenseConditionMemo {
        index,
        sequence,
        binding,
        decision_root,
        application,
    };
    if let Some(i) = old {
        memos[i] = memo
    } else {
        memos.push(memo)
    }
    Ok(applied)
}

/// Cadence preflight uses the selected native source descriptor and canonical physical slot
/// conditions. The existing Movement compare/commit remains the only position writer.
// Keep commit_paced_summon_follow ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
fn commit_paced_summon_follow(
    runtime: &mut ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    child: ExactActorRef,
    parent: ExactActorRef,
    facts: SummonFollowFacts<'_>,
    source: Option<&crate::movement::speed::NativeCreatureSpeed>,
    pacer: &mut crate::movement::pacing::CreatureStepPacer,
    now: SemanticTimeMicros,
) -> Result<Option<crate::foundation::NativeSummonMovementOutcome>, SummonFollowError> {
    let b = runtime.binding();
    if !fence.is_current_for_scope(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    ) || !fence.accepts_stamp(stamp)
    {
        return Err(SummonFollowError::StaleOwner);
    }
    if runtime
        .native_summon_role(child)
        .map_err(SummonFollowError::Carrier)?
        != Some(parent)
        || !runtime.contains_live_creature(parent)
    {
        return Err(SummonFollowError::StaleRelation);
    }
    let source = source.ok_or(SummonFollowError::MissingSpeed)?;
    let ground = facts.ground.ok_or(SummonFollowError::MissingMap)?;
    let at = runtime
        .read_actor_position(child)
        .map_err(SummonFollowError::Carrier)?
        .position();
    let (dx, dy) = match facts.direction {
        crate::movement::CardinalStep::North => (0, -1),
        crate::movement::CardinalStep::East => (1, 0),
        crate::movement::CardinalStep::South => (0, 1),
        crate::movement::CardinalStep::West => (-1, 0),
    };
    let onto = crate::content::LogicalCell {
        x: at
            .x
            .checked_add(dx)
            .ok_or(SummonFollowError::WrongDirection)?,
        y: at
            .y
            .checked_add(dy)
            .ok_or(SummonFollowError::WrongDirection)?,
        z: i32::from(at.floor),
    };
    let native_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get());
    let duration = crate::movement::speed::creature_step_duration(
        runtime, child, source, ground, onto, native_now,
    )
    .ok_or(SummonFollowError::MissingSpeed)?;
    if pacer.wait_until(native_now).is_some() {
        return Ok(None);
    }
    // All fallible timing work occurs before Movement. A blocked/removed child never advances it.
    pacer
        .prepare_moved_deadline(native_now, duration)
        .ok_or(SummonFollowError::ClockOverflow)?;
    let outcome = commit_summon_follow(runtime, fence, stamp, child, parent, facts)?;
    if matches!(
        &outcome,
        Some(crate::foundation::NativeSummonMovementOutcome::Moved(_))
    ) {
        pacer.record(native_now, Some(duration));
    }
    Ok(outcome)
}

impl MonsterCombatLane {
    fn run_regular_creature_movement<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        reader: &mut R,
        now: SemanticTimeMicros,
    ) -> Vec<(
        ExactActorRef,
        Result<
            Option<crate::foundation::MovementPositionSnapshot>,
            crate::movement::speed::CreatureCadenceError,
        >,
    )> {
        use crate::movement::speed::CreatureCadenceError as E;
        self.actors
            .iter_mut()
            .filter_map(|actor| {
                if !actor.behavior.movement.can_walk || !runtime.contains_live_creature(actor.actor)
                {
                    return None;
                }
                // Native summon owner already owns typed, removal-aware distance cleanup.
                match crate::monster_summon::current_source_creature_role(runtime, actor.actor) {
                    Ok(crate::monster_summon::CurrentSourceCreatureRole::Ordinary) => {}
                    Ok(_) => return None,
                    Err(error) => return Some((actor.actor, Err(E::Carrier(error)))),
                }
                let outcome = match reader.regular_creature_movement(
                    runtime,
                    actor.actor,
                    actor.selected_target,
                    &actor.creature,
                    stamp,
                ) {
                    Some(facts) if facts.tile_admits == Some(true) => {
                        match (
                            actor.movement_cadence.as_mut(),
                            actor.native_speed.as_ref(),
                            facts.ground,
                        ) {
                            (Some(cadence), Some(source), Some(ground)) => cadence.step(
                                runtime,
                                current,
                                stamp,
                                source,
                                ground,
                                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                                    now.get(),
                                ),
                                facts.scope,
                                facts.index,
                                facts.blocking,
                                facts.direction,
                            ),
                            (_, _, None) => Err(E::MissingMap),
                            _ => Err(E::MissingSpeed),
                        }
                    }
                    _ => Err(E::MissingMap),
                };
                Some((actor.actor, outcome))
            })
            .collect()
    }
}

struct IcicleMapReader<'a, R> {
    inner: &'a mut R,
    ability: &'a Ref,
}
impl<R: MonsterCombatReader> crate::creature_icicle::IcicleWorldReader for IcicleMapReader<'_, R> {
    fn icicle_callback_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        caster: ExactActorRef,
        target: ExactActorRef,
        _session: Option<GameSessionId>,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        let p = r.read_actor_position(target).ok()?;
        if p.context() != r.pinned_movement_context() {
            return None;
        }
        let at = p.position();
        if !self
            .inner
            .tile_allowed(r, caster, at.x, at.y, at.floor, stamp)?
        {
            return Some(false);
        }
        self.inner
            .area_heal_target_allowed(r, caster, target, self.ability, stamp)
    }
}
struct CallbackSpawnMapReader<'a, T: MonsterCombatReader> {
    inner: &'a mut T,
}
impl<T: MonsterCombatReader> crate::source_callback_spawn::SourceSpawnFacts
    for CallbackSpawnMapReader<'_, T>
{
    fn source_spawn_position(
        &mut self,
        r: &ChannelRuntimeV1,
        p: ExactActorRef,
        l: &crate::source_callback_spawn::SourceSpawnPlacement,
        d: u64,
        o: usize,
        s: RuntimeWorkStamp,
    ) -> Option<crate::foundation::MovementLocalPosition> {
        self.inner.source_callback_spawn_position(r, p, l, d, o, s)
    }
    fn source_spawn_tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        p: ExactActorRef,
        l: crate::foundation::MovementLocalPosition,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.source_callback_spawn_tile_allowed(r, p, l, s)
    }
    fn source_spawn_anchor_contains(
        &mut self,
        r: &ChannelRuntimeV1,
        p: ExactActorRef,
        l: &serde_json::Value,
        s: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.inner.source_callback_spawn_anchor_contains(r, p, l, s)
    }
}
impl MonsterCombatLane {
    fn register_callback_spawn_children(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        batches: &[Result<crate::source_callback_spawn::CallbackSpawnReceipt, AttackError>],
        closure: Option<&std::sync::Arc<NativeChildClosure>>,
        now: SemanticTimeMicros,
    ) -> Vec<ChildRegistrationReceipt> {
        let mut receipts = Vec::new();
        for batch in batches.iter().filter_map(|b| b.as_ref().ok()) {
            for spawned in &batch.children {
                let Ok(actor) = spawned.result else { continue };
                let creature = &spawned.creature;
                let result = if let Some(c) = closure {
                    if self.actors.iter().any(|a| {
                        a.actor == actor
                            && a.creature == *creature
                            && a.content == c.content
                            && r.matches_live_creature_identity(actor, creature.key.as_bytes())
                    }) {
                        Ok(None)
                    } else {
                        let result = self.register_native_definitions(
                            r,
                            f,
                            actor,
                            creature,
                            &c.records,
                            &c.profiles,
                            c.content,
                            now,
                        );
                        if result.is_ok()
                            && let Some(a) = self.actors.iter_mut().find(|a| a.actor == actor)
                        {
                            a.heals = c
                                .children
                                .iter()
                                .find(|(key, _)| key == creature)
                                .map(|(_, h)| h.clone())
                                .unwrap_or_default();
                            if let Some((_, casts, spawns)) = c
                                .callback_children
                                .iter()
                                .find(|(key, _, _)| key == creature)
                            {
                                a.callback_casts = casts.clone();
                                a.callback_spawn_sources = spawns.clone();
                            }
                        }
                        result.map(Some)
                    }
                } else {
                    Err(CombatLaneError::Attack(AttackError::InvalidSource))
                };
                let cleanup = if result.is_err() {
                    Some(r.remove_native_encounter_subject(
                        f,
                        stamp,
                        actor,
                        &creature.key,
                        r.content_pin().server_artifact_digest(),
                    ))
                } else {
                    None
                };
                // Continue every remaining physical child even after one registration fails: retained
                // earlier HP/spawn receipts never disappear behind a generic post-commit error.
                receipts.push(ChildRegistrationReceipt {
                    actor,
                    creature: creature.clone(),
                    result,
                    cleanup,
                });
            }
        }
        receipts
    }
}

#[cfg(test)]
mod project_native_registration_membership_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    #[ignore = "requires actual staged native manifest and retained eleven-document project"]
    fn current_native_registration_rejects_detached_project_before_mutation() {
        use crate::content::{
            ProjectEvidenceLimits, ProjectFilesystemLimits, capture_world_project,
        };
        let path = std::path::PathBuf::from(
            std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT").unwrap(),
        );
        let project = capture_world_project(
            path.parent().unwrap(),
            path.file_name().unwrap(),
            ProjectFilesystemLimits {
                project: ProjectEvidenceLimits {
                    max_documents: 11,
                    max_document_bytes: 96_000_000,
                    max_total_bytes: 160_000_000,
                    max_json_depth: 24,
                    max_decoded_fields: 2_400_000,
                    max_string_bytes: 43_000_000,
                    max_locator_bytes: 160,
                    max_locator_segments: 8,
                    max_reference_records: 70000,
                    max_import_records: 29,
                    max_reimport_states: 404,
                },
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .unwrap();
        let draft = project.migrate_to_v2();
        let world = project.lower_reference_source().unwrap().world_id;
        let manifest =
            std::path::PathBuf::from(std::env::var_os("OTERYN_FULL_SPELL_TEST_MANIFEST").unwrap());
        let input =
            crate::content::native_gameplay::NativeGameplayInput::from_manifest(&manifest).unwrap();
        let room = crate::content::qualify_selected_native_gameplay_room(world, &input).unwrap();
        let native = std::sync::Arc::new(
            crate::content::qualified_native_gameplay_test_state(room.compiled()).unwrap(),
        );
        let mut channel_id = [0u8; 16];
        channel_id[6] = 0x70;
        channel_id[8] = 0x80;
        channel_id[15] = 92;
        let mut node_id = channel_id;
        node_id[15] = 93;
        let start = room.entry_start();
        let pin = crate::foundation::ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&channel_id).unwrap(),
            crate::foundation::NodeId::decode(&node_id).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .unwrap();
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.rat".into(),
            revision: "definition-r1".into(),
        };
        let actor = runtime.crystal_router_fixture_actor(
            &creature.key,
            20,
            crate::foundation::MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        );
        let b = runtime.binding();
        let (mut fence, _) = crate::foundation::crystal_timer_fixture(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .unwrap();
        let mut lane = MonsterCombatLane::new(&runtime, &fence).unwrap();
        let fence_before = format!("{fence:?}");
        for mutation in 0..8 {
            let mut detached = draft.clone();
            match mutation {
                0 | 5 => {
                    let key = if mutation == 0 {
                        &creature.key
                    } else {
                        "oteryn:creature.demon"
                    };
                    let profile = detached
                        .state
                        .authoring_profiles
                        .iter_mut()
                        .find(|p| p.target.key == key)
                        .unwrap();
                    let Data::Creature(data) = &mut profile.data else {
                        panic!("Creature profile")
                    };
                    data.health = Some(data.health.unwrap() + 1);
                }
                1 => {
                    detached.state.authoring_profiles.pop();
                }
                2 => {
                    detached.core.records.pop();
                }
                3 => {
                    detached.state.sources[0].revision.push_str("-detached");
                }
                4 => {
                    detached.state.source_identity_bindings[0]
                        .source_revision
                        .push_str("-detached");
                }
                6 => {
                    let profile = detached
                        .state
                        .authoring_profiles
                        .iter_mut()
                        .find(|p| matches!(&p.data, Data::Behavior(b) if !b.attacks.is_empty()))
                        .unwrap();
                    let Data::Behavior(behavior) = &mut profile.data else {
                        panic!("Behavior profile")
                    };
                    behavior.attacks[0].interval_ms += 1;
                }
                7 => {
                    detached.state.declarations.pop();
                }
                _ => unreachable!(),
            }
            assert!(
                TrustedMonsterConditionTickInputs::from_trusted_native_loader(
                    &runtime, &room, &detached, &native
                )
                .is_none(),
                "condition grant mutation {mutation}"
            );
            let result = lane.register_project_with_native(
                &runtime,
                &mut fence,
                actor,
                &creature,
                &detached,
                native.clone(),
                SemanticTimeMicros::from_micros(0),
            );
            assert!(
                matches!(
                    result,
                    Err(CombatLaneError::Attack(AttackError::InvalidSource))
                ),
                "mutation {mutation}: {result:?}"
            );
            assert!(lane.actors.is_empty());
            assert!(lane.child_closure.is_none());
            assert!(lane.area_catalog.is_none());
            assert!(lane.native_appearance_content.is_none());
            assert_eq!(format!("{fence:?}"), fence_before);
        }
        lane.register_project_with_native(
            &runtime,
            &mut fence,
            actor,
            &creature,
            &draft,
            native,
            SemanticTimeMicros::from_micros(0),
        )
        .unwrap();
        let current_native = lane.native_appearance_content.as_ref().unwrap();
        assert!(
            TrustedMonsterConditionTickInputs::from_trusted_native_loader(
                &runtime,
                &room,
                &draft,
                current_native
            )
            .unwrap()
            .current(&runtime)
            .is_some()
        );
        assert_eq!(lane.actors.len(), 1);
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod native_ability_coverage {
    //! Data-coverage gate: every authored monster attack in `content/` must have a runtime owner
    //! constructor in this lane, or be listed in the reviewed baseline of known gaps.
    use super::*;
    use std::collections::BTreeSet;
    type Key = (String, String);
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|root| root.join("workspace-boundaries.toml").is_file())
            .expect("workspace root")
            .join("content")
    }
    fn shards(dir: &str) -> Vec<serde_json::Value> {
        let mut paths: Vec<_> = std::fs::read_dir(root().join(dir))
            .expect("content dir")
            .map(|e| e.expect("entry").path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains("-") && n.ends_with(".json") && n != "index.json")
                    && !p.to_string_lossy().contains("spell-native")
            })
            .collect();
        paths.sort();
        paths
            .into_iter()
            .map(|p| serde_json::from_slice(&std::fs::read(p).expect("read")).expect("json"))
            .collect()
    }
    fn refs(v: &serde_json::Value, out: &mut BTreeSet<Key>) {
        match v {
            serde_json::Value::Object(m) => {
                if let (Some(f), Some(k), Some(_)) =
                    (m.get("family"), m.get("key"), m.get("revision"))
                    && let (Some(f), Some(k)) = (f.as_str(), k.as_str())
                {
                    out.insert((f.to_owned(), k.to_owned()));
                }
                m.values().for_each(|x| refs(x, out));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| refs(x, out)),
            _ => {}
        }
    }
    struct Entry {
        record: ProjectReferenceRecord,
        profile: ProjectV2AuthoringProfile,
        refs: BTreeSet<Key>,
    }
    fn load() -> BTreeMap<Key, Entry> {
        let mut all = BTreeMap::new();
        for dir in [
            "creatures/definitions",
            "behaviors",
            "abilities/definitions",
            "abilities/effects",
            "abilities/formulas",
        ] {
            for shard in shards(dir) {
                let Some(rows) = shard["records"].as_array() else {
                    continue;
                };
                for r in rows {
                    let (Some(def), Some(auth)) = (r.get("definition"), r.get("authoring")) else {
                        continue;
                    };
                    let Ok(record) = serde_json::from_value::<ProjectReferenceRecord>(def.clone())
                    else {
                        continue;
                    };
                    let Ok(data) = serde_json::from_value::<Data>(auth.clone()) else {
                        continue;
                    };
                    let Ok(target) = serde_json::from_value::<Ref>(def["identity"].clone()) else {
                        continue;
                    };
                    let mut refs_set = BTreeSet::new();
                    refs(def, &mut refs_set);
                    refs(auth, &mut refs_set);
                    all.insert(
                        (format!("{:?}", target.family), target.key.clone()),
                        Entry {
                            record,
                            profile: ProjectV2AuthoringProfile { target, data },
                            refs: refs_set,
                        },
                    );
                }
            }
        }
        all
    }
    /// Known set of authored attacks with no source constructor (`UnsupportedShape`): the exact
    /// count is pinned. Any new authored attack that is not qualified, or any fix that qualifies
    /// one of them, changes the histogram and must update this baseline in the same change.
    #[test]
    fn authored_monster_attacks_have_a_runtime_source_or_are_in_the_pinned_gap_set() {
        let all = load();
        let creatures: Vec<_> = all.iter().filter(|(k, _)| k.0 == "Creature").collect();
        let mut stats: BTreeMap<String, usize> = BTreeMap::new();
        let mut gaps: Vec<String> = Vec::new();
        for (ck, ce) in creatures {
            let mut seen = BTreeSet::new();
            let mut todo = vec![ck.clone()];
            while let Some(k) = todo.pop() {
                if !seen.insert(k.clone()) {
                    continue;
                }
                if let Some(e) = all.get(&k) {
                    for r in &e.refs {
                        if !matches!(r.0.as_str(), "Creature" | "Item" | "Loot" | "Presentation") {
                            todo.push(r.clone());
                        }
                    }
                }
            }
            let records: Vec<_> = seen
                .iter()
                .filter_map(|k| all.get(k))
                .map(|e| e.record.clone())
                .collect();
            let profiles: Vec<_> = seen
                .iter()
                .filter_map(|k| all.get(k))
                .map(|e| e.profile.clone())
                .collect();
            let ProjectReferenceRecord::Creature { identity, .. } = &ce.record else {
                continue;
            };
            let cref = Ref {
                family: ProjectV2Family::Creature,
                key: identity.key.clone(),
                revision: identity.revision.clone(),
            };
            let Some(Data::Behavior(b)) = profiles
                .iter()
                .find(|p| p.target.family == ProjectV2Family::Behavior)
                .map(|p| &p.data)
            else {
                *stats.entry("no-behavior".to_owned()).or_default() += 1;
                continue;
            };
            for (i, a) in b.attacks.iter().enumerate() {
                let melee = profiles.iter().any(|p| p.target == a.ability && matches!(&p.data, Data::Ability(x) if x.details.as_ref().is_some_and(|d| d.kind == ProjectV2AbilityKind::Melee)));
                let r = if melee {
                    MeleeSource::from_native(&cref, i, &records, &profiles, [0; 32]).map(|_| ())
                } else {
                    SpellSource::from_native(&cref, i, &records, &profiles, [0; 32]).map(|_| ())
                };
                if r.is_err() {
                    gaps.push(format!("{} #{} {} {:?}", cref.key, i, a.ability.key, r));
                }
                *stats
                    .entry(format!("{}:{:?}", if melee { "melee" } else { "spell" }, r))
                    .or_default() += 1;
            }
        }
        gaps.sort();
        eprintln!("GAPS {}\n{}", gaps.len(), gaps.join("\n"));
        eprintln!("COVERAGE {stats:?}");
        let expected: BTreeMap<String, usize> = [
            ("melee:Ok(())", 1689),
            ("spell:Err(UnsupportedShape)", 116),
            ("spell:Ok(())", 4198),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(stats, expected);
    }
}

impl oteryn_gameplay_monster_ai::monster_combat_lane::EncounterHooks for MonsterCombatLane {
    type Pulse = CombatPulse;
    type Error = CombatLaneError;

    fn run_encounter<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        reader: &mut R,
        revisions: RevisionSet,
    ) -> Result<Self::Pulse, Self::Error> {
        self.run_encounter_turn(runtime, states, current, clock, reader, revisions)
    }
}
