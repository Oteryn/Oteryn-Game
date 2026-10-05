//! Existing CREATURE-AI-0 proposal → current Channel HP/condition owner. No new schedule/RNG owner.
use std::collections::BTreeMap;
#[path = "source_callback_cast.rs"]
mod callback_cast;
pub(crate) use callback_cast::{
    CallbackCastOwner, CallbackPulseBatch, CallbackSpeech, SourceCallbackCast,
};
#[path = "creature_damage_composed.rs"]
mod composed;
#[path = "creature_damage_spell_delayed.rs"]
mod delayed;
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord, ProjectV2AbilityArea, ProjectV2AbilityEffect,
    ProjectV2AbilityKind, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2FormulaAuthoring, ProjectV2Magnitude,
};
use crate::creature_attack_geometry::{self as geometry, Facing};
use crate::creature_auto_attack::{AttackError, AttackFacts, CurrentConditionPolicy};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ApplicationFacts, ConditionDefinition};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals, PlayerManaDrainReceipt};
pub(crate) use composed::{
    ComposedOutcome, ComposedOwner, ComposedSource, ComposedWorldReader, CreatureCombatFacts,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
#[derive(Debug, Clone)]
pub(crate) struct SpellSource {
    list: ScheduleList,
    creature_key: String,
    pub(crate) ability: Ref,
    entry_index: usize,
    minimum: u32,
    maximum: u32,
    critical: Option<crate::source_critical_baseline::SourceCriticalBaseline>,
    conditions: Vec<ConditionDefinition>,
    appearances: Vec<crate::content::ProjectV2InlineEffect>,
    appearance_cast: bool,
    formula_revision: Option<String>,
    fingerprint: [u8; 32],
    content_digest: [u8; 32],
    element: Option<String>,
    armor: bool,
    range: u16,
    magnitude: Option<ProjectV2Magnitude>,
    area: Option<ProjectV2AbilityArea>,
    chain: Option<crate::content::ProjectV2Chain>,
    presentation: Option<Box<crate::content::ProjectV2AbilityDetails>>,
    source_none_correction: bool,
    dispel_invisible: bool,
    variants: Vec<Result<SpellSource, AttackError>>,
    variant_child: Option<Ref>,
    windup: Option<Box<crate::content::ProjectV2AbilityDetails>>,
    needs_target: bool,
    needs_direction: bool,
}
impl SpellSource {
    pub(crate) fn from_native(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        content_digest: [u8; 32],
    ) -> Result<Self, AttackError> {
        Self::from_native_branch(
            creature,
            index,
            records,
            profiles,
            content_digest,
            None,
            None,
            ScheduleList::Attack,
        )
    }
    /// Appearance-only defense retains its native schedule and source body; it mutates
    /// the existing physical creature condition owner rather than a player roster.
    pub(crate) fn from_native_appearance_defense(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        digest: [u8; 32],
    ) -> Result<Self, AttackError> {
        let source = Self::from_native_branch(
            creature,
            index,
            records,
            profiles,
            digest,
            None,
            None,
            ScheduleList::Defence,
        )?;
        if source.appearances.len() != 1
            || !source.conditions.is_empty()
            || source.minimum != 0
            || source.maximum != 0
            || source.needs_direction
            || source.range != 0
        {
            return Err(AttackError::UnsupportedShape);
        }
        Ok(source)
    }
    pub(crate) fn has_appearance_area(&self) -> bool {
        !self.appearances.is_empty() && self.area.is_some()
    }
    pub(crate) fn current_appearance_conditions(
        &self,
        runtime: &ChannelRuntimeV1,
        content: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Result<Vec<ConditionDefinition>, AttackError> {
        if self.content_digest != content.source_digest()
            || runtime.content_pin().server_artifact_digest() != self.content_digest
        {
            return Err(AttackError::ContentChanged);
        }
        self.appearances
            .iter()
            .map(|effect| {
                crate::creature_appearance_content::lower_current_appearance(
                    runtime, content, effect,
                )
                .map_err(|_| AttackError::InvalidSource)
            })
            .collect()
    }
    /// Private selected source Defense extension, preserving the native parent list.
    /// Its exact31 child source bodies must match, not merely the generic leaf type.
    pub(crate) fn from_native_defense(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        content_digest: [u8; 32],
    ) -> Result<Self, AttackError> {
        if index != 1
            || !matches!(
                creature.key.as_str(),
                "oteryn:creature.the_sinister_hermit" | "oteryn:creature.the_sinister_hermit_dirty"
            )
        {
            return Err(AttackError::UnsupportedShape);
        }
        let expected: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_str(DEFENSE_VARIANT_SOURCE_BODY)
                .map_err(|_| AttackError::InvalidSource)?;
        for source in &expected {
            let actual = profiles
                .iter()
                .find(|p| p.target == source.target)
                .ok_or(AttackError::InvalidSource)?;
            if actual != source {
                return Err(AttackError::InvalidSource);
            }
        }
        let prepared = Self::from_native_branch(
            creature,
            index,
            records,
            profiles,
            content_digest,
            None,
            None,
            ScheduleList::Defence,
        )?;
        if prepared.ability.key != "oteryn:ability.spell.shock_head_skill_reducer_2"
            || prepared.variants.len() != 31
            || prepared.variants.iter().any(|c| c.is_err())
        {
            return Err(AttackError::InvalidSource);
        }
        Ok(prepared)
    }
    /// Private composed-owner seam. The full parent remains bound to its Creature and
    /// native record; this does not qualify a whole multi-effect spell by its primary.
    fn from_native_executable(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        content_digest: [u8; 32],
        executable_index: usize,
    ) -> Result<Self, AttackError> {
        Self::from_native_branch(
            creature,
            index,
            records,
            profiles,
            content_digest,
            None,
            Some(executable_index),
            ScheduleList::Attack,
        )
    }
    // Keep from_native_branch source/owner ABI explicit: exact actor/source binding, occurrence, immutable definition and separately owned runtime/policy facts must not be conflated.
    #[allow(clippy::too_many_arguments)]
    fn from_native_branch(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        content_digest: [u8; 32],
        branch: Option<usize>,
        executable_index: Option<usize>,
        list: ScheduleList,
    ) -> Result<Self, AttackError> {
        let mut map = BTreeMap::new();
        for p in profiles {
            if map.insert(p.target.clone(), &p.data).is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        if creature.family != ProjectV2Family::Creature {
            return Err(AttackError::InvalidSource);
        }
        let record=records.iter().find(|r| matches!(r,ProjectReferenceRecord::Creature{identity,..} if identity.key==creature.key && identity.revision==creature.revision)).ok_or(AttackError::InvalidSource)?;
        let ProjectReferenceRecord::Creature { behavior, .. } = record else {
            unreachable!()
        };
        let bref = Ref {
            family: ProjectV2Family::Behavior,
            key: behavior.key.clone(),
            revision: behavior.revision.clone(),
        };
        let Some(Data::Behavior(b)) = map.get(&bref).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let entries = match list {
            ScheduleList::Attack => {
                if index >= 16 || b.attacks.len() > 16 {
                    return Err(AttackError::InvalidSource);
                };
                &b.attacks
            }
            ScheduleList::Defence => {
                if index >= 8 || b.defenses.len() > 8 {
                    return Err(AttackError::InvalidSource);
                };
                &b.defenses
            }
        };
        let e = entries.get(index).ok_or(AttackError::InvalidSource)?;
        let Some(Data::Ability(parent)) = map.get(&e.ability).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let parent_details = parent
            .details
            .as_deref()
            .ok_or(AttackError::InvalidSource)?;
        let expected = match e.ability.key.as_str() {
            "oteryn:ability.spell.barbarian_brutetamer_skill_reducer" => 10,
            "oteryn:ability.spell.betrayed_wraith_skill_reducer" => 20,
            "oteryn:ability.spell.cliff_strider_skill_reducer" => 31,
            "oteryn:ability.spell.deepling_spellsinger_skill_reducer" => 31,
            "oteryn:ability.spell.demon_outcast_skill_reducer" => 6,
            "oteryn:ability.spell.diabolic_imp_skill_reducer" => 16,
            "oteryn:ability.spell.dreadbeast_skill_reducer" => 11,
            "oteryn:ability.spell.enslaved_dwarf_skill_reducer_2" => 41,
            "oteryn:ability.spell.feversleep_skill_reducer" => 26,
            "oteryn:ability.spell.glooth_fairy_skill_reducer" => 21,
            "oteryn:ability.spell.hirintror_skill_reducer" => 21,
            "oteryn:ability.spell.ice_golem_skill_reducer" => 21,
            "oteryn:ability.spell.lord_azaram_wave" => 2,
            "oteryn:ability.spell.pirate_corsair_skill_reducer" => 31,
            "oteryn:ability.spell.shock_head_skill_reducer_2" => 31,
            "oteryn:ability.spell.silencer_skill_reducer" => 31,
            "oteryn:ability.spell.stampor_skill_reducer" => 26,
            "oteryn:ability.spell.timira_explosion" => 2,
            "oteryn:ability.spell.timira_fire_ring" => 3,
            "oteryn:ability.spell.tyrn_skill_reducer" => 16,
            "oteryn:ability.spell.undead_dragon_curse" => 2,
            "oteryn:ability.spell.warlock_skill_reducer" => 11,
            "oteryn:ability.spell.werewolf_skill_reducer" => 21,
            "oteryn:ability.spell.white_weretiger_ice_ring" => 3,
            _ => 0,
        };
        if !parent_details.variants.is_empty() {
            if expected == 0
                || parent_details.variants.len() != expected
                || parent_details.kind != ProjectV2AbilityKind::Spell
                || parent_details.windup.is_some()
                || parent_details.encounter.is_some()
                || parent_details.chain.is_some()
                || !parent_details.effects.is_empty()
            {
                return Err(AttackError::UnsupportedShape);
            }
            for (i, child) in parent_details.variants.iter().enumerate() {
                if child.family!=ProjectV2Family::Ability||child.revision!=e.ability.revision
                    ||child.key!=format!("{}.variant-{}",e.ability.key,i+1)
                    ||!records.iter().any(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.family=="Ability"&&identity.key==child.key&&identity.revision==child.revision))
                    ||!matches!(map.get(child).copied(),Some(Data::Ability(_))){return Err(AttackError::InvalidSource)}
            }
            if branch.is_none() {
                let mut branches = Vec::new();
                for i in 0..expected {
                    let prepared = Self::from_native_branch(
                        creature,
                        index,
                        records,
                        profiles,
                        content_digest,
                        Some(i),
                        None,
                        list,
                    );
                    if let Err(error) = &prepared
                        && *error != AttackError::UnsupportedShape
                    {
                        return Err(*error);
                    }
                    branches.push(prepared);
                }
                let mut host = branches
                    .iter()
                    .find_map(|r| r.as_ref().ok())
                    .cloned()
                    .ok_or(AttackError::UnsupportedShape)?;
                let child_profiles = parent_details
                    .variants
                    .iter()
                    .map(|r| map.get(r).copied())
                    .collect::<Vec<_>>();
                host.fingerprint = Sha256::digest(
                    serde_json::to_vec(&(
                        creature,
                        list == ScheduleList::Defence,
                        index,
                        e,
                        parent,
                        child_profiles,
                        content_digest,
                        "SOURCE_UNIFORM_ONCE_PER_PARENT_CAST_PROJECT_DETERMINISTIC_RNG",
                    ))
                    .map_err(|_| AttackError::InvalidSource)?,
                )
                .into();
                host.range = e.range_tiles.unwrap_or(parent_details.range_tiles);
                host.variant_child = None;
                host.variants = branches;
                return Ok(host);
            }
        } else if branch.is_some() {
            return Err(AttackError::InvalidSource);
        }
        let selected = match branch {
            Some(i) => parent_details
                .variants
                .get(i)
                .ok_or(AttackError::InvalidSource)?,
            None => &e.ability,
        };
        let Some(Data::Ability(a)) = map.get(selected).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let d = a.details.as_deref().ok_or(AttackError::InvalidSource)?;
        if d.kind != ProjectV2AbilityKind::Spell || !d.variants.is_empty() || d.encounter.is_some()
        {
            return Err(AttackError::UnsupportedShape);
        }
        let windup = if let Some(w) = &d.windup {
            if e.ability.key != "oteryn:ability.spell.soulwars_fear"
                || !matches!(
                    creature.key.as_str(),
                    "oteryn:creature.bony_sea_devil"
                        | "oteryn:creature.goshnar_s_megalomania_green"
                        | "oteryn:creature.goshnar_s_megalomania_purple"
                        | "oteryn:creature.goshnar_s_spite"
                        | "oteryn:creature.hazardous_phantom"
                        | "oteryn:creature.turbulent_elemental"
                )
                || w.delay_ms != 2000
                || w.caster_asset_binding.as_str() != "canary.appearance:effect/ghost_smoke"
                || !d.needs_target
                || d.needs_direction
                || d.area.is_some()
                || d.chain.is_some()
                || d.effects.len() != 1
                || !matches!(&d.effects[0],ProjectV2AbilityEffect::Inline(e) if matches!(&e.operation,crate::content::ProjectV2InlineEffectOperation::Condition{duration_ms:Some(3000),condition} if condition.condition_type=="feared"))
            {
                return Err(AttackError::UnsupportedShape);
            }
            Some(Box::new(d.clone()))
        } else {
            None
        };
        if d.chain.as_ref().is_some_and(|c| {
            c.backtracking || c.max_targets == 0 || c.max_targets > 63 || d.area.is_some()
        }) {
            return Err(AttackError::UnsupportedShape);
        }
        let mut executable = Vec::new();
        let mut conditions = Vec::new();
        let mut appearances = Vec::new();
        let mut presentation_only = false;
        let dispel_invisible = e.ability.key == "oteryn:ability.spell.djinn_cancel_invisibility"
            && matches!(
                creature.key.as_str(),
                "oteryn:creature.blue_djinn" | "oteryn:creature.green_djinn"
            )
            && d.variants.is_empty()
            && d.windup.is_none()
            && d.encounter.is_none()
            && !d.needs_target
            && !d.needs_direction
            && d.range_tiles == 0
            && d.chain.is_none()
            && d.area
                == Some(ProjectV2AbilityArea::Matrix {
                    north: vec![
                        "..xxx..".into(),
                        ".xxxxx.".into(),
                        "xxxxxxx".into(),
                        "xxxCxxx".into(),
                        "xxxxxxx".into(),
                        ".xxxxx.".into(),
                        "..xxx..".into(),
                    ],
                    diagonal: Vec::new(),
                })
            && d.effects.len() == 1
            && matches!(&d.effects[0],ProjectV2AbilityEffect::Inline(inline) if matches!(&inline.operation,crate::content::ProjectV2InlineEffectOperation::RemoveCondition{condition} if condition=="invisible") && inline.presentation.is_none());
        for effect in &d.effects {
            match effect {
                ProjectV2AbilityEffect::Executable(r) => executable.push(r),
                ProjectV2AbilityEffect::Inline(inline) => {
                    if dispel_invisible {
                        continue;
                    }
                    if matches!(
                        inline.operation,
                        crate::content::ProjectV2InlineEffectOperation::PresentationOnly
                    ) {
                        presentation_only = true;
                        continue;
                    }
                    if matches!(
                        inline.operation,
                        crate::content::ProjectV2InlineEffectOperation::AppearanceTransform { .. }
                    ) {
                        if d.effects.len() != 1
                            || d.chain.is_some()
                            || d.windup.is_some()
                            || !d.variants.is_empty()
                        {
                            return Err(AttackError::UnsupportedShape);
                        }
                        appearances.push(inline.as_ref().clone());
                        continue;
                    }
                    let revision = e
                        .ability
                        .revision
                        .strip_prefix("definition-r")
                        .and_then(|r| r.parse::<u32>().ok())
                        .filter(|r| *r > 0)
                        .ok_or(AttackError::InvalidSource)?;
                    let resolved_formula = match &inline.operation {
                        crate::content::ProjectV2InlineEffectOperation::Condition {
                            condition,
                            ..
                        } => match &condition.speed_formula {
                            Some(reference) => {
                                let Some(Data::Formula(formula)) = map.get(reference).copied()
                                else {
                                    return Err(AttackError::InvalidSource);
                                };
                                Some((reference, formula))
                            }
                            None => None,
                        },
                        _ => None,
                    };
                    let definition = crate::creature_condition_content::lower_condition_definition(
                        inline,
                        revision,
                        resolved_formula,
                    )
                    .map_err(|_| AttackError::UnsupportedShape)?;
                    conditions.push(definition);
                }
            }
        }
        if d.chain.is_some() && !conditions.is_empty() {
            return Err(AttackError::UnsupportedShape);
        }
        if executable.is_empty()
            && (!conditions.is_empty()
                || !appearances.is_empty()
                || presentation_only
                || dispel_invisible)
        {
            let Some(Data::Creature(cp)) = map.get(creature).copied() else {
                return Err(AttackError::InvalidSource);
            };
            if !cp.abilities.contains(&e.ability) || !records.iter().any(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.key==e.ability.key&&identity.revision==e.ability.revision&&identity.family=="Ability")){return Err(AttackError::InvalidSource)}
            let fingerprint = Sha256::digest(
                serde_json::to_vec(&(
                    creature,
                    list == ScheduleList::Defence,
                    index,
                    e,
                    a,
                    content_digest,
                ))
                .map_err(|_| AttackError::InvalidSource)?,
            )
            .into();
            return Ok(Self {
                list,
                creature_key: creature.key.clone(),
                ability: e.ability.clone(),
                entry_index: index,
                minimum: 0,
                maximum: 0,
                critical: None,
                conditions,
                appearance_cast: !appearances.is_empty(),
                appearances: appearances.clone(),
                formula_revision: None,
                fingerprint,
                content_digest,
                element: None,
                armor: false,
                range: e.range_tiles.unwrap_or(d.range_tiles),
                magnitude: e.magnitude,
                area: d.area.clone(),
                chain: d.chain.clone(),
                presentation: ((presentation_only || !appearances.is_empty())
                    && executable_index.is_none())
                .then(|| Box::new(d.clone())),
                source_none_correction: false,
                dispel_invisible,
                variants: Vec::new(),
                variant_child: branch.map(|_| selected.clone()),
                windup,
                needs_target: d.needs_target,
                needs_direction: d.needs_direction,
            });
        }
        let effect = match executable_index {
            Some(i)
                if branch.is_none()
                    && d.variants.is_empty()
                    && d.windup.is_none()
                    && d.chain.is_none()
                    && conditions.is_empty() =>
            {
                *executable.get(i).ok_or(AttackError::InvalidSource)?
            }
            Some(_) => return Err(AttackError::UnsupportedShape),
            None => {
                let [effect] = executable.as_slice() else {
                    return Err(AttackError::UnsupportedShape);
                };
                *effect
            }
        };
        if effect.family != ProjectV2Family::Effect
            || e.ability.family != ProjectV2Family::Ability
            || behavior.family != "Behavior"
        {
            return Err(AttackError::InvalidSource);
        }
        let Some(Data::Creature(cp)) = map.get(creature).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let critical = crate::source_critical_baseline::SourceCriticalBaseline::qualify(
            &creature.key,
            cp.details.as_ref().map_or(0, |d| d.critical_chance_ppm),
        )
        .map_err(|_| AttackError::UnsupportedShape)?;
        if !cp.abilities.contains(&e.ability) {
            return Err(AttackError::InvalidSource);
        }
        let ar=records.iter().find(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..} if identity.key==selected.key && identity.revision==selected.revision && identity.family=="Ability")).ok_or(AttackError::InvalidSource)?;
        let ProjectReferenceRecord::Ability { effects, .. } = ar else {
            unreachable!()
        };
        if !effects
            .iter()
            .any(|r| r.family == "Effect" && r.key == effect.key && r.revision == effect.revision)
        {
            return Err(AttackError::InvalidSource);
        }
        let er=records.iter().find(|r| matches!(r,ProjectReferenceRecord::Effect{identity,..} if identity.key==effect.key && identity.revision==effect.revision)).ok_or(AttackError::InvalidSource)?;
        let ProjectReferenceRecord::Effect {
            effect_family: EffectFamilyDocument::Damage,
            formula,
            ..
        } = er
        else {
            return Err(AttackError::UnsupportedShape);
        };
        let Some(Data::Effect(ep)) = map.get(effect).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let known_none_source = creature.key == "oteryn:creature.angry_sugar_fairy"
            && matches!(index, 1 | 2)
            && ep.damage_type == "untyped";
        if !known_none_source
            && !matches!(
                ep.damage_type.as_str(),
                "physical"
                    | "fire"
                    | "earth"
                    | "energy"
                    | "ice"
                    | "holy"
                    | "death"
                    | "drowning"
                    | "life_drain"
                    | "mana_drain"
            )
            || ep.affects.is_some()
            || ep
                .mitigated_by
                .contains(&crate::content::ProjectV2Mitigation::Shield)
        {
            return Err(AttackError::UnsupportedShape);
        }
        if formula.family != "Formula" {
            return Err(AttackError::InvalidSource);
        }
        let fref = Ref {
            family: ProjectV2Family::Formula,
            key: formula.key.clone(),
            revision: formula.revision.clone(),
        };
        let Some(Data::Formula(f)) = map.get(&fref).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let (minimum, maximum) = match f {
            ProjectV2FormulaAuthoring::Range { minimum, maximum } => (*minimum, *maximum),
            // Exact integer ceil((skill*attack)/20+attack/2), cached Canary weapons.cpp.
            ProjectV2FormulaAuthoring::MeleeAttackSkill { attack, skill } => {
                crate::creature_auto_attack::canonical_melee_bounds(*attack, *skill)?
            }
            ProjectV2FormulaAuthoring::CasterMagnitude => {
                let m = e.magnitude.as_ref().ok_or(AttackError::InvalidSource)?;
                (m.minimum, m.maximum)
            }
            _ => return Err(AttackError::UnsupportedShape),
        };
        if ep.damage_type == "mana_drain" && !conditions.is_empty() {
            return Err(AttackError::UnsupportedShape);
        }
        if minimum > maximum || e.interval_ms == 0 || e.chance_ppm > 1_000_000 {
            return Err(AttackError::InvalidSource);
        }
        let minimum = u32::try_from(minimum).map_err(|_| AttackError::NumericOverflow)?;
        let maximum = u32::try_from(maximum).map_err(|_| AttackError::NumericOverflow)?;
        if known_none_source {
            let expected = if index == 1 { (100, 230) } else { (130, 280) };
            if (minimum, maximum) != expected
                || ep.affects.is_some()
                || !ep.mitigated_by.is_empty()
                || d.chain.is_some()
                || !conditions.is_empty()
            {
                return Err(AttackError::UnsupportedShape);
            }
            // Pinned Canary47df Fairy spells omit COMBAT type. Native COMBAT_NONE goes to
            // doCombatDefault/CombatNullFunc; min/max fields are NOT HP damage authority.
            let mut corrected = d.clone();
            corrected.effects = vec![ProjectV2AbilityEffect::Inline(Box::new(
                crate::content::ProjectV2InlineEffect {
                    key: effect.key.clone(),
                    operation: crate::content::ProjectV2InlineEffectOperation::PresentationOnly,
                    presentation: ep.presentation.clone(),
                },
            ))];
            let fingerprint = Sha256::digest(
                serde_json::to_vec(&(
                    creature,
                    list == ScheduleList::Defence,
                    index,
                    e,
                    a,
                    ep,
                    f,
                    content_digest,
                    "SOURCE_COMBAT_NONE_NO_HP_CORRECTION",
                ))
                .map_err(|_| AttackError::InvalidSource)?,
            )
            .into();
            return Ok(Self {
                list,
                creature_key: creature.key.clone(),
                ability: e.ability.clone(),
                entry_index: index,
                minimum: 0,
                maximum: 0,
                critical: None,
                conditions: Vec::new(),
                appearances: Vec::new(),
                appearance_cast: false,
                formula_revision: Some(fref.revision.clone()),
                fingerprint,
                content_digest,
                element: None,
                armor: false,
                range: e.range_tiles.unwrap_or(d.range_tiles),
                magnitude: e.magnitude,
                area: d.area.clone(),
                chain: None,
                presentation: Some(Box::new(corrected)),
                source_none_correction: true,
                dispel_invisible: false,
                variants: Vec::new(),
                variant_child: None,
                windup: None,
                needs_target: d.needs_target,
                needs_direction: d.needs_direction,
            });
        }
        let fingerprint = if let Some(policy) = critical {
            Sha256::digest(
                serde_json::to_vec(&(
                    creature,
                    list == ScheduleList::Defence,
                    index,
                    e,
                    a,
                    ep,
                    f,
                    content_digest,
                    policy.fingerprint_parts(),
                ))
                .map_err(|_| AttackError::InvalidSource)?,
            )
            .into()
        } else {
            Sha256::digest(
                serde_json::to_vec(&(
                    creature,
                    list == ScheduleList::Defence,
                    index,
                    e,
                    a,
                    ep,
                    f,
                    content_digest,
                ))
                .map_err(|_| AttackError::InvalidSource)?,
            )
            .into()
        };
        Ok(Self {
            list,
            creature_key: creature.key.clone(),
            ability: e.ability.clone(),
            entry_index: index,
            minimum,
            maximum,
            critical,
            conditions,
            appearance_cast: !appearances.is_empty(),
            appearances,
            formula_revision: Some(fref.revision.clone()),
            fingerprint,
            content_digest,
            element: Some(ep.damage_type.clone()),
            armor: ep
                .mitigated_by
                .contains(&crate::content::ProjectV2Mitigation::Armor),
            range: e.range_tiles.unwrap_or(d.range_tiles),
            magnitude: e.magnitude,
            area: d.area.clone(),
            chain: d.chain.clone(),
            presentation: None,
            source_none_correction: false,
            dispel_invisible,
            variants: Vec::new(),
            variant_child: branch.map(|_| selected.clone()),
            windup,
            needs_target: d.needs_target,
            needs_direction: d.needs_direction,
        })
    }
    pub(crate) fn validate_chain_proposal(
        &self,
        proposal: &ProfileAbilityProposal,
    ) -> Result<u64, AttackError> {
        if proposal.list != ScheduleList::Attack
            || proposal.ability != self.ability
            || proposal.entry_index != self.entry_index
            || proposal.range_tiles != self.range
            || proposal.magnitude != self.magnitude
            || self
                .formula_revision
                .as_ref()
                .is_some_and(|r| proposal.occurrence.revisions().formula() != r)
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(&proposal.issuer.placement_identity()));
        let tail = proposal
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (n, suffix) = tail.split_once(':').ok_or(AttackError::InvalidPlan)?;
        if suffix != format!("attack:{}", self.entry_index) {
            return Err(AttackError::InvalidPlan);
        }
        n.parse::<u64>().map_err(|_| AttackError::InvalidPlan)
    }
    pub(crate) fn chain_details(&self) -> Option<&crate::content::ProjectV2Chain> {
        self.chain.as_ref()
    }
    pub(crate) fn chain_binding(&self) -> (&str, usize, [u8; 32], [u8; 32]) {
        (
            &self.creature_key,
            self.entry_index,
            self.fingerprint,
            self.content_digest,
        )
    }
    pub(crate) fn range_tiles(&self) -> u16 {
        self.range
    }
    pub(crate) fn element(&self) -> Option<&str> {
        self.element.as_deref()
    }
}
/// Independent current Combat owner snapshot, never inferred from the source artifact.
#[derive(Clone)]
pub(crate) struct SpellCombatFacts {
    pub(crate) attack: AttackFacts,
    /// Elemental resistance only. Excludes armor/shield, already-authored base formula and primary damage draw.
    pub(crate) multiplier_ppm: u32,
    pub(crate) immune: bool,
    pub(crate) condition_policy: Option<CurrentConditionPolicy>,
}
pub(crate) trait SpellWorldReader {
    /// Actual immutable current loader owner. Missing membership fails closed before
    /// RNG, cast occurrence or condition mutation; source JSON is never this owner.
    fn native_gameplay<'a>(
        &'a self,
        _runtime: &ChannelRuntimeV1,
    ) -> Option<&'a crate::content::native_gameplay::NativeGameplayState> {
        None
    }

    /// Must enumerate current committed player ActorRefs/sessions from the same locked runtime.
    /// None refuses the entire cast. NPC damage belongs to the separate creature HP owner.
    fn current_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>>;
    fn current_facing(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing>;
    /// Current tile combat/LOS/floor-change policy, even if this tile has no player.
    fn tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        x: i32,
        y: i32,
        floor: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn combat(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellTargetOutcome {
    pub(crate) damage: Option<PlayerDamageReceipt>,
    pub(crate) mana: Option<PlayerManaDrainReceipt>,
    pub(crate) condition_only_applied: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourcePresentationEvent {
    pub(crate) issuer: ExactActorRef,
    pub(crate) ability: Ref,
    pub(crate) occurrence: String,
    pub(crate) content_digest: [u8; 32],
    pub(crate) stamp: RuntimeWorkStamp,
    pub(crate) tiles: Vec<(i32, i32, i16)>,
    pub(crate) source: Box<crate::content::ProjectV2AbilityDetails>,
    pub(crate) qualification: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceVariantReceipt {
    pub(crate) parent: Ref,
    pub(crate) child: Ref,
    pub(crate) index: usize,
    pub(crate) qualification: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct DelayedCastReceipt {
    pub(crate) due: SemanticTimeMicros,
    pub(crate) qualification: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellOutcome {
    pub(crate) delayed_cast: Option<DelayedCastReceipt>,
    pub(crate) source_variant: Option<SourceVariantReceipt>,
    pub(crate) presentation: Option<SourcePresentationEvent>,
    pub(crate) requested: u32,
    pub(crate) source_critical: Option<crate::source_critical_baseline::SourceCriticalEvent>,
    /// Explicit PROJECT_ARMOR_ONLY_ATTACK1_APPROXIMATION / GlobalUnverified: native armor formula, current ATTACK1 budget semantics.
    pub(crate) armor_only_attack1_approximation: bool,
    pub(crate) targets: Vec<(ExactActorRef, Result<SpellTargetOutcome, AttackError>)>,
}
/// Sealed, one-use read-only preparation. Every target policy, draw and owned
/// occurrence string is resolved before a composed caller starts any HP mutation.
struct PreparedPlayerCast {
    result: SpellOutcome,
    targets: Vec<PreparedPlayerTarget>,
}
struct PreparedPlayerTarget {
    target: ExactActorRef,
    session: GameSessionId,
    facts: SpellCombatFacts,
    amount: u32,
    blocked: bool,
    child_id: DecisionOccurrenceId,
    occurrence: String,
}
struct CastMemo {
    issuer: ExactActorRef,
    list: ScheduleList,
    entry: usize,
    sequence: u64,
    at: SemanticTimeMicros,
    binding: [u8; 32],
    result: Result<SpellOutcome, AttackError>,
}
#[derive(Default)]
pub(crate) struct DamageSpellOwner {
    casts: Vec<CastMemo>,
    delayed: Option<delayed::DelayedOwner>,
}
/// Narrow immutable loader context; every mutable world fact still comes from its owner.
struct NativeAppearanceReader<'a> {
    inner: &'a mut dyn SpellWorldReader,
    native: Option<&'a crate::content::native_gameplay::NativeGameplayState>,
}
impl SpellWorldReader for NativeAppearanceReader<'_> {
    fn native_gameplay<'a>(
        &'a self,
        r: &ChannelRuntimeV1,
    ) -> Option<&'a crate::content::native_gameplay::NativeGameplayState> {
        self.native.or_else(|| self.inner.native_gameplay(r))
    }
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        s: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        self.inner.current_players(r, s)
    }
    fn current_facing(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        s: RuntimeWorkStamp,
    ) -> Option<Facing> {
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
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        a: ExactActorRef,
        t: ExactActorRef,
        g: GameSessionId,
        e: Option<&str>,
        s: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        self.inner.combat(r, a, t, g, e, s)
    }
}
impl DamageSpellOwner {
    // Keep independently current runtime/fence, loaded content, source/proposal and owner facts explicit at this native composition boundary.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_with_native(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        now: SemanticTimeMicros,
        native: Option<&crate::content::native_gameplay::NativeGameplayState>,
    ) -> Result<SpellOutcome, AttackError> {
        self.execute(
            runtime,
            fence,
            stamp,
            vitals,
            source,
            proposal,
            &mut NativeAppearanceReader {
                inner: reader,
                native,
            },
            now,
        )
    }

    /// Called only for a real existing ProfileScheduleState prepared proposal. Timing/chance
    /// remain in that owner. This consumer rechecks source binding and current world authority.
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        now: SemanticTimeMicros,
    ) -> Result<SpellOutcome, AttackError> {
        if source.chain.is_some() {
            return Err(AttackError::UnsupportedShape);
        }
        let b = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if runtime.content_pin().server_artifact_digest() != source.content_digest {
            return Err(AttackError::ContentChanged);
        }
        if !runtime.matches_live_creature_identity(proposal.issuer, source.creature_key.as_bytes())
        {
            return Err(AttackError::StaleIssuer);
        }
        if proposal.list != source.list
            || proposal.ability != source.ability
            || proposal.entry_index != source.entry_index
            || proposal.range_tiles != source.range
            || proposal.magnitude != source.magnitude
            || source
                .formula_revision
                .as_ref()
                .is_some_and(|r| proposal.occurrence.revisions().formula() != r)
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(&proposal.issuer.placement_identity()));
        let tail = proposal
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (sequence, suffix) = tail.split_once(':').ok_or(AttackError::InvalidPlan)?;
        let sequence = sequence
            .parse::<u64>()
            .map_err(|_| AttackError::InvalidPlan)?;
        if suffix
            != format!(
                "{}:{}",
                if source.list == ScheduleList::Attack {
                    "attack"
                } else {
                    "defence"
                },
                source.entry_index
            )
        {
            return Err(AttackError::InvalidPlan);
        }
        let resolved;
        let source = if source.appearances.is_empty() {
            source
        } else {
            let content = reader
                .native_gameplay(runtime)
                .ok_or(AttackError::MissingCombatFacts)?;
            let definitions = source.current_appearance_conditions(runtime, content)?;
            resolved = {
                let mut current = source.clone();
                current.conditions = definitions;
                current.appearances.clear();
                current
            };
            &resolved
        };
        let binding = Sha256::digest(
            serde_json::to_vec(&(source.fingerprint, format!("{proposal:?}")))
                .map_err(|_| AttackError::InvalidPlan)?,
        )
        .into();
        self.casts
            .retain(|m| runtime.contains_live_creature(m.issuer));
        let index = self.casts.iter().position(|m| {
            m.issuer == proposal.issuer && m.list == source.list && m.entry == source.entry_index
        });
        if let Some(i) = index {
            let m = &self.casts[i];
            if sequence == m.sequence {
                return if binding == m.binding {
                    m.result.clone()
                } else {
                    Err(AttackError::OccurrenceConflict)
                };
            }
            if sequence < m.sequence {
                return Err(AttackError::OccurrenceSuperseded);
            }
            if now <= m.at {
                return Err(AttackError::NotDue);
            }
        }
        if index.is_none() {
            if self.casts.len() >= 64 * 16 {
                return Err(AttackError::LedgerFull);
            }
            self.casts
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
        }
        let result = if source.windup.is_some() {
            self.schedule_windup(runtime, fence, stamp, source, proposal, reader, now)
        } else if source.variants.is_empty() {
            self.apply(runtime, vitals, source, proposal, reader, stamp, now, fence)
        } else {
            let root = GameplayDecisionRoot::from_bytes(source.content_digest);
            let digest = Sha256::new()
                .chain_update(b"oteryn:creature-source-variant:v1")
                .chain_update(proposal.occurrence.id().as_str().as_bytes())
                .finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&digest[..16]);
            let decision = deterministic_decision_u64(
                &root,
                DecisionOccurrenceId::from_bytes(id),
                "source_variant_once_per_cast",
                source.entry_index as u64,
            )
            .map_err(|_| AttackError::InvalidPlan)?;
            let selected = usize::try_from(crate::spell::uniform_draw(
                decision,
                0,
                (source.variants.len() - 1) as i64,
            ))
            .map_err(|_| AttackError::InvalidPlan)?;
            match &source.variants[selected] {
                Err(error) => Err(*error),
                Ok(child) => {
                    let selected_child = child
                        .variant_child
                        .clone()
                        .ok_or(AttackError::InvalidSource)?;
                    self.apply(runtime,vitals,child,proposal,reader,stamp,now, fence).map(|mut result|{
     result.source_variant=Some(SourceVariantReceipt{parent:source.ability.clone(),child:selected_child,index:selected,qualification:"SOURCE_UNIFORM_SELECTOR_PROJECT_DETERMINISTIC_RNG_GLOBAL_UNVERIFIED"});result
    })
                }
            }
        };
        let memo = CastMemo {
            issuer: proposal.issuer,
            list: source.list,
            entry: source.entry_index,
            sequence,
            at: now,
            binding,
            result: result.clone(),
        };
        match index {
            Some(i) => self.casts[i] = memo,
            None => self.casts.push(memo),
        }
        result
    }

    // Keep schedule_windup ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn schedule_windup(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &SpellSource,
        p: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        now: SemanticTimeMicros,
    ) -> Result<SpellOutcome, AttackError> {
        let due = SemanticTimeMicros::from_micros(
            now.get()
                .checked_add(2_000_000)
                .ok_or(AttackError::NumericOverflow)?,
        );
        let players = reader
            .current_players(r, stamp)
            .ok_or(AttackError::MissingCombatFacts)?;
        if players.len() > 64 {
            return Err(AttackError::InvalidPlan);
        }
        let mut sessions = players.iter().filter(|(a, _)| *a == p.target);
        let (_, session) = sessions.next().ok_or(AttackError::StaleTarget)?;
        if sessions.next().is_some() {
            return Err(AttackError::InvalidPlan);
        }
        let session = *session;
        let control = r
            .player_control_facts(p.target, session)
            .map_err(|_| AttackError::StaleTarget)?;
        if control.control_loss.is_some() {
            return Err(AttackError::StaleTarget);
        }
        let origin = r
            .read_actor_position(p.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let target = r
            .read_actor_position(p.target)
            .map_err(|_| AttackError::StaleTarget)?;
        if origin.context() != target.context()
            || origin.position().floor != target.position().floor
        {
            return Err(AttackError::OutOfRange);
        }
        let pos = origin.position();
        // Prepare the bounded presentation producer before the timer publication. Network emission
        // remains the existing presentation owner's responsibility, never a claimed wire receipt.
        let mut start = s
            .windup
            .as_ref()
            .ok_or(AttackError::InvalidSource)?
            .as_ref()
            .clone();
        let binding = start
            .windup
            .as_ref()
            .ok_or(AttackError::InvalidSource)?
            .caster_asset_binding
            .clone();
        // Project only the source caster start phase; do not emit the due blueGhost impact early.
        start.windup = None;
        start.needs_target = false;
        start.effects = vec![ProjectV2AbilityEffect::Inline(Box::new(
            crate::content::ProjectV2InlineEffect {
                key: "oteryn:effect.spell.soulwars_fear.source-windup-start".into(),
                operation: crate::content::ProjectV2InlineEffectOperation::PresentationOnly,
                presentation: Some(crate::content::ProjectV2EffectPresentation {
                    impact_asset_binding: Some(binding),
                    projectile_asset_binding: None,
                    path_asset_binding: None,
                }),
            },
        ))];
        let presentation = SourcePresentationEvent {
            issuer: p.issuer,
            ability: s.ability.clone(),
            occurrence: p.occurrence.id().as_str().into(),
            content_digest: s.content_digest,
            stamp,
            tiles: vec![(pos.x, pos.y, pos.floor)],
            source: Box::new(start),
            qualification: "SOURCE_GHOST_SMOKE_WINDUP_START_PRODUCER_NETWORK_PENDING",
        };
        if self.delayed.is_none() {
            self.delayed = Some(delayed::DelayedOwner::new(r, f)?);
        }
        self.delayed
            .as_mut()
            .ok_or(AttackError::InvalidPlan)?
            .schedule(r, f, stamp, s, p, session, now, due)?;
        Ok(SpellOutcome {
            delayed_cast: Some(DelayedCastReceipt {
                due,
                qualification: "PROJECT_BOUNDED_DELAYED_CAST1_SOURCE2000MS",
            }),
            source_variant: None,
            presentation: Some(presentation),
            requested: 0,
            source_critical: None,
            armor_only_attack1_approximation: false,
            targets: Vec::new(),
        })
    }
    pub(crate) fn run_windup_due(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        vitals: &mut dyn PlayerLethalVitals,
        reader: &mut dyn SpellWorldReader,
        clock: &impl crate::foundation::owner_timer::OwnerClock,
    ) -> WindupCastResults {
        let mut out = Vec::new();
        out.try_reserve(64).map_err(|_| AttackError::LedgerFull)?;
        let Some(owner) = &mut self.delayed else {
            return Ok(out);
        };
        let (pulses, cancellations) = owner.drain(r, f, clock)?;
        for (ability, error) in cancellations {
            out.push((ability, Err(error)));
        }
        for pulse in pulses {
            let (source, proposal, session, stamp) = pulse.into_parts();
            let result = if !f.accepts_stamp(stamp) {
                Err(AttackError::StaleOwner)
            } else if r.content_pin().server_artifact_digest() != source.content_digest {
                Err(AttackError::ContentChanged)
            } else if !r
                .matches_live_creature_identity(proposal.issuer, source.creature_key.as_bytes())
            {
                Err(AttackError::StaleIssuer)
            } else {
                self.apply_target(
                    r,
                    vitals,
                    &source,
                    &proposal,
                    reader,
                    stamp,
                    clock.now(),
                    Some((proposal.target, session)),
                    f,
                )
            };
            out.push((source.ability, result));
        }
        Ok(out)
    }

    // Keep consume_chain_pulse ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn consume_chain_pulse(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        pulse: crate::creature_chain_attack::ChainPulse,
        reader: &mut dyn SpellWorldReader,
        now: SemanticTimeMicros,
    ) -> Result<SpellOutcome, AttackError> {
        let (source, proposal, session) = pulse.into_owned_parts();
        let b = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if runtime.content_pin().server_artifact_digest() != source.content_digest {
            return Err(AttackError::ContentChanged);
        }
        if !runtime.matches_live_creature_identity(proposal.issuer, source.creature_key.as_bytes())
        {
            return Err(AttackError::StaleIssuer);
        }
        self.apply_target(
            runtime,
            vitals,
            &source,
            &proposal,
            reader,
            stamp,
            now,
            Some((proposal.target, session)),
            fence,
        )
    }

    // Keep apply ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
        fence: &ScopeRuntimeFence,
    ) -> Result<SpellOutcome, AttackError> {
        if source.appearance_cast {
            return self.apply_appearance_cast(
                runtime, vitals, source, proposal, reader, stamp, now, fence,
            );
        }
        self.apply_target(
            runtime, vitals, source, proposal, reader, stamp, now, None, fence,
        )
    }
    // Keep independently current runtime/fence, loaded content, source/proposal and owner facts explicit at this native composition boundary.
    #[allow(clippy::too_many_arguments)]
    fn apply_appearance_cast(
        &self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
        fence: &ScopeRuntimeFence,
    ) -> Result<SpellOutcome, AttackError> {
        let content = reader
            .native_gameplay(runtime)
            .ok_or(AttackError::MissingCombatFacts)?;
        if content.source_digest() != source.content_digest {
            return Err(AttackError::ContentChanged);
        }
        let from = runtime
            .read_actor_position(proposal.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = runtime
            .read_actor_position(proposal.target)
            .map_err(|_| AttackError::StaleTarget)?;
        if from.context() != runtime.pinned_movement_context()
            || from.context() != to.context()
            || from.position().floor != to.position().floor
        {
            return Err(AttackError::OutOfRange);
        }
        let fp = from.position();
        let tp = to.position();
        if source.range != 0
            && (i64::from(fp.x) - i64::from(tp.x))
                .abs()
                .max((i64::from(fp.y) - i64::from(tp.y)).abs())
                > i64::from(source.range)
        {
            return Err(AttackError::OutOfRange);
        }
        let mut tiles = Vec::new();
        if let Some(area) = &source.area {
            let center = if source.needs_target {
                (tp.x, tp.y)
            } else {
                (fp.x, fp.y)
            };
            let facing = geometry::direction(
                i64::from(center.0) - i64::from(fp.x),
                i64::from(center.1) - i64::from(fp.y),
                false,
            );
            for (x, y) in
                geometry::offsets(area, facing).map_err(|_| AttackError::UnsupportedShape)?
            {
                tiles.push((
                    center
                        .0
                        .checked_add(x)
                        .ok_or(AttackError::NumericOverflow)?,
                    center
                        .1
                        .checked_add(y)
                        .ok_or(AttackError::NumericOverflow)?,
                ));
            }
        } else {
            tiles.push((tp.x, tp.y));
        }
        let census = runtime
            .positioned_actor_census()
            .map_err(|_| AttackError::StaleTarget)?;
        if census.len() > 64 {
            return Err(AttackError::LedgerFull);
        }
        let root = GameplayDecisionRoot::from_bytes(source.content_digest);
        let mut targets = Vec::new();
        targets
            .try_reserve(census.len())
            .map_err(|_| AttackError::LedgerFull)?;
        let mut shown = Vec::new();
        shown
            .try_reserve(tiles.len())
            .map_err(|_| AttackError::LedgerFull)?;
        // Resolve every tile and actor's policy before the existing owners stage any state.
        for (x, y) in tiles {
            match reader.tile_allowed(runtime, proposal.issuer, x, y, fp.floor, stamp) {
                None => return Err(AttackError::MissingCombatFacts),
                Some(false) => continue,
                Some(true) => {}
            }
            shown.push((x, y, fp.floor));
            for (actor, pos, session) in &census {
                if pos.context() != from.context()
                    || pos.position().floor != fp.floor
                    || pos.position().x != x
                    || pos.position().y != y
                {
                    continue;
                }
                let (policy, target_protected, source_protected) = if let Some(session) = session {
                    let facts = reader
                        .combat(runtime, proposal.issuer, *actor, *session, None, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?;
                    let a = facts.attack;
                    if a.issuer != proposal.issuer
                        || a.target != *actor
                        || a.session != *session
                        || a.revision == 0
                    {
                        return Err(AttackError::InvalidPlan);
                    }
                    if !a.visible
                        || a.issuer_pz
                        || a.target_pz
                        || a.issuer_protected
                        || a.target_protected
                    {
                        continue;
                    }
                    (
                        facts
                            .condition_policy
                            .ok_or(AttackError::MissingCombatFacts)?,
                        a.target_protected,
                        a.issuer_protected,
                    )
                } else {
                    let content = reader
                        .native_gameplay(runtime)
                        .ok_or(AttackError::MissingCombatFacts)?;
                    let key = runtime
                        .current_live_creature_identity(*actor)
                        .map_err(|_| AttackError::StaleTarget)?;
                    let profile = content
                        .creature_profiles()
                        .records
                        .iter()
                        .find(|r| {
                            r.profile.target.family == ProjectV2Family::Creature
                                && r.profile.target.revision == "definition-r1"
                                && r.profile.target.key.as_bytes() == key
                        })
                        .ok_or(AttackError::InvalidSource)?;
                    let Data::Creature(c) = &profile.profile.data else {
                        return Err(AttackError::InvalidSource);
                    };
                    let base_speed = u16::try_from(c.speed.ok_or(AttackError::InvalidSource)?)
                        .map_err(|_| AttackError::NumericOverflow)?;
                    let details = c.details.as_ref().ok_or(AttackError::InvalidSource)?;
                    let immunities = if details.condition_immunities.iter().any(|s| s == "outfit") {
                        vec![crate::foundation::ConditionType::Outfit]
                    } else {
                        Vec::new()
                    };
                    // A physical Creature has no Player re-entry window; these fields are
                    // not numeric/source-stat fallbacks. Base speed is its exact loaded base.
                    (
                        CurrentConditionPolicy {
                            base_speed,
                            immunities,
                        },
                        false,
                        false,
                    )
                };
                let mut digest = Sha256::new();
                digest.update(b"oteryn:source-appearance-target:v1");
                digest.update(proposal.occurrence.id().as_str().as_bytes());
                digest.update(actor.placement_identity());
                let bytes = digest.finalize();
                let mut id = [0; 16];
                id.copy_from_slice(&bytes[..16]);
                targets.push(crate::player_lethal::SourceAppearanceTarget {
                    actor: *actor,
                    session: *session,
                    facts: ApplicationFacts {
                        now: now.get(),
                        base_speed: policy.base_speed,
                        mana_shield_capacity: 0,
                        target_reentry_protected: target_protected,
                        source_reentry_protected: source_protected,
                        target_is_player: session.is_some(),
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes(id),
                    },
                    immunities: policy.immunities,
                });
            }
        }
        targets.sort_by_key(|t| t.actor.placement_identity());
        targets.dedup_by_key(|t| t.actor.placement_identity());
        let receipts = if targets.is_empty() {
            Vec::new()
        } else {
            vitals
                .apply_source_appearance_batch(
                    runtime,
                    fence,
                    stamp,
                    proposal.issuer,
                    &source.conditions,
                    &targets,
                )
                .ok_or(AttackError::MissingVitals)?
        };
        let outcomes = targets
            .iter()
            .map(|t| {
                (
                    t.actor,
                    Ok(SpellTargetOutcome {
                        damage: None,
                        mana: None,
                        condition_only_applied: receipts
                            .iter()
                            .any(|receipt| receipt.actor == t.actor && receipt.applied),
                    }),
                )
            })
            .collect();
        Ok(SpellOutcome {
            delayed_cast: None,
            source_variant: None,
            presentation: source
                .presentation
                .as_ref()
                .map(|details| SourcePresentationEvent {
                    issuer: proposal.issuer,
                    ability: source.ability.clone(),
                    occurrence: proposal.occurrence.id().as_str().into(),
                    content_digest: source.content_digest,
                    stamp,
                    tiles: shown,
                    source: details.clone(),
                    qualification: "SOURCE_APPEARANCE_EXISTING_NATIVE_OWNERS_CURRENT_ARTIFACT",
                }),
            requested: 0,
            source_critical: None,
            armor_only_attack1_approximation: false,
            targets: outcomes,
        })
    }

    // Keep apply_target ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_target(
        &self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
        forced: Option<(ExactActorRef, GameSessionId)>,
        fence: &ScopeRuntimeFence,
    ) -> Result<SpellOutcome, AttackError> {
        let prepared = self.prepare_target(runtime, source, proposal, reader, stamp, forced)?;
        self.commit_prepared(
            runtime, vitals, source, proposal, prepared, now, fence, stamp,
        )
    }
    fn prepare_target(
        &self,
        runtime: &ChannelRuntimeV1,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn SpellWorldReader,
        stamp: RuntimeWorkStamp,
        forced: Option<(ExactActorRef, GameSessionId)>,
    ) -> Result<PreparedPlayerCast, AttackError> {
        let from = runtime
            .read_actor_position(proposal.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = runtime
            .read_actor_position(proposal.target)
            .map_err(|_| AttackError::StaleTarget)?;
        let fp = from.position();
        let tp = to.position();
        if from.context() != to.context() || fp.floor != tp.floor {
            return Err(AttackError::OutOfRange);
        }
        let dx = i64::from(tp.x) - i64::from(fp.x);
        let dy = i64::from(tp.y) - i64::from(fp.y);
        if forced.is_none() && source.range != 0 && dx.abs().max(dy.abs()) > i64::from(source.range)
        {
            return Err(AttackError::OutOfRange);
        }
        let players = match forced {
            Some(player) => vec![player],
            None => reader
                .current_players(runtime, stamp)
                .ok_or(AttackError::MissingCombatFacts)?,
        };
        if players.len() > 64 {
            return Err(AttackError::LedgerFull);
        }
        let mut unique = std::collections::BTreeSet::new();
        if players
            .iter()
            .any(|(a, _)| !unique.insert(a.placement_identity()))
        {
            return Err(AttackError::InvalidPlan);
        }
        let mut tiles = Vec::new();
        if let Some(area) = &source.area {
            let center = if source.needs_target {
                (tp.x, tp.y)
            } else if source.needs_direction {
                let f = reader
                    .current_facing(runtime, proposal.issuer, stamp)
                    .ok_or(AttackError::MissingCombatFacts)?;
                let (sx, sy) = geometry::step(f);
                (
                    fp.x.checked_add(sx).ok_or(AttackError::NumericOverflow)?,
                    fp.y.checked_add(sy).ok_or(AttackError::NumericOverflow)?,
                )
            } else {
                (fp.x, fp.y)
            };
            let diagonal =
                matches!(area,ProjectV2AbilityArea::Matrix{diagonal,..} if !diagonal.is_empty());
            let facing = geometry::direction(
                i64::from(center.0) - i64::from(fp.x),
                i64::from(center.1) - i64::from(fp.y),
                diagonal,
            );
            for (x, y) in
                geometry::offsets(area, facing).map_err(|_| AttackError::UnsupportedShape)?
            {
                tiles.push((
                    center
                        .0
                        .checked_add(x)
                        .ok_or(AttackError::NumericOverflow)?,
                    center
                        .1
                        .checked_add(y)
                        .ok_or(AttackError::NumericOverflow)?,
                ));
            }
        } else {
            tiles.push((tp.x, tp.y));
        }
        let mut presentation_tiles = Vec::new();
        let mut admitted = Vec::new();
        for (x, y) in tiles {
            match reader.tile_allowed(runtime, proposal.issuer, x, y, fp.floor, stamp) {
                None => return Err(AttackError::MissingCombatFacts),
                Some(false) => continue,
                Some(true) => {}
            }
            presentation_tiles.push((x, y, fp.floor));
            for (target, session) in &players {
                if source.area.is_none() && *target != proposal.target {
                    continue;
                }
                let pos = runtime
                    .read_actor_position(*target)
                    .map_err(|_| AttackError::StaleTarget)?;
                if pos.context() != from.context()
                    || pos.position().floor != fp.floor
                    || pos.position().x != x
                    || pos.position().y != y
                {
                    continue;
                }
                let ctrl = runtime
                    .player_control_facts(*target, *session)
                    .map_err(|_| AttackError::StaleTarget)?;
                if ctrl.control_loss.is_some() {
                    return Err(AttackError::StaleTarget);
                }
                let facts = reader
                    .combat(
                        runtime,
                        proposal.issuer,
                        *target,
                        *session,
                        source.element.as_deref(),
                        stamp,
                    )
                    .ok_or(AttackError::MissingCombatFacts)?;
                let a = facts.attack;
                if a.issuer != proposal.issuer
                    || a.target != *target
                    || a.session != *session
                    || a.revision == 0
                    || facts.multiplier_ppm > 10_000_000
                {
                    return Err(AttackError::InvalidPlan);
                }
                if (!a.visible && !source.dispel_invisible)
                    || a.issuer_pz
                    || a.target_pz
                    || a.issuer_protected
                    || a.target_protected
                {
                    continue;
                }
                if !source.conditions.is_empty() && facts.condition_policy.is_none() {
                    return Err(AttackError::MissingCombatFacts);
                }
                admitted.push((*target, *session, facts));
            }
        }
        admitted.sort_by_key(|(a, _, _)| a.placement_identity());
        admitted.dedup_by_key(|(a, _, _)| a.placement_identity());
        let root = GameplayDecisionRoot::from_bytes(source.content_digest);
        let digest = Sha256::new()
            .chain_update(b"oteryn:creature-damage-spell:v1")
            .chain_update(proposal.occurrence.id().as_str().as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let id = DecisionOccurrenceId::from_bytes(id);
        let draw = deterministic_decision_u64(&root, id, "damage_draw", source.entry_index as u64)
            .map_err(|_| AttackError::InvalidPlan)?;
        let requested = u32::try_from(crate::spell::uniform_draw(
            draw,
            i64::from(source.minimum),
            i64::from(source.maximum),
        ))
        .map_err(|_| AttackError::InvalidPlan)?;
        let source_critical = source
            .critical
            .map(|policy| {
                policy
                    .draw(&root, id, source.entry_index as u64)
                    .map_err(|_| AttackError::InvalidPlan)
            })
            .transpose()?;
        // Source-default critical bonus ZERO: base HP/MP request unchanged. Dynamic boosts/VFX remain qualified gaps.
        // Preflight every source/policy/geometry before the first HP mutation. HP commits are
        // independent per target, not a durable all-target transaction. Partial receipts retained.
        let mut prepared = Vec::new();
        for (target, session, facts) in admitted {
            let child = format!(
                "{}:target:{}",
                proposal.occurrence.id().as_str(),
                hex(&target.placement_identity())
            );
            let digest = Sha256::new()
                .chain_update(b"oteryn:creature-damage-spell-target:v1")
                .chain_update(child.as_bytes())
                .finalize();
            let mut child_id = [0; 16];
            child_id.copy_from_slice(&digest[..16]);
            let child_id = DecisionOccurrenceId::from_bytes(child_id);
            let armor = if source.armor {
                let (lo, hi) =
                    crate::creature_auto_attack::canonical_armor_bounds(facts.attack.armor)?;
                if lo == hi {
                    lo
                } else {
                    let d = deterministic_decision_u64(
                        &root,
                        child_id,
                        "armor_draw",
                        source.entry_index as u64,
                    )
                    .map_err(|_| AttackError::InvalidPlan)?;
                    u32::try_from(crate::spell::uniform_draw(d, i64::from(lo), i64::from(hi)))
                        .map_err(|_| AttackError::NumericOverflow)?
                }
            } else {
                0
            };
            let remaining = requested.saturating_sub(armor);
            let blocked =
                source.element.is_some() && facts.immune || source.armor && remaining == 0;
            let amount = if blocked {
                0
            } else {
                u32::try_from(u64::from(remaining) * u64::from(facts.multiplier_ppm) / 1_000_000)
                    .map_err(|_| AttackError::NumericOverflow)?
            };
            prepared
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
            prepared.push(PreparedPlayerTarget {
                target,
                session,
                facts,
                amount,
                blocked,
                child_id,
                occurrence: child,
            });
        }
        let presentation=source.presentation.as_ref().map(|details|SourcePresentationEvent{issuer:proposal.issuer,ability:source.ability.clone(),occurrence:proposal.occurrence.id().as_str().into(),content_digest:source.content_digest,stamp,tiles:presentation_tiles,source:details.clone(),qualification:if source.source_none_correction{"SOURCE_COMBAT_NONE_NO_HP_CORRECTION;NATIVE_PRESENTATION_PRODUCER_NETWORK_PENDING"}else{"NATIVE_PRESENTATION_PRODUCER_NETWORK_PENDING"}});
        let mut result = SpellOutcome {
            delayed_cast: None,
            source_variant: None,
            presentation,
            requested,
            source_critical,
            armor_only_attack1_approximation: source.armor,
            targets: Vec::new(),
        };
        result
            .targets
            .try_reserve(prepared.len())
            .map_err(|_| AttackError::LedgerFull)?;
        Ok(PreparedPlayerCast {
            result,
            targets: prepared,
        })
    }
    // Keep commit_prepared ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn commit_prepared(
        &self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        prepared: PreparedPlayerCast,
        now: SemanticTimeMicros,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<SpellOutcome, AttackError> {
        let root = GameplayDecisionRoot::from_bytes(source.content_digest);
        let mut result = prepared.result;
        for PreparedPlayerTarget {
            target,
            session,
            facts,
            amount,
            blocked,
            child_id,
            occurrence,
        } in prepared.targets
        {
            if blocked {
                result.targets.push((
                    target,
                    Ok(SpellTargetOutcome {
                        damage: None,
                        mana: None,
                        condition_only_applied: false,
                    }),
                ));
                continue;
            }
            if source.dispel_invisible {
                let applied = vitals.remove_attack_invisibility(
                    runtime,
                    target,
                    session,
                    proposal.issuer,
                    now.get(),
                    fence,
                    stamp,
                );
                result.targets.push((
                    target,
                    applied
                        .then_some(SpellTargetOutcome {
                            damage: None,
                            mana: None,
                            condition_only_applied: true,
                        })
                        .ok_or(AttackError::MissingVitals),
                ));
                continue;
            }
            if amount == 0 {
                let applied = if source.conditions.is_empty() {
                    Ok(SpellTargetOutcome {
                        damage: None,
                        mana: None,
                        condition_only_applied: false,
                    })
                } else {
                    let policy = facts
                        .condition_policy
                        .as_ref()
                        .ok_or(AttackError::MissingCombatFacts)?;
                    let af = ApplicationFacts {
                        now: now.get(),
                        base_speed: policy.base_speed,
                        mana_shield_capacity: 0,
                        target_reentry_protected: facts.attack.target_protected,
                        source_reentry_protected: facts.attack.issuer_protected,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: child_id,
                    };
                    vitals
                        .apply_attack_conditions(
                            runtime,
                            target,
                            session,
                            &occurrence,
                            proposal.issuer,
                            &source.conditions,
                            &af,
                            &policy.immunities,
                            fence,
                            stamp,
                        )
                        .then_some(SpellTargetOutcome {
                            damage: None,
                            mana: None,
                            condition_only_applied: true,
                        })
                        .ok_or(AttackError::MissingVitals)
                };
                result.targets.push((target, applied));
                continue;
            }
            if source.element.as_deref() == Some("mana_drain") {
                let outcome = vitals
                    .apply_attack_mana_drain(runtime, target, session, amount, &occurrence, now)
                    .map(|mana| SpellTargetOutcome {
                        damage: None,
                        mana: Some(mana),
                        condition_only_applied: false,
                    })
                    .ok_or(AttackError::MissingVitals);
                result.targets.push((target, outcome));
                continue;
            }
            let receipt = if source.conditions.is_empty() {
                vitals.apply_attack_damage(runtime, target, session, amount, &occurrence, now)
            } else {
                let policy = facts
                    .condition_policy
                    .as_ref()
                    .ok_or(AttackError::MissingCombatFacts)?;
                let af = ApplicationFacts {
                    now: now.get(),
                    base_speed: policy.base_speed,
                    mana_shield_capacity: 0,
                    target_reentry_protected: facts.attack.target_protected,
                    source_reentry_protected: facts.attack.issuer_protected,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: child_id,
                };
                vitals.apply_composite_attack_damage(
                    runtime,
                    target,
                    session,
                    amount,
                    &occurrence,
                    proposal.issuer,
                    &source.conditions,
                    &af,
                    &policy.immunities,
                    fence,
                    stamp,
                )
            };
            // A missing target vital is reported per target; do not throw away earlier real receipts.
            result.targets.push((
                target,
                receipt
                    .map(|damage| SpellTargetOutcome {
                        damage: Some(damage),
                        mana: None,
                        condition_only_applied: false,
                    })
                    .ok_or(AttackError::MissingVitals),
            ));
        }
        Ok(result)
    }
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_think::{
        ThinkSequenceTracker,
        profile_schedule::{AttackTarget, ProfileScheduleState},
    };
    use crate::foundation::MovementLocalPosition;
    use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
    struct CurrentWorld {
        facts: SpellCombatFacts,
        missing: bool,
        blocked: bool,
        facing: Option<Facing>,
        extra: Vec<(ExactActorRef, GameSessionId, u32)>,
    }
    impl SpellWorldReader for CurrentWorld {
        fn current_players(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            r.player_control_facts(self.facts.attack.target, self.facts.attack.session)
                .ok()?;
            let mut result = vec![(self.facts.attack.target, self.facts.attack.session)];
            for (a, s, _) in &self.extra {
                r.player_control_facts(*a, *s).ok()?;
                result.push((*a, *s));
            }
            Some(result)
        }
        fn current_facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<Facing> {
            self.facing
        }
        fn tile_allowed(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: i32,
            _: i32,
            _: i16,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            (!self.missing).then_some(!self.blocked)
        }
        fn combat(
            &mut self,
            r: &ChannelRuntimeV1,
            i: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
            _: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            r.read_actor_position(i).ok()?;
            r.player_control_facts(t, s).ok()?;
            if self.missing || self.facts.attack.issuer != i {
                return None;
            }
            let mut facts = self.facts.clone();
            if self.facts.attack.target == t && self.facts.attack.session == s {
                return Some(facts);
            }
            let (_, _, m) = self
                .extra
                .iter()
                .find(|(a, session, _)| *a == t && *session == s)?;
            facts.attack.target = t;
            facts.attack.session = s;
            facts.multiplier_ppm = *m;
            Some(facts)
        }
    }

    fn test_uuid(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
        ]
    }
    fn runtime_three_slots() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        let w = WorldId::decode(&test_uuid(0x60)).expect("qualified fixture");
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&test_uuid(0x61)).expect("qualified fixture"),
            NodeId::decode(&test_uuid(0x62)).expect("qualified fixture"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            ChannelContentPin::test(w),
        )
        .expect("qualified fixture");
        let s = GameSessionId::decode(&test_uuid(0x71)).expect("qualified fixture");
        let reservation = r.reserve_fresh_session(s).expect("qualified fixture");
        let a = r
            .commit_fresh_session(reservation)
            .expect("qualified fixture");
        (r, a, s)
    }
    fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("creature_auto_attack_test_data.json"))
                .expect("qualified fixture");
        (
            serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
            serde_json::from_value(v["authoring_profiles"].clone()).expect("qualified fixture"),
        )
    }
    fn setup() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        SpellSource,
        ProfileAbilityProposal,
        CurrentWorld,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        setup_named("1st_mate_ratticus", 1)
    }
    fn setup_named(
        name: &str,
        index: usize,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        SpellSource,
        ProfileAbilityProposal,
        CurrentWorld,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let (mut r, t, s) = runtime_three_slots();
        r.initialize_source_pinned_lab_player_position(
            t,
            s,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .expect("qualified fixture");
        let (records, profiles) = if name == "bony_sea_devil" {
            let packet = native_windup_packet();
            (
                serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
                serde_json::from_value(packet["authoring_profiles"].clone())
                    .expect("qualified fixture"),
            )
        } else if matches!(
            name,
            "barbarian_brutetamer" | "demon_outcast" | "werewolf" | "warlock" | "undead_dragon"
        ) {
            let packet = native_variant_packet();
            (
                serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
                serde_json::from_value(packet["authoring_profiles"].clone())
                    .expect("qualified fixture"),
            )
        } else if matches!(name, "blue_djinn" | "green_djinn") {
            let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/dispel/native-fixture.json"))).expect("qualified fixture");
            (
                serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
                serde_json::from_value(packet["authoring_profiles"].clone())
                    .expect("qualified fixture"),
            )
        } else {
            native()
        };
        let cref = Ref {
            family: ProjectV2Family::Creature,
            key: format!("oteryn:creature.{name}"),
            revision: "definition-r1".into(),
        };
        let health = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Creature(c) if p.target == cref => c.health,
                _ => None,
            })
            .expect("qualified fixture");
        let issuer = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                &cref.key,
                health as i64,
            )
            .expect("qualified fixture");
        let source = SpellSource::from_native(
            &cref,
            index,
            &records,
            &profiles,
            r.content_pin().server_artifact_digest(),
        )
        .expect("qualified fixture");
        let abilities = profiles
            .iter()
            .filter_map(|p| match &p.data {
                Data::Ability(a) => Some((p.target.clone(), a.clone())),
                _ => None,
            })
            .collect();
        let behavior = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b) if p.target.key == format!("oteryn:behavior.creature.{name}") => {
                    Some(b)
                }
                _ => None,
            })
            .expect("qualified fixture");
        let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("qualified fixture");
        let mut schedule = ProfileScheduleState::new(issuer);
        let mut tracker = ThinkSequenceTracker::new();
        let mut found = None;
        for _ in 0..256 {
            let occurrence = tracker.next_occurrence(issuer);
            let summon_facts = behavior.summons.as_ref().map(|summons| {
                crate::ai_think::profile_schedule::MonsterSummonFacts {
                    occurrence,
                    is_summon: r
                        .native_summon_role(issuer)
                        .expect("qualified fixture")
                        .is_some(),
                    target_with_path: None,
                    total_count: r.native_summon_count(issuer, None),
                    entry_counts: summons
                        .entries
                        .iter()
                        .map(
                            |entry| crate::ai_think::profile_schedule::ObservedSummonCount {
                                creature: entry.creature.clone(),
                                count: r.native_summon_count(issuer, Some(&entry.creature.key)),
                            },
                        )
                        .collect(),
                }
            });
            let plan = schedule
                .prepare_with_summons(
                    crate::ai_think::profile_schedule::ProfileScheduleInput {
                        occurrence,
                        behavior,
                        abilities: &abilities,
                        target: Some(AttackTarget {
                            actor: t,
                            same_floor_distance: Some(1),
                        }),
                        revisions: &revisions,
                        root: &root,
                    },
                    summon_facts.as_ref(),
                )
                .expect("native source schedule with independently current carrier summon counts");
            if let Some(p) = plan
                .proposals
                .into_iter()
                .find(|p| p.ability == source.ability)
            {
                found = Some(p);
                break;
            }
        }
        let proposal = found.expect("actual source chance/timing eventually prepares native spell");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                t,
                s,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let facts = SpellCombatFacts {
            attack: AttackFacts {
                issuer,
                target: t,
                session: s,
                revision: 1,
                visible: true,
                issuer_pz: false,
                target_pz: false,
                issuer_protected: false,
                target_protected: false,
                defense: 0,
                armor: 0,
            },
            multiplier_ppm: 1_000_000,
            immune: false,
            condition_policy: None,
        };
        let b = r.binding();
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("qualified fixture");
        (
            r,
            states,
            source,
            proposal,
            CurrentWorld {
                facts,
                missing: false,
                blocked: false,
                facing: None,
                extra: vec![],
            },
            fence,
            stamp,
        )
    }

    #[test]
    fn composed_readonly_preflight_defers_actual_hp_and_rejects_missing_policy() {
        let (mut r, mut states, source, p, mut world, _, stamp) = setup();
        let owner = DamageSpellOwner::default();
        let prepared = owner
            .prepare_target(&r, &source, &p, &mut world, stamp, None)
            .expect("qualified fixture");
        assert!(!prepared.targets.is_empty());
        // A real one-point owner probe proves no primary HP was committed during preparation.
        assert_eq!(
            states
                .apply_attack_damage(
                    &mut r,
                    p.target,
                    world.facts.attack.session,
                    1,
                    "preflight-health-probe",
                    crate::foundation::owner_timer::OwnerClock::now(
                        &crate::foundation::owner_timer::VirtualOwnerClock::new(
                            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                        )
                    )
                )
                .expect("qualified fixture")
                .health_after,
            184
        );
        world.missing = true;
        assert!(matches!(
            owner.prepare_target(&r, &source, &p, &mut world, stamp, None),
            Err(AttackError::MissingCombatFacts)
        ));
        assert_eq!(
            states
                .apply_attack_damage(
                    &mut r,
                    p.target,
                    world.facts.attack.session,
                    1,
                    "preflight-second-probe",
                    crate::foundation::owner_timer::OwnerClock::now(
                        &crate::foundation::owner_timer::VirtualOwnerClock::new(
                            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                        )
                    )
                )
                .expect("qualified fixture")
                .health_after,
            183
        );
    }
    fn native_windup_packet() -> serde_json::Value {
        serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/windup/native-fixture.json"))).expect("qualified fixture")
    }
    fn windup_setup() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        SpellSource,
        ProfileAbilityProposal,
        CurrentWorld,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let packet = native_windup_packet();
        let index = packet["cases"]
            .as_array()
            .expect("qualified fixture")
            .iter()
            .find(|c| c["creature"]["key"] == "oteryn:creature.bony_sea_devil")
            .expect("qualified fixture")["entry"]
            .as_u64()
            .expect("qualified fixture") as usize;
        let mut setup = setup_named("bony_sea_devil", index);
        setup.4.facts.condition_policy = Some(CurrentConditionPolicy {
            immunities: vec![],
            base_speed: 180,
        });
        setup
    }
    #[test]
    fn native_all6_delayed_fear_sources_lower_without_immediate_condition() {
        let packet = native_windup_packet();
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
        let profiles: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture");
        for case in packet["cases"].as_array().expect("qualified fixture") {
            let creature: Ref =
                serde_json::from_value(case["creature"].clone()).expect("qualified fixture");
            let source = SpellSource::from_native(
                &creature,
                case["entry"].as_u64().expect("qualified fixture") as usize,
                &records,
                &profiles,
                [1; 32],
            )
            .expect("qualified fixture");
            assert_eq!(
                source
                    .windup
                    .as_ref()
                    .expect("qualified fixture")
                    .windup
                    .as_ref()
                    .expect("qualified fixture")
                    .delay_ms,
                2000
            );
            assert_eq!(source.conditions.len(), 1);
        }
    }
    #[test]
    fn native_delayed_fear_exact2000ms_real_store_scope_and_single_due_replay() {
        let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
        let session = w.facts.attack.session;
        let before = v
            .read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture")
            .clone();
        let mut owner = DamageSpellOwner::default();
        let now = SemanticTimeMicros::from_micros(2_000_000);
        let start = owner
            .execute(&mut r, &f, stamp, &mut v, &s, &p, &mut w, now)
            .expect("qualified fixture");
        assert_eq!(
            start
                .delayed_cast
                .as_ref()
                .expect("qualified fixture")
                .due
                .get(),
            4_000_000
        );
        assert!(start.targets.is_empty());
        assert!(start.presentation.is_some());
        assert!(v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
        ));
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(3_000_000)
                )
                .expect("qualified fixture"),
            start
        );
        assert_eq!(
            owner
                .delayed
                .as_ref()
                .expect("qualified fixture")
                .pending_len(),
            1
        );
        let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
            SemanticTimeMicros::from_micros(3_999_999),
        );
        assert!(
            owner
                .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
        assert!(v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(3_999_999)
        ));
        clock.advance(1);
        let fired = owner
            .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
            .expect("qualified fixture");
        assert_eq!(fired.len(), 1);
        let hit = fired[0].1.as_ref().expect("qualified fixture");
        assert!(hit.delayed_cast.is_none());
        assert!(
            hit.targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .condition_only_applied
        );
        assert!(!v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
        ));
        let committed = v
            .read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture")
            .clone();
        assert_eq!(committed.vitals(), before.vitals());
        assert_eq!(committed.revision(), before.revision() + 1);
        assert!(
            owner
                .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
        assert_eq!(
            owner
                .delayed
                .as_ref()
                .expect("qualified fixture")
                .pending_len(),
            0
        );
        assert!(v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(7_000_000)
        ));
        assert_eq!(
            v.read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture"),
            &committed
        );
    }
    #[test]
    fn native_delayed_fear_current_pz_missing_facts_and_source_death_never_install() {
        for mode in 0..3 {
            let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
            let mut owner = DamageSpellOwner::default();
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(2_000_000),
                )
                .expect("qualified fixture");
            match mode {
                0 => w.facts.attack.target_pz = true,
                1 => w.missing = true,
                _ => {
                    r.remove_test_actor(p.issuer).expect("qualified fixture");
                }
            }
            let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
                SemanticTimeMicros::from_micros(4_000_000),
            );
            let fired = owner
                .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
                .expect("qualified fixture");
            assert_eq!(fired.len(), 1);
            match mode {
                0 => assert!(
                    fired[0]
                        .1
                        .as_ref()
                        .expect("qualified fixture")
                        .targets
                        .is_empty()
                ),
                1 => assert_eq!(fired[0].1, Err(AttackError::MissingCombatFacts)),
                _ => assert_eq!(fired[0].1, Err(AttackError::StaleIssuer)),
            };
            assert!(v.native_condition_movement_allowed(
                &r,
                p.target,
                w.facts.attack.session,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
            ));
            assert_eq!(
                owner
                    .delayed
                    .as_ref()
                    .expect("qualified fixture")
                    .pending_len(),
                0
            );
            assert!(
                owner
                    .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
                    .expect("qualified fixture")
                    .is_empty()
            );
        }
    }
    #[test]
    fn native_delayed_fear_overflow_pending_cap_and_retired_scope_are_explicit() {
        let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
        let mut owner = DamageSpellOwner::default();
        assert_eq!(
            owner.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(u64::MAX - 1)
            ),
            Err(AttackError::NumericOverflow)
        );
        assert!(owner.delayed.is_none());
        let mut owner = DamageSpellOwner::default();
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        let queued = owner
            .delayed
            .as_ref()
            .expect("qualified fixture")
            .pending_len();
        // Adversarial owner admission probe of the private cap; no fabricated think or wire grant.
        assert_eq!(
            owner.delayed.as_mut().expect("qualified fixture").schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                w.facts.attack.session,
                SemanticTimeMicros::from_micros(3_000_000),
                SemanticTimeMicros::from_micros(5_000_000)
            ),
            Err(AttackError::LedgerFull)
        );
        assert_eq!(
            owner
                .delayed
                .as_ref()
                .expect("qualified fixture")
                .pending_len(),
            queued
        );
        f.apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
        )
        .expect("qualified fixture");
        let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
            SemanticTimeMicros::from_micros(4_000_000),
        );
        assert!(matches!(
            owner.run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock),
            Err(AttackError::StaleOwner)
        ));
        assert_eq!(
            owner
                .delayed
                .as_ref()
                .expect("qualified fixture")
                .pending_len(),
            queued
        );
        assert!(v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
        ));
    }

    fn native_variant_packet() -> serde_json::Value {
        serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/variants/native-fixture.json"))).expect("qualified fixture")
    }
    #[test]
    fn native_all42_parent_variants_have_exact_child_membership_without_creature_aliases() {
        let packet = native_variant_packet();
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
        let profiles: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture");
        for case in packet["cases"].as_array().expect("qualified fixture") {
            let creature: Ref =
                serde_json::from_value(case["creature"].clone()).expect("qualified fixture");
            let source = SpellSource::from_native(
                &creature,
                case["entry"].as_u64().expect("qualified fixture") as usize,
                &records,
                &profiles,
                [1; 32],
            )
            .expect("qualified fixture");
            assert_eq!(
                source.variants.len(),
                case["children"]
                    .as_array()
                    .expect("qualified fixture")
                    .len()
            );
            for (index, branch) in source.variants.iter().enumerate() {
                match branch {
                    Ok(b) => {
                        assert_eq!(b.ability, source.ability);
                        assert_eq!(
                            b.variant_child.as_ref().expect("qualified fixture").key,
                            format!("{}.variant-{}", source.ability.key, index + 1)
                        );
                    }
                    Err(e) => assert_eq!(*e, AttackError::UnsupportedShape),
                }
            }
        }
    }
    #[test]
    fn native_skill_variant_once_per_parent_cast_real_owner_and_replay_no_fake_damage() {
        let packet = native_variant_packet();
        let cases = packet["cases"].as_array().expect("qualified fixture");
        for name in [
            "barbarian_brutetamer",
            "demon_outcast",
            "werewolf",
            "warlock",
        ] {
            let case = cases
                .iter()
                .find(|c| c["creature"]["key"] == format!("oteryn:creature.{name}"))
                .expect("qualified fixture");
            let index = case["entry"].as_u64().expect("qualified fixture") as usize;
            let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(name, index);
            w.facing = Some(Facing::West);
            w.facts.condition_policy = Some(CurrentConditionPolicy {
                immunities: vec![],
                base_speed: 180,
            });
            let mut owner = DamageSpellOwner::default();
            let hit = owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(2_000_000),
                )
                .expect("qualified fixture");
            let selected = hit.source_variant.as_ref().expect("qualified fixture");
            assert!(selected.index < s.variants.len());
            assert_eq!(selected.parent, p.ability);
            assert_eq!(
                selected.child.key,
                format!("{}.variant-{}", p.ability.key, selected.index + 1)
            );
            assert_eq!(hit.targets.len(), 1);
            assert!(
                hit.targets[0]
                    .1
                    .as_ref()
                    .expect("qualified fixture")
                    .condition_only_applied
            );
            assert!(
                hit.targets[0]
                    .1
                    .as_ref()
                    .expect("qualified fixture")
                    .damage
                    .is_none()
            );
            assert_eq!(
                owner
                    .execute(
                        &mut r,
                        &f,
                        stamp,
                        &mut v,
                        &s,
                        &p,
                        &mut w,
                        SemanticTimeMicros::from_micros(9_000_000)
                    )
                    .expect("qualified fixture"),
                hit
            );
            assert_eq!(
                v.apply_attack_damage(
                    &mut r,
                    p.target,
                    w.facts.attack.session,
                    1,
                    "variant-no-hp-probe",
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("qualified fixture")
                .health_after,
                184
            );
        }
    }

    fn move_existing_player_through_native_owner_to_curse_cone(
        r: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) {
        use crate::content::static_cell_engine::{
            EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
            EngineeringStaticCellScope,
        };
        use crate::content::{
            CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
            MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
        };
        use crate::movement::{
            CardinalStep, MovementEngineeringSelection, MovementOwnerTurn, MovementTurnOutcome,
        };
        let scope = EngineeringStaticCellScope {
            world_id: r
                .read_actor_position(actor)
                .expect("qualified fixture")
                .world_id(),
            coordinate_frame: CoordinateFrameRef::new("movement-engineering-frame")
                .expect("qualified fixture"),
            map_revision: MapRevisionRef::new("movement-engineering-map")
                .expect("qualified fixture"),
            generation_digest: [7; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "movement-engineering-lock")
                    .expect("qualified fixture"),
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:variant-curse").expect("qualified fixture"),
                    ProductionAtom::new("revision", "variant-curse-r1").expect("qualified fixture"),
                    Sha256HexDigest::new(&"a".repeat(64)).expect("qualified fixture"),
                )],
            },
        };
        // Explicit local map claims, not a live/Global map authority. Every relocation below
        // commits through the actual existing four-cardinal movement owner and current carrier.
        for direction in [
            CardinalStep::North,
            CardinalStep::North,
            CardinalStep::North,
            CardinalStep::North,
            CardinalStep::North,
            CardinalStep::East,
        ] {
            let expected = r.read_actor_position(actor).expect("qualified fixture");
            let selected = MovementEngineeringSelection {
                owner_context: expected.context(),
                content_scope: &scope,
            };
            let p = expected.position();
            let cell = match direction {
                CardinalStep::North => LogicalCell {
                    x: p.x,
                    y: p.y - 1,
                    z: i32::from(p.floor),
                },
                CardinalStep::East => LogicalCell {
                    x: p.x + 1,
                    y: p.y,
                    z: i32::from(p.floor),
                },
                _ => unreachable!("explicit North/East source-fixture route"),
            };
            // One exact destination claim per owner turn stays below existing native cap4.
            let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
                scope: scope.clone(),
                cell,
                collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
            }])
            .expect("qualified fixture");
            let result = MovementOwnerTurn::begin(r, std::num::NonZeroUsize::MIN)
                .try_step(actor, expected, &selected, &index, direction);
            assert!(
                matches!(result, Ok(MovementTurnOutcome::Applied(_))),
                "native source-fixture relocation refused: {result:?}"
            );
        }
        assert_eq!(
            r.read_actor_position(actor)
                .expect("qualified fixture")
                .position(),
            MovementLocalPosition {
                x: 101,
                y: 95,
                floor: 7
            }
        );
    }

    #[test]
    fn native_curse_variant_one_choice_shared_across_real_player_roster_and_retry() {
        let packet = native_variant_packet();
        let case = packet["cases"]
            .as_array()
            .expect("qualified fixture")
            .iter()
            .find(|c| c["creature"]["key"] == "oteryn:creature.undead_dragon")
            .expect("qualified fixture");
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(
            "undead_dragon",
            case["entry"].as_u64().expect("qualified fixture") as usize,
        );
        w.facing = Some(Facing::North);
        w.facts.condition_policy = Some(CurrentConditionPolicy {
            immunities: vec![],
            base_speed: 180,
        });
        move_existing_player_through_native_owner_to_curse_cone(&mut r, p.target);
        let session = GameSessionId::decode(&test_uuid(0x75)).expect("qualified fixture");
        let reservation = r.reserve_fresh_session(session).expect("qualified fixture");
        let second = r
            .commit_fresh_session(reservation)
            .expect("qualified fixture");
        r.initialize_source_pinned_lab_player_position(
            second,
            session,
            MovementLocalPosition {
                x: 102,
                y: 95,
                floor: 7,
            },
        )
        .expect("qualified fixture");
        v.initialize(
            &r,
            second,
            session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
        w.extra.push((second, session, 1_000_000));
        let mut owner = DamageSpellOwner::default();
        let hit = owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        assert_eq!(hit.targets.len(), 2);
        let chosen = hit.source_variant.as_ref().expect("qualified fixture");
        assert!(chosen.index < 2);
        assert_eq!(chosen.parent, p.ability);
        for (_, target) in &hit.targets {
            let target = target.as_ref().expect("qualified fixture");
            assert!(target.condition_only_applied);
            assert!(target.damage.is_none());
        }
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("qualified fixture"),
            hit
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "variant-first-hp-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                second,
                session,
                1,
                "variant-second-hp-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }

    #[test]
    fn native_variant_child_substitution_missing_closure_and_current_pin_refuse() {
        let packet = native_variant_packet();
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
        let profiles: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture");
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.barbarian_brutetamer".into(),
            revision: "definition-r1".into(),
        };
        let case = packet["cases"]
            .as_array()
            .expect("qualified fixture")
            .iter()
            .find(|c| c["creature"]["key"] == creature.key)
            .expect("qualified fixture");
        let index = case["entry"].as_u64().expect("qualified fixture") as usize;
        let mut wrong = profiles.clone();
        for p in &mut wrong {
            if p.target.key == case["parent"]
                && let Data::Ability(a) = &mut p.data
            {
                a.details.as_mut().expect("qualified fixture").variants[0].key =
                    "oteryn:ability.spell.haste".into();
            }
        }
        assert_eq!(
            SpellSource::from_native(&creature, index, &records, &wrong, [1; 32])
                .expect_err("expected fixture rejection"),
            AttackError::InvalidSource
        );
        let missing = profiles
            .into_iter()
            .filter(|p| {
                p.target.key
                    != format!(
                        "{}.variant-1",
                        case["parent"].as_str().expect("qualified fixture")
                    )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            SpellSource::from_native(&creature, index, &records, &missing, [1; 32])
                .expect_err("expected fixture rejection"),
            AttackError::InvalidSource
        );
        let (mut r, mut v, mut source, proposal, mut reader, f, stamp) =
            setup_named("barbarian_brutetamer", index);
        source.content_digest = [9; 32];
        assert_eq!(
            DamageSpellOwner::default().execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &source,
                &proposal,
                &mut reader,
                SemanticTimeMicros::from_micros(2_000_000)
            ),
            Err(AttackError::ContentChanged)
        );
    }

    #[test]
    fn native_djinn_source_area_dispel_invisible_player_without_primary_hp_and_replay() {
        for name in ["blue_djinn", "green_djinn"] {
            let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(name, 6);
            assert!(s.dispel_invisible);
            let session = w.facts.attack.session;
            let before = v
                .read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture")
                .clone();
            let native_root =
                GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
            let invisible = crate::ability::condition::ConditionDefinition::new(
                "test.source.djinn.invisible",
                1,
                crate::ability::condition::ConditionValues::Invisible { duration_ms: 5000 },
            )
            .expect("qualified fixture");
            let seed_facts = crate::ability::condition::ApplicationFacts {
                now: 1_000_000,
                base_speed: 180,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &native_root,
                occurrence: DecisionOccurrenceId::from_bytes([91; 16]),
            };
            // Native player owned store, explicit current self-use and shared vitals CAS.
            assert!(v.install_owned_player_self_use_test_condition(
                &r,
                &f,
                stamp,
                p.target,
                session,
                &invisible,
                &seed_facts,
            ));
            let seeded = v
                .read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture")
                .clone();
            assert_eq!(seeded.vitals(), before.vitals());
            assert_eq!(seeded.revision(), before.revision() + 1);
            assert_eq!(
                v.native_actor_invisible(
                    &r,
                    p.target,
                    Some(session),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
                ),
                Some(true)
            );
            assert_eq!(
                v.read_owned_player_state_test_snapshot(&r, p.target, session)
                    .expect("qualified fixture"),
                &seeded
            );
            w.facts.attack.visible = false;
            let mut o = DamageSpellOwner::default();
            let result = o
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(2_000_000),
                )
                .expect("qualified fixture");
            assert_eq!(result.targets.len(), 1);
            let hit = result.targets[0].1.as_ref().expect("qualified fixture");
            assert!(hit.condition_only_applied);
            assert!(hit.damage.is_none() && hit.mana.is_none());
            assert_eq!(result.requested, 0);
            assert_eq!(
                v.native_actor_invisible(
                    &r,
                    p.target,
                    Some(session),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
                ),
                Some(false)
            );
            let cured = v
                .read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture")
                .clone();
            assert_eq!(cured.vitals(), before.vitals());
            assert_eq!(cured.revision(), seeded.revision() + 1);

            assert_eq!(
                o.execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("qualified fixture"),
                result
            );
            assert_eq!(
                v.read_owned_player_state_test_snapshot(&r, p.target, session)
                    .expect("qualified fixture"),
                &cured
            );
            assert_eq!(
                v.apply_attack_damage(
                    &mut r,
                    p.target,
                    w.facts.attack.session,
                    1,
                    "dispel-hp-probe",
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("qualified fixture")
                .health_after,
                184
            );
        }
    }
    #[test]
    fn native_djinn_dispel_visibility_exception_keeps_current_pz_and_source_guards() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("blue_djinn", 6);
        let mut o = DamageSpellOwner::default();
        w.facts.attack.visible = false;
        w.facts.attack.target_pz = true;
        assert!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .targets
            .is_empty()
        );
        let mut profiles = native();
        assert!(
            SpellSource::from_native(
                &Ref {
                    family: ProjectV2Family::Creature,
                    key: "oteryn:creature.blue_djinn".into(),
                    revision: "definition-r1".into()
                },
                6,
                &profiles.0,
                &profiles.1,
                [1; 32]
            )
            .is_err()
        );
        let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/dispel/native-fixture.json"))).expect("qualified fixture");
        profiles = (
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture"),
        );
        for profile in &mut profiles.1 {
            if profile.target.key == s.ability.key
                && let Data::Ability(a) = &mut profile.data
            {
                a.details.as_mut().expect("qualified fixture").range_tiles = 1;
            }
        }
        assert!(matches!(
            SpellSource::from_native(
                &Ref {
                    family: ProjectV2Family::Creature,
                    key: "oteryn:creature.blue_djinn".into(),
                    revision: "definition-r1".into()
                },
                6,
                &profiles.0,
                &profiles.1,
                [1; 32]
            ),
            Err(AttackError::UnsupportedShape)
        ));
    }

    #[test]
    fn actual_source_schedule_area_damage_real_hp_and_replay() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup();
        let mut o = DamageSpellOwner::default();
        let hit = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        assert_eq!(hit.targets.len(), 1);
        let receipt = hit.targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .damage
            .expect("qualified fixture");
        assert!(receipt.applied > 0);
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture"),
            hit
        );
        let mut conflict = p.clone();
        conflict.target = p.issuer;
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &conflict,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            ),
            Err(AttackError::OccurrenceConflict)
        );
    }
    #[test]
    fn independently_missing_los_policy_refuses_entire_cast_no_buffer() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup();
        let mut o = DamageSpellOwner::default();
        w.missing = true;
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000)
            ),
            Err(AttackError::MissingCombatFacts)
        );
        w.missing = false;
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(4_000_000)
            ),
            Err(AttackError::MissingCombatFacts)
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "independent-spell-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(4_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }
    #[test]
    fn independent_tile_combat_rejection_and_retired_scope() {
        let (mut r, mut v, s, p, mut w, mut f, stamp) = setup();
        w.blocked = true;
        let mut o = DamageSpellOwner::default();
        assert!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .targets
            .is_empty()
        );
        f.apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
        )
        .expect("qualified fixture");
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(3_000_000)
            ),
            Err(AttackError::StaleOwner)
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "tile-refusal-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(3_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }
    #[test]
    fn actual_source_inline_energy_condition_has_no_fake_primary_hp() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("abyssador", 1);
        assert_eq!((s.minimum, s.maximum), (0, 0));
        assert!(s.element.is_none());
        assert!(!s.conditions.is_empty());
        w.facts.condition_policy = Some(CurrentConditionPolicy {
            immunities: vec![],
            base_speed: 180,
        });
        let mut o = DamageSpellOwner::default();
        let result = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("actual typed condition owner admission");
        assert_eq!(result.targets.len(), 1);
        let target = result.targets[0].1.as_ref().expect("actual store commit");
        assert!(target.condition_only_applied);
        assert!(target.damage.is_none());
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("retained condition-only cast"),
            result
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "condition-no-primary-hp-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("real HP store")
            .health_after,
            184
        );
    }

    #[test]
    fn native_hunter_armor_only_blocks_real_arrow_without_vital_commit() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("hunter", 1);
        assert!(s.armor);
        w.facts.attack.armor = 10_000;
        let mut o = DamageSpellOwner::default();
        let result = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("native arrow source");
        assert!(result.armor_only_attack1_approximation);
        assert_eq!(result.targets.len(), 1);
        assert!(
            result.targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .damage
                .is_none()
        );
        assert!(
            !result.targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .condition_only_applied
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "armor-arrow-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }
    #[test]
    fn primary_native_immunity_skips_source_curse_secondary_condition() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("blightwalker", 3);
        assert!(s.element.is_some());
        assert!(!s.conditions.is_empty());
        w.facts.immune = true;
        w.facts.condition_policy = Some(CurrentConditionPolicy {
            immunities: vec![],
            base_speed: 180,
        });
        let mut o = DamageSpellOwner::default();
        let result = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("source combat immunity admission");
        assert_eq!(result.targets.len(), 1);
        let target = result.targets[0].1.as_ref().expect("qualified fixture");
        assert!(target.damage.is_none());
        assert!(!target.condition_only_applied);
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "immune-curse-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }

    #[test]
    fn all_target_amounts_preflight_before_first_real_hp_even_when_second_overflows() {
        let (mut r, mut v, mut s, p, mut w, f, stamp) = setup();
        let second_session = GameSessionId::decode(&test_uuid(0x72)).expect("qualified fixture");
        let reserve = r
            .reserve_fresh_session(second_session)
            .expect("qualified fixture");
        let second = r.commit_fresh_session(reserve).expect("qualified fixture");
        r.initialize_source_pinned_lab_player_position(
            second,
            second_session,
            MovementLocalPosition {
                x: 102,
                y: 100,
                floor: 7,
            },
        )
        .expect("qualified fixture");
        v.initialize(
            &r,
            second,
            second_session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
        if p.target.placement_identity() < second.placement_identity() {
            w.extra.push((second, second_session, 10_000_000));
        } else {
            w.facts.multiplier_ppm = 10_000_000;
            w.extra.push((second, second_session, 1_000_000));
        }
        // Adversarial private descriptor bounds test, not a claim that the donor uses this damage.
        // Both victims remain genuine ChannelSpellStates185HP; no model-health substitute.
        s.minimum = u32::MAX;
        s.maximum = u32::MAX;
        let mut o = DamageSpellOwner::default();
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000)
            ),
            Err(AttackError::NumericOverflow)
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "overflow-A-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                second,
                second_session,
                1,
                "overflow-B-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }

    #[test]
    fn native_glyph_mana_drain_actual_mp_clamp_revision_no_hp_and_replay() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
        assert_eq!(s.element.as_deref(), Some("mana_drain"));
        let mut o = DamageSpellOwner::default();
        let hit = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        let t = hit.targets[0].1.as_ref().expect("qualified fixture");
        assert!(t.damage.is_none());
        assert!(!t.condition_only_applied);
        let mp = t.mana.expect("qualified fixture");
        assert_eq!(
            (
                mp.applied,
                mp.mana_before,
                mp.mana_after,
                mp.vitals_revision
            ),
            (90, 90, 0, 2)
        );
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (2, 185, 0));
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture"),
            hit
        );
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (2, 185, 0));
    }
    #[test]
    fn actual_mana_owner_replay_conflict_session_and_current_death_guards() {
        let (mut r, mut v, _s, p, w, _f, _stamp) = setup_named("an_astral_glyph", 3);
        let session = w.facts.attack.session;
        let first = v
            .apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                40,
                "mana-owner-1",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                    ),
                ),
            )
            .expect("qualified fixture");
        assert_eq!(
            (
                first.applied,
                first.mana_before,
                first.mana_after,
                first.vitals_revision
            ),
            (40, 90, 50, 2)
        );
        assert_eq!(
            v.apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                40,
                "mana-owner-1",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            ),
            Some(first)
        );
        assert!(
            v.apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                41,
                "mana-owner-1",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .is_none()
        );
        assert!(
            v.apply_attack_mana_drain(
                &mut r,
                p.target,
                GameSessionId::decode(&test_uuid(0x79)).expect("qualified fixture"),
                1,
                "wrong-session",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .is_none()
        );
        let death = v
            .apply_attack_damage(
                &mut r,
                p.target,
                session,
                185,
                "independent-lethal-owner",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                    ),
                ),
            )
            .expect("qualified fixture");
        assert_eq!(death.health_after, 0);
        assert!(death.death.is_some());
        assert!(
            v.apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                1,
                "mana-after-death",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .is_none()
        );
        assert!(
            v.apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                40,
                "mana-owner-1",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .is_none()
        );
        let (revision, current) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, p.target, session)
                .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (3, 0, 50));
    }
    #[test]
    fn missing_current_world_facts_refuses_mana_without_hp_or_mp_buffer() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
        w.missing = true;
        let mut o = DamageSpellOwner::default();
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000)
            ),
            Err(AttackError::MissingCombatFacts)
        );
        w.missing = false;
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(4_000_000)
            ),
            Err(AttackError::MissingCombatFacts)
        );
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (1, 185, 90));
    }
    #[test]
    fn actual_exhausted_mp_noop_retains_revision_and_health() {
        let (mut r, mut v, _s, p, w, _f, _stamp) = setup_named("an_astral_glyph", 3);
        let session = w.facts.attack.session;
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            900,
            "mana-full-clamp",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
            ),
        )
        .expect("qualified fixture");
        let zero = v
            .apply_attack_mana_drain(
                &mut r,
                p.target,
                session,
                1,
                "mana-exhausted",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                    ),
                ),
            )
            .expect("qualified fixture");
        assert_eq!(
            (
                zero.applied,
                zero.mana_before,
                zero.mana_after,
                zero.vitals_revision
            ),
            (0, 0, 0, 2)
        );
        let (revision, current) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, p.target, session)
                .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (2, 185, 0));
    }

    #[test]
    fn newer_mana_occurrence_with_nonadvancing_clock_never_commits_mp() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
        let mut o = DamageSpellOwner::default();
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("qualified fixture");
        // Deliberately inconsistent newer event/current clock input, not a source scheduler fixture.
        let mut forged = p.clone();
        let old = o.casts[0].sequence;
        forged.occurrence = crate::ability::AbilityOccurrence::new(
            &format!(
                "ai-profile:{}:{}:attack:{}",
                hex(&p.issuer.placement_identity()),
                old + 1,
                s.entry_index
            ),
            p.occurrence.revisions().clone(),
        )
        .expect("qualified fixture");
        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &forged,
                &mut w,
                SemanticTimeMicros::from_micros(1_000_000)
            ),
            Err(AttackError::NotDue)
        );
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (2, 185, 0));
    }

    struct CurrentChainMap {
        missing: bool,
    }
    impl crate::creature_chain_attack::ChainMapFacts for CurrentChainMap {
        fn sight(
            &mut self,
            _: &ChannelRuntimeV1,
            _: crate::spell::chain::TilePosition,
            _: crate::spell::chain::TilePosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            (!self.missing).then_some(true)
        }
        fn path(
            &mut self,
            _: &ChannelRuntimeV1,
            _: crate::spell::chain::TilePosition,
            to: crate::spell::chain::TilePosition,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<crate::spell::chain::TilePosition>> {
            (!self.missing).then_some(vec![to])
        }
    }
    fn chain_second(
        r: &mut ChannelRuntimeV1,
        v: &mut ChannelSpellStates,
        w: &mut CurrentWorld,
    ) -> (ExactActorRef, GameSessionId) {
        let session = GameSessionId::decode(&test_uuid(0x72)).expect("qualified fixture");
        let reserve = r.reserve_fresh_session(session).expect("qualified fixture");
        let actor = r.commit_fresh_session(reserve).expect("qualified fixture");
        r.initialize_source_pinned_lab_player_position(
            actor,
            session,
            MovementLocalPosition {
                x: 102,
                y: 100,
                floor: 7,
            },
        )
        .expect("qualified fixture");
        v.initialize(
            r,
            actor,
            session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
        w.extra.push((actor, session, 1_000_000));
        (actor, session)
    }
    #[test]
    fn chain_native_hp_two_targets_not_before_fifty_ms_and_retry_once() {
        use crate::creature_chain_attack::ChainOwner;
        use crate::foundation::owner_timer::VirtualOwnerClock;
        let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
        let (second, session) = chain_second(&mut r, &mut v, &mut w);
        let mut map = CurrentChainMap { missing: false };
        let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
        let damage = DamageSpellOwner::default();
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let token = chain
            .schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                &mut w,
                &mut map,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        assert_eq!(
            chain
                .schedule(
                    &r,
                    &f,
                    stamp,
                    &s,
                    &p,
                    &mut w,
                    &mut map,
                    SemanticTimeMicros::from_micros(0)
                )
                .expect("qualified fixture"),
            token
        );
        let first = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        assert_eq!(first.len(), 1);
        assert!(
            first[0].2.as_ref().expect("qualified fixture").targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .damage
                .expect("qualified fixture")
                .applied
                > 0
        );
        let (_, before) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
                .expect("qualified fixture");
        assert_eq!(before.health, 185);
        clock.advance(49_000);
        assert!(
            chain
                .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
        clock.advance(1_000);
        let next = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        assert_eq!(next.len(), 1);
        assert!(
            next[0].2.as_ref().expect("qualified fixture").targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .damage
                .expect("qualified fixture")
                .applied
                > 0
        );
        assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
        assert!(
            chain
                .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
    }
    #[test]
    fn chain_native_mana_two_actual_player_stores_no_hp() {
        use crate::creature_chain_attack::ChainOwner;
        use crate::foundation::owner_timer::VirtualOwnerClock;
        let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("timira_the_many_headed", 3);
        let (second, session) = chain_second(&mut r, &mut v, &mut w);
        let mut map = CurrentChainMap { missing: false };
        let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let damage = DamageSpellOwner::default();
        chain
            .schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                &mut w,
                &mut map,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let first = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        let receipt = first[0].2.as_ref().expect("qualified fixture").targets[0]
            .1
            .as_ref()
            .expect("qualified fixture");
        assert!(receipt.damage.is_none());
        assert!(receipt.mana.expect("qualified fixture").applied > 0);
        clock.advance(49_000);
        assert!(
            chain
                .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
        let (_, before) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
                .expect("qualified fixture");
        assert_eq!((before.health, before.mana), (185, 90));
        clock.advance(1_000);
        let next = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        assert!(
            next[0].2.as_ref().expect("qualified fixture").targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .mana
                .expect("qualified fixture")
                .applied
                > 0
        );
        let (_, after) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
                .expect("qualified fixture");
        assert_eq!(after.health, 185);
        assert!(after.mana < 90);
    }
    #[test]
    fn chain_due_missing_current_los_retains_failure_and_never_retries_step() {
        use crate::creature_chain_attack::ChainOwner;
        use crate::foundation::owner_timer::VirtualOwnerClock;
        let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
        let (second, session) = chain_second(&mut r, &mut v, &mut w);
        let mut map = CurrentChainMap { missing: false };
        let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let damage = DamageSpellOwner::default();
        let token = chain
            .schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                &mut w,
                &mut map,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        clock.advance(50_000);
        map.missing = true;
        let next = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        assert_eq!(next[0].2, Err(AttackError::MissingCombatFacts));
        assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
        map.missing = false;
        assert!(
            chain
                .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
        let (_, after) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
                .expect("qualified fixture");
        assert_eq!(after.health, 185);
        f.apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
        )
        .expect("qualified fixture");
        assert_eq!(
            chain.run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock),
            Err(AttackError::StaleOwner)
        );
    }

    #[test]
    fn chain_retired_target_generation_refuses_frozen_second_step() {
        use crate::creature_chain_attack::ChainOwner;
        use crate::foundation::owner_timer::VirtualOwnerClock;
        let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
        let (second, _) = chain_second(&mut r, &mut v, &mut w);
        let mut map = CurrentChainMap { missing: false };
        let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let damage = DamageSpellOwner::default();
        let token = chain
            .schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                &mut w,
                &mut map,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        r.remove_test_actor(second).expect("qualified fixture");
        clock.advance(50_000);
        let result = chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture");
        assert_eq!(result[0].2, Err(AttackError::StaleTarget));
        assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
        assert!(
            chain
                .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
    }
    #[test]
    fn native_presentation_only_produces_source_cues_geometry_without_hp_mp_commit() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("apocalypse", 3);
        assert_eq!((s.minimum, s.maximum), (0, 0));
        let mut owner = DamageSpellOwner::default();
        let result = owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let event = result.presentation.as_ref().expect("qualified fixture");
        assert!(!event.tiles.is_empty());
        assert!(event.source.impact_cue.is_some());
        assert!(event.source.effects.iter().all(|e|matches!(e,ProjectV2AbilityEffect::Inline(i)if matches!(i.operation,crate::content::ProjectV2InlineEffectOperation::PresentationOnly))));
        let (revision, before) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, before.health, before.mana), (1, 185, 90));
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(999)
                )
                .expect("qualified fixture"),
            result
        );
    }
    #[test]
    fn fairy_combat_none_source_preserves_presentation_does_not_invent_ice_damage() {
        for index in [1, 2] {
            let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("angry_sugar_fairy", index);
            assert!(s.source_none_correction);
            assert!(s.element.is_none());
            assert_eq!((s.minimum, s.maximum), (0, 0));
            let result = DamageSpellOwner::default()
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(0),
                )
                .expect("qualified fixture");
            assert_eq!(result.requested, 0);
            let event = result.presentation.expect("qualified fixture");
            assert!(
                event
                    .qualification
                    .contains("SOURCE_COMBAT_NONE_NO_HP_CORRECTION")
            );
            assert!(event.source.effects.iter().all(|e|matches!(e,ProjectV2AbilityEffect::Inline(i)if matches!(i.operation,crate::content::ProjectV2InlineEffectOperation::PresentationOnly))));
            let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
                &r,
                &v,
                p.target,
                w.facts.attack.session,
            )
            .expect("qualified fixture");
            assert_eq!((revision, current.health, current.mana), (1, 185, 90));
        }
    }
    #[test]
    fn presentation_current_tile_policy_unavailable_refuses_and_no_vital_commit() {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("apocalypse", 3);
        w.missing = true;
        let mut owner = DamageSpellOwner::default();
        assert_eq!(
            owner.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(AttackError::MissingCombatFacts)
        );
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (1, 185, 90));
        w.missing = false;
        assert_eq!(
            owner.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2)
            ),
            Err(AttackError::MissingCombatFacts)
        );
    }
}

