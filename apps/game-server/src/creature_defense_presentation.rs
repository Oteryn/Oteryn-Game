//! Source-native self-position presentation producer. No HP, MP or network mutation.
use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityDetails, ProjectV2AbilityEffect, ProjectV2AbilityKind,
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2InlineEffectOperation,
};
use crate::creature_attack_geometry::{self as geometry, Facing};
use crate::creature_auto_attack::{AttackError, ProfileAbilityProposal};
use crate::durability::creature_schedule::ScheduleList;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence};

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

/// Independently current facing and tile policy supplied by the caller's world owner.
pub(crate) trait DefensePresentationWorldReader {
    fn current_facing(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing>;

    fn tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        x: i32,
        y: i32,
        floor: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
}

#[derive(Clone)]
pub(crate) struct DefensePresentationSource {
    key: String,
    index: usize,
    pub(crate) ability: Ref,
    pin: [u8; 32],
    range: u16,
    magnitude: Option<crate::content::ProjectV2Magnitude>,
    details: Box<ProjectV2AbilityDetails>,
}
impl DefensePresentationSource {
    pub(crate) fn from_native(
        creature: &Ref,
        index: usize,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        pin: [u8; 32],
    ) -> Result<Self, AttackError> {
        if creature.family != ProjectV2Family::Creature || index >= 8 {
            return Err(AttackError::InvalidSource);
        }
        let record=records.iter().find(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..}if identity.key==creature.key&&identity.revision==creature.revision)).ok_or(AttackError::InvalidSource)?;
        let ProjectReferenceRecord::Creature { behavior, .. } = record else {
            unreachable!()
        };
        let mut map = std::collections::BTreeMap::new();
        for profile in profiles {
            if map.insert(&profile.target, &profile.data).is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        let bref = Ref {
            family: ProjectV2Family::Behavior,
            key: behavior.key.clone(),
            revision: behavior.revision.clone(),
        };
        let Some(Data::Behavior(b)) = map.get(&bref).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let entry = b.defenses.get(index).ok_or(AttackError::InvalidSource)?;
        let Some(Data::Ability(a)) = map.get(&entry.ability).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let d = a.details.as_ref().ok_or(AttackError::InvalidSource)?;
        if d.kind!=ProjectV2AbilityKind::Spell||d.needs_target||d.effects.is_empty()||d.windup.is_some()||d.encounter.is_some()||d.chain.is_some()||!d.variants.is_empty()||!d.effects.iter().all(|e|matches!(e,ProjectV2AbilityEffect::Inline(i)if matches!(i.operation,ProjectV2InlineEffectOperation::PresentationOnly))){return Err(AttackError::UnsupportedShape)}
        let Some(Data::Creature(c)) = map.get(creature).copied() else {
            return Err(AttackError::InvalidSource);
        };
        if !c.abilities.contains(&entry.ability)||!records.iter().any(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.family=="Ability"&&identity.key==entry.ability.key&&identity.revision==entry.ability.revision)){return Err(AttackError::InvalidSource)}
        Ok(Self {
            key: creature.key.clone(),
            index,
            ability: entry.ability.clone(),
            pin,
            range: entry.range_tiles.unwrap_or(d.range_tiles),
            magnitude: entry.magnitude,
            details: d.clone(),
        })
    }
    pub(crate) fn produce(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        proposal: &ProfileAbilityProposal,
        reader: &mut (impl DefensePresentationWorldReader + ?Sized),
    ) -> Result<SourcePresentationEvent, AttackError> {
        let b = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if self.pin != runtime.content_pin().server_artifact_digest() {
            return Err(AttackError::ContentChanged);
        }
        if !runtime.matches_live_creature_identity(proposal.issuer, self.key.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        if proposal.list != ScheduleList::Defence
            || proposal.entry_index != self.index
            || proposal.ability != self.ability
            || proposal.range_tiles != self.range
            || proposal.magnitude != self.magnitude
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(proposal.issuer));
        let tail = proposal
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (seq, suffix) = tail.split_once(':').ok_or(AttackError::InvalidPlan)?;
        if seq.parse::<u64>().is_err() || suffix != format!("defence:{}", self.index) {
            return Err(AttackError::InvalidPlan);
        }
        let current = runtime
            .read_actor_position(proposal.issuer)
            .map_err(|_| AttackError::StaleIssuer)?
            .position();
        let facing = if self.details.needs_direction {
            reader
                .current_facing(runtime, proposal.issuer, stamp)
                .ok_or(AttackError::MissingCombatFacts)?
        } else {
            Facing::North
        };
        let offsets = match &self.details.area {
            Some(area) => {
                geometry::offsets(area, facing).map_err(|_| AttackError::UnsupportedShape)?
            }
            None => vec![(0, 0)],
        };
        let mut tiles = Vec::new();
        tiles
            .try_reserve_exact(offsets.len())
            .map_err(|_| AttackError::LedgerFull)?;
        for (dx, dy) in offsets {
            let x = current
                .x
                .checked_add(dx)
                .ok_or(AttackError::NumericOverflow)?;
            let y = current
                .y
                .checked_add(dy)
                .ok_or(AttackError::NumericOverflow)?;
            match reader.tile_allowed(runtime, proposal.issuer, x, y, current.floor, stamp) {
                None => return Err(AttackError::MissingCombatFacts),
                Some(false) => {}
                Some(true) => tiles.push((x, y, current.floor)),
            }
        }
        Ok(SourcePresentationEvent {
            issuer: proposal.issuer,
            ability: self.ability.clone(),
            occurrence: proposal.occurrence.id().as_str().into(),
            content_digest: self.pin,
            stamp,
            tiles,
            source: self.details.clone(),
            qualification: "NATIVE_SELF_PRESENTATION_PRODUCER_NETWORK_PENDING",
        })
    }
}
fn hex(a: ExactActorRef) -> String {
    a.placement_identity()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
