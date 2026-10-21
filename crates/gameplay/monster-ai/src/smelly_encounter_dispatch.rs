//! Exact Crystal Smelly schedule adapter to existing owner timers and native player HP.
//! No declarative Encounter interpreter; ambiguous top-creature tiles remain refused.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
};
use crate::creature_auto_attack::AttackError;
use crate::creature_damage_spell::{SourcePresentationEvent, SpellWorldReader};
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals};
use crate::smelly_cheese::{SmellyCastOccurrence, SmellyCheeseTimers, SmellyPulse};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub const CREATURE: &str = "oteryn:creature.smelly_cheese";
pub const PARENT: &str = "oteryn:ability.creature.smelly_cheese.berserk_bridge";
#[derive(Clone)]
pub struct SourceSmelly {
    pub ability: Ref,
    pub content: [u8; 32],
    presentation: Box<crate::content::ProjectV2AbilityDetails>,
}
#[derive(serde::Deserialize)]
struct Witness {
    profiles: BTreeMap<String, Data>,
    records: Vec<Record>,
}
impl SourceSmelly {
    pub fn ability(&self) -> &Ref {
        &self.ability
    }
    pub fn qualify(
        d: &ProjectV2Draft,
        caster: &Ref,
        content: [u8; 32],
    ) -> Result<Self, AttackError> {
        if caster.family != Family::Creature
            || caster.key != CREATURE
            || caster.revision != "definition-r1"
        {
            return Err(AttackError::InvalidSource);
        }
        for (family, key) in [
            (Family::Creature, CREATURE),
            (Family::Encounter, "oteryn:encounter.crystal_smelly_cheese"),
        ] {
            let (namespace, external_id) = if family == Family::Creature {
                (
                    "crystalserver/monster-file",
                    "data-global/monster/quests/a_pirates_tail_quest/smelly_cheese.lua",
                )
            } else {
                ("crystalserver/encounter", "crystal_smelly_cheese")
            };
            if !d.state.source_identity_bindings.iter().any(|b| {
                b.target.family == family
                    && b.target.key == key
                    && b.target.revision == "definition-r1"
                    && b.identity_namespace == namespace
                    && b.external_id == external_id
            }) {
                return Err(AttackError::InvalidSource);
            }
            if d.state
                .source_identity_bindings
                .iter()
                .filter(|b| {
                    b.target.family == family
                        && b.target.key == key
                        && b.target.revision == "definition-r1"
                })
                .count()
                != 1
            {
                return Err(AttackError::InvalidSource);
            }
            if !d.state.source_identity_bindings.iter().any(|b| {
                b.target.family == family
                    && b.target.key == key
                    && b.target.revision == "definition-r1"
                    && b.source_key == "oteryn:source.crystalserver"
                    && b.source_revision
                        == "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                    && b.disposition == crate::content::ProjectV2SourceIdentityDisposition::Exact
                    && d.state
                        .sources
                        .iter()
                        .any(|s| s.key == b.source_key && s.revision == b.source_revision)
            }) {
                return Err(AttackError::InvalidSource);
            }
        }
        let expected: Witness = serde_json::from_str(include_str!(
            "../../../../apps/game-server/src/smelly_encounter_source.json"
        ))
        .map_err(|_| AttackError::InvalidSource)?;
        for (key, data) in expected.profiles {
            let mut matches = d
                .state
                .authoring_profiles
                .iter()
                .filter(|p| p.target.key == key && p.target.revision == "definition-r1");
            if !matches.next().is_some_and(|p| p.data == data) || matches.next().is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        for record in expected.records {
            let mut matches = d
                .core
                .records
                .iter()
                .filter(|r| record_identity(r) == record_identity(&record));
            if matches.next() != Some(&record) || matches.next().is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        if d.core.records.iter().filter(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision)).count()!=1{return Err(AttackError::InvalidSource)}
        let Some(Record::Creature{behavior,..})=d.core.records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(AttackError::InvalidSource)};
        if d.state
            .authoring_profiles
            .iter()
            .filter(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .count()
            != 1
        {
            return Err(AttackError::InvalidSource);
        }
        let Some(Data::Behavior(b)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        let e = b.attacks.first().ok_or(AttackError::InvalidSource)?;
        if e.ability.key != PARENT
            || e.ability.revision != "definition-r1"
            || e.interval_ms != 2000
            || e.chance_ppm != 500000
        {
            return Err(AttackError::InvalidSource);
        }
        let presentation = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == "oteryn:ability.creature.smelly_cheese.berserk_pulse_2")
            .and_then(|p| match &p.data {
                Data::Ability(a) => a.details.clone(),
                _ => None,
            })
            .ok_or(AttackError::InvalidSource)?;
        Ok(Self {
            ability: e.ability.clone(),
            content,
            presentation,
        })
    }
    fn current(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        r.content_pin().server_artifact_digest() == self.content
            && r.matches_live_creature_identity(a, CREATURE.as_bytes())
    }
    fn sequence(&self, p: &ProfileAbilityProposal) -> Result<u64, AttackError> {
        if p.list != ScheduleList::Attack
            || p.entry_index != 0
            || p.ability != self.ability
            || p.range_tiles != 0
            || p.magnitude.is_some()
        {
            return Err(AttackError::InvalidSource);
        }
        let atom = format!("actor:{}", hex(p.issuer.placement_identity()));
        if p.target != p.issuer
            || p.intent.proposal_source() != crate::ability::ProposalSource::Ai
            || p.intent.actor() != atom
            || p.intent.candidate_count() != 1
            || p.intent.resolved_targets().len() != 1
            || p.intent.resolved_targets()[0].as_str() != atom
        {
            return Err(AttackError::InvalidPlan);
        }
        let prefix = format!("ai-profile:{}:", hex(p.issuer.placement_identity()));
        let tail = p
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (seq, suffix) = tail.split_once(':').ok_or(AttackError::InvalidPlan)?;
        if suffix != "attack:0" {
            return Err(AttackError::InvalidPlan);
        }
        seq.parse().map_err(|_| AttackError::InvalidPlan)
    }
}
pub struct Entry {
    actor: ExactActorRef,
    source: SourceSmelly,
    timers: SmellyCheeseTimers,
}
#[derive(Default)]
pub struct SmellyEncounterOwner {
    pub entries: Vec<Entry>,
}
#[derive(Debug)]
// Presentation is returned by value from source-qualified owner dispatch; boxing the result would add a heap allocation after committed owner state. Preserve the prepublication allocation boundary.
#[allow(clippy::large_enum_variant)]
pub enum SmellyEffect {
    Damage(Vec<(ExactActorRef, Result<PlayerDamageReceipt, AttackError>)>),
    Presentation(SourcePresentationEvent),
}
impl SmellyEncounterOwner {
    pub fn schedule(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &SourceSmelly,
        p: &ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<bool, AttackError> {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if !s.current(r, p.issuer) {
            return Err(AttackError::StaleIssuer);
        }
        let sequence = s.sequence(p)?;
        self.entries.retain(|e| e.source.current(r, e.actor));
        let i = match self.entries.iter().position(|e| e.actor == p.issuer) {
            Some(i) => i,
            None => {
                if self.entries.len() >= 64 {
                    return Err(AttackError::LedgerFull);
                }
                self.entries
                    .try_reserve(1)
                    .map_err(|_| AttackError::LedgerFull)?;
                let b = r.binding();
                let timers = SmellyCheeseTimers::new(
                    RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                    b.scope_generation(),
                )
                .map_err(|_| AttackError::InvalidPlan)?;
                self.entries.push(Entry {
                    actor: p.issuer,
                    source: s.clone(),
                    timers,
                });
                self.entries.len() - 1
            }
        };
        if self.entries[i].source.content != s.content {
            return Err(AttackError::ContentChanged);
        }
        self.entries[i]
            .timers
            .schedule_once(
                r,
                f,
                stamp,
                SmellyCastOccurrence {
                    caster: p.issuer,
                    sequence,
                },
                now,
            )
            .map_err(|_| AttackError::InvalidPlan)
    }
    pub fn drain(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        v: &mut impl PlayerLethalVitals,
        reader: &mut impl SpellWorldReader,
        clock: &impl OwnerClock,
    ) -> SmellyPulseResults {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        self.entries.retain(|e| e.source.current(r, e.actor));
        let mut result = Vec::new();
        for e in &mut self.entries {
            for pulse in e.timers.drain(clock, f, |a| e.source.current(r, a)) {
                let hit = execute_pulse(r, f, stamp, v, reader, &e.source, pulse, clock.now());
                result.push((e.actor, hit));
            }
        }
        Ok(result)
    }
}
// Keep execute_pulse ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
fn execute_pulse(
    r: &mut ChannelRuntimeV1,
    _f: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    v: &mut impl PlayerLethalVitals,
    reader: &mut impl SpellWorldReader,
    s: &SourceSmelly,
    p: crate::smelly_cheese::SmellyDuePulse,
    now: SemanticTimeMicros,
) -> Result<SmellyEffect, AttackError> {
    let b = r.binding();
    if !_f.is_current_for_scope(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    ) || !_f.accepts_stamp(stamp)
    {
        return Err(AttackError::StaleOwner);
    }
    let actor = p.caster();
    if !s.current(r, actor) {
        return Err(AttackError::StaleIssuer);
    }
    let center = r
        .read_actor_position(actor)
        .map_err(|_| AttackError::StaleIssuer)?
        .position();
    let mut tiles = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            let x = center
                .x
                .checked_add(dx)
                .ok_or(AttackError::NumericOverflow)?;
            let y = center
                .y
                .checked_add(dy)
                .ok_or(AttackError::NumericOverflow)?;
            match reader.tile_allowed(r, actor, x, y, center.floor, stamp) {
                Some(true) => tiles.push((x, y, center.floor)),
                Some(false) => {}
                None => return Err(AttackError::MissingCombatFacts),
            }
        }
    }
    let id = format!(
        "smelly:{}:{}",
        hex(actor.placement_identity()),
        p.sequence()
    );
    if p.kind() == SmellyPulse::Presentation700 {
        return Ok(SmellyEffect::Presentation(SourcePresentationEvent {
            issuer: actor,
            ability: Ref {
                family: Family::Ability,
                key: "oteryn:ability.creature.smelly_cheese.berserk_pulse_2".into(),
                revision: "definition-r1".into(),
            },
            occurrence: format!("{id}:700"),
            content_digest: s.content,
            stamp,
            tiles,
            source: s.presentation.clone(),
            qualification: "SOURCE_CRYSTAL_SMELLY_700MS_PRESENTATION_NETWORK_PENDING",
        }));
    }
    let targets = r
        .smelly_unambiguous_targets(actor)
        .map_err(|_| AttackError::InvalidPlan)?;
    let mut accepted = Vec::new();
    for (target, session) in targets {
        let pos = r
            .read_actor_position(target)
            .map_err(|_| AttackError::StaleTarget)?
            .position();
        if !tiles.contains(&(pos.x, pos.y, pos.floor)) {
            continue;
        }
        let facts = reader
            .combat(r, actor, target, session, Some("earth"), stamp)
            .ok_or(AttackError::MissingCombatFacts)?;
        let a = facts.attack;
        if a.issuer != actor
            || a.target != target
            || a.session != session
            || a.revision == 0
            || facts.multiplier_ppm > 10_000_000
        {
            return Err(AttackError::InvalidPlan);
        }
        if !a.visible
            || a.issuer_pz
            || a.target_pz
            || a.issuer_protected
            || a.target_protected
            || facts.immune
        {
            continue;
        }
        r.player_control_facts(target, session)
            .map_err(|_| AttackError::StaleTarget)?;
        let hash = Sha256::new()
            .chain_update(s.content)
            .chain_update(id.as_bytes())
            .chain_update(target.placement_identity())
            .finalize();
        let mut n = [0; 8];
        n.copy_from_slice(&hash[..8]);
        let raw = crate::spell::uniform_draw(u64::from_le_bytes(n), 400, 800);
        let amount = u32::try_from((raw as u64) * u64::from(facts.multiplier_ppm) / 1_000_000)
            .map_err(|_| AttackError::NumericOverflow)?;
        accepted.push((
            target,
            session,
            amount,
            format!("{id}:500:{}", hex(target.placement_identity())),
        ));
    }
    let mut committed = Vec::new();
    committed
        .try_reserve(accepted.len())
        .map_err(|_| AttackError::LedgerFull)?;
    for (target, session, amount, id) in accepted {
        if amount == 0 {
            continue;
        }
        let receipt = v
            .apply_attack_damage(r, target, session, amount, &id, now)
            .ok_or(AttackError::MissingVitals);
        committed.push((target, receipt))
    }
    Ok(SmellyEffect::Damage(committed))
}
fn record_identity(r: &Record) -> (&str, &str, &str) {
    let i = match r {
        Record::Ability { identity, .. }
        | Record::Effect { identity, .. }
        | Record::Formula { identity, .. }
        | Record::Item { identity, .. }
        | Record::Generic { identity, .. }
        | Record::Creature { identity, .. }
        | Record::Loot { identity, .. }
        | Record::LocalObject { identity, .. } => identity,
    };
    (&i.family, &i.key, &i.revision)
}
pub fn hex(v: [u8; 16]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}

pub type SmellyPulseResults =
    Result<Vec<(ExactActorRef, Result<SmellyEffect, AttackError>)>, AttackError>;
