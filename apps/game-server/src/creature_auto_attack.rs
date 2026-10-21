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
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2FormulaAuthoring, ProjectV2Magnitude,
    ProjectV2Mitigation,
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

/// Concrete native profile data over the shared Foundation proposal shape.
pub type ProfileAbilityProposal =
    crate::foundation::profile_schedule::ProfileAbilityProposal<Ref, ProjectV2Magnitude>;

/// Existing main formula evaluator and checked ATTACK constants are the only arithmetic owner.
pub(crate) fn canonical_melee_bounds(attack: u32, skill: u32) -> Result<(u64, u64), AttackError> {
    let constants = AttackConstants::checked_in().ok_or(AttackError::InvalidSource)?;
    let inputs = crate::combat::formula::FormulaInputs {
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
    pub(crate) entry_index: usize,
    interval_micros: u64,
    chance_ppm: u32,
    pub(crate) minimum: u32,
    pub(crate) maximum: u32,
    critical: Option<crate::source_critical_baseline::SourceCriticalBaseline>,
    conditions: Vec<ConditionDefinition>,
    formula_revision: String,
    armor: bool,
    shield: bool,
    fingerprint: [u8; 32],
    pub(crate) content_digest: [u8; 32],
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
pub(crate) struct DefenseBudget {
    pub(crate) target: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) budget: BlockBudget,
}
#[derive(Default)]
pub(crate) struct AutoAttackOwner {
    entries: Vec<Entry>,
    pub(crate) blocks: Vec<DefenseBudget>,
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
        selection: crate::foundation::ThinkOccurrence,
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
            u32::try_from(crate::durability::uniform_draw::uniform_draw(
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

#[cfg(test)]
impl AutoAttackOwner {
    pub(crate) fn has_active_target_test(&self, actor: ExactActorRef) -> bool {
        self.entries.iter().any(|e| e.issuer == actor && e.active)
    }
}

type SwingDispatchResult = Result<Vec<(Ref, Result<SwingOutcome, AttackError>)>, AttackError>;

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
