//! Sealed, nine-source callback child casts; reuses the existing native damage consumer.
//! Source Encounter bytes qualify actions; there is no general Encounter interpreter.
#[cfg(test)]
#[path = "source_callback_cast_cutoff_tests.rs"]
mod cutoff_tests;

use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord, ProjectV2AbilityEffect, ProjectV2AbilityKind,
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref, ProjectV2Family,
    ProjectV2FormulaAuthoring, ProjectV2Magnitude,
};
use crate::creature_auto_attack::{AttackError, ProfileAbilityProposal};
use crate::creature_damage_spell::*;
use crate::player_lethal::PlayerLethalVitals;
use oteryn_gameplay_foundation::foundation::owner_timer::SemanticTimeMicros;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use {
    oteryn_gameplay_entities::ChannelRuntimeV1,
    oteryn_gameplay_foundation::foundation::{ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence},
};
pub type CallbackPulseBatch =
    Result<Vec<(ExactActorRef, Ref, usize, Result<SpellOutcome, AttackError>)>, AttackError>;
use crate::content::{ProjectV2Draft, ProjectV2SourceIdentityBinding};
pub use oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::CallbackSpeech;
use oteryn_gameplay_foundation::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, TimerFamily,
};
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
#[derive(serde::Deserialize)]
pub struct Witness {
    pub cases: BTreeMap<String, Case>,
    profiles: BTreeMap<String, Data>,
    records: Vec<ProjectReferenceRecord>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
}
#[derive(serde::Deserialize)]
pub struct Case {
    pub caster: String,
    pub index: usize,
    phase: String,
    pub encounter: String,
    pub casts: Vec<CastWitness>,
    entry: serde_json::Value,
}
#[derive(serde::Deserialize)]
pub struct CastWitness {
    delay_ms: Delay,
    action: serde_json::Value,
}
#[derive(serde::Deserialize)]
struct Delay {
    min: u64,
    max: u64,
}
#[derive(Clone)]
pub struct SourceCallbackCast {
    creature: String,
    pub parent: Ref,
    pub index: usize,
    pub range: u16,
    pub magnitude: Option<ProjectV2Magnitude>,
    pub content: [u8; 32],
    stages: Vec<(u64, SpellSource)>,
    native_parent:
        Option<oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallback>,
}
impl SourceCallbackCast {
    pub fn qualify_native(
        native: &crate::content::native_gameplay::NativeGameplayState,
        creature: &Ref,
        index: usize,
        list: ScheduleList,
    ) -> Result<Option<Self>, AttackError> {
        if list != ScheduleList::Attack {
            return Ok(None);
        }
        let Some(proof) = super::native_witness::qualify(native, creature, index, list)? else {
            return Ok(None);
        };
        static FIXTURE: std::sync::OnceLock<Option<Witness>> = std::sync::OnceLock::new();
        let w = FIXTURE
            .get_or_init(|| {
                serde_json::from_str(include_str!(
                    "../../../../../../apps/game-server/src/source_callback_cast_fixture.json"
                ))
                .ok()
            })
            .as_ref()
            .ok_or(AttackError::InvalidSource)?;
        let Some((key, c)) = w
            .cases
            .iter()
            .find(|(_, c)| c.caster == creature.key && c.index == index && c.phase == "attacks")
        else {
            return Ok(None);
        };
        if proof.entry().ability.key != *key || proof.entry().encounter.key != c.encounter {
            return Err(AttackError::InvalidSource);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == *creature)
            .ok_or(AttackError::InvalidSource)?;
        let Data::Creature(cp) = &record.profile.data else {
            return Err(AttackError::InvalidSource);
        };
        let Data::Behavior(behavior) = &record
            .behavior
            .as_ref()
            .ok_or(AttackError::InvalidSource)?
            .data
        else {
            return Err(AttackError::InvalidSource);
        };
        let entry = behavior
            .attacks
            .get(index)
            .ok_or(AttackError::InvalidSource)?;
        if serde_json::to_value(entry).map_err(|_| AttackError::InvalidSource)? != c.entry {
            return Err(AttackError::InvalidSource);
        }
        let mut source = Self::from_reviewed_case(w, key, c, cp, entry, proof.content())?;
        for (index, (_, stage)) in source.stages.iter_mut().enumerate() {
            stage.fingerprint = proof
                .as_content_source()
                .stage(index)
                .map_err(|_| AttackError::InvalidSource)?
                .ok_or(AttackError::InvalidSource)?
                .fingerprint();
        }
        source.native_parent = Some(proof.as_content_source().clone());
        Ok(Some(source))
    }
    pub fn parent(&self) -> &Ref {
        &self.parent
    }
    pub fn native_parent(
        &self,
    ) -> Option<&oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallback>
    {
        self.native_parent.as_ref()
    }
    pub fn validates_current_native_proposal(
        &self,
        runtime: &ChannelRuntimeV1,
        proposal: &ProfileAbilityProposal,
    ) -> bool {
        self.native_parent
            .as_ref()
            .is_some_and(|parent| parent.content() == self.content)
            && self.current(runtime, proposal.issuer)
            && self.sequence(proposal).is_ok()
            && super::source_callback_intent::validate(runtime, proposal).is_ok()
    }
    pub fn stages(&self) -> &[(u64, SpellSource)] {
        &self.stages
    }
    pub fn qualify(
        d: &ProjectV2Draft,
        creature: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Option<Self>, AttackError> {
        if !matches!(
            creature.key.as_str(),
            "oteryn:creature.charging_outburst"
                | "oteryn:creature.energy_pulse"
                | "oteryn:creature.eruption_of_destruction"
                | "oteryn:creature.foam_stalker"
                | "oteryn:creature.gaz_haragoth"
                | "oteryn:creature.lady_tenebris"
                | "oteryn:creature.plagirath"
                | "oteryn:creature.rootthing_amber_shaper"
                | "oteryn:creature.rootthing_nutshell"
        ) {
            return Ok(None);
        }
        let w: Witness = serde_json::from_str(include_str!(
            "../../../../../../apps/game-server/src/source_callback_cast_fixture.json"
        ))
        .map_err(|_| AttackError::InvalidSource)?;
        let Some((key, c)) = w
            .cases
            .iter()
            .find(|(_, c)| c.caster == creature.key && c.index == index)
        else {
            return Ok(None);
        };
        if creature.family != ProjectV2Family::Creature
            || creature.revision != "definition-r1"
            || c.phase != "attacks"
        {
            return Err(AttackError::InvalidSource);
        }
        for (key, data) in &w.profiles {
            if !d.state.authoring_profiles.iter().any(|p| {
                p.target.key == *key && p.target.revision == "definition-r1" && p.data == *data
            }) {
                return Err(AttackError::InvalidSource);
            }
        }
        for rec in &w.records {
            if !d.core.records.contains(rec) {
                return Err(AttackError::InvalidSource);
            }
        }
        let expected = w
            .bindings
            .iter()
            .find(|b| b.target.key == c.encounter)
            .ok_or(AttackError::InvalidSource)?;
        if !d.state.source_identity_bindings.contains(expected)
            || !d
                .state
                .sources
                .iter()
                .any(|s| s.key == expected.source_key && s.revision == expected.source_revision)
        {
            return Err(AttackError::InvalidSource);
        }
        let Some(ProjectReferenceRecord::Creature{behavior,..})=d.core.records.iter().find(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..}if identity.key==creature.key&&identity.revision==creature.revision))else{return Err(AttackError::InvalidSource)};
        let Some(Data::Behavior(b)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        let entry = b.attacks.get(index).ok_or(AttackError::InvalidSource)?;
        if serde_json::to_value(entry).map_err(|_| AttackError::InvalidSource)? != c.entry
            || entry.ability.key != *key
        {
            return Err(AttackError::InvalidSource);
        }
        let Some(Data::Creature(cp)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *creature)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        Self::from_reviewed_case(&w, key, c, cp, entry, content).map(Some)
    }
    fn from_reviewed_case(
        w: &Witness,
        key: &str,
        c: &Case,
        cp: &crate::content::ProjectV2CreatureAuthoring,
        entry: &crate::content::ProjectV2AbilitySchedule,
        content: [u8; 32],
    ) -> Result<Self, AttackError> {
        let parent = entry.ability.clone();
        let index = c.index;
        let mut stages = Vec::new();
        let critical = crate::source_critical_baseline::SourceCriticalBaseline::qualify(
            &c.caster,
            cp.details.as_ref().map_or(0, |c| c.critical_chance_ppm),
            false,
        )
        .map_err(|_| AttackError::UnsupportedShape)?;
        for stage in &c.casts {
            if stage.delay_ms.min != stage.delay_ms.max
                || stage.delay_ms.max > 10000
                || stage
                    .action
                    .get("at")
                    .and_then(|a| a.get("kind"))
                    .and_then(|v| v.as_str())
                    != Some("subject_position")
            {
                return Err(AttackError::InvalidSource);
            }
            let (minimum, maximum, element, area, direction, presentation, armor) = if let Some(
                child,
            ) =
                stage.action.get("ability")
            {
                let child: Ref = serde_json::from_value(child.clone())
                    .map_err(|_| AttackError::InvalidSource)?;
                let Some(Data::Ability(a)) = w.profiles.get(&child.key) else {
                    return Err(AttackError::InvalidSource);
                };
                let details = a.details.as_deref().ok_or(AttackError::InvalidSource)?;
                if details.kind != ProjectV2AbilityKind::Spell
                    || details.needs_target
                    || details.encounter.is_some()
                    || details.chain.is_some()
                    || details.windup.is_some()
                    || !details.variants.is_empty()
                {
                    return Err(AttackError::UnsupportedShape);
                }
                let [ProjectV2AbilityEffect::Executable(effect)] = details.effects.as_slice()
                else {
                    return Err(AttackError::UnsupportedShape);
                };
                let Some(ProjectReferenceRecord::Ability{effects,..})=w.records.iter().find(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.key==child.key&&identity.revision==child.revision))else{return Err(AttackError::InvalidSource)};
                if !effects
                    .iter()
                    .any(|e| e.key == effect.key && e.revision == effect.revision)
                {
                    return Err(AttackError::InvalidSource);
                }
                let Some(ProjectReferenceRecord::Effect{effect_family:EffectFamilyDocument::Damage,formula,..})=w.records.iter().find(|r|matches!(r,ProjectReferenceRecord::Effect{identity,..}if identity.key==effect.key&&identity.revision==effect.revision))else{return Err(AttackError::UnsupportedShape)};
                let Some(Data::Effect(ep)) = w.profiles.get(&effect.key) else {
                    return Err(AttackError::InvalidSource);
                };
                let Some(Data::Formula(ProjectV2FormulaAuthoring::Range { minimum, maximum })) =
                    w.profiles.get(&formula.key)
                else {
                    return Err(AttackError::UnsupportedShape);
                };
                if ep.affects.is_some()
                    || ep
                        .mitigated_by
                        .contains(&crate::content::ProjectV2Mitigation::Shield)
                {
                    return Err(AttackError::UnsupportedShape);
                }
                (
                    *minimum,
                    *maximum,
                    ep.damage_type.clone(),
                    details.area.clone(),
                    details.needs_direction,
                    None,
                    ep.mitigated_by
                        .contains(&crate::content::ProjectV2Mitigation::Armor),
                )
            } else {
                let Some(Data::Encounter(e)) = w.profiles.get(&c.encounter) else {
                    return Err(AttackError::InvalidSource);
                };
                let details = e.details.as_ref().ok_or(AttackError::InvalidSource)?;
                let ability = stage
                    .action
                    .get("encounter_ability")
                    .and_then(|v| v.as_str())
                    .ok_or(AttackError::InvalidSource)?;
                let a = details
                    .abilities
                    .iter()
                    .find(|a| a.key == ability)
                    .ok_or(AttackError::InvalidSource)?;
                if !a.affects.players
                    || !a.affects.creatures.is_empty()
                    || a.area.shape != crate::content::ProjectV2NearShape::Circle
                {
                    return Err(AttackError::UnsupportedShape);
                }
                (
                    a.damage.min,
                    a.damage.max,
                    a.damage.damage_type.clone(),
                    Some(oteryn_gameplay_content::content::native_monster_callbacks::source_callback_area(&c.caster)
                        .ok_or(AttackError::InvalidSource)?),
                    false,
                    None,
                    false,
                )
            };
            if minimum > maximum
                || !matches!(
                    element.as_str(),
                    "physical" | "energy" | "ice" | "fire" | "earth" | "death" | "holy"
                )
            {
                return Err(AttackError::UnsupportedShape);
            }
            let minimum = u32::try_from(minimum).map_err(|_| AttackError::NumericOverflow)?;
            let maximum = u32::try_from(maximum).map_err(|_| AttackError::NumericOverflow)?;
            let fingerprint = Sha256::digest(
                serde_json::to_vec(&(key, &stage.action, content))
                    .map_err(|_| AttackError::InvalidSource)?,
            )
            .into();
            stages.push((
                stage.delay_ms.min,
                SpellSource {
                    list: ScheduleList::Attack,
                    creature_key: c.caster.clone(),
                    ability: parent.clone(),
                    entry_index: index,
                    minimum,
                    maximum,
                    reference_level: cp
                        .details
                        .as_ref()
                        .and_then(|details| details.reference_level),
                    reference_level_overrides: cp
                        .details
                        .as_ref()
                        .map_or_else(Vec::new, |details| {
                            details.reference_level_overrides.clone()
                        }),
                    critical,
                    conditions: Vec::new(),
                    formula_revision: None,
                    fingerprint,
                    content_digest: content,
                    element: Some(element),
                    armor,
                    range: 0,
                    magnitude: None,
                    area,
                    chain: None,
                    presentation,
                    appearance_cast: false,
                    appearances: Vec::new(),
                    source_none_correction: false,
                    dispel_invisible: false,
                    variants: Vec::new(),
                    variant_child: None,
                    windup: None,
                    needs_target: false,
                    needs_direction: direction,
                },
            ));
        }
        if stages.is_empty() || stages.len() > 3 {
            return Err(AttackError::InvalidSource);
        }
        Ok(Self {
            creature: c.caster.clone(),
            parent,
            index,
            range: entry.range_tiles.unwrap_or(0),
            magnitude: entry.magnitude,
            content,
            stages,
            native_parent: None,
        })
    }
    fn sequence(&self, p: &ProfileAbilityProposal) -> Result<u64, AttackError> {
        if p.ability != self.parent
            || p.list != ScheduleList::Attack
            || p.entry_index != self.index
            || p.range_tiles != self.range
            || p.magnitude != self.magnitude
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(&p.issuer.placement_identity()));
        let rest = p
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (seq, tail) = rest.split_once(':').ok_or(AttackError::InvalidPlan)?;
        if tail != format!("attack:{}", self.index) {
            return Err(AttackError::InvalidPlan);
        }
        seq.parse().map_err(|_| AttackError::InvalidPlan)
    }
    fn current(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        r.content_pin().server_artifact_digest() == self.content
            && r.matches_live_creature_identity(a, self.creature.as_bytes())
    }
    /// A retained native Lua Combat closure resolves Creature(cid), which remains
    /// registered at HP0 until its queued onDeath completes. This never admits a
    /// new cast: scheduling continues to require `current` above. The immutable
    /// source closure remains the captured one even if an earlier task setType.
    fn current_retained(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        let Some(proof) = self.native_parent.as_ref() else {
            return self.current(r, a);
        };
        let entry = proof.entry();
        proof.content() == self.content
            && r.content_pin().server_artifact_digest() == self.content
            && entry.creature.key == self.creature
            && entry.ability == self.parent
            && entry.index == self.index
            && r.source_creature_is_registered(a).unwrap_or(false)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Family(u8);
const CHARGING_REMOVAL_FAMILY: Family = Family(48);
const CHARGING_REMOVAL_STAGE: u8 = u8::MAX;
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        3
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Key {
    token: u64,
    stage: u8,
}
pub struct Cast {
    token: u64,
    source: SourceCallbackCast,
    proposal: ProfileAbilityProposal,
    done: Vec<bool>,
    anchors: Vec<Option<crate::creature_damage_spell::SourcePositionAnchor>>,
    retained_caster: Option<oteryn_gameplay_entities::NativeRetainedCallbackCaster>,
    charging_removal: Option<(SemanticTimeMicros, bool)>,
}
#[allow(dead_code, reason = "retained source callback occurrence metadata")]
struct Memo {
    actor: ExactActorRef,
    ability: Ref,
    sequence: u64,
    at: u64,
    token: u64,
    proposal: ProfileAbilityProposal,
}
/// Bounded current-owner adapter, source occurrence high-water independent of child stage IDs.
pub struct CallbackCastOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Family, Key>,
    pub casts: Vec<Cast>,
    memos: Vec<Memo>,
    next: u64,
    speech: Vec<CallbackSpeech>,
    removals: Vec<(
        ExactActorRef,
        Result<(), oteryn_gameplay_domain::domain::carrier_values::CarrierError>,
    )>,
}
/// Identity of one retained source stage; its constructor stays in this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallbackStageId {
    token: u64,
    stage: u8,
}

