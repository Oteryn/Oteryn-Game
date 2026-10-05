//! Source-bound ATTACK-1 creature melee owner work item. Not a think/bite adapter.
//! Registered at crate root: standalone Ability fixtures have no Content dependency.
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet,
};
use crate::combat_attack::{
    AttackConstants, BlockBudget, armor_reduction_bounds, creature_melee_formula,
};
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord, ProjectV2AbilityEffect, ProjectV2AbilityKind,
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2FormulaAuthoring, ProjectV2Mitigation,
};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ApplicationFacts, ConditionDefinition, ConditionType, ConditionValues};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
/// Existing main formula evaluator and checked ATTACK constants are the only arithmetic owner.
pub(crate) fn canonical_melee_bounds(attack: u32, skill: u32) -> Result<(u64, u64), AttackError> {
    let constants = AttackConstants::checked_in().ok_or(AttackError::InvalidSource)?;
    let inputs = crate::spell::formula::FormulaInputs {
        level: 1,
        magic_level: 0,
        base_power: None,
        attack_skill: skill,
        attack_value: attack,
        attack_factor: 1.0,
        shielding_skill: 0,
        shield_defense: None,
    };
    let (lo, hi) = creature_melee_formula(constants)
        .bounds(&inputs)
        .map_err(|_| AttackError::NumericOverflow)?;
    Ok((
        u64::try_from(lo).map_err(|_| AttackError::NumericOverflow)?,
        u64::try_from(hi).map_err(|_| AttackError::NumericOverflow)?,
    ))
}
pub(crate) fn canonical_armor_bounds(armor: u32) -> Result<(u32, u32), AttackError> {
    Ok(armor_reduction_bounds(
        AttackConstants::checked_in().ok_or(AttackError::InvalidSource)?,
        armor,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackError {
    InvalidSource,
    UnsupportedShape,
    StaleIssuer,
    StaleTarget,
    ContentChanged,
    TargetChanged,
    NotDue,
    OutOfRange,
    Protected,
    OccurrenceConflict,
    OccurrenceSuperseded,
    LedgerFull,
    InvalidPlan,
    MissingVitals,
    NumericOverflow,
    MissingCombatFacts,
    StaleOwner,
    StaleAiSelection,
}
#[derive(Debug, Clone)]
pub(crate) struct MeleeSource {
    creature_key: String,
    pub(crate) ability: Ref,
    entry_index: usize,
    interval_micros: u64,
    chance_ppm: u32,
    minimum: u32,
    maximum: u32,
    critical: Option<crate::source_critical_baseline::SourceCriticalBaseline>,
    conditions: Vec<ConditionDefinition>,
    formula_revision: String,
    armor: bool,
    shield: bool,
    fingerprint: [u8; 32],
    content_digest: [u8; 32],
}
impl MeleeSource {
    /// All profiles/records must come from the owner's currently pinned native artifact.
    /// The complete source closure is fingerprinted; unsupported multi-effect attacks refuse.
    pub(crate) fn from_native(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        content_digest: [u8; 32],
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
        if index >= 16 || b.attacks.len() > 16 {
            return Err(AttackError::InvalidSource);
        }
        let e = b.attacks.get(index).ok_or(AttackError::InvalidSource)?;
        let Some(Data::Ability(a)) = map.get(&e.ability).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let d = a.details.as_deref().ok_or(AttackError::InvalidSource)?;
        if d.kind != ProjectV2AbilityKind::Melee
            || d.area.is_some()
            || !d.variants.is_empty()
            || d.chain.is_some()
            || d.encounter.is_some()
            || d.windup.is_some()
            || d.range_tiles != 1
            || e.range_tiles.is_some_and(|r| r != 1)
        {
            return Err(AttackError::UnsupportedShape);
        }
        let mut executable = Vec::new();
        let mut conditions = Vec::new();
        for effect in &d.effects {
            match effect {
                ProjectV2AbilityEffect::Executable(r) => executable.push(r),
                ProjectV2AbilityEffect::Inline(inline) => {
                    let revision = e
                        .ability
                        .revision
                        .strip_prefix("definition-r")
                        .and_then(|r| r.parse::<u32>().ok())
                        .filter(|r| *r > 0)
                        .ok_or(AttackError::InvalidSource)?;
                    let definition = crate::creature_condition_content::lower_condition_definition(
                        inline, revision, None,
                    )
                    .map_err(|_| AttackError::UnsupportedShape)?;
                    if !matches!(
                        definition.values(),
                        ConditionValues::DamageSchedule { .. }
                            | ConditionValues::DamageOverTime { .. }
                    ) {
                        return Err(AttackError::UnsupportedShape);
                    }
                    conditions.push(definition);
                }
            }
        }
        let [effect] = executable.as_slice() else {
            return Err(AttackError::UnsupportedShape);
        };
        let effect = *effect;
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
        let ar=records.iter().find(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..} if identity.key==e.ability.key && identity.revision==e.ability.revision && identity.family=="Ability")).ok_or(AttackError::InvalidSource)?;
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
        if ep.damage_type != "physical" || ep.affects.is_some() {
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
                canonical_melee_bounds(*attack, *skill)?
            }
            ProjectV2FormulaAuthoring::CasterMagnitude => {
                let m = e.magnitude.as_ref().ok_or(AttackError::InvalidSource)?;
                (m.minimum, m.maximum)
            }
            _ => return Err(AttackError::UnsupportedShape),
        };
        if minimum > maximum || e.interval_ms == 0 || e.chance_ppm > 1_000_000 {
            return Err(AttackError::InvalidSource);
        }
        let minimum = u32::try_from(minimum).map_err(|_| AttackError::NumericOverflow)?;
        let maximum = u32::try_from(maximum).map_err(|_| AttackError::NumericOverflow)?;
        let fingerprint = if let Some(policy) = critical {
            Sha256::digest(
                serde_json::to_vec(&(
                    creature,
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
                serde_json::to_vec(&(creature, index, e, a, ep, f, content_digest))
                    .map_err(|_| AttackError::InvalidSource)?,
            )
            .into()
        };
        Ok(Self {
            creature_key: creature.key.clone(),
            ability: e.ability.clone(),
            entry_index: index,
            interval_micros: e
                .interval_ms
                .checked_mul(1000)
                .ok_or(AttackError::NumericOverflow)?,
            chance_ppm: e.chance_ppm,
            minimum,
            maximum,
            critical,
            conditions,
            formula_revision: fref.revision.clone(),
            armor: ep.mitigated_by.contains(&ProjectV2Mitigation::Armor),
            shield: ep.mitigated_by.contains(&ProjectV2Mitigation::Shield),
            fingerprint,
            content_digest,
        })
    }
}

/// Independently current world/combat owner facts, resolved under the same Channel lock.
/// These fields do not acquire authority from Content. The caller must read current PZ,
/// visibility, re-entry and equipment/defence owners; revision is retained in replay binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AttackFacts {
    pub(crate) issuer: ExactActorRef,
    pub(crate) target: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) revision: u64,
    pub(crate) visible: bool,
    pub(crate) issuer_pz: bool,
    pub(crate) target_pz: bool,
    pub(crate) issuer_protected: bool,
    pub(crate) target_protected: bool,
    /// Owner-evaluated defence maximum from accepted player-expression/equipment facts.
    pub(crate) defense: u32,
    pub(crate) armor: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwingOutcome {
    pub(crate) occurrence: String,
    pub(crate) requested: u32,
    pub(crate) blocked: u32,
    pub(crate) damage: Option<PlayerDamageReceipt>,
    pub(crate) chance_passed: bool,
    pub(crate) source_critical: Option<crate::source_critical_baseline::SourceCriticalEvent>,
}
struct Entry {
    active: bool,
    issuer: ExactActorRef,
    entry_index: usize,
    interval_micros: u64,
    target: ExactActorRef,
    session: GameSessionId,
    lineage: String,
    source: [u8; 32],
    ready: SemanticTimeMicros,
    next_sequence: u64,
    last: Option<(u64, [u8; 32], Result<SwingOutcome, AttackError>)>,
}
struct DefenseBudget {
    target: ExactActorRef,
    session: GameSessionId,
    budget: BlockBudget,
}
#[derive(Default)]
pub(crate) struct AutoAttackOwner {
    entries: Vec<Entry>,
    blocks: Vec<DefenseBudget>,
    selections: Vec<AiSelection>,
}
/// Current accepted AI selection; its occurrence comes from the existing ThinkSequenceTracker.
struct AiSelection {
    issuer: ExactActorRef,
    sequence: u64,
    target: ExactActorRef,
    session: GameSessionId,
    sources: [u8; 32],
}
/// Implemented by the independently current world/combat owners, never the profile loader.
/// None is fail-closed and consumes this due swing, so missing facts are not buffered.
#[derive(Clone)]
pub(crate) struct CurrentConditionPolicy {
    pub(crate) immunities: Vec<ConditionType>,
    pub(crate) base_speed: u16,
}
pub(crate) trait CurrentCombatFactsReader {
    fn read_condition_policy(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _issuer: ExactActorRef,
        _target: ExactActorRef,
        _session: GameSessionId,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<CurrentConditionPolicy> {
        None
    }
    fn read_attack(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<AttackFacts>;
}
impl AutoAttackOwner {
    /// Consume a source-bound AI target choice under an existing current owner fence.
    /// No command authority or target-selection strategy is fabricated here.
    // Keep accept_ai_target ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn accept_ai_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        selection: crate::ai_think::ThinkOccurrence,
        target: ExactActorRef,
        session: GameSessionId,
        sources: &[MeleeSource],
        now: SemanticTimeMicros,
    ) -> Result<(), AttackError> {
        require_owner(runtime, fence, stamp)?;
        if sources.is_empty() || sources.len() > 16 {
            return Err(AttackError::InvalidSource);
        }
        let target_control = runtime
            .player_control_facts(target, session)
            .map_err(|_| AttackError::StaleTarget)?;
        if target_control.control_loss.is_some() {
            return Err(AttackError::StaleTarget);
        }
        let mut indices = std::collections::BTreeSet::new();
        for source in sources {
            validate_source(runtime, selection.actor, source)?;
            if !indices.insert(source.entry_index) {
                return Err(AttackError::InvalidSource);
            }
            now.get()
                .checked_add(source.interval_micros)
                .ok_or(AttackError::NumericOverflow)?;
        }
        self.selections
            .retain(|s| runtime.contains_live_creature(s.issuer));
        let mut fingerprints: Vec<_> = sources
            .iter()
            .map(|s| (s.entry_index, s.fingerprint))
            .collect();
        fingerprints.sort_by_key(|p| p.0);
        let binding = Sha256::digest(
            serde_json::to_vec(&fingerprints).map_err(|_| AttackError::InvalidSource)?,
        )
        .into();
        let old = self
            .selections
            .iter()
            .position(|s| s.issuer == selection.actor);
        if let Some(i) = old {
            let prior = &self.selections[i];
            if selection.sequence < prior.sequence {
                return Err(AttackError::StaleAiSelection);
            }
            if selection.sequence == prior.sequence {
                return if prior.target == target
                    && prior.session == session
                    && prior.sources == binding
                {
                    Ok(())
                } else {
                    Err(AttackError::OccurrenceConflict)
                };
            }
        }
        if let Some(i) = old
            && self.selections[i].target == target
            && self.selections[i].session == session
            && self.selections[i].sources == binding
        {
            // A later think retaining the same target does not postpone melee deadlines.
            self.selections[i].sequence = selection.sequence;
            return Ok(());
        }
        if old.is_none() && self.selections.len() >= 64 {
            return Err(AttackError::LedgerFull);
        }
        if old.is_none() {
            self.selections
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
        }
        let needed = sources
            .iter()
            .filter(|s| {
                !self
                    .entries
                    .iter()
                    .any(|e| e.issuer == selection.actor && e.entry_index == s.entry_index)
            })
            .count();
        if self.entries.len() + needed > 64 * 16 {
            return Err(AttackError::LedgerFull);
        }
        self.entries
            .try_reserve(needed)
            .map_err(|_| AttackError::LedgerFull)?;
        let lineage = format!(
            "ai-target:{}:{}",
            hex(&selection.actor.placement_identity()),
            selection.sequence
        );
        for source in sources {
            self.set_target(
                runtime,
                selection.actor,
                target,
                session,
                source,
                &lineage,
                now,
            )?;
        }
        let value = AiSelection {
            issuer: selection.actor,
            sequence: selection.sequence,
            target,
            session,
            sources: binding,
        };
        match old {
            Some(i) => self.selections[i] = value,
            None => self.selections.push(value),
        }
        Ok(())
    }
    /// Bounded current owner-cycle consumer, independent from AI think cadence.
    /// Per-entry deadlines and current facts are rechecked immediately before HP commit.
    // Keep dispatch_due ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_due(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        sources: &[(ExactActorRef, MeleeSource)],
        reader: &mut dyn CurrentCombatFactsReader,
        revisions: RevisionSet,
        now: SemanticTimeMicros,
    ) -> SwingDispatchResult {
        require_owner(runtime, fence, stamp)?;
        if sources.len() > 64 * 16 {
            return Err(AttackError::InvalidSource);
        }
        let mut scheduled = Vec::new();
        let mut keys = std::collections::BTreeSet::new();
        for (issuer, source) in sources {
            if !keys.insert((issuer.placement_identity(), source.entry_index)) {
                return Err(AttackError::InvalidSource);
            }
            if let Some(e) = self.entries.iter().find(|e| {
                e.issuer == *issuer
                    && e.entry_index == source.entry_index
                    && e.active
                    && now >= e.ready
            }) {
                scheduled.push((
                    e.ready.get(),
                    issuer.placement_identity(),
                    source.entry_index,
                    *issuer,
                    source,
                    e.target,
                    e.session,
                    e.next_sequence,
                ));
            }
        }
        scheduled.sort_by_key(|e| (e.0, e.1, e.2));
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut result = Vec::new();
        for (_, _, _, issuer, source, target, session, sequence) in scheduled {
            let outcome = match reader.read_attack(runtime, issuer, target, session, stamp) {
                Some(facts)
                    if facts.issuer == issuer
                        && facts.target == target
                        && facts.session == session =>
                {
                    let policy = if source.conditions.is_empty() {
                        None
                    } else {
                        reader.read_condition_policy(runtime, issuer, target, session, stamp)
                    };
                    self.swing_with_condition_policy(
                        runtime,
                        vitals,
                        source,
                        facts,
                        policy.as_ref(),
                        sequence,
                        revisions.clone(),
                        &root,
                        now,
                        fence,
                        stamp,
                    )
                }
                _ => {
                    let e = self
                        .entries
                        .iter_mut()
                        .find(|e| e.issuer == issuer && e.entry_index == source.entry_index)
                        .ok_or(AttackError::TargetChanged)?;
                    let next = sequence
                        .checked_add(1)
                        .ok_or(AttackError::NumericOverflow)?;
                    let ready = now
                        .get()
                        .checked_add(source.interval_micros)
                        .ok_or(AttackError::NumericOverflow)?;
                    let binding = Sha256::new()
                        .chain_update(b"missing-current-combat-facts")
                        .chain_update(source.fingerprint)
                        .chain_update(sequence.to_be_bytes())
                        .finalize()
                        .into();
                    e.next_sequence = next;
                    e.ready = SemanticTimeMicros::from_micros(ready);
                    e.last = Some((sequence, binding, Err(AttackError::MissingCombatFacts)));
                    Err(AttackError::MissingCombatFacts)
                }
            };
            result.push((source.ability.clone(), outcome));
        }
        Ok(result)
    }
    // Keep set_target source/owner ABI explicit: exact actor/source binding, occurrence, immutable definition and separately owned runtime/policy facts must not be conflated.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        source: &MeleeSource,
        lineage: &str,
        now: SemanticTimeMicros,
    ) -> Result<(), AttackError> {
        validate_source(runtime, issuer, source)?;
        let control = runtime
            .player_control_facts(target, session)
            .map_err(|_| AttackError::StaleTarget)?;
        if control.control_loss.is_some() {
            return Err(AttackError::StaleTarget);
        }
        if lineage.is_empty() || lineage.len() > 256 {
            return Err(AttackError::InvalidSource);
        }
        self.entries
            .retain(|e| runtime.contains_live_creature(e.issuer));
        let ready = now
            .get()
            .checked_add(source.interval_micros)
            .ok_or(AttackError::NumericOverflow)?;
        if !self
            .entries
            .iter()
            .any(|e| e.issuer == issuer && e.entry_index == source.entry_index)
            && self.entries.len() >= 64 * 16
        {
            return Err(AttackError::LedgerFull);
        }
        // One AI target per actor, independent authored entry deadlines/occurrences.
        // Preflight all deadline arithmetic before mutating any retained entry.
        for e in self.entries.iter().filter(|e| {
            e.issuer == issuer
                && (e.target != target || e.session != session || e.lineage != lineage)
        }) {
            now.get()
                .checked_add(e.interval_micros)
                .ok_or(AttackError::NumericOverflow)?;
        }
        // Same exact current victim/session shares one canonical budget across attackers.
        // PROJECT_TARGET_OBSERVATION_BLOCK_PHASE: no invented player spawn timestamp.
        if source.shield {
            let constants = AttackConstants::checked_in().ok_or(AttackError::InvalidSource)?;
            self.blocks
                .retain(|b| runtime.player_control_facts(b.target, b.session).is_ok());
            if !self
                .blocks
                .iter()
                .any(|b| b.target == target && b.session == session)
            {
                if self.blocks.len() >= 64 {
                    return Err(AttackError::LedgerFull);
                }
                self.blocks
                    .try_reserve(1)
                    .map_err(|_| AttackError::LedgerFull)?;
                self.blocks.push(DefenseBudget {
                    target,
                    session,
                    budget: BlockBudget::new(
                        &constants.block,
                        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get()),
                    ),
                });
            }
        }
        for e in self.entries.iter_mut().filter(|e| {
            e.issuer == issuer
                && (e.target != target || e.session != session || e.lineage != lineage)
        }) {
            e.target = target;
            e.session = session;
            e.lineage = lineage.to_owned();
            e.active = true;
            e.ready = SemanticTimeMicros::from_micros(now.get() + e.interval_micros);
            e.last = None;
        }
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.issuer == issuer && e.entry_index == source.entry_index)
        {
            if e.active
                && e.target == target
                && e.session == session
                && e.lineage == lineage
                && e.source == source.fingerprint
            {
                return Ok(());
            }
            e.active = true;
            e.interval_micros = source.interval_micros;
            e.target = target;
            e.session = session;
            e.lineage = lineage.to_owned();
            e.source = source.fingerprint;
            e.ready = SemanticTimeMicros::from_micros(ready);
            // Never reset sequence on target changes: old swing identity cannot be reused.
            e.last = None;
            return Ok(());
        }
        if self.entries.len() >= 64 * 16 {
            return Err(AttackError::LedgerFull);
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.entries.push(Entry {
            active: true,
            issuer,
            entry_index: source.entry_index,
            interval_micros: source.interval_micros,
            target,
            session,
            lineage: lineage.to_owned(),
            source: source.fingerprint,
            ready: SemanticTimeMicros::from_micros(ready),
            next_sequence: 1,
            last: None,
        });
        Ok(())
    }
    pub(crate) fn next_sequence(&self, issuer: ExactActorRef, entry_index: usize) -> Option<u64> {
        self.entries
            .iter()
            .find(|e| e.issuer == issuer && e.entry_index == entry_index)
            .map(|e| e.next_sequence)
    }
    pub(crate) fn clear_target(&mut self, issuer: ExactActorRef) {
        for e in self.entries.iter_mut().filter(|e| e.issuer == issuer) {
            e.active = false;
        }
    }
    // Keep swing ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn swing(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &MeleeSource,
        facts: AttackFacts,
        sequence: u64,
        revisions: RevisionSet,
        root: &GameplayDecisionRoot,
        now: SemanticTimeMicros,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Result<SwingOutcome, AttackError> {
        self.swing_with_condition_policy(
            runtime, vitals, source, facts, None, sequence, revisions, root, now, fence, stamp,
        )
    }
    // Keep swing_with_condition_policy ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn swing_with_condition_policy(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &MeleeSource,
        facts: AttackFacts,
        condition_policy: Option<&CurrentConditionPolicy>,
        sequence: u64,
        revisions: RevisionSet,
        root: &GameplayDecisionRoot,
        now: SemanticTimeMicros,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Result<SwingOutcome, AttackError> {
        require_owner(runtime, fence, stamp)?;
        validate_source(runtime, facts.issuer, source)?;
        if root != &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest())
        {
            return Err(AttackError::InvalidSource);
        }
        if facts.revision == 0 || revisions.formula() != source.formula_revision {
            return Err(AttackError::InvalidSource);
        }
        let i = self
            .entries
            .iter()
            .position(|e| e.issuer == facts.issuer && e.entry_index == source.entry_index)
            .ok_or(AttackError::TargetChanged)?;
        let e = &self.entries[i];
        let binding = Sha256::digest(
            serde_json::to_vec(&(
                source.fingerprint,
                format!("{facts:?}"),
                format!("{revisions:?}"),
                condition_policy.map(|p| format!("{:?}:{}", p.immunities, p.base_speed)),
                &e.lineage,
            ))
            .map_err(|_| AttackError::InvalidSource)?,
        )
        .into();
        if let Some((seq, prior, result)) = &e.last
            && sequence == *seq
        {
            return if *prior == binding {
                result.clone()
            } else {
                Err(AttackError::OccurrenceConflict)
            };
        }
        if sequence < e.next_sequence {
            return Err(AttackError::OccurrenceSuperseded);
        }
        if sequence != e.next_sequence {
            return Err(AttackError::OccurrenceConflict);
        }
        if !e.active
            || e.target != facts.target
            || e.session != facts.session
            || e.source != source.fingerprint
            || e.lineage.is_empty()
        {
            return Err(AttackError::TargetChanged);
        }
        // Not-due calls never consume a swing; owner waits until actual deadline.
        if now < e.ready {
            return Err(AttackError::NotDue);
        }
        let next = sequence
            .checked_add(1)
            .ok_or(AttackError::NumericOverflow)?;
        let ready = now
            .get()
            .checked_add(source.interval_micros)
            .ok_or(AttackError::NumericOverflow)?;
        let result = self.apply_swing(
            runtime,
            vitals,
            source,
            facts,
            condition_policy,
            sequence,
            revisions,
            root,
            now,
            fence,
            stamp,
        );
        let clear = matches!(&result, Err(AttackError::StaleTarget))
            || result
                .as_ref()
                .is_ok_and(|o| o.damage.as_ref().is_some_and(|d| d.health_after == 0));
        let e = &mut self.entries[i];
        e.next_sequence = next;
        e.ready = SemanticTimeMicros::from_micros(ready);
        e.last = Some((sequence, binding, result.clone()));
        if clear {
            for e in self
                .entries
                .iter_mut()
                .filter(|e| e.issuer == facts.issuer && e.target == facts.target)
            {
                e.active = false;
            }
        }
        result
    }
    // Keep apply_swing ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_swing(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &MeleeSource,
        facts: AttackFacts,
        condition_policy: Option<&CurrentConditionPolicy>,
        sequence: u64,
        revisions: RevisionSet,
        root: &GameplayDecisionRoot,
        now: SemanticTimeMicros,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Result<SwingOutcome, AttackError> {
        let control = runtime
            .player_control_facts(facts.target, facts.session)
            .map_err(|_| AttackError::StaleTarget)?;
        if control.control_loss.is_some() {
            return Err(AttackError::StaleTarget);
        }
        let from = runtime
            .read_actor_position(facts.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = runtime
            .read_actor_position(facts.target)
            .map_err(|_| AttackError::StaleTarget)?;
        if from.context() != to.context()
            || from.position().floor != to.position().floor
            || (i64::from(from.position().x) - i64::from(to.position().x))
                .abs()
                .max((i64::from(from.position().y) - i64::from(to.position().y)).abs())
                != 1
        {
            return Err(AttackError::OutOfRange);
        }
        if !facts.visible
            || facts.issuer_pz
            || facts.target_pz
            || facts.issuer_protected
            || facts.target_protected
        {
            return Err(AttackError::Protected);
        }
        if !source.conditions.is_empty() && condition_policy.is_none() {
            return Err(AttackError::MissingCombatFacts);
        }
        let occurrence = format!(
            "auto-attack:{}:{}:{sequence}",
            hex(&facts.issuer.placement_identity()),
            source.entry_index
        );
        let digest = Sha256::new()
            .chain_update(b"oteryn:auto-attack:v1")
            .chain_update(occurrence.as_bytes())
            .finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        let id = DecisionOccurrenceId::from_bytes(bytes);
        let draw = |purpose: &str, min: u32, max: u32| -> Result<u32, AttackError> {
            let d = deterministic_decision_u64(root, id, purpose, source.entry_index as u64)
                .map_err(|_| AttackError::InvalidPlan)?;
            u32::try_from(crate::spell::uniform_draw(
                d,
                i64::from(min),
                i64::from(max),
            ))
            .map_err(|_| AttackError::InvalidPlan)
        };
        let chance = deterministic_decision_u64(root, id, "AI_ATTACK", source.entry_index as u64)
            .map_err(|_| AttackError::InvalidPlan)?
            % 1_000_000;
        if chance >= u64::from(source.chance_ppm) {
            return Ok(SwingOutcome {
                occurrence,
                requested: 0,
                blocked: 0,
                damage: None,
                chance_passed: false,
                source_critical: None,
            });
        }
        let requested = draw("damage_draw", source.minimum, source.maximum)?;
        let source_critical = source
            .critical
            .map(|policy| {
                policy
                    .draw(root, id, source.entry_index as u64)
                    .map_err(|_| AttackError::InvalidPlan)
            })
            .transpose()?;
        // Source-default bonus is ZERO; preserve actual base requested HP. Dynamic boosts/VFX remain qualified gaps.
        let mut remaining = requested;
        let mut budget_next = None;
        if source.shield {
            self.blocks
                .retain(|b| runtime.player_control_facts(b.target, b.session).is_ok());
            let bi = self
                .blocks
                .iter()
                .position(|b| b.target == facts.target && b.session == facts.session);
            if bi.is_none() {
                if self.blocks.len() >= 64 {
                    return Err(AttackError::LedgerFull);
                }
                self.blocks
                    .try_reserve(1)
                    .map_err(|_| AttackError::LedgerFull)?;
            }
            let constants = AttackConstants::checked_in().ok_or(AttackError::InvalidSource)?;
            let mut budget = bi.map(|i| self.blocks[i].budget).unwrap_or_else(|| {
                BlockBudget::new(
                    &constants.block,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get()),
                )
            });
            if budget.try_block(
                &constants.block,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get()),
            ) {
                // facts.defense is independently evaluated upstream, including actual player
                // equipment, fight mode and swing state; do not fabricate those missing inputs.
                remaining = remaining.saturating_sub(draw(
                    "defence_draw",
                    facts.defense / 2,
                    facts.defense,
                )?);
            }
            budget_next = Some((bi, budget));
        }

        if source.armor {
            let (lo, hi) = canonical_armor_bounds(facts.armor)?;
            let armor = if lo == hi {
                lo
            } else {
                draw("armor_draw", lo, hi)?
            };
            remaining = remaining.saturating_sub(armor);
        }
        let damage = if remaining == 0 {
            None
        } else {
            let issuer = format!("actor:{}", hex(&facts.issuer.placement_identity()));
            let target = format!("actor:{}", hex(&facts.target.placement_identity()));
            let plan = EffectPlan::immediate(
                AbilityOccurrence::new(&occurrence, revisions)
                    .map_err(|_| AttackError::InvalidPlan)?,
                AbilityIntent::resolve(ProposalSource::Ai, &issuer, &[&target], &[&target])
                    .map_err(|_| AttackError::InvalidPlan)?,
                vec![
                    Effect::damage(&target, i64::from(remaining))
                        .map_err(|_| AttackError::InvalidPlan)?,
                ],
                Vec::new(),
                CommitGroup::atomic("channel-owner", &occurrence)
                    .map_err(|_| AttackError::InvalidPlan)?,
            )
            .map_err(|_| AttackError::InvalidPlan)?;
            let [Effect::Damage { magnitude, .. }] = plan.effects() else {
                return Err(AttackError::InvalidPlan);
            };
            let magnitude = u32::try_from(*magnitude).map_err(|_| AttackError::InvalidPlan)?;
            Some(
                if source.conditions.is_empty() {
                    vitals.apply_attack_damage(
                        runtime,
                        facts.target,
                        facts.session,
                        magnitude,
                        &occurrence,
                        now,
                    )
                } else {
                    let policy = condition_policy.ok_or(AttackError::MissingCombatFacts)?;
                    let application = ApplicationFacts {
                        now: now.get(),
                        base_speed: policy.base_speed,
                        mana_shield_capacity: 0,
                        target_reentry_protected: facts.target_protected,
                        source_reentry_protected: facts.issuer_protected,
                        target_is_player: true,
                        decision_root: root,
                        occurrence: id,
                    };
                    vitals.apply_composite_attack_damage(
                        runtime,
                        facts.target,
                        facts.session,
                        magnitude,
                        &occurrence,
                        facts.issuer,
                        &source.conditions,
                        &application,
                        &policy.immunities,
                        fence,
                        stamp,
                    )
                }
                .ok_or(AttackError::MissingVitals)?,
            )
        };
        if let Some((bi, budget)) = budget_next {
            match bi {
                Some(i) => self.blocks[i].budget = budget,
                None => self.blocks.push(DefenseBudget {
                    target: facts.target,
                    session: facts.session,
                    budget,
                }),
            }
        }
        Ok(SwingOutcome {
            occurrence,
            requested,
            blocked: requested - remaining,
            damage,
            chance_passed: true,
            source_critical,
        })
    }
}
fn require_owner(
    runtime: &ChannelRuntimeV1,
    fence: &crate::foundation::ScopeRuntimeFence,
    stamp: crate::foundation::RuntimeWorkStamp,
) -> Result<(), AttackError> {
    let b = runtime.binding();
    let scope = crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
    if !fence.is_current_for_scope(scope, b.scope_generation()) || !fence.accepts_stamp(stamp) {
        return Err(AttackError::StaleOwner);
    }
    Ok(())
}
fn validate_source(
    runtime: &ChannelRuntimeV1,
    issuer: ExactActorRef,
    source: &MeleeSource,
) -> Result<(), AttackError> {
    if !runtime.matches_live_creature_identity(issuer, source.creature_key.as_bytes()) {
        return Err(AttackError::StaleIssuer);
    }
    if runtime.content_pin().server_artifact_digest() != source.content_digest {
        return Err(AttackError::ContentChanged);
    }
    Ok(())
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    // Positive-only fixture grant; real runtime paths receive the independently current owner fence.
    impl AutoAttackOwner {
        fn fixture_swing(
            &mut self,
            runtime: &mut ChannelRuntimeV1,
            vitals: &mut dyn PlayerLethalVitals,
            source: &MeleeSource,
            facts: AttackFacts,
            sequence: u64,
            revisions: RevisionSet,
            root: &GameplayDecisionRoot,
            now: SemanticTimeMicros,
        ) -> Result<SwingOutcome, AttackError> {
            let (fence, stamp) = current_fence(runtime);
            self.swing(
                runtime, vitals, source, facts, sequence, revisions, root, now, &fence, stamp,
            )
        }
    }

    #[test]
    fn source_swing_superseded_external_grant_refuses_before_primary_or_sequence_write() {
        let (mut runtime, mut states, source, facts, root) = setup("1st_mate_ratticus", false);
        let mut attack = owner(&runtime, &source, facts);
        let before = crate::gameplay_transport::actor_spell::observe_vitals(
            &runtime,
            &states,
            facts.target,
            facts.session,
        );
        let sequence = attack.next_sequence(facts.issuer, source.entry_index);
        let (mut fence, stamp) = current_fence(&runtime);
        fence
            .apply_external_grant(
                crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
            )
            .expect("qualified fixture");
        assert_eq!(
            attack.swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
                &fence,
                stamp
            ),
            Err(AttackError::StaleOwner)
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(
                &runtime,
                &states,
                facts.target,
                facts.session
            ),
            before
        );
        assert_eq!(
            attack.next_sequence(facts.issuer, source.entry_index),
            sequence
        );
    }

    use super::*;
    use crate::foundation::MovementLocalPosition;
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    fn clock(us: u64) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(us)
    }
    fn revisions() -> RevisionSet {
        RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("revisions")
    }
    fn native_data() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("creature_auto_attack_test_data.json"))
                .expect("source fixture");
        (
            serde_json::from_value(value["records"].clone()).expect("records"),
            serde_json::from_value(value["authoring_profiles"].clone()).expect("profiles"),
        )
    }
    fn setup(
        name: &str,
        far: bool,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        MeleeSource,
        AttackFacts,
        GameplayDecisionRoot,
    ) {
        let (mut runtime, target, session) = runtime_with_player(0x71);
        runtime
            .initialize_movement_test_position(
                target,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("actual position");
        let (r, p) = native_data();
        let key = format!("oteryn:creature.{name}");
        let health = p
            .iter()
            .find_map(|p| match &p.data {
                Data::Creature(c) if p.target.key == key => c.health,
                _ => None,
            })
            .expect("native source health");
        let issuer = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: if far { 110 } else { 101 },
                    y: 100,
                    floor: 7,
                },
                &key,
                i64::try_from(health).expect("source health bound"),
            )
            .expect("actual native admission");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("actual vitals");
        let cref = Ref {
            family: ProjectV2Family::Creature,
            key: format!("oteryn:creature.{name}"),
            revision: "definition-r1".into(),
        };
        let source = MeleeSource::from_native(
            &cref,
            0,
            &r,
            &p,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("source-bound melee");
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = AttackFacts {
            issuer,
            target,
            session,
            revision: 1,
            visible: true,
            issuer_pz: false,
            target_pz: false,
            issuer_protected: false,
            target_protected: false,
            defense: 0,
            armor: 0,
        };
        (runtime, states, source, facts, root)
    }
    fn owner(runtime: &ChannelRuntimeV1, s: &MeleeSource, f: AttackFacts) -> AutoAttackOwner {
        let mut o = AutoAttackOwner::default();
        o.set_target(
            runtime,
            f.issuer,
            f.target,
            f.session,
            s,
            "target-command:1",
            clock(0),
        )
        .expect("current target");
        o
    }
    #[test]
    fn native_range_auto_attack_commits_lethal_real_vitals_once() {
        let (mut runtime, mut states, source, facts, root) = setup("1st_mate_ratticus", false);
        let mut owner = owner(&runtime, &source, facts);
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(1)
            ),
            Err(AttackError::NotDue)
        );
        let hit = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
            )
            .expect("real owner hit");
        assert!((200..=300).contains(&hit.requested));
        let damage = hit.damage.expect("actual damage receipt");
        assert_eq!(damage.applied, 185);
        assert_eq!(damage.health_after, 0);
        assert!(damage.death.is_some());
        assert_eq!(
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source,
                    facts,
                    1,
                    revisions(),
                    &root,
                    clock(9_000_000)
                )
                .expect("replay"),
            hit
        );
        let mut changed = facts;
        changed.revision = 2;
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                changed,
                1,
                revisions(),
                &root,
                clock(9_000_000)
            ),
            Err(AttackError::OccurrenceConflict)
        );
    }
    #[test]
    fn actual_armor_shield_and_stall_do_not_replay_backlog() {
        let (mut runtime, mut states, source, mut facts, root) = setup("1st_mate_ratticus", false);
        facts.defense = 1000;
        let mut owner = owner(&runtime, &source, facts);
        let blocked = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(100_000_000),
            )
            .expect("blocked hit");
        assert_eq!(blocked.blocked, blocked.requested);
        assert!(blocked.damage.is_none());
        assert_eq!(
            owner.next_sequence(facts.issuer, source.entry_index),
            Some(2)
        );
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(100_000_000)
            ),
            Err(AttackError::NotDue)
        );
        facts.defense = 0;
        facts.armor = 3;
        facts.revision = 2;
        let hit = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(102_000_000),
            )
            .expect("next actual deadline");
        assert_eq!(hit.blocked, 1);
        assert_eq!(hit.damage.expect("lethal").health_after, 0);
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(104_000_000)
            ),
            Err(AttackError::OccurrenceSuperseded)
        );
    }
    #[test]
    fn protected_due_occurrence_is_retained_not_buffered() {
        let (mut runtime, mut states, source, mut facts, root) = setup("1st_mate_ratticus", false);
        facts.target_protected = true;
        let mut owner = owner(&runtime, &source, facts);
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000)
            ),
            Err(AttackError::Protected)
        );
        facts.target_protected = false;
        facts.revision = 2;
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(3_000_000)
            ),
            Err(AttackError::OccurrenceConflict)
        );
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(3_000_000)
            ),
            Err(AttackError::NotDue)
        );
        assert_eq!(
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source,
                    facts,
                    2,
                    revisions(),
                    &root,
                    clock(4_000_000)
                )
                .expect("fresh swing")
                .damage
                .expect("HP receipt")
                .health_after,
            0
        );
    }
    #[test]
    fn independent_current_range_generation_and_content_guards() {
        let (mut runtime, mut states, mut source, facts, root) = setup("1st_mate_ratticus", true);
        let mut owner = owner(&runtime, &source, facts);
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000)
            ),
            Err(AttackError::OutOfRange)
        );
        source.content_digest = [0; 32];
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(4_000_000)
            ),
            Err(AttackError::ContentChanged)
        );
        runtime
            .remove_test_actor(facts.issuer)
            .expect("despawn current actor");
        assert_eq!(
            owner.fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(4_000_000)
            ),
            Err(AttackError::StaleIssuer)
        );
    }
    #[test]
    fn donor_melee_attack_skill_formula_and_missing_native_ref_refusal() {
        let (runtime, _states, source, _facts, _root) = setup("abyssador", false);
        assert_eq!((source.minimum, source.maximum), (0, 2325));
        let (r, mut p) = native_data();
        let cref = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.abyssador".into(),
            revision: "definition-r1".into(),
        };
        p.retain(|p| p.target.family != ProjectV2Family::Formula);
        assert!(matches!(
            MeleeSource::from_native(
                &cref,
                0,
                &r,
                &p,
                runtime.content_pin().server_artifact_digest()
            ),
            Err(AttackError::InvalidSource)
        ));
    }
    #[test]
    fn native_two_melee_entries_have_independent_sequences_and_occurrences() {
        let (mut runtime, mut states, source0, mut facts, root) = setup("an_astral_glyph", false);
        facts.defense = 10000;
        let mut owner = owner(&runtime, &source0, facts);
        let (r, p) = native_data();
        let cref = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.an_astral_glyph".into(),
            revision: "definition-r1".into(),
        };
        let source1 = MeleeSource::from_native(
            &cref,
            1,
            &r,
            &p,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("actual second melee");
        owner
            .set_target(
                &runtime,
                facts.issuer,
                facts.target,
                facts.session,
                &source1,
                "target-command:1",
                clock(0),
            )
            .expect("second native source entry");
        let first = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source0,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
            )
            .expect("first entry");
        assert_eq!(owner.next_sequence(facts.issuer, 0), Some(2));
        assert_eq!(owner.next_sequence(facts.issuer, 1), Some(1));
        let second = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source1,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
            )
            .expect("independent due entry");
        assert_ne!(first.occurrence, second.occurrence);
        assert!(first.damage.is_none());
        assert!(second.damage.is_none());
        assert_eq!(
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source0,
                    facts,
                    1,
                    revisions(),
                    &root,
                    clock(4_000_000)
                )
                .expect("first replay"),
            first
        );
        assert_eq!(
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source1,
                    facts,
                    1,
                    revisions(),
                    &root,
                    clock(4_000_000)
                )
                .expect("second replay"),
            second
        );
    }
    fn test_uuid(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
        ]
    }
    #[test]
    fn shield_budget_65_session_turnovers_purge_stale_and_preserve_live_other_tokens() {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        let world = WorldId::decode(&test_uuid(0x60)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&test_uuid(0x61)).expect("channel"),
            NodeId::decode(&test_uuid(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            ChannelContentPin::test(world),
        )
        .expect("real three-slot runtime");
        let persistent_session = GameSessionId::decode(&test_uuid(0x71)).expect("session");
        let reservation = runtime
            .reserve_fresh_session(persistent_session)
            .expect("reserve");
        let persistent = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .initialize_movement_test_position(
                persistent,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("position");
        let (r, p) = native_data();
        let cref = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.1st_mate_ratticus".into(),
            revision: "definition-r1".into(),
        };
        let source = MeleeSource::from_native(
            &cref,
            0,
            &r,
            &p,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("native melee");
        let issuer = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                &cref.key,
                200000,
            )
            .expect("actual creature source health");
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut states = ChannelSpellStates::default();
        let init_time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0);
        states
            .initialize(
                &runtime,
                persistent,
                persistent_session,
                FACTS,
                (0, 0),
                init_time,
            )
            .expect("actual persistent vitals");
        let mut facts = AttackFacts {
            issuer,
            target: persistent,
            session: persistent_session,
            revision: 1,
            visible: true,
            issuer_pz: false,
            target_pz: false,
            issuer_protected: false,
            target_protected: false,
            defense: 10000,
            armor: 0,
        };
        let mut owner = owner(&runtime, &source, facts);
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
            )
            .expect("actual first shield use");
        assert_eq!(u32::from(owner.blocks[0].budget.blocks()), 1);
        for n in 0..65u8 {
            let session = GameSessionId::decode(&test_uuid(0x80 + n)).expect("new session");
            let reservation = runtime
                .reserve_fresh_session(session)
                .expect("turnover reserve");
            let target = runtime
                .commit_fresh_session(reservation)
                .expect("turnover commit");
            runtime
                .initialize_movement_test_position(
                    target,
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                )
                .expect("current position");
            states
                .initialize(&runtime, target, session, FACTS, (0, 0), init_time)
                .expect("actual new vitals");
            facts.target = target;
            facts.session = session;
            facts.revision = u64::from(n) + 2;
            let now = (u64::from(n) + 1) * 2_000_000;
            owner
                .set_target(
                    &runtime,
                    issuer,
                    target,
                    session,
                    &source,
                    &format!("target-command:{}", n + 2),
                    clock(now),
                )
                .expect("new current target");
            let sequence = owner.next_sequence(issuer, 0).expect("retained sequence");
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source,
                    facts,
                    sequence,
                    revisions(),
                    &root,
                    clock(now + 2_000_000),
                )
                .expect("65-turnover guard regression");
            assert!(owner.blocks.len() <= 2);
            assert_eq!(
                owner
                    .blocks
                    .iter()
                    .find(|b| b.target == persistent && b.session == persistent_session)
                    .expect("live other budget retained")
                    .budget
                    .blocks(),
                1
            );
            runtime
                .remove_terminal_session(session, target)
                .expect("current owner removes terminal actor");
        }
    }

    struct ActualFactsReader(AttackFacts);
    impl CurrentCombatFactsReader for ActualFactsReader {
        fn read_attack(
            &mut self,
            runtime: &ChannelRuntimeV1,
            issuer: ExactActorRef,
            target: ExactActorRef,
            session: GameSessionId,
            _stamp: crate::foundation::RuntimeWorkStamp,
        ) -> Option<AttackFacts> {
            runtime.player_control_facts(target, session).ok()?;
            runtime.read_actor_position(issuer).ok()?;
            runtime.read_actor_position(target).ok()?;
            (issuer == self.0.issuer && target == self.0.target && session == self.0.session)
                .then_some(self.0)
        }
    }
    struct MissingFacts;
    impl CurrentCombatFactsReader for MissingFacts {
        fn read_attack(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: GameSessionId,
            _: crate::foundation::RuntimeWorkStamp,
        ) -> Option<AttackFacts> {
            None
        }
    }
    fn current_fence(
        runtime: &ChannelRuntimeV1,
    ) -> (
        crate::foundation::ScopeRuntimeFence,
        crate::foundation::RuntimeWorkStamp,
    ) {
        let b = runtime.binding();
        crate::foundation::crystal_timer_fixture(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("existing test-only native owner grant")
    }
    #[test]
    fn typed_first_ai_sequence_zero_valid_replay_and_later_think_does_not_starve_melee() {
        let (mut runtime, mut states, source, facts, _root) = setup("1st_mate_ratticus", false);
        let mut owner = AutoAttackOwner::default();
        let (fence, stamp) = current_fence(&runtime);
        let mut tracker = crate::ai_think::ThinkSequenceTracker::new();
        let zero = tracker.next_occurrence(facts.issuer);
        assert_eq!(zero.sequence, 0);
        owner
            .accept_ai_target(
                &runtime,
                &fence,
                stamp,
                zero,
                facts.target,
                facts.session,
                &[source.clone()],
                clock(0),
            )
            .expect("lawful first zero");
        owner
            .accept_ai_target(
                &runtime,
                &fence,
                stamp,
                zero,
                facts.target,
                facts.session,
                &[source.clone()],
                clock(500_000),
            )
            .expect("duplicate selection");
        let one = tracker.next_occurrence(facts.issuer);
        owner
            .accept_ai_target(
                &runtime,
                &fence,
                stamp,
                one,
                facts.target,
                facts.session,
                &[source.clone()],
                clock(1_000_000),
            )
            .expect("same target later think");
        assert_eq!(
            owner.accept_ai_target(
                &runtime,
                &fence,
                stamp,
                zero,
                facts.target,
                facts.session,
                &[source.clone()],
                clock(1_500_000)
            ),
            Err(AttackError::StaleAiSelection)
        );
        let got = owner
            .dispatch_due(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                &[(facts.issuer, source)],
                &mut ActualFactsReader(facts),
                revisions(),
                clock(2_000_000),
            )
            .expect("actual due dispatcher");
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0]
                .1
                .as_ref()
                .expect("native HP commit")
                .damage
                .expect("actual HP receipt")
                .health_after,
            0
        );
    }
    #[test]
    fn missing_current_combat_facts_consumes_due_without_hp_and_stale_owner_refuses() {
        let (mut runtime, mut states, source, facts, _root) = setup("1st_mate_ratticus", false);
        let mut owner = AutoAttackOwner::default();
        let (mut fence, stamp) = current_fence(&runtime);
        let mut tracker = crate::ai_think::ThinkSequenceTracker::new();
        let occurrence = tracker.next_occurrence(facts.issuer);
        owner
            .accept_ai_target(
                &runtime,
                &fence,
                stamp,
                occurrence,
                facts.target,
                facts.session,
                &[source.clone()],
                clock(0),
            )
            .expect("target");
        let sources = [(facts.issuer, source.clone())];
        let got = owner
            .dispatch_due(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                &sources,
                &mut MissingFacts,
                revisions(),
                clock(2_000_000),
            )
            .expect("failclosed due");
        assert_eq!(got[0].1, Err(AttackError::MissingCombatFacts));
        assert_eq!(owner.next_sequence(facts.issuer, 0), Some(2));
        assert!(
            owner
                .dispatch_due(
                    &mut runtime,
                    &fence,
                    stamp,
                    &mut states,
                    &sources,
                    &mut ActualFactsReader(facts),
                    revisions(),
                    clock(2_000_000)
                )
                .expect("no buffered damage")
                .is_empty()
        );
        fence
            .apply_external_grant(
                crate::foundation::ScopeOwnershipGeneration::new(2).expect("generation"),
            )
            .expect("actual fence handoff");
        assert!(matches!(
            owner.dispatch_due(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                &sources,
                &mut ActualFactsReader(facts),
                revisions(),
                clock(4_000_000)
            ),
            Err(AttackError::StaleOwner)
        ));
        // Read through a real one-point vital commit: prior refused work left all185HP intact.
        let probe = states
            .apply_attack_damage(
                &mut runtime,
                facts.target,
                facts.session,
                1,
                "independent-vitals-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(4_000_000),
            )
            .expect("real vital store");
        assert_eq!(probe.health_after, 184);
    }

    #[test]
    fn native_cat_zero_damage_is_legal_noop_without_vital_or_death_commit() {
        let (mut runtime, mut states, source, facts, root) = setup("cat", false);
        assert_eq!((source.minimum, source.maximum), (0, 0));
        let mut owner = owner(&runtime, &source, facts);
        let hit = owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(2_000_000),
            )
            .expect("source-native zero magnitude");
        assert_eq!(hit.requested, 0);
        assert!(hit.damage.is_none());
        assert!(hit.chance_passed);
        assert_eq!(
            owner
                .fixture_swing(
                    &mut runtime,
                    &mut states,
                    &source,
                    facts,
                    1,
                    revisions(),
                    &root,
                    clock(9_000_000)
                )
                .expect("noop replay"),
            hit
        );
        assert_eq!(
            states
                .apply_attack_damage(
                    &mut runtime,
                    facts.target,
                    facts.session,
                    1,
                    "zero-melee-probe",
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("real unchanged vital store")
                .health_after,
            184
        );
    }
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod canonical_kernel_tests {
    use super::*;
    #[test]
    fn source_ranges_reuse_main_formula_for_supported_native_skill_inputs() {
        for (attack, skill, expected) in [(20, 20, 30), (35, 70, 140), (1, 1, 1), (222, 88, 1088)] {
            let range = canonical_melee_bounds(attack, skill).expect("qualified fixture");
            assert_eq!(range, (0, expected));
        }
        assert_eq!(
            canonical_armor_bounds(0).expect("qualified fixture"),
            (0, 0)
        );
        assert_eq!(
            canonical_armor_bounds(3).expect("qualified fixture"),
            (1, 1)
        );
        assert_eq!(
            canonical_armor_bounds(44).expect("qualified fixture"),
            (22, 43)
        );
    }
    #[test]
    fn initial_zero_and_fixed_phase_refill_uses_existing_main_budget() {
        let c = AttackConstants::checked_in().expect("qualified fixture");
        let t = |v| oteryn_simulation_determinism::SemanticTimeMicros::from_micros(v);
        let mut b = BlockBudget::new(&c.block, t(123));
        assert_eq!(b.blocks(), 0);
        assert!(!b.try_block(&c.block, t(123)));
        assert!(!b.try_block(&c.block, t(1_000_122)));
        assert!(b.try_block(&c.block, t(1_000_123)));
        assert!(!b.try_block(&c.block, t(1_000_123)));
        assert!(b.try_block(&c.block, t(100_000_123)));
        assert!(b.try_block(&c.block, t(100_000_123)));
        assert!(!b.try_block(&c.block, t(100_000_123)));
        assert!(b.try_block(&c.block, t(101_000_123)));
    }
}

#[cfg(test)]
impl AutoAttackOwner {
    pub(crate) fn has_active_target_test(&self, actor: ExactActorRef) -> bool {
        self.entries.iter().any(|e| e.issuer == actor && e.active)
    }
}

type SwingDispatchResult = Result<Vec<(Ref, Result<SwingOutcome, AttackError>)>, AttackError>;
