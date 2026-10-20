//! Sixteen fixed source callback adapters sharing the existing native physical lifecycle.
//! Immutable Source specs do not grant current map/scope authority. No Encounter interpreter.
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
    ProjectV2SourceIdentityBinding,
};
use crate::creature_auto_attack::AttackError;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::durability::creature_schedule::ScheduleList;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, NativeEncounterSpawnSpec,
    NativeSummonAdmissionSpec, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration,
    ScopeRuntimeFence,
};
use crate::foundation::{ConditionType, DotElement};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::collections::BTreeMap;
#[derive(serde::Deserialize)]
pub(crate) struct Witness {
    pub(crate) cases: Vec<Case>,
    profiles: BTreeMap<String, Data>,
    records: Vec<Record>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    targets: Vec<String>,
}
#[derive(serde::Deserialize)]
pub(crate) struct Case {
    pub(crate) caster: String,
    parent: String,
    pub(crate) index: usize,
    pub(crate) phase: String,
    pub(crate) encounter: String,
    entry: serde_json::Value,
}
#[derive(Debug, Clone)]
pub(crate) enum SourceSpawnPlacement {
    Subject,
    Offset(u32),
    Relative(i32, i32),
    Anchor {
        key: String,
        definition: serde_json::Value,
    },
    Room {
        key: String,
        definition: serde_json::Value,
    },
}
/// Current map-owner translation/placement/terrain; None always refuses, never allow-default.
pub(crate) trait SourceSpawnFacts {
    fn source_spawn_position(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        placement: &SourceSpawnPlacement,
        draw: u64,
        ordinal: usize,
        stamp: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition>;
    fn source_spawn_tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        position: MovementLocalPosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn source_spawn_anchor_contains(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        anchor: &serde_json::Value,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Gaz,
    GeneratorRelease,
    Tenebris,
    Mazoran,
    Thorn,
    Plagirath,
    Maxxen,
    Razzagorn,
    Shulgrax,
    Soulcatcher,
    Blazing,
    Destruction,
    Freezing,
    Hunger,
    Rage,
    Devourer,
}
#[derive(Clone)]
pub(crate) struct Child {
    pub(crate) creature: Ref,
    pub(crate) health: i64,
}
#[derive(Clone)]
pub(crate) struct SourceCallbackSpawn {
    pub(crate) kind: Kind,
    pub(crate) creature: String,
    pub(crate) parent: Ref,
    pub(crate) index: usize,
    pub(crate) list: ScheduleList,
    pub(crate) content: [u8; 32],
    pub(crate) children: Vec<Child>,
    anchors: Vec<serde_json::Value>,
    pub(crate) range: u16,
    pub(crate) magnitude: Option<crate::content::ProjectV2Magnitude>,
}
fn kind(c: &str, p: &str) -> Option<Kind> {
    Some(match (c, p) {
        ("gaz_haragoth", "gaz_haragoth_summon") => Kind::Gaz,
        ("glooth_generator", "glooth_generator_summon") => Kind::GeneratorRelease,
        ("lady_tenebris", "tenebris_summon") => Kind::Tenebris,
        ("mazoran", "mazoran_fire") => Kind::Mazoran,
        ("mounted_thorn_knight", "thorn_summon") => Kind::Thorn,
        ("plagirath", "plagirath_summon") => Kind::Plagirath,
        ("professor_maxxen", "generator") => Kind::Maxxen,
        ("razzagorn", "razzagorn_summon") => Kind::Razzagorn,
        ("shulgrax", "shulgrax_summon") => Kind::Shulgrax,
        ("soulcatcher", "soulcatcher_summon") => Kind::Soulcatcher,
        ("the_blazing_time_guardian", "time_guardian_lost_time") => Kind::Blazing,
        ("the_destruction", "destruction_summon") => Kind::Destruction,
        ("the_freezing_time_guardian", "time_guardian_lost_time") => Kind::Freezing,
        ("the_hunger", "hunger_summon") => Kind::Hunger,
        ("the_rage", "rage_summon") => Kind::Rage,
        ("world_devourer", "devourer_summon") => Kind::Devourer,
        _ => return None,
    })
}
pub(crate) fn fixture() -> Result<Witness, AttackError> {
    serde_json::from_str(include_str!("source_callback_spawn_fixture.json"))
        .map_err(|_| AttackError::InvalidSource)
}
impl SourceCallbackSpawn {
    pub(crate) fn ability(&self) -> &Ref {
        &self.parent
    }
    pub(crate) fn qualify(
        d: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        list: ScheduleList,
        content: [u8; 32],
    ) -> Result<Option<Self>, AttackError> {
        let Some(name) = caster.key.strip_prefix("oteryn:creature.") else {
            return Ok(None);
        };
        if !matches!(
            name,
            "gaz_haragoth"
                | "glooth_generator"
                | "lady_tenebris"
                | "mazoran"
                | "mounted_thorn_knight"
                | "plagirath"
                | "professor_maxxen"
                | "razzagorn"
                | "shulgrax"
                | "soulcatcher"
                | "the_blazing_time_guardian"
                | "the_destruction"
                | "the_freezing_time_guardian"
                | "the_hunger"
                | "the_rage"
                | "world_devourer"
        ) {
            return Ok(None);
        }
        let w = fixture()?;
        let phase = if list == ScheduleList::Attack {
            "attacks"
        } else {
            "defenses"
        };
        let Some(c) = w
            .cases
            .iter()
            .find(|c| c.caster == caster.key && c.index == index && c.phase == phase)
        else {
            return Ok(None);
        };
        if caster.family != Family::Creature || caster.revision != "definition-r1" {
            return Err(AttackError::InvalidSource);
        }
        for key in [&c.encounter, &c.parent] {
            let data = w.profiles.get(key).ok_or(AttackError::InvalidSource)?;
            if !d.state.authoring_profiles.iter().any(|p| {
                p.target.key == *key && p.target.revision == "definition-r1" && p.data == *data
            }) {
                return Err(AttackError::InvalidSource);
            }
        }
        for record in w
            .records
            .iter()
            .filter(|r| matches!(r,Record::Ability{identity,..}if identity.key==c.parent))
        {
            if !d.core.records.contains(record) {
                return Err(AttackError::InvalidSource);
            }
        }
        let binding = w
            .bindings
            .iter()
            .find(|b| b.target.key == c.encounter)
            .ok_or(AttackError::InvalidSource)?;
        if !d.state.source_identity_bindings.contains(binding)
            || !d
                .state
                .sources
                .iter()
                .any(|s| s.key == binding.source_key && s.revision == binding.source_revision)
        {
            return Err(AttackError::InvalidSource);
        }
        let Some(Record::Creature{behavior,..})=d.core.records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(AttackError::InvalidSource)};
        let Some(Data::Behavior(b)) = d
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        let entries = if list == ScheduleList::Attack {
            &b.attacks
        } else {
            &b.defenses
        };
        let entry = entries.get(index).ok_or(AttackError::InvalidSource)?;
        if serde_json::to_value(entry).map_err(|_| AttackError::InvalidSource)? != c.entry {
            return Err(AttackError::InvalidSource);
        }
        let kind = kind(
            name,
            c.parent
                .strip_prefix("oteryn:ability.spell.")
                .ok_or(AttackError::InvalidSource)?,
        )
        .ok_or(AttackError::InvalidSource)?;
        let mut children = Vec::new();
        for name in child_names(kind) {
            let key = format!("oteryn:creature.{name}");
            let Some(p) = d
                .state
                .authoring_profiles
                .iter()
                .find(|p| p.target.family == Family::Creature && p.target.key == key)
            else {
                return Err(AttackError::InvalidSource);
            };
            let Data::Creature(cp) = &p.data else {
                return Err(AttackError::InvalidSource);
            };
            let health = i64::try_from(cp.health.ok_or(AttackError::InvalidSource)?)
                .map_err(|_| AttackError::NumericOverflow)?;
            if health<=0||!d.core.records.iter().any(|r|matches!(r,Record::Creature{identity,..}if identity.key==key&&identity.revision==p.target.revision)){return Err(AttackError::InvalidSource)}
            children.push(Child {
                creature: p.target.clone(),
                health,
            });
        }
        let e = serde_json::to_value(
            w.profiles
                .get(&c.encounter)
                .ok_or(AttackError::InvalidSource)?,
        )
        .map_err(|_| AttackError::InvalidSource)?;
        let anchors = e["profile"]["details"]["anchors"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        Ok(Some(Self {
            kind,
            creature: caster.key.clone(),
            parent: entry.ability.clone(),
            index,
            list,
            content,
            children,
            anchors,
            range: entry.range_tiles.unwrap_or(0),
            magnitude: entry.magnitude,
        }))
    }
    fn child(&self, name: &str) -> Result<&Child, AttackError> {
        self.children
            .iter()
            .find(|c| c.creature.key == format!("oteryn:creature.{name}"))
            .ok_or(AttackError::InvalidSource)
    }
    fn anchor(&self, key: &str) -> Result<serde_json::Value, AttackError> {
        self.anchors
            .iter()
            .find(|a| a["key"].as_str() == Some(key))
            .cloned()
            .ok_or(AttackError::InvalidSource)
    }
    fn location(&self, key: &str, room: bool) -> Result<SourceSpawnPlacement, AttackError> {
        let definition = self.anchor(key)?;
        Ok(if room {
            SourceSpawnPlacement::Room {
                key: key.into(),
                definition,
            }
        } else {
            SourceSpawnPlacement::Anchor {
                key: key.into(),
                definition,
            }
        })
    }
    fn current(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> bool {
        r.content_pin().server_artifact_digest() == self.content
            && r.matches_live_creature_identity(a, self.creature.as_bytes())
    }
    fn sequence(&self, p: &ProfileAbilityProposal) -> Result<u64, AttackError> {
        if p.list != self.list
            || p.entry_index != self.index
            || p.ability != self.parent
            || p.magnitude != self.magnitude
            || p.range_tiles != self.range
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(p.issuer.placement_identity()));
        let suffix = p
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (n, tail) = suffix.split_once(':').ok_or(AttackError::InvalidPlan)?;
        let phase = if self.list == ScheduleList::Attack {
            "attack"
        } else {
            "defence"
        };
        if tail != format!("{phase}:{}", self.index) {
            return Err(AttackError::InvalidPlan);
        }
        n.parse().map_err(|_| AttackError::InvalidPlan)
    }
}
/// Child membership comes from exact source fixture, not a new identity or cloned Draft.
pub(crate) fn qualified_native_children(d: &ProjectV2Draft) -> Result<Vec<Ref>, AttackError> {
    let w = fixture()?;
    let mut refs = std::collections::BTreeSet::new();
    for c in &w.cases {
        if !d
            .state
            .authoring_profiles
            .iter()
            .any(|p| p.target.key == c.parent)
        {
            continue;
        }
        for key in [&c.encounter, &c.parent] {
            let expected = w.profiles.get(key).ok_or(AttackError::InvalidSource)?;
            if !d
                .state
                .authoring_profiles
                .iter()
                .any(|p| p.target.key == *key && p.data == *expected)
            {
                return Err(AttackError::InvalidSource);
            }
        }
        let binding = w
            .bindings
            .iter()
            .find(|b| b.target.key == c.encounter)
            .ok_or(AttackError::InvalidSource)?;
        if !d.state.source_identity_bindings.contains(binding)
            || !d
                .state
                .sources
                .iter()
                .any(|s| s.key == binding.source_key && s.revision == binding.source_revision)
        {
            return Err(AttackError::InvalidSource);
        }
        let k = kind(
            c.caster
                .strip_prefix("oteryn:creature.")
                .ok_or(AttackError::InvalidSource)?,
            c.parent
                .strip_prefix("oteryn:ability.spell.")
                .ok_or(AttackError::InvalidSource)?,
        )
        .ok_or(AttackError::InvalidSource)?;
        for name in child_names(k) {
            let key = format!("oteryn:creature.{name}");
            let p = d
                .state
                .authoring_profiles
                .iter()
                .find(|p| p.target.family == Family::Creature && p.target.key == key)
                .ok_or(AttackError::InvalidSource)?;
            refs.insert(p.target.clone());
        }
    }
    Ok(refs.into_iter().collect())
}
fn child_names(k: Kind) -> &'static [&'static str] {
    match k {
        Kind::Gaz => &["minion_of_gaz_haragoth"],
        Kind::GeneratorRelease => &["energy_pulse"],
        Kind::Tenebris => &["shadow_fiend"],
        Kind::Mazoran => &["rage_of_mazoran"],
        Kind::Thorn => &["thorn_minion"],
        Kind::Plagirath => &["disgusting_ooze"],
        Kind::Maxxen => &["glooth_generator"],
        Kind::Razzagorn => &["demon"],
        Kind::Shulgrax => &["sin_devourer", "damned_soul"],
        Kind::Soulcatcher => &["corrupted_soul"],
        Kind::Blazing | Kind::Freezing => &["lost_time", "time_waster"],
        Kind::Destruction => &["disruption"],
        Kind::Hunger => &["greed"],
        Kind::Rage => &["frenzy"],
        Kind::Devourer => &["greed", "frenzy", "disruption"],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Timer;
impl TimerFamily for Timer {
    fn registered_maximum(self) -> usize {
        64
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Work(u64);
enum Admission {
    Intrinsic(NativeSummonAdmissionSpec),
    Masterless(NativeEncounterSpawnSpec),
}
pub(crate) struct Pending {
    token: u64,
    source: SourceCallbackSpawn,
    proposal: ProfileAbilityProposal,
}
pub(crate) struct Memo {
    actor: ExactActorRef,
    ability: Ref,
    sequence: u64,
    at: u64,
    proposal: ProfileAbilityProposal,
    counter: u32,
    next_allowed: u64,
}
#[derive(Debug)]
pub(crate) struct SpawnChildReceipt {
    pub(crate) creature: Ref,
    pub(crate) result: Result<ExactActorRef, AttackError>,
    pub(crate) intrinsic: bool,
    pub(crate) cleanup: Option<Result<(), crate::foundation::CarrierError>>,
}
#[derive(Debug)]
pub(crate) struct CallbackSpawnReceipt {
    pub(crate) actor: ExactActorRef,
    pub(crate) ability: Ref,
    pub(crate) requested: u32,
    pub(crate) children: Vec<SpawnChildReceipt>,
    pub(crate) remove: Option<Result<(), crate::foundation::CarrierError>>,
    pub(crate) flags: &'static [&'static str],
}
pub(crate) struct CallbackSpawnOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Timer, Work>,
    pub(crate) pending: Vec<Pending>,
    pub(crate) memos: Vec<Memo>,
    next: u64,
    speech: Vec<crate::source_callback_cast::CallbackSpeech>,
    speech_errors: Vec<(ExactActorRef, AttackError)>,
}
impl CallbackSpawnOwner {
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
            [(
                Timer,
                FamilyPolicy {
                    max_pending: 64,
                    catch_up: CatchUpPolicy::DeadlineState,
                },
            )],
        )
        .map_err(|_| AttackError::InvalidPlan)?;
        Ok(Self {
            scope,
            generation,
            timers,
            pending: Vec::new(),
            memos: Vec::new(),
            next: 1,
            speech: Vec::new(),
            speech_errors: Vec::new(),
        })
    }
    pub(crate) fn take_source_speech(
        &mut self,
    ) -> Vec<crate::source_callback_cast::CallbackSpeech> {
        std::mem::take(&mut self.speech)
    }
    pub(crate) fn take_speech_errors(&mut self) -> Vec<(ExactActorRef, AttackError)> {
        std::mem::take(&mut self.speech_errors)
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
    pub(crate) fn schedule(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &SourceCallbackSpawn,
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
        for c in &self.pending {
            if !c.source.current(r, c.proposal.issuer) {
                self.timers.cancel(Timer, Work(c.token));
            }
        }
        self.pending
            .retain(|c| c.source.current(r, c.proposal.issuer));
        self.memos.retain(|m| r.contains_live_creature(m.actor));
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
        if self.pending.len() >= 64 || self.memos.len() >= 1024 {
            return Err(AttackError::LedgerFull);
        }
        self.pending
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.memos
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        let token = self.next;
        let next = token.checked_add(1).ok_or(AttackError::NumericOverflow)?;
        let delay = if s.kind == Kind::GeneratorRelease {
            14000000
        } else {
            0
        };
        let due = now
            .get()
            .checked_add(delay)
            .ok_or(AttackError::NumericOverflow)?;
        self.timers
            .schedule(
                f,
                stamp,
                Timer,
                Work(token),
                Some(p.issuer),
                SemanticTimeMicros::from_micros(due),
            )
            .map_err(|_| AttackError::LedgerFull)?;
        if let Some(m) = self
            .memos
            .iter_mut()
            .find(|m| m.actor == p.issuer && m.ability == p.ability)
        {
            m.sequence = sequence;
            m.at = now.get();
            m.proposal = p.clone();
        } else {
            self.memos.push(Memo {
                actor: p.issuer,
                ability: p.ability.clone(),
                sequence,
                at: now.get(),
                proposal: p.clone(),
                counter: 0,
                next_allowed: 0,
            })
        }
        self.next = next;
        self.pending.push(Pending {
            token,
            source: s.clone(),
            proposal: p.clone(),
        });
        Ok(true)
    }
    fn draw(
        content: [u8; 32],
        p: &ProfileAbilityProposal,
        label: &str,
    ) -> Result<u64, AttackError> {
        use sha2::{Digest, Sha256};
        let digest = Sha256::new()
            .chain_update(b"oteryn:source-callback-spawn:v1")
            .chain_update(p.occurrence.id().as_str().as_bytes())
            .finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(content),
            DecisionOccurrenceId::from_bytes(bytes),
            label,
            0,
        )
        .map_err(|_| AttackError::InvalidPlan)
    }
    fn execute(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        p: Pending,
        reader: &mut impl SourceSpawnFacts,
        now: SemanticTimeMicros,
    ) -> Result<CallbackSpawnReceipt, AttackError> {
        let s = &p.source;
        if !self.current(r, f, stamp) {
            return Err(AttackError::StaleOwner);
        }
        if !s.current(r, p.proposal.issuer) {
            return Err(AttackError::StaleIssuer);
        }
        let actor = p.proposal.issuer;
        let total = r.native_summon_count(actor, None);
        let draw = Self::draw(s.content, &p.proposal, "selection")?;
        let memo_index = self
            .memos
            .iter()
            .position(|m| m.actor == actor && m.ability == s.parent)
            .ok_or(AttackError::InvalidPlan)?;
        let counter = self.memos[memo_index].counter;
        let available = self.memos[memo_index].next_allowed <= now.get();
        let mut requests: Vec<(&str, u32, bool, SourceSpawnPlacement, u32)> = Vec::new();
        match s.kind {
            Kind::Gaz => {
                let count = match total {
                    0 => 2,
                    1 => 1,
                    2..=6 if draw % 1000000 < 247525 => 1,
                    _ => 0,
                };
                requests.push((
                    "minion_of_gaz_haragoth",
                    count,
                    true,
                    SourceSpawnPlacement::Subject,
                    7,
                ));
            }
            Kind::GeneratorRelease => {
                requests.push(("energy_pulse", 1, false, SourceSpawnPlacement::Subject, 16))
            }
            Kind::Tenebris => requests.push((
                "shadow_fiend",
                1,
                false,
                s.location("tenebris_room", true)?,
                16,
            )),
            Kind::Mazoran if total < 4 => requests.push((
                "rage_of_mazoran",
                4,
                true,
                s.location("mazoran_summon_room", true)?,
                16,
            )),
            Kind::Thorn => requests.push((
                "thorn_minion",
                1,
                false,
                SourceSpawnPlacement::Offset(3),
                16,
            )),
            Kind::Plagirath if total < 4 => requests.push((
                "disgusting_ooze",
                4 - total,
                true,
                s.location("plagirath_room", true)?,
                4,
            )),
            Kind::Maxxen => requests.push((
                "glooth_generator",
                1,
                false,
                s.location(&format!("generator_landing_{}", draw % 4), false)?,
                16,
            )),
            Kind::Razzagorn => {
                requests.push(("demon", 4, false, s.location("razzagorn_arena", true)?, 16))
            }
            Kind::Shulgrax if total < 8 => {
                let location = s.location("shulgrax_arena", true)?;
                requests.push(("sin_devourer", 4, true, location.clone(), 16));
                requests.push(("damned_soul", 4, false, location, 16));
            }
            Kind::Soulcatcher => {
                if r.actor_active_speed_delta(
                    actor,
                    None,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get()),
                )
                .is_err()
                {
                    return Err(AttackError::InvalidPlan);
                }
                let store = r
                    .actor_conditions(actor, None)
                    .map_err(|_| AttackError::StaleIssuer)?;
                if store.instances().iter().any(|i| {
                    matches!(
                        i.definition().condition_type(),
                        ConditionType::DamageOverTime(DotElement::Poison | DotElement::Bleeding)
                    )
                }) {
                    requests.push((
                        "corrupted_soul",
                        1,
                        false,
                        SourceSpawnPlacement::Relative(0, -1),
                        16,
                    ));
                }
            }
            Kind::Blazing | Kind::Freezing => {
                let anchor = s.anchor("script_floor_15")?;
                match reader.source_spawn_anchor_contains(r, actor, &anchor, stamp) {
                    None => return Err(AttackError::MissingCombatFacts),
                    Some(false) => {}
                    Some(true) => {
                        let name = if s.kind == Kind::Blazing {
                            "time_waster"
                        } else {
                            "lost_time"
                        };
                        requests.push((
                            name,
                            1,
                            false,
                            SourceSpawnPlacement::Relative(
                                (draw % 5) as i32 - 2,
                                ((draw / 5) % 5) as i32 - 2,
                            ),
                            16,
                        ));
                    }
                }
            }
            Kind::Destruction if counter < 3 && available => {
                requests.push(("disruption", 1, false, SourceSpawnPlacement::Offset(1), 16))
            }
            Kind::Hunger if counter < 3 && available => {
                requests.push(("greed", 1, false, SourceSpawnPlacement::Offset(1), 16))
            }
            Kind::Rage if counter < 3 => {
                requests.push(("frenzy", 1, false, SourceSpawnPlacement::Offset(1), 16))
            }
            Kind::Devourer if counter < 3 => requests.push((
                ["greed", "frenzy", "disruption"][(draw % 3) as usize],
                1,
                false,
                SourceSpawnPlacement::Offset(1),
                16,
            )),
            _ => {}
        }
        let requested = requests.iter().map(|(_, n, _, _, _)| *n).sum();
        let mut output = CallbackSpawnReceipt {
            actor,
            ability: s.parent.clone(),
            requested,
            children: Vec::new(),
            remove: None,
            flags: &[
                "PROJECT_NATIVE_SUMMON_CAP16_SCOPE64",
                "PROJECT_CURRENT_MAP_PLACEMENT_PROVIDER",
                "PROJECT_PER_EXACT_ACTOR_WORLDEVOURER_COUNTER",
            ],
        };
        output
            .children
            .try_reserve_exact(requested as usize)
            .map_err(|_| AttackError::LedgerFull)?;
        // Resolve every fallible current placement before the first physical mutation.
        let mut placements = Vec::new();
        placements
            .try_reserve_exact(requested as usize)
            .map_err(|_| AttackError::LedgerFull)?;
        for (name, count, intrinsic, location, cap) in &requests {
            let child = s.child(name)?;
            let policy = r
                .companion_policy(&child.creature.key)
                .map_err(|_| AttackError::InvalidSource)?;
            if policy.definition_key != child.creature.key
                || policy.definition_revision != child.creature.revision
                || policy.maximum_health != child.health
            {
                return Err(AttackError::InvalidSource);
            }
            for _ in 0..*count {
                let ordinal = placements.len();
                let at = reader
                    .source_spawn_position(r, actor, location, draw, ordinal, stamp)
                    .ok_or(AttackError::MissingCombatFacts)?;
                let allowed = reader
                    .source_spawn_tile_allowed(r, actor, at, stamp)
                    .ok_or(AttackError::MissingCombatFacts)?;
                let spec = if *intrinsic {
                    Admission::Intrinsic(
                        NativeSummonAdmissionSpec::qualified_callback(
                            &s.creature,
                            &child.creature.key,
                            child.health,
                            &s.parent.key,
                            &s.parent.revision,
                            p.proposal.occurrence.id().as_str(),
                            *cap,
                            *cap,
                            s.content,
                        )
                        .map_err(|_| AttackError::InvalidSource)?,
                    )
                } else {
                    Admission::Masterless(
                        NativeEncounterSpawnSpec::qualified(
                            &s.creature,
                            &child.creature.key,
                            &child.creature.revision,
                            s.content,
                        )
                        .map_err(|_| AttackError::InvalidSource)?,
                    )
                };
                placements.push((child.clone(), *intrinsic, spec, at, allowed));
            }
        }
        // Preallocate fixed-source speech/errors before any physical admission; no late allocation refusal can erase committed child receipts.
        if self.speech.len() + placements.len() + 1 > 512 {
            return Err(AttackError::LedgerFull);
        }
        self.speech
            .try_reserve(placements.len() + 1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.speech_errors
            .try_reserve(placements.len() + 1)
            .map_err(|_| AttackError::LedgerFull)?;
        for (child, intrinsic, spec, at, allowed) in placements {
            let mut cleanup = None;
            let result = if !allowed {
                Err(AttackError::OutOfRange)
            } else {
                match spec {
                    Admission::Intrinsic(spec) => {
                        match r.admit_native_summon(f, stamp, actor, &spec, at) {
                            Err(_) => Err(AttackError::InvalidPlan),
                            Ok(new) => match r.install_creature_policy(new, &child.creature.key) {
                                Ok(()) => Ok(new),
                                Err(_) => {
                                    cleanup = Some(
                                        r.retire_native_summon_registration_failure(f, stamp, new),
                                    );
                                    Err(AttackError::InvalidPlan)
                                }
                            },
                        }
                    }
                    Admission::Masterless(spec) => r
                        .admit_native_encounter_masterless(f, stamp, actor, &spec, at, |_, _| {
                            Some(allowed)
                        })
                        .map_err(|_| AttackError::InvalidPlan),
                }
            };
            if let Ok(new) = &result {
                let line = match s.kind {
                    Kind::Tenebris => crate::source_callback_cast::CallbackSpeech::spawn(
                        r,
                        *new,
                        &child.creature.key,
                    ),
                    Kind::Maxxen => {
                        crate::source_callback_cast::CallbackSpeech::generator_warning(r, *new)
                            .map(Some)
                    }
                    _ => Ok(None),
                };
                match line {
                    Ok(Some(line)) => self.speech.push(line),
                    Err(e) => self.speech_errors.push((*new, e)),
                    Ok(None) => {}
                }
            }
            output.children.push(SpawnChildReceipt {
                creature: child.creature,
                result,
                intrinsic,
                cleanup,
            });
        }
        // Source counters measure successful source casts; failed current-map admissions do not
        // fabricate successful counts. This per-exact-actor project projection is explicit above.
        if output.children.iter().any(|c| c.result.is_ok())
            && matches!(
                s.kind,
                Kind::Destruction | Kind::Hunger | Kind::Rage | Kind::Devourer
            )
        {
            let m = &mut self.memos[memo_index];
            m.counter = m.counter.saturating_add(1);
            if matches!(s.kind, Kind::Destruction | Kind::Hunger) {
                m.next_allowed = now.get().saturating_add(15000000);
            }
        }
        if output.children.iter().any(|c| c.result.is_ok())
            && matches!(s.kind, Kind::Gaz | Kind::GeneratorRelease)
        {
            match crate::source_callback_cast::CallbackSpeech::spawn(r, actor, &s.creature) {
                Ok(Some(line)) => self.speech.push(line),
                Err(e) => self.speech_errors.push((actor, e)),
                Ok(None) => {}
            }
        }
        if s.kind == Kind::GeneratorRelease && output.children.iter().any(|c| c.result.is_ok()) {
            output.remove =
                Some(r.remove_native_encounter_subject(f, stamp, actor, &s.creature, s.content));
        }
        Ok(output)
    }
    pub(crate) fn drain(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &mut ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        reader: &mut impl SourceSpawnFacts,
        clock: &impl OwnerClock,
    ) -> Result<Vec<Result<CallbackSpawnReceipt, AttackError>>, AttackError> {
        if !self.current(r, f, stamp) {
            return Err(AttackError::StaleOwner);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(64)
            .map_err(|_| AttackError::LedgerFull)?;
        let due = self
            .timers
            .drain_due(clock, f, |a| r.contains_live_creature(a));
        for timer in due {
            if let Some(index) = self
                .pending
                .iter()
                .position(|p| p.token == timer.occurrence.0)
            {
                let pending = self.pending.remove(index);
                out.push(self.execute(r, f, stamp, pending, reader, clock.now()));
            }
        }
        Ok(out)
    }
}
pub(crate) fn hex(b: [u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
