//! Exact Crystal Smelly schedule adapter to existing owner timers and native player HP.
//! No declarative Encounter interpreter; ambiguous top-creature tiles remain refused.
use crate::bosses::smelly_cheese::native_owner::SmellyCheeseCarrierExt;
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
};
use crate::creature_auto_attack::AttackError;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::creature_damage_spell::{SourcePresentationEvent, SpellWorldReader};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals};
use oteryn_gameplay_encounters::bosses::smelly_cheese::timers::{
    SmellyCastOccurrence, SmellyCheeseTimers, SmellyPulse,
};
use oteryn_gameplay_foundation::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use {
    oteryn_gameplay_entities::ChannelRuntimeV1,
    oteryn_gameplay_foundation::foundation::{ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence},
};
#[path = "native_source.rs"]
mod native_source;
pub const CREATURE: &str = "oteryn:creature.smelly_cheese";
pub const PARENT: &str = "oteryn:ability.creature.smelly_cheese.berserk_bridge";
#[derive(Clone)]
pub struct SourceSmelly {
    pub ability: Ref,
    pub content: [u8; 32],
    presentation: Box<crate::content::ProjectV2AbilityDetails>,
    native: Option<crate::content::native_monster_callbacks::QualifiedNativeCallback>,
}
#[derive(serde::Deserialize)]
struct Witness {
    profiles: BTreeMap<String, Data>,
    records: Vec<Record>,
}
impl SourceSmelly {
    pub fn qualify_native(
        native: &crate::content::native_gameplay::NativeGameplayState,
        caster: &Ref,
    ) -> Result<Option<Self>, AttackError> {
        let Some(plan) = native_source::plan(native.creature_profiles(), caster)? else {
            return Ok(None);
        };
        let proof = crate::content::native_monster_callbacks::qualify_native_callback(
            native,
            caster,
            0,
            crate::content::native_monster_callbacks::NativeCallbackList::Attack,
        )
        .map_err(|_| AttackError::InvalidSource)?
        .ok_or(AttackError::InvalidSource)?;
        Ok(Some(Self {
            ability: plan.ability,
            content: native.source_digest(),
            presentation: plan.presentation,
            native: Some(proof),
        }))
    }

