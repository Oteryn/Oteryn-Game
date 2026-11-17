//! Source-known AREA/NAMED defense healing into actual native Creature HP.
//! Creature-only partialcast; Player/NPC healing and presentation delivery remain explicit.
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord as Record, ProjectV2AbilityDetails,
    ProjectV2AbilityEffect, ProjectV2AffectsKind, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2EffectAffects, ProjectV2Family,
    ProjectV2FormulaAuthoring, ProjectV2InlineEffectOperation, ProjectV2SourceIdentityDisposition,
};
use crate::creature_auto_attack::ProfileAbilityProposal;
use oteryn_gameplay_content::content::creature_attack_geometry::{self as geometry, Facing};
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use {
    crate::foundation::{
        ChannelRuntimeV1, ExactActorRef, OwnerDamageResult, RuntimeWorkStamp, ScopeRuntimeFence,
    },
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaHealError {
    InvalidSource,
    UnsupportedSource,
    MissingCurrentPolicy,
    StaleOwner,
    InvalidOccurrence,
    ReplayConflict,
    Numeric,
    Capacity,
    Carrier(CarrierError),
}
#[derive(Debug, Clone)]
pub struct CreatureHealCatalog {
    content: [u8; 32],
    pub world: crate::foundation::WorldId,
    pub maximum: BTreeMap<String, u64>,
}
impl CreatureHealCatalog {
    pub fn matches_current(&self, r: &ChannelRuntimeV1) -> bool {
        self.world == r.content_pin().world_id()
            && self.content == r.content_pin().server_artifact_digest()
    }

    /// Current native identity+positive HP under this exact source catalog and content pin.
    pub fn current_target(
        &self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
    ) -> Result<(String, u64), AreaHealError> {
        if r.content_pin().world_id() != self.world
            || r.content_pin().server_artifact_digest() != self.content
        {
            return Err(AreaHealError::StaleOwner);
        }
        let key = std::str::from_utf8(
            r.current_live_creature_identity(target)
                .map_err(AreaHealError::Carrier)?,
        )
        .map_err(|_| AreaHealError::InvalidSource)?;
        let max = *self.maximum.get(key).ok_or(AreaHealError::InvalidSource)?;
        r.read_source_creature_health(target, key, max)
            .map_err(AreaHealError::Carrier)?;
        Ok((key.to_owned(), max))
    }

    /// Build the same identity/HP catalog from an already-qualified active native artifact.
    /// Detached caller profiles cannot supply this current content binding.
    pub fn from_current_native(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Result<Self, AreaHealError> {
        runtime
            .owner_fence()
            .map_err(|_| AreaHealError::StaleOwner)?;
        if runtime.content_pin().server_artifact_digest() != native.source_digest() {
            return Err(AreaHealError::InvalidSource);
        }
        let mut maximum = BTreeMap::new();
        for record in &native.creature_profiles().records {
            let Data::Creature(creature) = &record.profile.data else {
                return Err(AreaHealError::InvalidSource);
            };
            // The qualified loader owns each admitted revision. Canonical source
            // definitions and retained legacy creatures share this active catalog;
            // a different admitted revision is not a missing HP value.
            if record.profile.target.family != ProjectV2Family::Creature
                || record.profile.target.key.is_empty()
                || record.profile.target.revision.is_empty()
            {
                return Err(AreaHealError::InvalidSource);
            }
            let max = creature
                .health
                .filter(|health| *health > 0 && *health <= i64::MAX as u64)
                .ok_or(AreaHealError::InvalidSource)?;
            if maximum
                .insert(record.profile.target.key.clone(), max)
                .is_some()
            {
                return Err(AreaHealError::InvalidSource);
            }
        }
        Ok(Self {
            content: native.source_digest(),
            world: runtime.content_pin().world_id(),
            maximum,
        })
    }
    pub fn from_native(draft: &ProjectV2Draft, content: [u8; 32]) -> Result<Self, AreaHealError> {
        let value = draft.core.world_id.as_bytes();
        if value.len() != 32
            || !value
                .iter()
                .all(|b| b.is_ascii_digit() || matches!(*b, b'a'..=b'f'))
        {
            return Err(AreaHealError::InvalidSource);
        }
        let mut bytes = [0u8; 16];
        for (i, pair) in value.as_chunks::<2>().0.iter().enumerate() {
            let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
            bytes[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        let world =
            crate::foundation::WorldId::decode(&bytes).map_err(|_| AreaHealError::InvalidSource)?;
        let mut maximum = BTreeMap::new();
        for p in &draft.state.authoring_profiles {
            let Data::Creature(c) = &p.data else { continue };
            if p.target.family != ProjectV2Family::Creature || p.target.revision != "definition-r1"
            {
                return Err(AreaHealError::InvalidSource);
            }
            if !draft.core.records.iter().any(|r|matches!(r,Record::Creature{identity,..}if identity.key==p.target.key&&identity.revision==p.target.revision)){return Err(AreaHealError::InvalidSource)}
            if !draft.state.source_identity_bindings.iter().any(|b| {
                b.target == p.target
                    && b.disposition == ProjectV2SourceIdentityDisposition::Exact
                    && draft
                        .state
                        .sources
                        .iter()
                        .any(|s| s.key == b.source_key && s.revision == b.source_revision)
            }) {
                return Err(AreaHealError::InvalidSource);
            }
            let max = c
                .health
                .filter(|h| *h > 0 && *h <= i64::MAX as u64)
                .ok_or(AreaHealError::InvalidSource)?;
            if maximum.insert(p.target.key.clone(), max).is_some() {
                return Err(AreaHealError::InvalidSource);
            }
        }
        Ok(Self {
            content,
            world,
            maximum,
        })
    }
}
#[derive(Debug, Clone)]
pub struct AreaHealSource {
    content: [u8; 32],
    caster: Ref,
    pub ability: Ref,
    index: usize,
    details: Box<ProjectV2AbilityDetails>,
    pub minimum: u64,
    pub maximum: u64,
    pub filter: Option<ProjectV2EffectAffects>,
    remove_paralysis: bool,
    fingerprint: [u8; 32],
    pub source_load_magnitude: Option<u64>,
}
impl AreaHealSource {
    pub fn from_native(
        draft: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, AreaHealError> {
        Self::from_parts(
            caster,
            index,
            content,
            &draft.core.records,
            &draft.state.authoring_profiles,
        )
    }
    /// Only the currently admitted native closure supplies this rule body.
    pub fn from_current_native(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        caster: &Ref,
        index: usize,
    ) -> Result<Self, AreaHealError> {
        runtime
            .owner_fence()
            .map_err(|_| AreaHealError::StaleOwner)?;
        if runtime.content_pin().server_artifact_digest() != native.source_digest() {
            return Err(AreaHealError::InvalidSource);
        }
        let mut records = native
            .creature_profiles()
            .records
            .iter()
            .filter(|record| record.profile.target == *caster);
        let record = records.next().ok_or(AreaHealError::InvalidSource)?;
        if records.next().is_some() {
            return Err(AreaHealError::InvalidSource);
        }
        let definitions = record
            .attack_definitions
            .as_ref()
            .ok_or(AreaHealError::InvalidSource)?;
        for actual in [Some(&record.profile), record.behavior.as_ref()]
            .into_iter()
            .flatten()
        {
            let mut bodies = definitions
                .profiles
                .iter()
                .filter(|body| body.target == actual.target);
            if bodies.next() != Some(actual) || bodies.next().is_some() {
                return Err(AreaHealError::InvalidSource);
            }
        }
        Self::from_parts(
            caster,
            index,
            native.source_digest(),
            &definitions.records,
            &definitions.profiles,
        )
    }
    fn from_parts(
        caster: &Ref,
        index: usize,
        content: [u8; 32],
        records: &[Record],
        profiles: &[crate::content::ProjectV2AuthoringProfile],
    ) -> Result<Self, AreaHealError> {
        use AreaHealError as E;
        if caster.family != ProjectV2Family::Creature || caster.revision != "definition-r1" {
            return Err(E::InvalidSource);
        }
        let Some(Record::Creature{behavior,..})=records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(E::InvalidSource)};
        let Some(Data::Creature(cp)) = profiles
            .iter()
            .find(|p| p.target == *caster)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        let Some(Data::Behavior(b)) = profiles
            .iter()
            .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        let e = b.defenses.get(index).ok_or(E::InvalidSource)?;
        if !cp.abilities.contains(&e.ability) || index >= 8 {
            return Err(E::InvalidSource);
        }
        let Some(Data::Ability(a)) = profiles
            .iter()
            .find(|p| p.target == e.ability)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        let d = a.details.as_deref().ok_or(E::InvalidSource)?;
        if d.kind != crate::content::ProjectV2AbilityKind::Spell
            || d.needs_target
            || d.area.is_none()
            || !d.variants.is_empty()
            || d.chain.is_some()
            || d.windup.is_some()
            || d.encounter.is_some()
        {
            return Err(E::UnsupportedSource);
        }
        let mut effect = None;
        let mut remove_paralysis = false;
        for ef in &d.effects {
            match ef {
                ProjectV2AbilityEffect::Executable(r) => {
                    if effect.replace(r).is_some() {
                        return Err(E::UnsupportedSource);
                    }
                }
                ProjectV2AbilityEffect::Inline(i) => match &i.operation {
                    ProjectV2InlineEffectOperation::PresentationOnly => {}
                    ProjectV2InlineEffectOperation::RemoveCondition { condition }
                        if condition == "paralyze" && !remove_paralysis =>
                    {
                        remove_paralysis = true
                    }
                    _ => return Err(E::UnsupportedSource),
                },
            }
        }
        let effect = effect.ok_or(E::InvalidSource)?;
        if effect.family != ProjectV2Family::Effect || e.ability.family != ProjectV2Family::Ability
        {
            return Err(E::InvalidSource);
        }
        let Some(Record::Ability{effects,..})=records.iter().find(|r|matches!(r,Record::Ability{identity,..}if identity.key==e.ability.key&&identity.revision==e.ability.revision))else{return Err(E::InvalidSource)};
        if effects.len() != 1
            || effects[0].family != "Effect"
            || effects[0].key != effect.key
            || effects[0].revision != effect.revision
        {
            return Err(E::InvalidSource);
        }
        let Some(Record::Effect{effect_family:EffectFamilyDocument::Heal,formula,..})=records.iter().find(|r|matches!(r,Record::Effect{identity,..}if identity.key==effect.key&&identity.revision==effect.revision))else{return Err(E::UnsupportedSource)};
        let Some(Data::Effect(ep)) = profiles
            .iter()
            .find(|p| p.target == *effect)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        if ep.damage_type != "healing" || !ep.mitigated_by.is_empty() {
            return Err(E::UnsupportedSource);
        }
        if !records.iter().any(|r|matches!(r,Record::Formula{identity}if identity.key==formula.key&&identity.revision==formula.revision)){return Err(E::InvalidSource)}
        let Some(Data::Formula(f)) = profiles
            .iter()
            .find(|p| p.target.key == formula.key && p.target.revision == formula.revision)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        let (minimum, maximum) = match f {
            ProjectV2FormulaAuthoring::Range { minimum, maximum } => (*minimum, *maximum),
            ProjectV2FormulaAuthoring::CasterMagnitude => {
                let m = e.magnitude.as_ref().ok_or(E::InvalidSource)?;
                (m.minimum, m.maximum)
            }
            _ => return Err(E::UnsupportedSource),
        };
        if minimum > maximum || maximum > i64::MAX as u64 {
            return Err(E::InvalidSource);
        }
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(caster, index, e, a, ep, f, content))
                .map_err(|_| E::InvalidSource)?,
        )
        .into();
        let source_load_magnitude = if caster.key == "oteryn:creature.minotaur_cult_prophet" {
            let filter = ep.affects.as_ref().ok_or(E::InvalidSource)?;
            let expected = [
                "oteryn:creature.minotaur_cult_follower",
                "oteryn:creature.minotaur_cult_prophet",
                "oteryn:creature.minotaur_cult_zealot",
            ];
            if minimum != 200
                || maximum != 350
                || filter.kind != ProjectV2AffectsKind::NamedCreatures
                || !filter.includes_caster
                || filter.excludes_caster_name
                || !filter.top_creature_only
                || filter.creatures.len() != 3
                || !expected.iter().all(|key| {
                    filter.creatures.iter().any(|v| {
                        v.key == *key
                            && v.family == ProjectV2Family::Creature
                            && v.revision == "definition-r1"
                    })
                })
            {
                return Err(E::InvalidSource);
            }
            Some(project_prophet_source_load_magnitude(fingerprint))
        } else {
            None
        };
        Ok(Self {
            content,
            caster: caster.clone(),
            ability: e.ability.clone(),
            index,
            details: Box::new(d.clone()),
            minimum,
            maximum,
            filter: ep.affects.clone(),
            remove_paralysis,
            fingerprint,
            source_load_magnitude,
        })
    }
    pub fn requires_paralysis_removal(&self) -> bool {
        self.remove_paralysis
    }
}
/// Current Combat/map relation facts; absence refuses the cast before HP writes.
/// Player/NPC HP is not supplied by this Creature-only owner. A true top-creature decision
/// must come from actual tile stack ownership, never slot-order approximation.
pub trait AreaHealPolicy {
    fn facing(
        &mut self,
        r: &ChannelRuntimeV1,
        caster: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing>;
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        caster: ExactActorRef,
        x: i32,
        y: i32,
        z: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn target_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        caster: ExactActorRef,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn masterless(
        &mut self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn is_top_creature(
        &mut self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
}
/// Atomic heal/cure publication supplied by the existing Channel spell-state owner.
pub trait AreaHealStates {
    #[allow(clippy::too_many_arguments)]
    fn commit_source_area_heal_and_paralysis_removal_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        content: [u8; 32],
        heal: &[(ExactActorRef, String, u64, u64)],
        cure: &[ExactActorRef],
        now: u64,
        stamp: RuntimeWorkStamp,
    ) -> Result<Vec<OwnerDamageResult>, oteryn_gameplay_domain::domain::carrier_values::CarrierError>;
    #[allow(clippy::too_many_arguments)]
    fn commit_source_area_heal_and_paralysis_removal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        content: [u8; 32],
        heal: &[(ExactActorRef, String, u64, u64)],
        cure: &[ExactActorRef],
        now: u64,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<Vec<OwnerDamageResult>, oteryn_gameplay_domain::domain::carrier_values::CarrierError>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaHealOutcome {
    pub targets: Vec<(ExactActorRef, OwnerDamageResult)>,
    pub qualification: &'static str,
}
pub struct Memo {
    caster: ExactActorRef,
    ability: Ref,
    index: usize,
    pub sequence: u64,
    occurrence: String,
    binding: [u8; 32],
    activation: u64,
    result: AreaHealOutcome,
}
#[derive(Default)]
pub struct AreaHealOwner {
    pub memos: Vec<Memo>,
}
impl AreaHealOwner {
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        r: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        catalog: &CreatureHealCatalog,
        source: &AreaHealSource,
        proposal: &ProfileAbilityProposal,
        policy: &mut dyn AreaHealPolicy,
        states: &mut impl AreaHealStates,
        now: u64,
    ) -> Result<AreaHealOutcome, AreaHealError> {
        self.execute_bound(
            r,
            Some(fence),
            stamp,
            catalog,
            source,
            proposal,
            policy,
            states,
            now,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn execute_current(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        catalog: &CreatureHealCatalog,
        source: &AreaHealSource,
        proposal: &ProfileAbilityProposal,
        policy: &mut dyn AreaHealPolicy,
        states: &mut dyn AreaHealStates,
        now: u64,
    ) -> Result<AreaHealOutcome, AreaHealError> {
        self.execute_bound(
            r, None, stamp, catalog, source, proposal, policy, states, now,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn execute_bound(
        &mut self,
        r: &mut ChannelRuntimeV1,
        fence: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        catalog: &CreatureHealCatalog,
        source: &AreaHealSource,
        proposal: &ProfileAbilityProposal,
        policy: &mut dyn AreaHealPolicy,
        states: &mut dyn AreaHealStates,
        now: u64,
    ) -> Result<AreaHealOutcome, AreaHealError> {
        use AreaHealError as E;
        let binding = r.binding();
        if !fence.or_else(|| r.owner_fence().ok()).is_some_and(|fence| {
            fence.is_current_for_scope(binding.runtime_scope(), binding.scope_generation())
                && fence.accepts_stamp(stamp)
        }) {
            return Err(E::StaleOwner);
        }
        if source.content != catalog.content
            || r.content_pin().world_id() != catalog.world
            || r.content_pin().server_artifact_digest() != catalog.content
            || !r.matches_live_creature_identity(proposal.issuer, source.caster.key.as_bytes())
        {
            return Err(E::StaleOwner);
        }
        if proposal.list != ScheduleList::Defence
            || proposal.entry_index != source.index
            || proposal.ability != source.ability
        {
            return Err(E::InvalidOccurrence);
        }
        let occurrence = proposal.occurrence.id().as_str();
        let actor_hex = proposal
            .issuer
            .placement_identity()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let prefix = format!("ai-profile:{actor_hex}:");
        let tail = occurrence
            .strip_prefix(&prefix)
            .ok_or(E::InvalidOccurrence)?;
        let (raw_sequence, suffix) = tail.split_once(':').ok_or(E::InvalidOccurrence)?;
        let sequence = raw_sequence
            .parse::<u64>()
            .map_err(|_| E::InvalidOccurrence)?;
        if suffix != format!("defence:{}", source.index) || proposal.target != proposal.issuer {
            return Err(E::InvalidOccurrence);
        }
        let activation = r.content_pin().activation_sequence();
        self.memos
            .retain(|m| r.contains_live_creature(m.caster) && m.activation == activation);
        let old = self.memos.iter().position(|m| {
            m.caster == proposal.issuer && m.ability == source.ability && m.index == source.index
        });
        if let Some(i) = old {
            let m = &self.memos[i];
            if sequence < m.sequence {
                return Err(E::ReplayConflict);
            }
            if sequence == m.sequence {
                return if m.binding == source.fingerprint && m.occurrence == occurrence {
                    Ok(m.result.clone())
                } else {
                    Err(E::ReplayConflict)
                };
            }
        }
        if old.is_none() {
            if self.memos.len()
                >= r.actor_slot_capacity()
                    .min(crate::foundation::carrier::CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX)
                    * crate::foundation::carrier::CREATUREAI0_DEFENCES_PER_CREATURE_MAX
            {
                return Err(E::Capacity);
            }
            self.memos.try_reserve(1).map_err(|_| E::Capacity)?;
        }
        let from = r.read_actor_position(proposal.issuer).map_err(E::Carrier)?;
        if from.context() != r.pinned_movement_context() {
            return Err(E::StaleOwner);
        }
        let p = from.position();
        let center = if source.details.needs_direction {
            let facing = policy
                .facing(r, proposal.issuer, stamp)
                .ok_or(E::MissingCurrentPolicy)?;
            let (x, y) = geometry::step(facing);
            (
                p.x.checked_add(x).ok_or(E::Numeric)?,
                p.y.checked_add(y).ok_or(E::Numeric)?,
            )
        } else {
            (p.x, p.y)
        };
        let area = source.details.area.as_ref().ok_or(E::InvalidSource)?;
        let diagonal = matches!(area,crate::content::ProjectV2AbilityArea::Matrix{diagonal,..}if !diagonal.is_empty());
        let facing = geometry::direction(
            i64::from(center.0) - i64::from(p.x),
            i64::from(center.1) - i64::from(p.y),
            diagonal,
        );
        let candidates = r
            .current_live_creature_candidates(
                r.actor_slot_capacity()
                    .min(crate::foundation::carrier::CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX),
            )
            .map_err(E::Carrier)?;
        let mut requests = Vec::new();
        // Source performs caster:addHealth before Combat target-tile callbacks. Joint
        // publication is deliberately atomic here; absent Combat facts refuses all HP.
        if let Some(draw) = source.source_load_magnitude {
            let (key, max) = catalog.current_target(r, proposal.issuer)?;
            requests.try_reserve(1).map_err(|_| E::Capacity)?;
            requests.push((proposal.issuer, key, max, draw));
        }
        let mut cure = Vec::new();
        cure.try_reserve(candidates.len())
            .map_err(|_| E::Capacity)?;
        let root = GameplayDecisionRoot::from_bytes(catalog.content);
        let digest = Sha256::new()
            .chain_update(b"oteryn:source-area-heal:v1")
            .chain_update(occurrence.as_bytes())
            .chain_update(proposal.issuer.placement_identity())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let id = DecisionOccurrenceId::from_bytes(id);
        for (dx, dy) in geometry::offsets(area, facing).map_err(|_| E::UnsupportedSource)? {
            let x = center.0.checked_add(dx).ok_or(E::Numeric)?;
            let y = center.1.checked_add(dy).ok_or(E::Numeric)?;
            match policy.tile_allowed(r, proposal.issuer, x, y, p.floor, stamp) {
                None => return Err(E::MissingCurrentPolicy),
                Some(false) => continue,
                Some(true) => {}
            }
            for target in &candidates {
                if source.source_load_magnitude.is_some() && *target == proposal.issuer {
                    continue;
                }
                let at = r.read_actor_position(*target).map_err(E::Carrier)?;
                let q = at.position();
                if at.context() != from.context() || q.x != x || q.y != y || q.floor != p.floor {
                    continue;
                }
                let key = std::str::from_utf8(
                    r.current_live_creature_identity(*target)
                        .map_err(E::Carrier)?,
                )
                .map_err(|_| E::InvalidSource)?;
                match policy.target_allowed(r, proposal.issuer, *target, stamp) {
                    None => return Err(E::MissingCurrentPolicy),
                    Some(false) => continue,
                    Some(true) => {}
                }
                if source.remove_paralysis {
                    cure.push(*target);
                }
                if let Some(filter) = &source.filter {
                    if *target == proposal.issuer && !filter.includes_caster
                        || filter.excludes_caster_name && key == source.caster.key
                    {
                        continue;
                    }
                    let admitted = match filter.kind {
                        ProjectV2AffectsKind::NamedCreatures => {
                            filter.creatures.iter().any(|c| c.key == key)
                        }
                        ProjectV2AffectsKind::MasterlessMonsters => policy
                            .masterless(r, *target, stamp)
                            .ok_or(E::MissingCurrentPolicy)?,
                        ProjectV2AffectsKind::NonPlayerSide => policy
                            .non_player_side(r, *target, stamp)
                            .ok_or(E::MissingCurrentPolicy)?,
                        ProjectV2AffectsKind::Players | ProjectV2AffectsKind::PlayerSide => {
                            return Err(E::UnsupportedSource);
                        }
                    };
                    if !admitted {
                        continue;
                    }
                    if filter.top_creature_only
                        && !policy
                            .is_top_creature(r, *target, stamp)
                            .ok_or(E::MissingCurrentPolicy)?
                    {
                        continue;
                    }
                }
                if requests.iter().any(|(a, _, _, _)| a == target) {
                    return Err(E::InvalidSource);
                }
                let max = *catalog.maximum.get(key).ok_or(E::InvalidSource)?;
                let target_id = if source.filter.is_some() {
                    let child = Sha256::new()
                        .chain_update(b"oteryn:source-area-heal-target:v1")
                        .chain_update(occurrence.as_bytes())
                        .chain_update(proposal.issuer.placement_identity())
                        .chain_update(target.placement_identity())
                        .finalize();
                    let mut child_id = [0; 16];
                    child_id.copy_from_slice(&child[..16]);
                    DecisionOccurrenceId::from_bytes(child_id)
                } else {
                    id
                };
                let draw = deterministic_decision_u64(
                    &root,
                    target_id,
                    "source_heal_magnitude",
                    source.index as u64,
                )
                .map_err(|_| E::Numeric)?;
                let draw = source.source_load_magnitude.unwrap_or_else(|| {
                    oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
                        draw,
                        source.minimum as i64,
                        source.maximum as i64,
                    ) as u64
                });
                requests.try_reserve(1).map_err(|_| E::Capacity)?;
                requests.push((*target, key.to_owned(), max, draw));
            }
        }
        let mut result = AreaHealOutcome {
            targets: Vec::new(),
            qualification: if source.source_load_magnitude.is_some() {
                "SOURCE_PROPHET_NAMES_TOP_STACK_ENDPOINTS_EXACT;PROJECT_CONTENT_BOUND_LOAD_RANDOM_SNAPSHOT_GLOBAL_UNVERIFIED;PROJECT_ATOMIC_SELF_PLUS_ALLIES;PRESENTATION_PENDING"
            } else {
                "SOURCE_ENDPOINTS_EXACT_CREATURE_ONLY;PROJECT_UNIFORM_DRAW;PLAYER_NPC_HEAL_AND_PRESENTATION_NOT_EXECUTED"
            },
        };
        result
            .targets
            .try_reserve(requests.len())
            .map_err(|_| E::Capacity)?;
        let mut memo_result = AreaHealOutcome {
            targets: Vec::new(),
            qualification: result.qualification,
        };
        memo_result
            .targets
            .try_reserve(requests.len())
            .map_err(|_| E::Capacity)?;
        // Construct memo-owned strings before HP. Native batch preflights ALL targets itself.
        let ability = source.ability.clone();
        let occurrence = occurrence.to_owned();
        let receipts = if source.remove_paralysis {
            match fence {
                Some(fence) => states.commit_source_area_heal_and_paralysis_removal(
                    r,
                    catalog.content,
                    &requests,
                    &cure,
                    now,
                    fence,
                    stamp,
                ),
                None => states.commit_source_area_heal_and_paralysis_removal_current(
                    r,
                    catalog.content,
                    &requests,
                    &cure,
                    now,
                    stamp,
                ),
            }
        } else {
            r.commit_source_creature_heal_batch(catalog.content, &requests)
        }
        .map_err(E::Carrier)?;
        for ((target, _, _, _), receipt) in requests.into_iter().zip(receipts) {
            memo_result.targets.push((target, receipt));
            result.targets.push((target, receipt));
        }
        let memo = Memo {
            caster: proposal.issuer,
            ability,
            index: source.index,
            sequence,
            occurrence,
            binding: source.fingerprint,
            activation,
            result: memo_result,
        };
        if let Some(i) = old {
            self.memos[i] = memo
        } else {
            self.memos.push(memo)
        }
        Ok(result)
    }
}

/// Explicit local source-load randomness substitute. Same pinned source envelope shares
/// ONE value for all actors/targets/casts. It is not the unavailable donor Lua RNG output.
fn project_prophet_source_load_magnitude(source: [u8; 32]) -> u64 {
    let digest = Sha256::new()
        .chain_update(b"oteryn:project-prophet-load-snapshot:v1")
        .chain_update(source)
        .finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(u64::from_le_bytes(bytes), 200, 350)
        as u64
}

#[cfg(test)]
mod regression_tests {
    #![allow(clippy::expect_used, reason = "native content and owner fixture setup")]
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, WorldId};
    fn fixture() -> (ChannelRuntimeV1, ProjectV2Draft, Ref) {
        let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/area-heal-native-fixture.json"))).expect("qualified fixture");
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let w = WorldId::decode(&id(1)).expect("qualified fixture");
        let r = ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("qualified fixture"),
            NodeId::decode(&id(3)).expect("qualified fixture"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            128,
            ChannelContentPin::test(w),
        )
        .expect("qualified fixture");
        // Fixture envelope only; each source definition/binding below is retained native readback.
        let draft = ProjectV2Draft {
            core: crate::content::ProjectDraft {
                project_revision: "fixture-r1".into(),
                package_key: "oteryn:fixture".into(),
                semantic_schema_version: "v2".into(),
                licensing_metadata: "fixture".into(),
                world_id: r
                    .content_pin()
                    .world_id()
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                coordinate_frame: "fixture".into(),
                records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
                imports: vec![],
                metadata: vec![],
            },
            state: crate::content::ProjectV2State {
                authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                    .expect("qualified fixture"),
                source_identity_bindings: serde_json::from_value(
                    v["source_identity_bindings"].clone(),
                )
                .expect("qualified fixture"),
                sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
                ..Default::default()
            },
        };
        (
            r,
            draft,
            Ref {
                family: ProjectV2Family::Creature,
                key: "oteryn:creature.frozen_minion".into(),
                revision: "definition-r1".into(),
            },
        )
    }
    struct FixturePolicy {
        missing: bool,
    }
    impl AreaHealPolicy for FixturePolicy {
        fn facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<Facing> {
            Some(Facing::North)
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
            (!self.missing).then_some(true)
        }
        fn target_allowed(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            Some(true)
        }
        fn non_player_side(
            &mut self,
            r: &ChannelRuntimeV1,
            target: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.native_summon_role(target).ok().map(|_| true)
        }
        fn masterless(
            &mut self,
            r: &ChannelRuntimeV1,
            target: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.native_summon_role(target).ok().map(|m| m.is_none())
        }
        fn is_top_creature(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            None
        }
    }
    fn proposal_fixture(actor: ExactActorRef, source: &AreaHealSource) -> ProfileAbilityProposal {
        // Explicit owner-boundary proposal fixture, not an autonomous scheduler/chance proof.
        let hex = actor
            .placement_identity()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let atom = format!("fixture:{hex}");
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("qualified fixture");
        ProfileAbilityProposal {
            issuer: actor,
            target: actor,
            ability: source.ability.clone(),
            list: ScheduleList::Defence,
            entry_index: 0,
            magnitude: None,
            range_tiles: 0,
            occurrence: crate::ability::AbilityOccurrence::new(
                &format!("ai-profile:{hex}:0:defence:0"),
                revisions,
            )
            .expect("qualified fixture"),
            intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
                .expect("qualified fixture"),
        }
    }

    struct States;
    impl AreaHealStates for States {
        fn commit_source_area_heal_and_paralysis_removal_current(
            &mut self,
            runtime: &mut ChannelRuntimeV1,
            content: [u8; 32],
            heal: &[(ExactActorRef, String, u64, u64)],
            cure: &[ExactActorRef],
            _now: u64,
            _stamp: RuntimeWorkStamp,
        ) -> Result<Vec<OwnerDamageResult>, CarrierError> {
            assert!(cure.is_empty());
            runtime.commit_source_creature_heal_batch(content, heal)
        }
        fn commit_source_area_heal_and_paralysis_removal(
            &mut self,
            runtime: &mut ChannelRuntimeV1,
            content: [u8; 32],
            heal: &[(ExactActorRef, String, u64, u64)],
            cure: &[ExactActorRef],
            _now: u64,
            _fence: &ScopeRuntimeFence,
            _stamp: RuntimeWorkStamp,
        ) -> Result<Vec<OwnerDamageResult>, CarrierError> {
            assert!(cure.is_empty());
            runtime.commit_source_creature_heal_batch(content, heal)
        }
    }

    #[test]
    fn area_heal_rejects_previous_source_digest_before_replay_or_mutation() {
        for replay in [false, true] {
            let (mut r, d, c) = fixture();
            let digest = r.content_pin().server_artifact_digest();
            let catalog = CreatureHealCatalog::from_native(&d, digest).expect("catalog");
            let current = AreaHealSource::from_native(&d, &c, 0, digest).expect("current source");
            let mut previous_digest = digest;
            previous_digest[0] ^= 1;
            let previous =
                AreaHealSource::from_native(&d, &c, 0, previous_digest).expect("previous source");
            let caster = r
                .admit_source_pinned_lab_creature(
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                    &c.key,
                    1000,
                )
                .expect("caster");
            let binding = r.binding();
            let (fence, stamp) = crate::foundation::crystal_timer_fixture(
                binding.runtime_scope(),
                binding.scope_generation(),
            )
            .expect("fence");
            let proposal = proposal_fixture(caster, &current);
            let mut owner = AreaHealOwner::default();
            let mut policy = FixturePolicy { missing: false };
            let mut states = States;
            if replay {
                owner
                    .execute(
                        &mut r,
                        &fence,
                        stamp,
                        &catalog,
                        &current,
                        &proposal,
                        &mut policy,
                        &mut states,
                        0,
                    )
                    .expect("current heal");
            }
            let health = r.live_creature_health(caster);
            let memo_count = owner.memos.len();
            assert_eq!(
                owner.execute(
                    &mut r,
                    &fence,
                    stamp,
                    &catalog,
                    &previous,
                    &proposal,
                    &mut policy,
                    &mut states,
                    0
                ),
                Err(AreaHealError::StaleOwner)
            );
            assert_eq!(r.live_creature_health(caster), health);
            assert_eq!(owner.memos.len(), memo_count);
        }
    }

    #[test]
    fn area_heal_ignores_distant_creatures_beyond_old_channel_ceiling() {
        let (mut r, d, c) = fixture();
        let digest = r.content_pin().server_artifact_digest();
        let catalog = CreatureHealCatalog::from_native(&d, digest).expect("catalog");
        let source = AreaHealSource::from_native(&d, &c, 0, digest).expect("source");
        let caster = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &c.key,
                1000,
            )
            .expect("caster");
        for x in 0..65 {
            r.admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 1000 + x,
                    y: 1000,
                    floor: 7,
                },
                &c.key,
                1000,
            )
            .expect("distant creature");
        }
        let binding = r.binding();
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(
            binding.runtime_scope(),
            binding.scope_generation(),
        )
        .expect("fence");
        let result = AreaHealOwner::default()
            .execute(
                &mut r,
                &fence,
                stamp,
                &catalog,
                &source,
                &proposal_fixture(caster, &source),
                &mut FixturePolicy { missing: false },
                &mut States,
                0,
            )
            .expect("heal despite distant creatures");
        assert!(
            result
                .targets
                .iter()
                .any(|(actor, hit)| *actor == caster && hit.health_after > 1000)
        );
    }
}