const DEFENSE_VARIANT_SOURCE_BODY: &str = r##"[{"data":{"kind":"Ability","profile":{"details":{"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0,"variants":[{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-1","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-2","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-3","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-4","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-5","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-6","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-7","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-8","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-9","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-10","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-11","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-12","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-13","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-14","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-15","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-16","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-17","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-18","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-19","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-20","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-21","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-22","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-23","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-24","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-25","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-26","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-27","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-28","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-29","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-30","revision":"definition-r1"},{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-31","revision":"definition-r1"}]}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-1.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":20}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-1","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-10.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":29}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-10","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-11.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":30}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-11","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-12.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":31}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-12","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-13.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":32}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-13","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-14.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":33}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-14","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-15.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":34}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-15","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-16.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":35}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-16","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-17.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":36}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-17","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-18.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":37}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-18","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-19.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":38}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-19","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-2.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":21}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-2","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-20.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":39}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-20","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-21.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":40}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-21","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-22.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":41}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-22","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-23.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":42}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-23","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-24.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":43}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-24","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-25.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":44}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-25","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-26.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":45}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-26","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-27.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":46}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-27","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-28.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":47}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-28","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-29.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":48}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-29","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-3.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":22}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-3","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-30.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":49}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-30","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-31.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":50}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-31","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-4.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":23}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-4","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-5.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":24}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-5","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-6.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":25}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-6","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-7.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":26}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-7","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-8.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":27}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-8","revision":"definition-r1"}},{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["...xxx...","..xxxxx..",".xxxxxxx.","xxxxxxxxx","xxxxCxxxx","xxxxxxxxx",".xxxxxxx.","..xxxxx..","...xxx..."],"shape":"Matrix"},"effects":[{"effect":{"key":"oteryn:effect.spell.shock_head_skill_reducer_2.variant-9.effect-condition-1","operation":{"condition":{"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"PercentOfBase","value":28}],"condition_type":"attributes","lifetime":"FixedDuration"},"duration_ms":6000,"operation":"Condition"},"presentation":{"impact_asset_binding":"canary.appearance:effect/magic_red"}},"kind":"Inline"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.shock_head_skill_reducer_2.variant-9","revision":"definition-r1"}}]"##;
#[cfg(test)]
#[path = "creature_defense_variant_tests.rs"]
mod defense_variant_tests;

#[cfg(test)]
#[path = "creature_appearance_execution_tests.rs"]
mod appearance_execution_tests;

pub(crate) type WindupCastResults =
    Result<Vec<(Ref, Result<SpellOutcome, AttackError>)>, AttackError>;
