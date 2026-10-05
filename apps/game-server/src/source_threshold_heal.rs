//! Accepted PROJECT_SOURCE_MARKER_LOCAL. Three pinned boss heal callbacks, not a Lua interpreter.
//! Native Creature HP; private condition88888-equivalent cooldown. No Global condition interoperability.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
    ProjectV2SourceIdentityDisposition as Disposition,
};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, OwnerDamageResult, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeRuntimeFence,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
const QUALIFICATION: &str = "PROJECT_SOURCE_MARKER_LOCAL;SOURCE_THRESHOLD_RANGE_DELAY_COOLDOWN;GLOBAL_CONDITION88888_INTEROPERABILITY_UNVERIFIED;PROJECT_UNIFORM_DRAW;SPEECH_VFX_NOT_DELIVERED";
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ThresholdHealError {
    Source,
    Fence,
    Occurrence,
    Replay,
    Clock,
    Capacity,
    Carrier(CarrierError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThresholdHealOutcome {
    pub(crate) started: bool,
    pub(crate) pending: bool,
    pub(crate) healed: Option<OwnerDamageResult>,
    pub(crate) qualification: &'static str,
}
#[derive(Clone, Debug)]
pub(crate) struct ThresholdHealSource {
    caster: Ref,
    ability: Ref,
    index: usize,
    world: String,
    content: [u8; 32],
    maximum: u64,
    threshold_ppm: u64,
    minimum: u64,
    maximum_draw: u64,
    delay_us: u64,
    cooldown_us: u64,
    fingerprint: [u8; 32],
}
impl ThresholdHealSource {
    pub(crate) fn from_native(
        d: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, ThresholdHealError> {
        use ThresholdHealError as E;
        if caster.family != Family::Creature || caster.revision != "definition-r1" {
            return Err(E::Source);
        }
        let (ability, encounter, threshold, minimum, maximum_draw, delay_ms, cooldown_ms) =
            match caster.key.as_str() {
                "oteryn:creature.tyrn" => (
                    "oteryn:ability.creature.tyrn.field-fill-tyrn_heal",
                    "oteryn:encounter.field_fill_tyrn",
                    200000,
                    5000,
                    7500,
                    0u64,
                    900000u64,
                ),
                "oteryn:creature.lisa" => (
                    "oteryn:ability.creature.lisa.field-fill-lisa_heal",
                    "oteryn:encounter.field_fill_lisa",
                    70000,
                    18000,
                    23000,
                    6000,
                    6000,
                ),
                "oteryn:creature.professor_maxxen" => (
                    "oteryn:ability.spell.glooth_fairy_healing",
                    "oteryn:encounter.professor_maxxen",
                    100000,
                    7500,
                    8000,
                    10000,
                    30000,
                ),
                _ => return Err(E::Source),
            };
        for (family, key) in [
            (Family::Creature, caster.key.as_str()),
            (Family::Encounter, encounter),
        ] {
            if !d.state.source_identity_bindings.iter().any(|b| {
                b.target.family == family
                    && b.target.key == key
                    && b.target.revision == "definition-r1"
                    && b.disposition == Disposition::Exact
                    && b.source_key == "oteryn:source.canary"
                    && b.source_revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                    && d.state
                        .sources
                        .iter()
                        .any(|s| s.key == b.source_key && s.revision == b.source_revision)
            }) {
                return Err(E::Source);
            }
        }
        let Some(Record::Creature{behavior,..})=d.core.records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(E::Source)};
        let Some(Data::Creature(c)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *caster)
            .map(|p| &p.data)
        else {
            return Err(E::Source);
        };
        let Some(Data::Behavior(b)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == Family::Behavior
                    && p.target.key == behavior.key
                    && p.target.revision == behavior.revision
            })
            .map(|p| &p.data)
        else {
            return Err(E::Source);
        };
        let entry = b.defenses.get(index).ok_or(E::Source)?;
        if entry.ability.family != Family::Ability
            || entry.ability.key != ability
            || entry.ability.revision != "definition-r1"
            || entry.interval_ms != 1000
            || entry.chance_ppm != 1000000
            || !c.abilities.contains(&entry.ability)
        {
            return Err(E::Source);
        }
        let Some(Data::Ability(a)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == entry.ability)
            .map(|p| &p.data)
        else {
            return Err(E::Source);
        };
        let details = a.details.as_deref().ok_or(E::Source)?;
        let er = details.encounter.as_ref().ok_or(E::Source)?;
        if er.family != Family::Encounter
            || er.key != encounter
            || er.revision != "definition-r1"
            || details.needs_target
            || details.needs_direction
            || details.area.is_some()
            || details.chain.is_some()
            || details.windup.is_some()
            || !details.effects.is_empty()
            || !details.variants.is_empty()
        {
            return Err(E::Source);
        }
        if !d.core.records.iter().any(|r|matches!(r,Record::Ability{identity,effects}if identity.key==ability&&identity.revision=="definition-r1"&&effects.is_empty())){return Err(E::Source)}
        if !d.state.declarations.iter().any(|decl|matches!(decl,crate::content::ProjectV2Declaration::Encounter{identity,..}if identity.key==encounter&&identity.revision=="definition-r1")){return Err(E::Source)}
        let ep = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *er)
            .ok_or(E::Source)?;
        // Compare actual typed Encounter data, not merely its name/hash; source-specific handlers
        // must be requalified if authoring semantics change. This is a pinned source adapter fixture.
        let expected:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/callback18/threshold-heal-expected-encounters.json"))).map_err(|_|E::Source)?;
        let expected_data: Data =
            serde_json::from_value(expected[encounter].clone()).map_err(|_| E::Source)?;
        if serde_json::to_value(&ep.data).map_err(|_| E::Source)?
            != serde_json::to_value(expected_data).map_err(|_| E::Source)?
        {
            return Err(E::Source);
        }
        let maximum = c
            .health
            .filter(|v| *v > 0 && *v <= i64::MAX as u64)
            .ok_or(E::Source)?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                caster,
                index,
                entry,
                a,
                ep,
                maximum,
                content,
                &d.core.world_id,
            ))
            .map_err(|_| E::Source)?,
        )
        .into();
        Ok(Self {
            caster: caster.clone(),
            ability: entry.ability.clone(),
            index,
            world: d.core.world_id.clone(),
            content,
            maximum,
            threshold_ppm: threshold,
            minimum,
            maximum_draw,
            delay_us: delay_ms.checked_mul(1000).ok_or(E::Source)?,
            cooldown_us: cooldown_ms.checked_mul(1000).ok_or(E::Source)?,
            fingerprint,
        })
    }
    pub(crate) fn ability(&self) -> &Ref {
        &self.ability
    }
    fn guard(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
    ) -> Result<(), ThresholdHealError> {
        let binding = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !f.accepts_stamp(stamp)
            || r.content_pin().server_artifact_digest() != self.content
            || r.content_pin()
                .world_id()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                != self.world
            || !r.matches_live_creature_identity(actor, self.caster.key.as_bytes())
        {
            return Err(ThresholdHealError::Fence);
        }
        Ok(())
    }
}
struct Marker {
    actor: ExactActorRef,
    source: [u8; 32],
    content: [u8; 32],
    activation: u64,
    sequence: u64,
    occurrence: String,
    last_us: u64,
    unlock_us: u64,
    pending: Option<(u64, u64)>,
    last: ThresholdHealOutcome,
}
#[derive(Default)]
pub(crate) struct ThresholdHealOwner {
    markers: Vec<Marker>,
}
impl ThresholdHealOwner {
    pub(crate) fn cast(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        p: &ProfileAbilityProposal,
        now: u64,
    ) -> Result<ThresholdHealOutcome, ThresholdHealError> {
        use ThresholdHealError as E;
        s.guard(r, f, stamp, p.issuer)?;
        if p.list != ScheduleList::Defence
            || p.entry_index != s.index
            || p.ability != s.ability
            || p.target != p.issuer
        {
            return Err(E::Occurrence);
        }
        let actor_hex = p
            .issuer
            .placement_identity()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let prefix = format!("ai-profile:{actor_hex}:");
        let occurrence = p.occurrence.id().as_str();
        let (raw, tail) = occurrence
            .strip_prefix(&prefix)
            .ok_or(E::Occurrence)?
            .split_once(':')
            .ok_or(E::Occurrence)?;
        let sequence = raw.parse::<u64>().map_err(|_| E::Occurrence)?;
        if tail != format!("defence:{}", s.index) {
            return Err(E::Occurrence);
        }
        let activation = r.content_pin().activation_sequence();
        let existing = self.markers.iter().position(|m| {
            m.actor == p.issuer && m.source == s.fingerprint && m.activation == activation
        });
        if let Some(i) = existing {
            let m = &self.markers[i];
            if now < m.last_us {
                return Err(E::Clock);
            }
            if sequence < m.sequence {
                return Err(E::Replay);
            }
            if sequence == m.sequence {
                return if m.occurrence == occurrence {
                    Ok(m.last.clone())
                } else {
                    Err(E::Replay)
                };
            }
        }
        let health = r
            .read_source_creature_health(p.issuer, &s.caster.key, s.maximum)
            .map_err(E::Carrier)?;
        let locked = existing
            .is_some_and(|i| now < self.markers[i].unlock_us || self.markers[i].pending.is_some());
        let started = !locked
            && u128::from(health) * 1000000 < u128::from(s.maximum) * u128::from(s.threshold_ppm);
        let mut pending = existing.and_then(|i| self.markers[i].pending);
        let mut unlock = existing.map_or(0, |i| self.markers[i].unlock_us);
        let mut healed = None;
        let occurrence_owned = occurrence.to_owned();
        let key = s.caster.key.clone();
        if existing.is_none() {
            // Guarded source/occurrence/clock and native incoming HP checks precede cleanup.
            // Exact native liveness includes scope, occupied slot, generation and HP>0.
            // Never prune a live pending/cooldown because a different source is casting.
            let current_content = r.content_pin().server_artifact_digest();
            self.markers.retain(|m| {
                m.activation == activation
                    && m.content == current_content
                    && r.contains_live_creature(m.actor)
            });
            if self.markers.len() >= 64 {
                return Err(E::Capacity);
            }
            self.markers.try_reserve(1).map_err(|_| E::Capacity)?;
        }
        if started {
            unlock = now.checked_add(s.cooldown_us).ok_or(E::Clock)?;
            let due = now.checked_add(s.delay_us).ok_or(E::Clock)?;
            let digest = Sha256::new()
                .chain_update(b"oteryn:threshold-heal:v1")
                .chain_update(occurrence.as_bytes())
                .chain_update(p.issuer.placement_identity())
                .finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&digest[..16]);
            let draw = deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(s.content),
                DecisionOccurrenceId::from_bytes(id),
                "threshold_heal_magnitude",
                s.index as u64,
            )
            .map_err(|_| E::Source)?;
            let draw =
                crate::spell::uniform_draw(draw, s.minimum as i64, s.maximum_draw as i64) as u64;
            if s.delay_us == 0 {
                healed = Some(
                    r.commit_source_creature_heal_batch(
                        s.content,
                        &[(p.issuer, key, s.maximum, draw)],
                    )
                    .map_err(E::Carrier)?[0],
                );
            } else {
                pending = Some((due, draw));
            }
        }
        let result = ThresholdHealOutcome {
            started,
            pending: pending.is_some(),
            healed,
            qualification: QUALIFICATION,
        };
        let marker = Marker {
            actor: p.issuer,
            source: s.fingerprint,
            content: s.content,
            activation,
            sequence,
            occurrence: occurrence_owned,
            last_us: now,
            unlock_us: unlock,
            pending,
            last: result.clone(),
        };
        if let Some(i) = existing {
            self.markers[i] = marker
        } else {
            self.markers.push(marker)
        }
        Ok(result)
    }
    /// Called by the owning AI pulse independently of fresh defense chance/casts. Pending
    /// heal checks live original generation/pin/current fence before HP; it never retargets.
    pub(crate) fn tick(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<Option<ThresholdHealOutcome>, ThresholdHealError> {
        use ThresholdHealError as E;
        s.guard(r, f, stamp, actor)?;
        let activation = r.content_pin().activation_sequence();
        let Some(i) = self.markers.iter().position(|m| {
            m.actor == actor && m.source == s.fingerprint && m.activation == activation
        }) else {
            return Ok(None);
        };
        let m = &self.markers[i];
        if now < m.last_us {
            return Err(E::Clock);
        }
        let pending = m.pending;
        let Some((due, draw)) = pending else {
            self.markers[i].last_us = now;
            return Ok(None);
        };
        if now < due {
            self.markers[i].last_us = now;
            return Ok(None);
        };
        r.read_source_creature_health(actor, &s.caster.key, s.maximum)
            .map_err(E::Carrier)?;
        let key = s.caster.key.clone();
        let healed = r
            .commit_source_creature_heal_batch(s.content, &[(actor, key, s.maximum, draw)])
            .map_err(E::Carrier)?[0];
        let m = &mut self.markers[i];
        m.pending = None;
        m.last_us = now;
        Ok(Some(ThresholdHealOutcome {
            started: false,
            pending: false,
            healed: Some(healed),
            qualification: QUALIFICATION,
        }))
    }
}
#[cfg(test)]
#[path = "source_threshold_heal_tests.rs"]
mod tests;
