//! Source-native self-position presentation producer. No HP, MP or network mutation.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityDetails, ProjectV2AbilityEffect, ProjectV2AbilityKind,
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2InlineEffectOperation,
};
use crate::creature_attack_geometry::{self as geometry, Facing};
use crate::creature_auto_attack::AttackError;
use crate::creature_damage_spell::{SourcePresentationEvent, SpellWorldReader};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence};
#[derive(Clone)]
pub(crate) struct DefensePresentationSource {
    key: String,
    index: usize,
    ability: Ref,
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
        reader: &mut dyn SpellWorldReader,
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_think::{
        ThinkSequenceTracker,
        profile_schedule::{ProfileScheduleInput, ProfileScheduleState},
    };
    use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("creature_auto_attack_test_data.json")).unwrap();
        (
            serde_json::from_value(v["records"].clone()).unwrap(),
            serde_json::from_value(v["authoring_profiles"].clone()).unwrap(),
        )
    }
    struct TilePolicy {
        missing: bool,
    }
    impl SpellWorldReader for TilePolicy {
        fn current_players(
            &mut self,
            _: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, crate::foundation::GameSessionId)>> {
            None
        }
        fn current_facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<Facing> {
            None
        }
        fn tile_allowed(
            &mut self,
            r: &ChannelRuntimeV1,
            a: ExactActorRef,
            _: i32,
            _: i32,
            _: i16,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.read_actor_position(a).ok()?;
            (!self.missing).then_some(true)
        }
        fn combat(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: crate::foundation::GameSessionId,
            _: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<crate::creature_damage_spell::SpellCombatFacts> {
            None
        }
    }
    #[test]
    fn all_twelve_actual_native_defense_presentation_sources_qualify() {
        let (records, profiles) = native();
        let packet: serde_json::Value =
            serde_json::from_str(include_str!("creature_defense_presentation_test_data.json"))
                .unwrap();
        let mut count = 0;
        for row in packet["rows"].as_array().unwrap() {
            let c: Ref = serde_json::from_value(row["creature"].clone()).unwrap();
            DefensePresentationSource::from_native(
                &c,
                row["index"].as_u64().unwrap() as usize,
                &records,
                &profiles,
                [1; 32],
            )
            .unwrap();
            count += 1;
        }
        assert_eq!(count, 12);
    }
    #[test]
    fn actual_self_defense_schedule_emits_at_caster_without_player_target_or_hp_mp() {
        let (mut r, player, session) = runtime_with_player(0x58);
        r.initialize_movement_test_position(
            player,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .unwrap();
        let (records, profiles) = native();
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.blood_hand".into(),
            revision: "definition-r1".into(),
        };
        let hp = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Creature(c) if p.target == creature => c.health,
                _ => None,
            })
            .unwrap();
        let actor = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 103,
                    y: 104,
                    floor: 7,
                },
                &creature.key,
                hp as i64,
            )
            .unwrap();
        let source = DefensePresentationSource::from_native(
            &creature,
            1,
            &records,
            &profiles,
            r.content_pin().server_artifact_digest(),
        )
        .unwrap();
        let behavior = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b) if p.target.key == "oteryn:behavior.creature.blood_hand" => {
                    Some(b)
                }
                _ => None,
            })
            .unwrap();
        let abilities = profiles
            .iter()
            .filter_map(|p| match &p.data {
                Data::Ability(a) => Some((p.target.clone(), a.clone())),
                _ => None,
            })
            .collect();
        let mut schedule = ProfileScheduleState::new(actor);
        let mut tracker = ThinkSequenceTracker::new();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
            r.content_pin().server_artifact_digest(),
        );
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .unwrap();
        let mut found = None;
        for _ in 0..256 {
            let plan = schedule
                .prepare_with_summons(
                    ProfileScheduleInput {
                        occurrence: tracker.next_occurrence(actor),
                        behavior,
                        abilities: &abilities,
                        target: None,
                        revisions: &revisions,
                        root: &root,
                    },
                    None,
                )
                .unwrap();
            if let Some(p) = plan
                .proposals
                .into_iter()
                .find(|p| p.ability == source.ability)
            {
                found = Some(p);
                break;
            }
        }
        let p = found.expect("actual defense schedule supports no player target");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                player,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        let b = r.binding();
        let (mut f, stamp) = crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .unwrap();
        let mut policy = TilePolicy { missing: false };
        let event = source.produce(&r, &f, stamp, &p, &mut policy).unwrap();
        assert!(event.tiles.contains(&(103, 104, 7)));
        assert!(event.source.impact_cue.is_some());
        let (revision, vitals) =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session)
                .unwrap();
        assert_eq!((revision, vitals.health, vitals.mana), (1, 185, 90));
        policy.missing = true;
        assert_eq!(
            source.produce(&r, &f, stamp, &p, &mut policy),
            Err(AttackError::MissingCombatFacts)
        );
        f.apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).unwrap())
            .unwrap();
        assert_eq!(
            source.produce(&r, &f, stamp, &p, &mut policy),
            Err(AttackError::StaleOwner)
        );
    }
}