/// Read-only preparation data. Neither a stage ID nor these data fire a timer.
pub struct CallbackStagePlan {
    id: CallbackStageId,
    source: SpellSource,
    proposal: ProfileAbilityProposal,
    anchor: Option<crate::creature_damage_spell::SourcePositionAnchor>,
    native_stage: Option<
        oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallbackStage,
    >,
    retained_caster: Option<oteryn_gameplay_entities::NativeRetainedCallbackCaster>,
}
impl CallbackStagePlan {
    pub fn retained_caster(
        &self,
    ) -> Option<&oteryn_gameplay_entities::NativeRetainedCallbackCaster> {
        self.retained_caster.as_ref()
    }
    pub fn id(&self) -> CallbackStageId {
        self.id
    }
    pub fn native_stage(
        &self,
    ) -> Option<
        &oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallbackStage,
    > {
        self.native_stage.as_ref()
    }
    pub fn position_anchor(&self) -> Option<&crate::creature_damage_spell::SourcePositionAnchor> {
        self.anchor.as_ref()
    }
    pub fn source(&self) -> &SpellSource {
        &self.source
    }
    /// Qualified impact binding for this source stage, independent of HP targets.
    pub fn source_impact_binding(&self) -> Option<&str> {
        Some(match (self.source.creature_key.as_str(), self.id.stage) {
            ("oteryn:creature.energy_pulse", _) => "canary.appearance:effect/energyarea",
            ("oteryn:creature.foam_stalker", _) => "canary.appearance:effect/watersplash",
            ("oteryn:creature.charging_outburst" | "oteryn:creature.gaz_haragoth", _) => {
                "canary.appearance:effect/purpleenergy"
            }
            ("oteryn:creature.eruption_of_destruction", _) => "canary.appearance:effect/firearea",
            ("oteryn:creature.lady_tenebris", _) => "canary.appearance:effect/mortarea",
            ("oteryn:creature.plagirath", _) => "canary.appearance:effect/poison",
            ("oteryn:creature.rootthing_amber_shaper", 0) => "canary.appearance:effect/smallplants",
            ("oteryn:creature.rootthing_amber_shaper", 1) => "canary.appearance:effect/fireattack",
            ("oteryn:creature.rootthing_nutshell", 0) => "canary.appearance:effect/holydamage",
            ("oteryn:creature.rootthing_nutshell", 1) => "canary.appearance:effect/groundshaker",
            _ => return None,
        })
    }
    pub fn proposal(&self) -> &ProfileAbilityProposal {
        &self.proposal
    }
    /// Source callback target policy; ordinary spells do not use this selector.
    pub fn permits_player_vocation(
        &self,
        vocation: oteryn_gameplay_content::content::spell_data::Vocation,
    ) -> bool {
        use oteryn_gameplay_content::content::spell_data::Vocation;
        !matches!(
            self.source.creature_key.as_str(),
            "oteryn:creature.gaz_haragoth"
                | "oteryn:creature.eruption_of_destruction"
                | "oteryn:creature.lady_tenebris"
        ) || matches!(
            vocation,
            Vocation::Sorcerer
                | Vocation::MasterSorcerer
                | Vocation::Druid
                | Vocation::ElderDruid
                | Vocation::Paladin
                | Vocation::RoyalPaladin
                | Vocation::Knight
                | Vocation::EliteKnight
        )
    }
    pub fn anchor_position(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<oteryn_gameplay_entities::MovementLocalPosition>, AttackError> {
        self.anchor
            .as_ref()
            .map(|anchor| anchor.current_position(runtime, &self.source, &self.proposal))
            .transpose()
    }
}

/// Current DATA prepared for one exact delayed stage, including its own anchor.
pub trait CallbackStageReaders {
    /// A terminal saved-variant failure consumes its original event. Missing
    /// current map/combat facts instead keep the event at its original due time.
    fn terminal_error(&self, _stage: CallbackStageId) -> Option<AttackError> {
        None
    }
    fn ready(&self, stage: CallbackStageId) -> bool;
    fn reader(&mut self, stage: CallbackStageId) -> Option<&mut (dyn SpellWorldReader + '_)>;
    /// Execute a stage through the existing composed owner when current creature
    /// facts were prepared alongside its player facts. Legacy readers use damage.
    fn execute_current(
        &mut self,
        _stage: CallbackStageId,
        _runtime: &mut ChannelRuntimeV1,
        _stamp: RuntimeWorkStamp,
        _vitals: &mut dyn PlayerLethalVitals,
        _now: SemanticTimeMicros,
    ) -> Option<Result<SpellOutcome, AttackError>> {
        None
    }
}
struct SingleStageReader<'a> {
    reader: &'a mut dyn SpellWorldReader,
}
impl CallbackStageReaders for SingleStageReader<'_> {
    fn ready(&self, _: CallbackStageId) -> bool {
        true
    }
    fn reader(&mut self, _: CallbackStageId) -> Option<&mut (dyn SpellWorldReader + '_)> {
        Some(&mut *self.reader)
    }
}
fn stage_proposal(cast: &Cast, stage: usize) -> Result<ProfileAbilityProposal, AttackError> {
    let mut proposal = cast.proposal.clone();
    if cast.source.creature != "oteryn:creature.plagirath" {
        proposal.target = proposal.issuer;
    }
    let actor = format!("actor:{}", hex(&proposal.issuer.placement_identity()));
    let target = format!("actor:{}", hex(&proposal.target.placement_identity()));
    proposal.intent = crate::ability::AiAbilityAdapter::normalize(&actor, &[&target])
        .map_err(|_| AttackError::InvalidPlan)?;
    proposal.occurrence = crate::ability::AbilityOccurrence::new(
        &format!(
            "callback:{}:stage:{}",
            proposal.occurrence.id().as_str(),
            stage
        ),
        proposal.occurrence.revisions().clone(),
    )
    .map_err(|_| AttackError::InvalidPlan)?;
    Ok(proposal)
}
impl CallbackCastOwner {
    pub fn next_due_time(&self) -> Option<SemanticTimeMicros> {
        self.timers
            .earliest_due()
            .into_iter()
            .chain(
                self.casts
                    .iter()
                    .filter_map(|cast| match cast.charging_removal {
                        Some((due, false)) => Some(due),
                        _ => None,
                    }),
            )
            .min()
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn source_parent_status_for_test(&self, issuer: ExactActorRef) -> (bool, bool) {
        (
            self.memos.iter().any(|memo| memo.actor == issuer),
            self.casts.iter().any(|cast| {
                cast.proposal.issuer == issuer
                    && (cast.done.iter().any(|done| !done) || cast.charging_removal.is_some())
            }),
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn source_stage_done_for_test(&self, issuer: ExactActorRef) -> Option<Vec<bool>> {
        let mut casts = self
            .casts
            .iter()
            .filter(|cast| cast.proposal.issuer == issuer);
        let cast = casts.next()?;
        if casts.next().is_some() {
            return None;
        }
        Some(cast.done.clone())
    }

    /// Exact due-stage projection under the independently current physical owner.
    pub fn due_stages_current(
        &self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
    ) -> Result<Vec<CallbackStagePlan>, AttackError> {
        self.due_stages(
            runtime,
            runtime.owner_fence().map_err(|_| AttackError::StaleOwner)?,
            stamp,
            now,
        )
    }

    /// Readiness before an already queued death, retaining the real execution clock.
    pub fn due_stages_current_through(
        &self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        eligible_through: SemanticTimeMicros,
        now: SemanticTimeMicros,
    ) -> Result<Vec<CallbackStagePlan>, AttackError> {
        self.due_stages_current(runtime, stamp, now.min(eligible_through))
    }

    pub fn due_stages(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
    ) -> Result<Vec<CallbackStagePlan>, AttackError> {
        if !self.current(runtime, fence, stamp) {
            return Err(AttackError::StaleOwner);
        }
        let mut plans = Vec::new();
        for cast in &self.casts {
            if !cast.source.current_retained(runtime, cast.proposal.issuer) {
                continue;
            }
            for (stage, (_, source)) in cast.source.stages.iter().enumerate() {
                if cast.done[stage] {
                    continue;
                }
                let id = CallbackStageId {
                    token: cast.token,
                    stage: stage as u8,
                };
                if !self
                    .timers
                    .occurrence_due(
                        Family((cast.source.index * 3 + stage) as u8),
                        Key {
                            token: id.token,
                            stage: id.stage,
                        },
                    )
                    .is_some_and(|due| due <= now)
                {
                    continue;
                }
                plans.push(CallbackStagePlan {
                    id,
                    source: source.clone(),
                    proposal: stage_proposal(cast, stage)?,
                    anchor: cast.anchors[stage].clone(),
                    retained_caster: cast.retained_caster.clone(),
                    native_stage: match cast.source.native_parent.as_ref() {
                        Some(parent) => Some(
                            parent
                                .stage(stage)
                                .map_err(|_| AttackError::InvalidSource)?
                                .ok_or(AttackError::InvalidSource)?,
                        ),
                        None => None,
                    },
                });
            }
        }
        Ok(plans)
    }

    /// Freeze the first due Eruption event before its master heal starts. A
    /// retained partial prelude must never rebase its original event geometry.
    pub fn freeze_eruption_event_current(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        id: CallbackStageId,
        now: SemanticTimeMicros,
    ) -> Result<(), AttackError> {
        if !self.current(
            runtime,
            runtime.owner_fence().map_err(|_| AttackError::StaleOwner)?,
            stamp,
        ) {
            return Err(AttackError::StaleOwner);
        }
        let cast = self
            .casts
            .iter_mut()
            .find(|cast| cast.token == id.token)
            .ok_or(AttackError::InvalidPlan)?;
        let index = usize::from(id.stage);
        if cast.source.creature != "oteryn:creature.eruption_of_destruction"
            || cast.done.get(index).copied() != Some(false)
            || !self
                .timers
                .occurrence_due(
                    Family((cast.source.index * 3 + index) as u8),
                    Key {
                        token: id.token,
                        stage: id.stage,
                    },
                )
                .is_some_and(|due| due <= now)
        {
            return Err(AttackError::InvalidPlan);
        }
        if cast.anchors[index].is_none() {
            let proposal = stage_proposal(cast, index)?;
            let anchor =
                SourcePositionAnchor::capture(runtime, &cast.source.stages[index].1, &proposal)?;
            cast.anchors[index] = Some(anchor);
        }
        Ok(())
    }

    pub fn new_current(r: &ChannelRuntimeV1) -> Result<Self, AttackError> {
        Self::new(r, r.owner_fence().map_err(|_| AttackError::StaleOwner)?)
    }
    pub fn new(r: &ChannelRuntimeV1, f: &ScopeRuntimeFence) -> Result<Self, AttackError> {
        let b = r.binding();
        let scope = b.runtime_scope();
        let generation = b.scope_generation();
        if !f.is_current_for_scope(scope, generation) {
            return Err(AttackError::StaleOwner);
        }
        let timers = OwnerTimerLane::for_generation(
            scope,
            generation,
            (0..49).map(|i| {
                (
                    Family(i),
                    FamilyPolicy {
                        max_pending: 3,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )
            }),
        )
        .map_err(|_| AttackError::InvalidPlan)?;
        Ok(Self {
            scope,
            generation,
            timers,
            casts: Vec::new(),
            memos: Vec::new(),
            next: 1,
            speech: Vec::new(),
            removals: Vec::new(),
        })
    }
    pub fn take_source_speech(&mut self) -> Vec<CallbackSpeech> {
        std::mem::take(&mut self.speech)
    }
    pub fn take_source_removals(
        &mut self,
    ) -> Vec<(
        ExactActorRef,
        Result<(), oteryn_gameplay_domain::domain::carrier_values::CarrierError>,
    )> {
        std::mem::take(&mut self.removals)
    }
    fn current(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> bool {
        let b = r.binding();
        self.scope == b.runtime_scope()
            && self.generation == b.scope_generation()
            && f.is_current_for_scope(self.scope, self.generation)
            && f.accepts_stamp(stamp)
    }
    fn prune(&mut self, r: &ChannelRuntimeV1) {
        for c in &self.casts {
            if !c.source.current_retained(r, c.proposal.issuer)
                || (c.done.iter().all(|d| *d) && c.charging_removal.is_none())
            {
                for stage in 0..c.done.len() {
                    self.timers.cancel(
                        Family((c.source.index * 3 + stage) as u8),
                        Key {
                            token: c.token,
                            stage: stage as u8,
                        },
                    );
                }
                self.timers.cancel(
                    CHARGING_REMOVAL_FAMILY,
                    Key {
                        token: c.token,
                        stage: CHARGING_REMOVAL_STAGE,
                    },
                );
            }
        }
        self.casts.retain(|c| {
            c.source.current_retained(r, c.proposal.issuer)
                && (!c.done.iter().all(|d| *d) || c.charging_removal.is_some())
        });
        self.memos.retain(|m| r.contains_live_creature(m.actor));
    }
    pub fn schedule_current(
        &mut self,
        r: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        source: &SourceCallbackCast,
        proposal: &ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<bool, AttackError> {
        self.schedule(
            r,
            r.owner_fence().map_err(|_| AttackError::StaleOwner)?,
            stamp,
            source,
            proposal,
            now,
        )
    }
    pub fn schedule(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &SourceCallbackCast,
        p: &ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<bool, AttackError> {
        if !self.current(r, f, stamp) {
            return Err(AttackError::StaleOwner);
        }
        if !s.current(r, p.issuer) {
            return Err(AttackError::StaleIssuer);
        }
        let sequence = s.sequence(p)?;
        oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(
            r, p,
        )?;
        self.prune(r);
        if let Some(m) = self
            .memos
            .iter()
            .find(|m| m.actor == p.issuer && m.ability == p.ability)
        {
            if sequence < m.sequence || now.get() < m.at {
                return Err(AttackError::InvalidPlan);
            }
            if sequence == m.sequence {
                return if m.proposal == *p {
                    Ok(false)
                } else {
                    Err(AttackError::InvalidPlan)
                };
            }
        }
        self.casts.retain(|c| !c.done.iter().all(|d| *d));
        if self.casts.len() >= 64 || self.memos.len() >= 1024 {
            return Err(AttackError::LedgerFull);
        }
        self.casts
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.memos
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        if self.speech.len() >= 128 {
            return Err(AttackError::LedgerFull);
        }
        self.speech
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        let warning = if s.native_parent.is_some() && s.creature == "oteryn:creature.lady_tenebris"
        {
            // Native prelude emits after the actual source teleport, at its new position.
            None
        } else {
            CallbackSpeech::cast(r, p.issuer, &s.creature, false)?
        };
        let mut anchors = Vec::new();
        anchors
            .try_reserve_exact(s.stages.len())
            .map_err(|_| AttackError::LedgerFull)?;
        for (_, stage) in &s.stages {
            anchors.push(if s.creature == "oteryn:creature.foam_stalker" {
                Some(crate::creature_damage_spell::SourcePositionAnchor::capture(
                    r, stage, p,
                )?)
            } else if s.creature == "oteryn:creature.plagirath" {
                Some(
                    crate::creature_damage_spell::SourcePositionAnchor::capture_target_variant(
                        r, stage, p,
                    )?,
                )
            } else {
                None
            });
        }
        let retained_caster = s
            .native_parent
            .as_ref()
            .map(|parent| {
                r.capture_native_retained_callback_caster(p.issuer, parent, stamp)
                    .map_err(|_| AttackError::InvalidSource)
            })
            .transpose()?;
        let token = self.next;
        let next = token.checked_add(1).ok_or(AttackError::NumericOverflow)?;
        let mut timers = self.timers.clone();
        for (i, (delay, _)) in s.stages.iter().enumerate() {
            let due = SemanticTimeMicros::from_micros(
                now.get()
                    .checked_add(
                        delay
                            .checked_mul(1000)
                            .ok_or(AttackError::NumericOverflow)?,
                    )
                    .ok_or(AttackError::NumericOverflow)?,
            );
            timers
                .schedule(
                    f,
                    stamp,
                    Family((s.index * 3 + i) as u8),
                    Key {
                        token,
                        stage: i as u8,
                    },
                    Some(p.issuer),
                    due,
                )
                .map_err(|_| AttackError::LedgerFull)?;
        }
        self.timers = timers;
        self.next = next;
        self.memos
            .retain(|m| !(m.actor == p.issuer && m.ability == p.ability));
        self.memos.push(Memo {
            actor: p.issuer,
            ability: p.ability.clone(),
            sequence,
            at: now.get(),
            token,
            proposal: p.clone(),
        });
        self.casts.push(Cast {
            token,
            source: s.clone(),
            proposal: p.clone(),
            done: vec![false; s.stages.len()],
            anchors,
            retained_caster,
            charging_removal: None,
        });
        if let Some(line) = warning {
            self.speech.push(line)
        }
        Ok(true)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Owner ABI keeps independently qualified source, current fence, exact actor and commit facts explicit"
    )]
    pub fn drain(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        v: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        reader: &mut dyn SpellWorldReader,
        clock: &impl OwnerClock,
    ) -> CallbackPulseBatch {
        self.drain_bound(
            r,
            Some(f),
            stamp,
            v,
            damage,
            &mut SingleStageReader { reader },
            clock,
            None,
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Current source callback owner shares the exact existing commit inputs"
    )]
    pub fn drain_current(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        v: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        reader: &mut dyn SpellWorldReader,
        clock: &impl OwnerClock,
    ) -> CallbackPulseBatch {
        self.drain_bound(
            r,
            None,
            stamp,
            v,
            damage,
            &mut SingleStageReader { reader },
            clock,
            None,
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "same current commit inputs with one reader per retained stage"
    )]
    pub fn drain_current_prepared(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        readers: &mut dyn CallbackStageReaders,
        clock: &impl OwnerClock,
    ) -> CallbackPulseBatch {
        self.drain_bound(runtime, None, stamp, vitals, damage, readers, clock, None)
    }
    /// Consume only original deadlines preceding a queued death. HP and all
    /// source side effects still use the actual execution clock.
    #[allow(
        clippy::too_many_arguments,
        reason = "the original deadline cutoff is distinct from the current commit clock"
    )]
    pub fn drain_current_prepared_through(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        readers: &mut dyn CallbackStageReaders,
        eligible_through: SemanticTimeMicros,
        clock: &impl OwnerClock,
    ) -> CallbackPulseBatch {
        self.drain_bound(
            runtime,
            None,
            stamp,
            vitals,
            damage,
            readers,
            clock,
            Some(eligible_through),
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Shared fence-bound callback commit implementation"
    )]
    fn drain_bound(
        &mut self,
        r: &mut ChannelRuntimeV1,
        mut f: Option<&mut ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        v: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        readers: &mut dyn CallbackStageReaders,
        clock: &impl OwnerClock,
        eligible_through: Option<SemanticTimeMicros>,
    ) -> CallbackPulseBatch {
        if !self.current(
            r,
            f.as_deref()
                .unwrap_or(r.owner_fence().map_err(|_| AttackError::StaleOwner)?),
            stamp,
        ) {
            return Err(AttackError::StaleOwner);
        }
        self.prune(r);
        self.arm_charging_removals(r, stamp)?;
        let eligible = if let Some(cutoff) = eligible_through {
            let cutoff = cutoff.min(clock.now());
            let mut eligible = Vec::new();
            eligible
                .try_reserve_exact(
                    self.casts
                        .len()
                        .checked_mul(4)
                        .ok_or(AttackError::NumericOverflow)?,
                )
                .map_err(|_| AttackError::LedgerFull)?;
            for cast in &self.casts {
                for stage in 0..cast.done.len() {
                    let key = Key {
                        token: cast.token,
                        stage: stage as u8,
                    };
                    if self
                        .timers
                        .occurrence_due(Family((cast.source.index * 3 + stage) as u8), key)
                        .is_some_and(|due| due <= cutoff)
                    {
                        eligible.push(key);
                    }
                }
                let key = Key {
                    token: cast.token,
                    stage: CHARGING_REMOVAL_STAGE,
                };
                if self
                    .timers
                    .occurrence_due(CHARGING_REMOVAL_FAMILY, key)
                    .is_some_and(|due| due <= cutoff)
                {
                    eligible.push(key);
                }
            }
            Some(eligible)
        } else {
            None
        };
        if self.speech.len() > 320 {
            return Err(AttackError::LedgerFull);
        }
        self.speech
            .try_reserve(192)
            .map_err(|_| AttackError::LedgerFull)?;
        let mut out = Vec::new();
        out.try_reserve_exact(192)
            .map_err(|_| AttackError::LedgerFull)?;
        self.removals
            .try_reserve(192)
            .map_err(|_| AttackError::LedgerFull)?;
        let casts = &self.casts;
        let fired = self.timers.drain_due_matching(
            clock,
            f.as_deref()
                .unwrap_or(r.owner_fence().map_err(|_| AttackError::StaleOwner)?),
            |actor| {
                casts.iter().any(|cast| {
                    cast.proposal.issuer == actor && cast.source.current_retained(r, actor)
                })
            },
            |_, key| {
                if eligible
                    .as_ref()
                    .is_some_and(|eligible| !eligible.contains(&key))
                {
                    return false;
                }
                if key.stage == CHARGING_REMOVAL_STAGE {
                    return true;
                }
                let id = CallbackStageId {
                    token: key.token,
                    stage: key.stage,
                };
                if readers.terminal_error(id).is_some() {
                    return true;
                }
                if !readers.ready(id) {
                    return false;
                }
                let Some(cast) = casts.iter().find(|cast| cast.token == key.token) else {
                    return true;
                };
                let stage = usize::from(key.stage);
                let Ok(proposal) = stage_proposal(cast, stage) else {
                    return true;
                };
                let Some(reader) = readers.reader(id) else {
                    return false;
                };
                if matches!(
                    damage.prepare_target_anchored(
                        r,
                        &cast.source.stages[stage].1,
                        &proposal,
                        reader,
                        stamp,
                        None,
                        clock.now(),
                        cast.anchors[stage].as_ref()
                    ),
                    Err(AttackError::MissingCombatFacts)
                ) {
                    out.push((
                        cast.proposal.issuer,
                        cast.proposal.ability.clone(),
                        stage,
                        Err(AttackError::MissingCombatFacts),
                    ));
                    return false;
                }
                true
            },
        );
        for timer in fired {
            let Some(c) = self
                .casts
                .iter_mut()
                .find(|c| c.token == timer.occurrence.token)
            else {
                continue;
            };
            if timer.occurrence.stage == CHARGING_REMOVAL_STAGE {
                if c.charging_removal.is_none() {
                    continue;
                }
                let removal = match f.as_deref_mut() {
                    Some(fence) => r.remove_native_encounter_subject(
                        fence,
                        stamp,
                        c.proposal.issuer,
                        &c.source.creature,
                        c.source.content,
                    ),
                    None => r.remove_current_native_encounter_subject(
                        stamp,
                        c.proposal.issuer,
                        &c.source.creature,
                        c.source.content,
                    ),
                };
                if removal.is_ok() {
                    c.charging_removal = None;
                } else if let Some((_, armed)) = &mut c.charging_removal {
                    *armed = false;
                }
                self.removals.push((c.proposal.issuer, removal));
                continue;
            }
            let stage = usize::from(timer.occurrence.stage);
            if c.done[stage] {
                continue;
            }
            c.done[stage] = true;
            if c.source.current_retained(r, c.proposal.issuer)
                && let Some(line) =
                    CallbackSpeech::cast(r, c.proposal.issuer, &c.source.creature, true)?
            {
                self.speech.push(line)
            }
            let charging = c.source.native_parent.is_some()
                && c.source.creature == "oteryn:creature.charging_outburst";
            let removal_due = clock
                .now()
                .get()
                .checked_add(1_000_000)
                .map(SemanticTimeMicros::from_micros);
            let result = if charging && removal_due.is_none() {
                Err(AttackError::NumericOverflow)
            } else if !c.source.current_retained(r, c.proposal.issuer) {
                Err(AttackError::StaleIssuer)
            } else if let Some(error) = readers.terminal_error(CallbackStageId {
                token: c.token,
                stage: stage as u8,
            }) {
                Err(error)
            } else {
                let p = stage_proposal(c, stage)?;
                let stage_id = CallbackStageId {
                    token: c.token,
                    stage: stage as u8,
                };
                if f.is_none()
                    && let Some(result) =
                        readers.execute_current(stage_id, r, stamp, v, clock.now())
                {
                    result
                } else {
                    let reader = readers
                        .reader(stage_id)
                        .ok_or(AttackError::MissingCombatFacts)?;
                    match f.as_deref_mut() {
                        Some(fence) => damage.apply_target_anchored(
                            r,
                            v,
                            &c.source.stages[stage].1,
                            &p,
                            reader,
                            stamp,
                            clock.now(),
                            None,
                            fence,
                            c.anchors[stage].as_ref(),
                        ),
                        None => damage.apply_target_current(
                            r,
                            v,
                            &c.source.stages[stage].1,
                            &p,
                            reader,
                            stamp,
                            clock.now(),
                            None,
                            c.anchors[stage].as_ref(),
                        ),
                    }
                }
            };
            if result.is_ok() && charging {
                c.charging_removal = removal_due.map(|due| (due, false));
            }
            // Source remove follows a successfully committed cast, never mints death/rewards.
            // Preserve the damage outcome even if independently guarded administrative removal refuses.
            if result.is_ok()
                && matches!(
                    c.source.creature.as_str(),
                    "oteryn:creature.energy_pulse" | "oteryn:creature.eruption_of_destruction"
                )
            {
                self.removals.push((
                    c.proposal.issuer,
                    match f.as_deref_mut() {
                        Some(fence) => r.remove_native_encounter_subject(
                            fence,
                            stamp,
                            c.proposal.issuer,
                            &c.source.creature,
                            c.source.content,
                        ),
                        None => r.remove_current_native_encounter_subject(
                            stamp,
                            c.proposal.issuer,
                            &c.source.creature,
                            c.source.content,
                        ),
                    },
                ));
            }
            out.push((c.proposal.issuer, c.proposal.ability.clone(), stage, result));
        }
        self.arm_charging_removals(r, stamp)?;
        Ok(out)
    }

    // A successful Combat retains its original removal deadline before arming.
    // Capacity refusal retries the same datum; it cannot repeat damage, rebase
    // the source callback, or leave the completed cast eligible for pruning.
    fn arm_charging_removals(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Result<(), AttackError> {
        let fence = runtime.owner_fence().map_err(|_| AttackError::StaleOwner)?;
        for cast in &mut self.casts {
            let Some((due, armed)) = &mut cast.charging_removal else {
                continue;
            };
            if *armed {
                continue;
            }
            match self.timers.schedule(fence, stamp, CHARGING_REMOVAL_FAMILY,
                Key { token: cast.token, stage: CHARGING_REMOVAL_STAGE },
                Some(cast.proposal.issuer), *due) {
                Ok(()) => *armed = true,
                Err(oteryn_gameplay_foundation::foundation::owner_timer::OwnerTimerError::PendingLimitReached) => {},
                Err(_) => return Err(AttackError::InvalidPlan),
            }
        }
        Ok(())
    }
}
