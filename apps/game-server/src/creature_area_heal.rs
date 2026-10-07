//! Source-known AREA/NAMED defense healing into actual native Creature HP.
//! Creature-only partialcast; Player/NPC healing and presentation delivery remain explicit.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord as Record, ProjectV2AbilityDetails,
    ProjectV2AbilityEffect, ProjectV2AffectsKind, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2EffectAffects, ProjectV2Family,
    ProjectV2FormulaAuthoring, ProjectV2InlineEffectOperation, ProjectV2SourceIdentityDisposition,
};
use crate::creature_attack_geometry::{self as geometry, Facing};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, OwnerDamageResult, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeRuntimeFence,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AreaHealError {
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
pub(crate) struct CreatureHealCatalog {
    content: [u8; 32],
    world: crate::foundation::WorldId,
    maximum: BTreeMap<String, u64>,
}
impl CreatureHealCatalog {
    pub(crate) fn matches_current(&self, r: &ChannelRuntimeV1) -> bool {
        self.world == r.content_pin().world_id()
            && self.content == r.content_pin().server_artifact_digest()
    }

    /// Current native identity+positive HP under this exact source catalog and content pin.
    pub(crate) fn current_target(
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

    pub(crate) fn from_native(
        draft: &ProjectV2Draft,
        content: [u8; 32],
    ) -> Result<Self, AreaHealError> {
        let value = draft.core.world_id.as_bytes();
        if value.len() != 32
            || !value
                .iter()
                .all(|b| b.is_ascii_digit() || matches!(*b, b'a'..=b'f'))
        {
            return Err(AreaHealError::InvalidSource);
        }
        let mut bytes = [0u8; 16];
        for (i, pair) in value.chunks_exact(2).enumerate() {
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
pub(crate) struct AreaHealSource {
    caster: Ref,
    ability: Ref,
    index: usize,
    details: Box<ProjectV2AbilityDetails>,
    minimum: u64,
    maximum: u64,
    filter: Option<ProjectV2EffectAffects>,
    remove_paralysis: bool,
    fingerprint: [u8; 32],
    source_load_magnitude: Option<u64>,
}
impl AreaHealSource {
    pub(crate) fn from_native(
        draft: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, AreaHealError> {
        use AreaHealError as E;
        if caster.family != ProjectV2Family::Creature || caster.revision != "definition-r1" {
            return Err(E::InvalidSource);
        }
        let Some(Record::Creature{behavior,..})=draft.core.records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(E::InvalidSource)};
        let Some(Data::Creature(cp)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *caster)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        let Some(Data::Behavior(b)) = draft
            .state
            .authoring_profiles
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
        let Some(Data::Ability(a)) = draft
            .state
            .authoring_profiles
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
        let Some(Record::Ability{effects,..})=draft.core.records.iter().find(|r|matches!(r,Record::Ability{identity,..}if identity.key==e.ability.key&&identity.revision==e.ability.revision))else{return Err(E::InvalidSource)};
        if effects.len() != 1
            || effects[0].family != "Effect"
            || effects[0].key != effect.key
            || effects[0].revision != effect.revision
        {
            return Err(E::InvalidSource);
        }
        let Some(Record::Effect{effect_family:EffectFamilyDocument::Heal,formula,..})=draft.core.records.iter().find(|r|matches!(r,Record::Effect{identity,..}if identity.key==effect.key&&identity.revision==effect.revision))else{return Err(E::UnsupportedSource)};
        let Some(Data::Effect(ep)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *effect)
            .map(|p| &p.data)
        else {
            return Err(E::InvalidSource);
        };
        if ep.damage_type != "healing" || !ep.mitigated_by.is_empty() {
            return Err(E::UnsupportedSource);
        }
        if !draft.core.records.iter().any(|r|matches!(r,Record::Formula{identity}if identity.key==formula.key&&identity.revision==formula.revision)){return Err(E::InvalidSource)}
        let Some(Data::Formula(f)) = draft
            .state
            .authoring_profiles
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
    #[cfg(test)]
    pub(crate) fn requires_paralysis_removal(&self) -> bool {
        self.remove_paralysis
    }
}
/// Current Combat/map relation facts; absence refuses the cast before HP writes.
/// Player/NPC HP is not supplied by this Creature-only owner. A true top-creature decision
/// must come from actual tile stack ownership, never slot-order approximation.
pub(crate) trait AreaHealPolicy {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AreaHealOutcome {
    pub(crate) targets: Vec<(ExactActorRef, OwnerDamageResult)>,
    pub(crate) qualification: &'static str,
}
struct Memo {
    caster: ExactActorRef,
    ability: Ref,
    index: usize,
    sequence: u64,
    occurrence: String,
    binding: [u8; 32],
    activation: u64,
    result: AreaHealOutcome,
}
#[derive(Default)]
pub(crate) struct AreaHealOwner {
    memos: Vec<Memo>,
}
impl AreaHealOwner {
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &mut self,
        r: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        catalog: &CreatureHealCatalog,
        source: &AreaHealSource,
        proposal: &ProfileAbilityProposal,
        policy: &mut dyn AreaHealPolicy,
        states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
        now: u64,
    ) -> Result<AreaHealOutcome, AreaHealError> {
        use AreaHealError as E;
        let binding = r.binding();
        if !fence.is_current_for_scope(
            RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(E::StaleOwner);
        }
        if r.content_pin().world_id() != catalog.world
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
            if self.memos.len() >= 64 * 8 {
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
        let candidates = r.current_live_creature_candidates(64).map_err(E::Carrier)?;
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
                    crate::spell::uniform_draw(draw, source.minimum as i64, source.maximum as i64)
                        as u64
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
            states.commit_source_area_heal_and_paralysis_removal(
                r,
                catalog.content,
                &requests,
                &cure,
                now,
                fence,
                stamp,
            )
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
    crate::spell::uniform_draw(u64::from_le_bytes(bytes), 200, 350) as u64
}

#[cfg(test)]
#[path = "creature_area_heal_tests.rs"]
mod tests;
