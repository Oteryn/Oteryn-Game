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
const CREATURE: &str = "oteryn:creature.smelly_cheese";
const PARENT: &str = "oteryn:ability.creature.smelly_cheese.berserk_bridge";
#[derive(Clone)]
pub(crate) struct SourceSmelly {
    ability: Ref,
    content: [u8; 32],
    presentation: Box<crate::content::ProjectV2AbilityDetails>,
}
#[derive(serde::Deserialize)]
struct Witness {
    profiles: BTreeMap<String, Data>,
    records: Vec<Record>,
}
impl SourceSmelly {
    pub(crate) fn ability(&self) -> &Ref {
        &self.ability
    }
    pub(crate) fn qualify(
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
        let expected: Witness = serde_json::from_str(include_str!("smelly_encounter_source.json"))
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
struct Entry {
    actor: ExactActorRef,
    source: SourceSmelly,
    timers: SmellyCheeseTimers,
}
#[derive(Default)]
pub(crate) struct SmellyEncounterOwner {
    entries: Vec<Entry>,
}
#[derive(Debug)]
// Presentation is returned by value from source-qualified owner dispatch; boxing the result would add a heap allocation after committed owner state. Preserve the prepublication allocation boundary.
#[allow(clippy::large_enum_variant)]
pub(crate) enum SmellyEffect {
    Damage(Vec<(ExactActorRef, Result<PlayerDamageReceipt, AttackError>)>),
    Presentation(SourcePresentationEvent),
}
impl SmellyEncounterOwner {
    pub(crate) fn schedule(
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
    pub(crate) fn drain(
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
fn hex(v: [u8; 16]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::creature_auto_attack::AttackFacts;
    use crate::creature_damage_spell::SpellCombatFacts;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{GameSessionId, MovementLocalPosition};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    fn setup() -> (
        ChannelRuntimeV1,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
        ChannelSpellStates,
        ProjectV2Draft,
        SourceSmelly,
    ) {
        let (mut r, p, session) = runtime_with_player(0x75);
        r.initialize_movement_test_position(
            p,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
        )
        .expect("smelly_encounter_dispatch.rs:tests:485: qualified fixture operation must succeed");
        let c = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                CREATURE,
                5000,
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:496: qualified fixture operation must succeed",
            );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                p,
                session,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 2000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:510: qualified fixture operation must succeed",
            );
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "smelly_encounter_native_fixture.json"
        ))
        .expect("smelly_encounter_dispatch.rs:tests:512: qualified fixture operation must succeed");
        let d = ProjectV2Draft {
            core: crate::content::ProjectDraft {
                project_revision: "g4-npc-wave-a-r9".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "PENDING".into(),
                world_id: hex(*r.binding().world_id().as_bytes()),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: serde_json::from_value(v["records"].clone()).expect("smelly_encounter_dispatch.rs:tests:521: qualified fixture operation must succeed"),
                imports: vec![],
                metadata: vec![],
            },
            state: crate::content::ProjectV2State {
                declarations: serde_json::from_value(v["declarations"].clone()).expect("smelly_encounter_dispatch.rs:tests:526: qualified fixture operation must succeed"),
                authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                    .expect("smelly_encounter_dispatch.rs:tests:528: qualified fixture operation must succeed"),
                source_identity_bindings: serde_json::from_value(
                    v["source_identity_bindings"].clone(),
                )
                .expect("smelly_encounter_dispatch.rs:tests:532: qualified fixture operation must succeed"),
                sources: serde_json::from_value(v["sources"].clone()).expect("smelly_encounter_dispatch.rs:tests:533: qualified fixture operation must succeed"),
                ..Default::default()
            },
        };
        let s = SourceSmelly::qualify(
            &d,
            &Ref {
                family: Family::Creature,
                key: CREATURE.into(),
                revision: "definition-r1".into(),
            },
            r.content_pin().server_artifact_digest(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:546: qualified fixture operation must succeed");
        (r, p, session, c, states, d, s)
    }
    fn proposal(c: ExactActorRef, s: &SourceSmelly, seq: u64) -> ProfileAbilityProposal {
        let atom = format!("actor:{}", hex(c.placement_identity()));
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("smelly_encounter_dispatch.rs:tests:558: qualified fixture operation must succeed");
        ProfileAbilityProposal {
            issuer: c,
            target: c,
            ability: s.ability.clone(),
            list: ScheduleList::Attack,
            entry_index: 0,
            magnitude: None,
            range_tiles: 0,
            occurrence: crate::ability::AbilityOccurrence::new(
                &format!("ai-profile:{}:{seq}:attack:0", hex(c.placement_identity())),
                revisions,
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:571: qualified fixture operation must succeed",
            ),
            intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).expect(
                "smelly_encounter_dispatch.rs:tests:572: qualified fixture operation must succeed",
            ),
        }
    }
    struct Reader {
        p: ExactActorRef,
        session: GameSessionId,
        available: bool,
        pz: bool,
        multiplier: u32,
    }
    impl SpellWorldReader for Reader {
        fn current_players(
            &mut self,
            _: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            Some(vec![(self.p, self.session)])
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
            self.available.then_some(true)
        }
        fn combat(
            &mut self,
            _: &ChannelRuntimeV1,
            c: ExactActorRef,
            p: ExactActorRef,
            session: GameSessionId,
            element: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            assert_eq!(element, Some("earth"));
            self.available.then_some(SpellCombatFacts {
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
                multiplier_ppm: self.multiplier,
                immune: false,
                condition_policy: None,
            })
        }
    }
    fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
        let b = r.binding();
        crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:645: qualified fixture operation must succeed")
    }
    #[test]
    fn actual_source_zero_occurrence_damage500_earth_policy_and_presentation700() {
        let (mut r, p, session, c, mut states, _, s) = setup();
        let (f, stamp) = fence(&r);
        let mut o = SmellyEncounterOwner::default();
        let proposal = proposal(c, &s, 0);
        let mut reader = Reader {
            p,
            session,
            available: true,
            pz: false,
            multiplier: 500000,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        assert!(
            o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:663: qualified fixture operation must succeed")
        );
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:667: qualified fixture operation must succeed")
        );
        assert!(
            o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
                .expect("smelly_encounter_dispatch.rs:tests:671: qualified fixture operation must succeed")
                .is_empty()
        );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:677: qualified fixture operation must succeed",
            );
        let damage = match hits[0].1.as_ref().expect(
            "smelly_encounter_dispatch.rs:tests:678: qualified fixture operation must succeed",
        ) {
            SmellyEffect::Damage(damage) => Some(damage),
            _ => None,
        }
        .expect("source500ms pulse must be Damage");
        assert_eq!(damage.len(), 1);
        let receipt = damage[0].1.as_ref().expect(
            "smelly_encounter_dispatch.rs:tests:682: qualified fixture operation must succeed",
        );
        assert!((200..=400).contains(&receipt.applied));
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:686: qualified fixture operation must succeed")
        );
        assert!(
            o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
                .expect("smelly_encounter_dispatch.rs:tests:690: qualified fixture operation must succeed")
                .is_empty()
        );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(700000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:696: qualified fixture operation must succeed",
            );
        assert!(matches!(&hits[0].1,Ok(SmellyEffect::Presentation(event))if event.tiles.len()==9));
    }
    #[test]
    fn source_substitution_missing_map_pz_dead_caster_refuse_actual_hp() {
        let (mut r, p, session, c, mut states, mut d, s) = setup();
        let (f, stamp) = fence(&r);
        let mut o = SmellyEncounterOwner::default();
        let mut reader = Reader {
            p,
            session,
            available: false,
            pz: false,
            multiplier: 1000000,
        };
        let now = SemanticTimeMicros::from_micros(0);
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
            .expect(
                "smelly_encounter_dispatch.rs:tests:715: qualified fixture operation must succeed",
            );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:719: qualified fixture operation must succeed",
            );
        assert!(matches!(hits[0].1, Err(AttackError::MissingCombatFacts)));
        d.state.source_identity_bindings[0].source_revision = "wrong-pin".into();
        assert!(
            SourceSmelly::qualify(
                &d,
                &Ref {
                    family: Family::Creature,
                    key: CREATURE.into(),
                    revision: "definition-r1".into()
                },
                s.content
            )
            .is_err()
        );
        reader.available = true;
        reader.pz = true;
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(700000));
        o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:738: qualified fixture operation must succeed",
            );
        o.schedule(
            &r,
            &f,
            stamp,
            &s,
            &proposal(c, &s, 1),
            SemanticTimeMicros::from_micros(2000000),
        )
        .expect("smelly_encounter_dispatch.rs:tests:747: qualified fixture operation must succeed");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(2500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:751: qualified fixture operation must succeed",
            );
        assert!(
            matches!(&hits.iter().find(|(_,v)|matches!(v,Ok(SmellyEffect::Damage(_)))).expect("smelly_encounter_dispatch.rs:tests:753: qualified fixture operation must succeed").1,Ok(SmellyEffect::Damage(v))if v.is_empty())
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before
        );
        r.remove_test_actor(c).expect(
            "smelly_encounter_dispatch.rs:tests:759: qualified fixture operation must succeed",
        );
        assert!(
            o.schedule(
                &r,
                &f,
                stamp,
                &s,
                &proposal(c, &s, 2),
                SemanticTimeMicros::from_micros(4000000)
            )
            .is_err()
        );
    }
    #[test]
    fn smelly_duplicate_source_identity_and_profiles_refuse_qualification() {
        let (r, _, _, _, _, d, s) = setup();
        let caster = Ref {
            family: Family::Creature,
            key: CREATURE.into(),
            revision: "definition-r1".into(),
        };
        let mut duplicate = d.clone();
        let b = duplicate
            .state
            .source_identity_bindings
            .iter()
            .find(|b| b.target.key == CREATURE)
            .expect(
                "smelly_encounter_dispatch.rs:tests:786: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.state.source_identity_bindings.push(b);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        let mut duplicate = d.clone();
        let p = duplicate
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == PARENT)
            .expect(
                "smelly_encounter_dispatch.rs:tests:799: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.state.authoring_profiles.push(p);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        let mut duplicate = d.clone();
        let record = duplicate
            .core
            .records
            .iter()
            .find(|r| matches!(r,Record::Ability{identity,..}if identity.key==PARENT))
            .expect(
                "smelly_encounter_dispatch.rs:tests:812: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.core.records.push(record);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        for family in [Family::Creature, Family::Encounter] {
            let mut forged = d.clone();
            let b = forged
                .state
                .source_identity_bindings
                .iter_mut()
                .find(|b| b.target.family == family)
                .expect("smelly_encounter_dispatch.rs:tests:826: qualified fixture operation must succeed");
            b.identity_namespace = "crystalserver/forged".into();
            assert!(matches!(
                SourceSmelly::qualify(&forged, &caster, s.content),
                Err(AttackError::InvalidSource)
            ));
            let mut forged = d.clone();
            let b = forged
                .state
                .source_identity_bindings
                .iter_mut()
                .find(|b| b.target.family == family)
                .expect("smelly_encounter_dispatch.rs:tests:838: qualified fixture operation must succeed");
            b.external_id = "foreign-source-file-or-encounter".into();
            assert!(matches!(
                SourceSmelly::qualify(&forged, &caster, s.content),
                Err(AttackError::InvalidSource)
            ));
        }
        assert!(
            SourceSmelly::qualify(&d, &caster, r.content_pin().server_artifact_digest()).is_ok()
        );
    }
    #[test]
    fn smelly_foreign_owner_and_forged_proposal_do_not_mutate_or_consume_real_cast() {
        let (mut r, p, session, c, mut states, _, s) = setup();
        let (f, stamp) = fence(&r);
        let b = r.binding();
        let mut world = *b.world_id().as_bytes();
        world[15] ^= 1;
        let (foreign, _) = crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(
                crate::foundation::WorldId::decode(&world).expect("smelly_encounter_dispatch.rs:tests:858: qualified fixture operation must succeed"),
                b.channel_id(),
            ),
            b.scope_generation(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:863: qualified fixture operation must succeed");
        let mut o = SmellyEncounterOwner::default();
        let now = SemanticTimeMicros::from_micros(0);
        assert!(matches!(
            o.schedule(&r, &foreign, stamp, &s, &proposal(c, &s, 0), now),
            Err(AttackError::StaleOwner)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        forged.target = p;
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        let other = format!("actor:{}", hex(p.placement_identity()));
        forged.intent = crate::ability::AiAbilityAdapter::normalize(&other, &[&other]).expect(
            "smelly_encounter_dispatch.rs:tests:880: qualified fixture operation must succeed",
        );
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        let atom = format!("actor:{}", hex(c.placement_identity()));
        forged.intent = crate::ability::AiAbilityAdapter::normalize(&atom, &[&other]).expect(
            "smelly_encounter_dispatch.rs:tests:888: qualified fixture operation must succeed",
        );
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        assert!(
            o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
                .expect("smelly_encounter_dispatch.rs:tests:896: qualified fixture operation must succeed")
        );
        assert_eq!(o.entries.len(), 1);
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        let mut reader = Reader {
            p,
            session,
            available: true,
            pz: false,
            multiplier: 1000000,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:911: qualified fixture operation must succeed",
            );
        assert!(matches!(&hits[0].1,Ok(SmellyEffect::Damage(v))if v.len()==1&&v[0].1.is_ok()));
        assert_ne!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before
        );
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
                .expect("smelly_encounter_dispatch.rs:tests:919: qualified fixture operation must succeed")
        );
    }
}

pub(crate) type SmellyPulseResults =
    Result<Vec<(ExactActorRef, Result<SmellyEffect, AttackError>)>, AttackError>;
