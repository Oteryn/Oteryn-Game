//! Three pinned boss heal callbacks using native Creature HP and the shared
//! canonical regeneration/subID88888 condition. Pending timers retain their
//! exact source occurrence independently of fresh defense proposals.
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
    ProjectV2SourceIdentityDisposition as Disposition,
};
use crate::creature_auto_attack::ProfileAbilityProposal;
use oteryn_gameplay_domain::domain::carrier_values::CarrierError;
pub use oteryn_gameplay_encounter_rules::thresholds::below_health_threshold;
pub use oteryn_gameplay_encounter_rules::timers::checked_deadline;
use oteryn_gameplay_entities::ChannelRuntimeV1;
use oteryn_gameplay_entities::OwnerDamageResult;
use oteryn_gameplay_foundation::foundation::ExactActorRef;

use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
use oteryn_gameplay_foundation::foundation::ScopeRuntimeFence;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

const QUALIFICATION: &str = "PROJECT_SOURCE_MARKER_LOCAL;SOURCE_THRESHOLD_RANGE_DELAY_COOLDOWN;GLOBAL_CONDITION88888_INTEROPERABILITY_UNVERIFIED;PROJECT_UNIFORM_DRAW;SPEECH_VFX_NOT_DELIVERED";
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThresholdHealError {
    Source,
    Fence,
    Occurrence,
    Replay,
    Clock,
    Capacity,
    Carrier(CarrierError),
    Condition(oteryn_gameplay_entities::ConditionOwnerError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdHealOutcome {
    pub started: bool,
    pub pending: bool,
    pub healed: Option<OwnerDamageResult>,
    pub qualification: &'static str,
}
#[derive(Clone, Debug)]
pub struct ThresholdHealSource {
    caster: Ref,
    pub ability: Ref,
    pub index: usize,
    pub world: String,
    pub content: [u8; 32],
    pub maximum: u64,
    threshold_ppm: u64,
    pub minimum: u64,
    pub maximum_draw: u64,
    pub delay_us: u64,
    pub cooldown_us: u64,
    fingerprint: [u8; 32],
}
impl ThresholdHealSource {
    /// Qualifies actual active native bodies without manufacturing a Project Draft.
    /// The world is descriptor data; mutation separately checks its current owner fence.
    pub fn qualify_native(
        native: &crate::content::native_gameplay::NativeGameplayState,
        caster: &Ref,
        index: usize,
        world: [u8; 16],
    ) -> Result<Self, ThresholdHealError> {
        use ThresholdHealError as E;
        let proof = crate::bosses::source_callbacks::native_witness::qualify(
            native,
            caster,
            index,
            ScheduleList::Defence,
        )
        .map_err(|_| E::Source)?
        .ok_or(E::Source)?;
        let (threshold, minimum, maximum_draw, delay_ms, cooldown_ms) =
            match (caster.key.as_str(), proof.entry().ability.key.as_str()) {
                ("oteryn:creature.lisa", "oteryn:ability.creature.lisa.field-fill-lisa_heal") => {
                    (70000, 18000, 23000, 6000_u64, 6000_u64)
                }
                ("oteryn:creature.tyrn", "oteryn:ability.creature.tyrn.field-fill-tyrn_heal") => {
                    (200000, 5000, 7500, 0, 900000)
                }
                (
                    "oteryn:creature.professor_maxxen",
                    "oteryn:ability.spell.glooth_fairy_healing",
                ) => (100000, 7500, 8000, 10000, 30000),
                _ => return Err(E::Source),
            };
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == *caster)
            .ok_or(E::Source)?;
        let Data::Creature(creature) = &record.profile.data else {
            return Err(E::Source);
        };
        let Data::Behavior(behavior) = &record.behavior.as_ref().ok_or(E::Source)?.data else {
            return Err(E::Source);
        };
        let entry = behavior.defenses.get(index).ok_or(E::Source)?;
        if entry.interval_ms != 1000 || entry.chance_ppm != 1000000 {
            return Err(E::Source);
        }
        let definitions = record.attack_definitions.as_ref().ok_or(E::Source)?;
        let parent = definitions
            .profiles
            .iter()
            .find(|profile| profile.target == entry.ability)
            .ok_or(E::Source)?;
        let Data::Ability(ability) = &parent.data else {
            return Err(E::Source);
        };
        let encounter = proof.encounter_profile().map_err(|_| E::Source)?;
        static EXPECTED: std::sync::OnceLock<Option<std::collections::BTreeMap<String, Data>>> =
            std::sync::OnceLock::new();
        let expected=EXPECTED.get_or_init(||serde_json::from_str(include_str!("../../../../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/callback18/threshold-heal-expected-encounters.json")).ok()).as_ref().ok_or(E::Source)?;
        if expected.get(&encounter.target.key) != Some(&encounter.data) {
            return Err(E::Source);
        }
        let maximum = creature
            .health
            .filter(|health| *health > 0 && *health <= i64::MAX as u64)
            .ok_or(E::Source)?;
        let world = world
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let content = proof.content();
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                caster, index, entry, ability, encounter, maximum, content, &world,
            ))
            .map_err(|_| E::Source)?,
        )
        .into();
        Ok(Self {
            caster: caster.clone(),
            ability: entry.ability.clone(),
            index,
            world,
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
    pub fn from_native(
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
        let expected:serde_json::Value=serde_json::from_str(include_str!("../../../../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/callback18/threshold-heal-expected-encounters.json")).map_err(|_|E::Source)?;
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
    pub fn ability(&self) -> &Ref {
        &self.ability
    }
    fn guard(
        &self,
        r: &ChannelRuntimeV1,
        f: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
    ) -> Result<(), ThresholdHealError> {
        let binding = r.binding();
        let f = f
            .or_else(|| r.owner_fence().ok())
            .ok_or(ThresholdHealError::Fence)?;
        if !f.is_current_for_scope(binding.runtime_scope(), binding.scope_generation())
            || !f.accepts_stamp(stamp)
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
pub struct Marker {
    pub actor: ExactActorRef,
    source: [u8; 32],
    pub content: [u8; 32],
    pub activation: u64,
    sequence: u64,
    occurrence: String,
    last_us: u64,
    pub pending: Option<(u64, u64)>,
    last: ThresholdHealOutcome,
}
#[derive(Default)]
pub struct ThresholdHealOwner {
    pub markers: Vec<Marker>,
}
impl ThresholdHealOwner {
    pub fn cast(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        p: &ProfileAbilityProposal,
        now: u64,
    ) -> Result<ThresholdHealOutcome, ThresholdHealError> {
        self.cast_bound(r, Some(f), stamp, s, p, now)
    }
    pub fn cast_current(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        p: &ProfileAbilityProposal,
        now: u64,
    ) -> Result<ThresholdHealOutcome, ThresholdHealError> {
        self.cast_bound(r, None, stamp, s, p, now)
    }
    #[allow(clippy::too_many_arguments)]
    fn cast_bound(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: Option<&ScopeRuntimeFence>,
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
        let shared_marker = r
            .actor_conditions_at(p.issuer, None, now)
            .map_err(E::Condition)?
            // Lua getCondition returns the stored pointer until the source
            // postattack executeConditions phase, including the exact deadline.
            .get(oteryn_gameplay_domain::domain::condition::ConflictKey::SpellRegeneration(88888))
            .is_some();
        let locked = shared_marker || existing.is_some_and(|i| self.markers[i].pending.is_some());
        let started = !locked && below_health_threshold(health, s.maximum, s.threshold_ppm);
        let mut pending = existing.and_then(|i| self.markers[i].pending);
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
            checked_deadline(now, s.cooldown_us).ok_or(E::Clock)?;
            let due = checked_deadline(now, s.delay_us).ok_or(E::Clock)?;
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
            // Lua installs CONDITION_REGENERATION/subID88888 before healing.
            // Its zero integral gains are a marker in the SAME canonical actor
            // condition store consumed by other pinned native callbacks.
            use oteryn_gameplay_domain::domain::condition::ConditionValues;
            use oteryn_gameplay_entities::{ActorConditionTransition, ConditionSource};
            use oteryn_gameplay_foundation::foundation::condition::{
                ApplicationFacts, ConditionDefinition, ConditionSourceKind,
            };
            let root = GameplayDecisionRoot::from_bytes(s.content);
            // Regeneration markers have no speed operation. As in the
            // canonical non-speed condition owners, this unused fact is zero;
            // marker admission does not require a companion relationship.
            let base_speed = 0;
            let definition = ConditionDefinition::new(
                "source.monster.regeneration-marker.88888",
                1,
                ConditionValues::SpellRegeneration {
                    duration_ms: u32::try_from(s.cooldown_us / 1000).map_err(|_| E::Clock)?,
                    sub_id: 88888,
                    health_gain: 0,
                    health_interval_ms: u32::try_from(s.cooldown_us / 1000)
                        .map_err(|_| E::Clock)?,
                    mana_gain: 0,
                    mana_interval_ms: 1000,
                },
            )
            .ok_or(E::Source)?;
            let at = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
            let plan = r
                .prepare_actor_condition(
                    p.issuer,
                    None,
                    ActorConditionTransition::Apply {
                        definition: &definition,
                        source: ConditionSource {
                            actor: p.issuer,
                            session: None,
                            kind: ConditionSourceKind::SelfUse,
                        },
                        immunities: &[],
                        facts: ApplicationFacts {
                            now,
                            base_speed,
                            mana_shield_capacity: 0,
                            target_reentry_protected: false,
                            source_reentry_protected: false,
                            target_is_player: false,
                            decision_root: &root,
                            occurrence: DecisionOccurrenceId::from_bytes(id),
                        },
                    },
                    at,
                )
                .map_err(E::Condition)?;
            r.commit_actor_condition(&plan, at).map_err(E::Condition)?;
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
    pub fn tick(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<Option<ThresholdHealOutcome>, ThresholdHealError> {
        self.tick_bound(r, Some(f), stamp, s, actor, now)
    }
    pub fn tick_current(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        s: &ThresholdHealSource,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<Option<ThresholdHealOutcome>, ThresholdHealError> {
        self.tick_bound(r, None, stamp, s, actor, now)
    }
    #[allow(clippy::too_many_arguments)]
    fn tick_bound(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: Option<&ScopeRuntimeFence>,
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