    pub fn native_callback(
        &self,
    ) -> Option<&crate::content::native_monster_callbacks::QualifiedNativeCallback> {
        self.native.as_ref().filter(|proof| {
            proof.content() == self.content && proof.entry().ability == self.ability
        })
    }
    pub fn qualifies_native_runtime(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        self.current_retained(r, a)
    }
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
            "../../../../../../apps/game-server/src/smelly_encounter_source.json"
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
            native: None,
        })
    }
    fn current(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        r.content_pin().server_artifact_digest() == self.content
            && r.matches_live_creature_identity(a, CREATURE.as_bytes())
    }
    fn current_retained(&self, r: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
        let Some(proof) = self.native_callback() else {
            return false;
        };
        let Ok(snapshot) = r.companion_snapshot_including_dead(actor) else {
            return false;
        };
        r.source_creature_is_registered(actor)
            .is_ok_and(|registered| registered)
            && r.content_pin().server_artifact_digest() == self.content
            && snapshot.state.policy.definition_key == proof.entry().creature.key
            && snapshot.state.policy.definition_revision == proof.entry().creature.revision
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
        // The real Attack schedule retains the attacked creature as proposal
        // DATA even for needs_target=false. The sealed source's VAR is still
        // the caster position, captured separately at scheduling time.
        let recipient = if self.native_callback().is_some() {
            if p.target.runtime_scope() != p.issuer.runtime_scope()
                || p.target.scope_generation() != p.issuer.scope_generation()
            {
                return Err(AttackError::InvalidPlan);
            }
            p.target
        } else {
            if p.target != p.issuer {
                return Err(AttackError::InvalidPlan);
            }
            p.issuer
        };
        let target = format!("actor:{}", hex(recipient.placement_identity()));
        if p.intent.proposal_source() != crate::ability::ProposalSource::Ai
            || p.intent.actor() != atom
            || p.intent.candidate_count() != 1
            || p.intent.resolved_targets().len() != 1
            || p.intent.resolved_targets()[0].as_str() != target
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
    native_cast: Option<(
        u64,
        oteryn_gameplay_entities::MovementLocalPosition,
        ProfileAbilityProposal,
    )>,
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
        if !f.is_current_for_scope(b.runtime_scope(), b.scope_generation())
            || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if !s.current(r, p.issuer) {
            return Err(AttackError::StaleIssuer);
        }
        let sequence = s.sequence(p)?;
        self.entries.retain(|e| {
            if e.source.native_callback().is_some() {
                e.source.current_retained(r, e.actor)
            } else {
                e.source.current(r, e.actor)
            }
        });
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
                let timers = SmellyCheeseTimers::new(b.runtime_scope(), b.scope_generation())
                    .map_err(|_| AttackError::InvalidPlan)?;
                self.entries.push(Entry {
                    actor: p.issuer,
                    source: s.clone(),
                    timers,
                    native_cast: None,
                });
                self.entries.len() - 1
            }
        };
        if self.entries[i].source.content != s.content {
            return Err(AttackError::ContentChanged);
        }
        let center = r
            .read_actor_position(p.issuer)
            .map_err(|_| AttackError::StaleIssuer)?
            .position();
        let scheduled = if let Some(proof) = s.native_callback() {
            self.entries[i].timers.schedule_native_once(
                r,
                f,
                stamp,
                SmellyCastOccurrence {
                    caster: p.issuer,
                    sequence,
                },
                proof,
                now,
            )
        } else {
            self.entries[i].timers.schedule_once(
                r,
                f,
                stamp,
                SmellyCastOccurrence {
                    caster: p.issuer,
                    sequence,
                },
                now,
            )
        }
        .map_err(|_| AttackError::InvalidPlan)?;
        if scheduled && s.native_callback().is_some() {
            self.entries[i].native_cast = Some((sequence, center, p.clone()));
        }
        Ok(scheduled)
    }
    pub fn next_due(&self) -> Option<u64> {
        self.entries
            .iter()
            .filter_map(|e| e.timers.next_due().map(|t| t.get()))
            .min()
    }
    /// Lua Creature(cid) becomes nil at source death completion, before the
    /// delayed physical corpse retirement. Purge before preparing map facts.
    pub fn prune_retired_native(&mut self, runtime: &ChannelRuntimeV1) {
        self.entries.retain(|entry| {
            entry.source.native_callback().is_none()
                || entry.source.current_retained(runtime, entry.actor)
        });
    }
    pub fn native_due_through(
        &self,
        now: u64,
        eligible_through_us: u64,
    ) -> Vec<(
        ExactActorRef,
        SourceSmelly,
        oteryn_gameplay_entities::MovementLocalPosition,
        ProfileAbilityProposal,
    )> {
        self.entries
            .iter()
            .filter(|e| {
                e.source.native_callback().is_some()
                    && e.timers
                        .next_due()
                        .is_some_and(|t| t.get() <= now.min(eligible_through_us))
            })
            .filter_map(|e| {
                e.native_cast.as_ref().map(|(_, center, proposal)| {
                    (e.actor, e.source.clone(), *center, proposal.clone())
                })
            })
            .collect()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "current actor/source, owner stamp, HP reader, real clock and deadline eligibility are independent proofs"
    )]
    pub fn drain_native_actor_through(
        &mut self,
        actor: ExactActorRef,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        v: &mut impl PlayerLethalVitals,
        reader: &mut impl SpellWorldReader,
        clock: &impl OwnerClock,
        eligible_through_us: u64,
    ) -> SmellyPulseResults {
        let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.actor == actor && e.source.native_callback().is_some())
        else {
            return Err(AttackError::InvalidSource);
        };
        let (sequence, center) = e
            .native_cast
            .as_ref()
            .map(|(sequence, center, _)| (*sequence, *center))
            .ok_or(AttackError::InvalidPlan)?;
        let mut result = Vec::new();
        let pulses = {
            let fence = r.owner_fence().map_err(|_| AttackError::StaleOwner)?;
            if !fence.accepts_stamp(stamp) {
                return Err(AttackError::StaleOwner);
            }
            e.timers.drain_through(
                clock,
                fence,
                |a| e.source.current_retained(r, a),
                SmellyCastOccurrence {
                    caster: e.actor,
                    sequence,
                },
                eligible_through_us,
            )
        };
        for pulse in pulses {
            result.push((
                e.actor,
                execute_pulse(
                    r,
                    None,
                    stamp,
                    v,
                    reader,
                    &e.source,
                    pulse,
                    clock.now(),
                    Some(center),
                ),
            ));
        }
        Ok(result)
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
        if !f.is_current_for_scope(b.runtime_scope(), b.scope_generation())
            || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        self.entries.retain(|e| e.source.current(r, e.actor));
        let mut result = Vec::new();
        for e in &mut self.entries {
            for pulse in e.timers.drain(clock, f, |a| e.source.current(r, a)) {
                let hit = execute_pulse(
                    r,
                    Some(f),
                    stamp,
                    v,
                    reader,
                    &e.source,
                    pulse,
                    clock.now(),
                    None,
                );
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
    _f: Option<&ScopeRuntimeFence>,
    stamp: RuntimeWorkStamp,
    v: &mut impl PlayerLethalVitals,
    reader: &mut impl SpellWorldReader,
    s: &SourceSmelly,
    p: oteryn_gameplay_encounters::bosses::smelly_cheese::timers::SmellyDuePulse,
    now: SemanticTimeMicros,
    retained_center: Option<oteryn_gameplay_entities::MovementLocalPosition>,
) -> Result<SmellyEffect, AttackError> {
    let b = r.binding();
    let _f = match _f {
        Some(fence) => fence,
        None => r.owner_fence().map_err(|_| AttackError::StaleOwner)?,
    };
    if !_f.is_current_for_scope(b.runtime_scope(), b.scope_generation()) || !_f.accepts_stamp(stamp)
    {
        return Err(AttackError::StaleOwner);
    }
    let actor = p.caster();
    if !(if retained_center.is_some() && s.native_callback().is_some() {
        s.current_retained(r, actor)
    } else {
        s.current(r, actor)
    }) {
        return Err(AttackError::StaleIssuer);
    }
    let center = match retained_center {
        Some(center) => center,
        None => r
            .read_actor_position(actor)
            .map_err(|_| AttackError::StaleIssuer)?
            .position(),
    };
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
    let targets = if s.native_callback().is_some() {
        let census = r
            .positioned_actor_census()
            .map_err(|_| AttackError::MissingCombatFacts)?;
        let mut targets = Vec::new();
        for &(x, y, floor) in &tiles {
            let at = oteryn_gameplay_entities::MovementLocalPosition { x, y, floor };
            if let Some(top) = r
                .current_top_actor_on_cell(at)
                .map_err(|_| AttackError::MissingCombatFacts)?
                && let Some((_, _, Some(session))) =
                    census.iter().find(|(actor, _, _)| *actor == top)
            {
                targets.push((top, *session));
            }
        }
        targets
    } else {
        r.smelly_unambiguous_targets(actor)
            .map_err(|_| AttackError::InvalidPlan)?
    };
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
        let receipt = if s.native_callback().is_some() {
            v.apply_source_typed_attack_damage(
                r,
                actor,
                target,
                session,
                amount,
                crate::player_lethal::PlayerHitKind::Earth,
                &id,
                now,
            )
        } else {
            v.apply_attack_damage(r, target, session, amount, &id, now)
        }
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
