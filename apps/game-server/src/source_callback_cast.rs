//! Sealed, nine-source callback child casts; reuses the existing native damage consumer.
//! Source Encounter bytes qualify actions; there is no general Encounter interpreter.
use super::*;
pub(crate) type CallbackPulseBatch =
    Result<Vec<(ExactActorRef, Ref, usize, Result<SpellOutcome, AttackError>)>, AttackError>;
#[path = "source_callback_speech.rs"]
mod speech;
use crate::content::{ProjectV2Draft, ProjectV2SourceIdentityBinding};
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, TimerFamily,
};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
pub(crate) use speech::CallbackSpeech;
#[derive(serde::Deserialize)]
struct Witness {
    cases: BTreeMap<String, Case>,
    profiles: BTreeMap<String, Data>,
    records: Vec<ProjectReferenceRecord>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
}
#[derive(serde::Deserialize)]
struct Case {
    caster: String,
    index: usize,
    phase: String,
    encounter: String,
    casts: Vec<CastWitness>,
    entry: serde_json::Value,
}
#[derive(serde::Deserialize)]
struct CastWitness {
    delay_ms: Delay,
    action: serde_json::Value,
}
#[derive(serde::Deserialize)]
struct Delay {
    min: u64,
    max: u64,
}
#[derive(Clone)]
pub(crate) struct SourceCallbackCast {
    creature: String,
    parent: Ref,
    index: usize,
    range: u16,
    magnitude: Option<ProjectV2Magnitude>,
    content: [u8; 32],
    stages: Vec<(u64, SpellSource)>,
}
impl SourceCallbackCast {
    pub(crate) fn parent(&self) -> &Ref {
        &self.parent
    }
    pub(crate) fn qualify(
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
        let w: Witness = serde_json::from_str(include_str!("source_callback_cast_fixture.json"))
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
        let parent = entry.ability.clone();
        let mut stages = Vec::new();
        let Some(Data::Creature(cp)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *creature)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        let critical = crate::source_critical_baseline::SourceCriticalBaseline::qualify(
            &creature.key,
            cp.details.as_ref().map_or(0, |c| c.critical_chance_ppm),
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
                    Some(ProjectV2AbilityArea::Circle {
                        radius_tiles: u16::try_from(a.area.radius)
                            .map_err(|_| AttackError::NumericOverflow)?,
                    }),
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
                    creature_key: creature.key.clone(),
                    ability: parent.clone(),
                    entry_index: index,
                    minimum,
                    maximum,
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
        Ok(Some(Self {
            creature: creature.key.clone(),
            parent,
            index,
            range: entry.range_tiles.unwrap_or(0),
            magnitude: entry.magnitude,
            content,
            stages,
        }))
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
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Family(u8);
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
struct Cast {
    token: u64,
    source: SourceCallbackCast,
    proposal: ProfileAbilityProposal,
    done: Vec<bool>,
}
struct Memo {
    actor: ExactActorRef,
    ability: Ref,
    sequence: u64,
    at: u64,
    token: u64,
    proposal: ProfileAbilityProposal,
}
/// Bounded current-owner adapter, source occurrence high-water independent of child stage IDs.
pub(crate) struct CallbackCastOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Family, Key>,
    casts: Vec<Cast>,
    memos: Vec<Memo>,
    next: u64,
    speech: Vec<CallbackSpeech>,
    removals: Vec<(ExactActorRef, Result<(), crate::foundation::CarrierError>)>,
}
impl CallbackCastOwner {
    pub(crate) fn new(r: &ChannelRuntimeV1, f: &ScopeRuntimeFence) -> Result<Self, AttackError> {
        let b = r.binding();
        let scope = RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        let generation = b.scope_generation();
        if !f.is_current_for_scope(scope, generation) {
            return Err(AttackError::StaleOwner);
        }
        let timers = OwnerTimerLane::for_generation(
            scope,
            generation,
            (0..48).map(|i| {
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
    pub(crate) fn take_source_speech(&mut self) -> Vec<CallbackSpeech> {
        std::mem::take(&mut self.speech)
    }
    pub(crate) fn take_source_removals(
        &mut self,
    ) -> Vec<(ExactActorRef, Result<(), crate::foundation::CarrierError>)> {
        std::mem::take(&mut self.removals)
    }
    fn current(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> bool {
        let b = r.binding();
        self.scope == RuntimeScopeRefV1::channel(b.world_id(), b.channel_id())
            && self.generation == b.scope_generation()
            && f.is_current_for_scope(self.scope, self.generation)
            && f.accepts_stamp(stamp)
    }
    fn prune(&mut self, r: &ChannelRuntimeV1) {
        for c in &self.casts {
            if !c.source.current(r, c.proposal.issuer) || c.done.iter().all(|d| *d) {
                for stage in 0..c.done.len() {
                    self.timers.cancel(
                        Family((c.source.index * 3 + stage) as u8),
                        Key {
                            token: c.token,
                            stage: stage as u8,
                        },
                    );
                }
            }
        }
        self.casts
            .retain(|c| c.source.current(r, c.proposal.issuer) && !c.done.iter().all(|d| *d));
        self.memos.retain(|m| r.contains_live_creature(m.actor));
    }
    pub(crate) fn schedule(
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
        crate::source_callback_intent::validate(r, p)?;
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
        let warning = CallbackSpeech::cast(r, p.issuer, &s.creature, false)?;
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
    pub(crate) fn drain(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        v: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        reader: &mut dyn SpellWorldReader,
        clock: &impl OwnerClock,
    ) -> CallbackPulseBatch {
        if !self.current(r, f, stamp) {
            return Err(AttackError::StaleOwner);
        }
        self.prune(r);
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
        let fired = self
            .timers
            .drain_due(clock, f, |a| r.contains_live_creature(a));
        for timer in fired {
            let Some(c) = self
                .casts
                .iter_mut()
                .find(|c| c.token == timer.occurrence.token)
            else {
                continue;
            };
            let stage = usize::from(timer.occurrence.stage);
            if c.done[stage] {
                continue;
            }
            c.done[stage] = true;
            if c.source.current(r, c.proposal.issuer)
                && let Some(line) =
                    CallbackSpeech::cast(r, c.proposal.issuer, &c.source.creature, true)?
            {
                self.speech.push(line)
            }
            let result = if !c.source.current(r, c.proposal.issuer) {
                Err(AttackError::StaleIssuer)
            } else {
                let mut p = c.proposal.clone();
                p.target = p.issuer;
                p.occurrence = crate::ability::AbilityOccurrence::new(
                    &format!("callback:{}:stage:{}", p.occurrence.id().as_str(), stage),
                    p.occurrence.revisions().clone(),
                )
                .map_err(|_| AttackError::InvalidPlan)?;
                damage.apply_target(
                    r,
                    v,
                    &c.source.stages[stage].1,
                    &p,
                    reader,
                    stamp,
                    clock.now(),
                    None,
                    f,
                )
            };
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
                    r.remove_native_encounter_subject(
                        f,
                        stamp,
                        c.proposal.issuer,
                        &c.source.creature,
                        c.source.content,
                    ),
                ));
            }
            out.push((c.proposal.issuer, c.proposal.ability.clone(), stage, result));
        }
        Ok(out)
    }
}
#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{GameSessionId, MovementLocalPosition};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    fn draft(r: &ChannelRuntimeV1) -> ProjectV2Draft {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("source_callback_native_fixture.json"))
                .expect("qualified fixture");
        ProjectV2Draft {
            core: crate::content::ProjectDraft {
                project_revision: "g4-npc-wave-a-r9".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "PENDING".into(),
                world_id: hex(r.binding().world_id().as_bytes()),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
                imports: vec![],
                metadata: vec![],
            },
            state: crate::content::ProjectV2State {
                declarations: serde_json::from_value(v["declarations"].clone())
                    .expect("qualified fixture"),
                authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                    .expect("qualified fixture"),
                source_identity_bindings: serde_json::from_value(
                    v["source_identity_bindings"].clone(),
                )
                .expect("qualified fixture"),
                sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
                ..Default::default()
            },
        }
    }
    fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
        let b = r.binding();
        crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("qualified fixture")
    }
    fn proposal(a: ExactActorRef, s: &SourceCallbackCast, sequence: u64) -> ProfileAbilityProposal {
        let atom = format!("actor:{}", hex(&a.placement_identity()));
        ProfileAbilityProposal {
            issuer: a,
            target: a,
            ability: s.parent.clone(),
            list: ScheduleList::Attack,
            entry_index: s.index,
            magnitude: s.magnitude,
            range_tiles: s.range,
            occurrence: crate::ability::AbilityOccurrence::new(
                &format!(
                    "ai-profile:{}:{sequence}:attack:{}",
                    hex(&a.placement_identity()),
                    s.index
                ),
                crate::ability::RevisionSet::new(
                    "rules-r1",
                    "content-r1",
                    "policy-r1",
                    "definition-r1",
                    "sim-r1",
                )
                .expect("qualified fixture"),
            )
            .expect("qualified fixture"),
            intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
                .expect("qualified fixture"),
        }
    }
    struct Reader {
        player: ExactActorRef,
        session: GameSessionId,
        facts: bool,
        pz: bool,
    }
    impl SpellWorldReader for Reader {
        fn current_players(
            &mut self,
            _: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            Some(vec![(self.player, self.session)])
        }
        fn current_facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<crate::creature_attack_geometry::Facing> {
            Some(crate::creature_attack_geometry::Facing::North)
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
            self.facts.then_some(true)
        }
        fn combat(
            &mut self,
            _: &ChannelRuntimeV1,
            c: ExactActorRef,
            p: ExactActorRef,
            session: GameSessionId,
            _: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            self.facts.then_some(SpellCombatFacts {
                attack: AttackFacts {
                    issuer: c,
                    target: p,
                    session,
                    revision: 1,
                    visible: true,
                    issuer_pz: false,
                    target_pz: self.pz,
                    issuer_protected: false,
                    target_protected: false,
                    defense: 0,
                    armor: 0,
                },
                multiplier_ppm: 1000000,
                immune: false,
                condition_policy: None,
            })
        }
    }
    #[test]
    fn nine_source_callbacks_actual_native_damage_delays_stages_and_replay() {
        let w: Witness = serde_json::from_str(include_str!("source_callback_cast_fixture.json"))
            .expect("qualified fixture");
        for case in w.cases.values() {
            let (mut r, p, session) = runtime_with_player(0x78);
            let at = if case.caster.ends_with("rootthing_amber_shaper") {
                MovementLocalPosition {
                    x: 96,
                    y: 100,
                    floor: 7,
                }
            } else {
                MovementLocalPosition {
                    x: 100,
                    y: 99,
                    floor: 7,
                }
            };
            r.initialize_movement_test_position(p, at)
                .expect("qualified fixture");
            let actor = r
                .admit_monster_lab_creature(
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                    &case.caster,
                    300000,
                )
                .expect("qualified fixture");
            let d = draft(&r);
            let creature = Ref {
                family: ProjectV2Family::Creature,
                key: case.caster.clone(),
                revision: "definition-r1".into(),
            };
            let source = SourceCallbackCast::qualify(
                &d,
                &creature,
                case.index,
                r.content_pin().server_artifact_digest(),
            )
            .expect("qualified fixture")
            .expect("qualified fixture");
            let mut states = ChannelSpellStates::default();
            states
                .initialize(
                    &r,
                    p,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        max_health: 100000,
                        ..FACTS
                    },
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                )
                .expect("qualified fixture");
            let before =
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
            let (mut f, stamp) = fence(&r);
            let mut owner = CallbackCastOwner::new(&r, &f).expect("qualified fixture");
            let mut proposal = proposal(actor, &source, 0);
            proposal.target = p;
            proposal.intent = crate::ability::AiAbilityAdapter::normalize(
                &format!("actor:{}", hex(&actor.placement_identity())),
                &[&format!("actor:{}", hex(&p.placement_identity()))],
            )
            .expect("qualified fixture");
            assert!(
                owner
                    .schedule(
                        &r,
                        &f,
                        stamp,
                        &source,
                        &proposal,
                        SemanticTimeMicros::from_micros(0)
                    )
                    .expect("qualified fixture")
            );
            assert!(
                !owner
                    .schedule(
                        &r,
                        &f,
                        stamp,
                        &source,
                        &proposal,
                        SemanticTimeMicros::from_micros(0)
                    )
                    .expect("qualified fixture")
            );
            let initial_warning = owner.take_source_speech();
            assert_eq!(
                initial_warning.len(),
                usize::from(matches!(
                    case.caster.as_str(),
                    "oteryn:creature.gaz_haragoth" | "oteryn:creature.lady_tenebris"
                ))
            );
            assert!(owner.take_source_speech().is_empty());
            let mut forged = proposal.clone();
            forged.intent =
                crate::ability::AiAbilityAdapter::normalize("actor:forged", &["actor:forged"])
                    .expect("qualified fixture");
            let timers_before = owner.casts.len();
            assert!(matches!(
                owner.schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &forged,
                    SemanticTimeMicros::from_micros(0)
                ),
                Err(AttackError::InvalidPlan)
            ));
            assert_eq!(owner.casts.len(), timers_before);
            let mut forged = proposal.clone();
            forged.target = actor;
            assert!(matches!(
                owner.schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &forged,
                    SemanticTimeMicros::from_micros(0)
                ),
                Err(AttackError::InvalidPlan)
            ));
            assert_eq!(owner.casts.len(), timers_before);
            let mut reader = Reader {
                player: p,
                session,
                facts: true,
                pz: false,
            };
            let damage = DamageSpellOwner::default();
            let mut receipts = 0;
            for delay in [0, 1000, 2000, 3000, 4000, 5000, 7000, 10000] {
                let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(delay * 1000));
                let results = owner
                    .drain(
                        &mut r,
                        &mut f,
                        stamp,
                        &mut states,
                        &damage,
                        &mut reader,
                        &clock,
                    )
                    .expect("qualified fixture");
                receipts += results.len();
                for (_, _, _, result) in results {
                    assert!(result.is_ok(), "{} {:?}", case.caster, result);
                }
            }
            let completion_lines = owner.take_source_speech();
            assert_eq!(
                completion_lines.len(),
                usize::from(case.caster == "oteryn:creature.gaz_haragoth")
            );
            assert_eq!(receipts, case.casts.len());
            assert_ne!(
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
                before,
                "actual HP unchanged {}",
                case.caster
            );
            let after =
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
            let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(20000000));
            if matches!(
                case.caster.as_str(),
                "oteryn:creature.energy_pulse" | "oteryn:creature.eruption_of_destruction"
            ) {
                assert!(!r.contains_live_creature(actor));
                let removals = owner.take_source_removals();
                assert_eq!(removals.len(), 1);
                assert!(removals[0].1.is_ok());
                assert!(matches!(
                    owner.schedule(&r, &f, stamp, &source, &proposal, clock.now()),
                    Err(AttackError::StaleIssuer)
                ));
            } else {
                assert!(owner.take_source_removals().is_empty());
                assert!(
                    !owner
                        .schedule(&r, &f, stamp, &source, &proposal, clock.now())
                        .expect("qualified fixture")
                );
            }
            assert!(
                owner
                    .drain(
                        &mut r,
                        &mut f,
                        stamp,
                        &mut states,
                        &damage,
                        &mut reader,
                        &clock
                    )
                    .expect("qualified fixture")
                    .is_empty()
            );
            assert_eq!(
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
                after
            );
        }
    }
    #[test]
    fn every_source_callback_missing_facts_provenance_scope_dead_parent_refuses() {
        let w: Witness = serde_json::from_str(include_str!("source_callback_cast_fixture.json"))
            .expect("qualified fixture");
        for case in w.cases.values() {
            let (mut r, p, session) = runtime_with_player(0x79);
            r.initialize_movement_test_position(
                p,
                MovementLocalPosition {
                    x: 100,
                    y: 99,
                    floor: 7,
                },
            )
            .expect("qualified fixture");
            let actor = r
                .admit_monster_lab_creature(
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                    &case.caster,
                    300000,
                )
                .expect("qualified fixture");
            let mut d = draft(&r);
            let cr = Ref {
                family: ProjectV2Family::Creature,
                key: case.caster.clone(),
                revision: "definition-r1".into(),
            };
            let source = SourceCallbackCast::qualify(
                &d,
                &cr,
                case.index,
                r.content_pin().server_artifact_digest(),
            )
            .expect("qualified fixture")
            .expect("qualified fixture");
            let mut states = ChannelSpellStates::default();
            states
                .initialize(
                    &r,
                    p,
                    session,
                    FACTS,
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                )
                .expect("qualified fixture");
            let before =
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
            let (mut f, stamp) = fence(&r);
            let mut owner = CallbackCastOwner::new(&r, &f).expect("qualified fixture");
            let proposal = proposal(actor, &source, 0);
            owner
                .schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &proposal,
                    SemanticTimeMicros::from_micros(0),
                )
                .expect("qualified fixture");
            let mut reader = Reader {
                player: p,
                session,
                facts: false,
                pz: false,
            };
            let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(10000000));
            let hits = owner
                .drain(
                    &mut r,
                    &mut f,
                    stamp,
                    &mut states,
                    &DamageSpellOwner::default(),
                    &mut reader,
                    &clock,
                )
                .expect("qualified fixture");
            assert!(
                hits.iter()
                    .all(|v| matches!(v.3, Err(AttackError::MissingCombatFacts)))
            );
            assert_eq!(
                crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
                before
            );
            let binding = d
                .state
                .source_identity_bindings
                .iter_mut()
                .find(|b| b.target.key == case.encounter)
                .expect("qualified fixture");
            binding.source_revision = "wrong".into();
            assert!(SourceCallbackCast::qualify(&d, &cr, case.index, source.content).is_err());
            let b = r.binding();
            let (other_f, other_stamp) = crate::foundation::crystal_timer_fixture(
                RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
            )
            .expect("qualified fixture");
            assert!(matches!(
                owner.schedule(&r, &other_f, other_stamp, &source, &proposal, clock.now()),
                Err(AttackError::StaleOwner)
            ));
            r.remove_test_actor(actor).expect("qualified fixture");
            assert!(matches!(
                owner.schedule(&r, &f, stamp, &source, &proposal, clock.now()),
                Err(AttackError::StaleIssuer)
            ));
        }
    }
}
