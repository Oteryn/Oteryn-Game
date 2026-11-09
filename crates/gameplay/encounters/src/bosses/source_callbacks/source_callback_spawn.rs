//! Sixteen fixed source callback adapters sharing the existing native physical lifecycle.
//! Immutable Source specs do not grant current map/scope authority. No Encounter interpreter.
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family,
    ProjectV2SourceIdentityBinding,
};
use crate::creature_auto_attack::AttackError;
use crate::creature_auto_attack::ProfileAbilityProposal;
use oteryn_gameplay_entities::{ConditionType, DotElement};
use oteryn_gameplay_foundation::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::collections::BTreeMap;
use {
    oteryn_gameplay_entities::{
        ChannelRuntimeV1, MovementLocalPosition, NativeEncounterSpawnSpec,
        NativeSummonAdmissionSpec,
    },
    oteryn_gameplay_foundation::foundation::{
        ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration,
        ScopeRuntimeFence,
    },
};
#[derive(serde::Deserialize)]
pub struct Witness {
    pub cases: Vec<Case>,
    profiles: BTreeMap<String, Data>,
    records: Vec<Record>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    #[allow(
        dead_code,
        reason = "retained source-qualified metadata; shipping activation remains separate"
    )]
    targets: Vec<String>,
}
#[derive(serde::Deserialize)]
pub struct Case {
    pub caster: String,
    parent: String,
    pub index: usize,
    pub phase: String,
    pub encounter: String,
    entry: serde_json::Value,
}
pub use oteryn_gameplay_monster_ai::monster_combat_lane::SourceSpawnPlacement;
/// Current map-owner translation/placement/terrain; None always refuses, never allow-default.
pub trait SourceSpawnFacts {
    #[allow(
        clippy::too_many_arguments,
        reason = "source placement reads the exact child and original occurrence"
    )]
    fn source_spawn_child_position(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        _: &Ref,
        placement: &SourceSpawnPlacement,
        draw: u64,
        ordinal: usize,
        stamp: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        self.source_spawn_position(runtime, parent, placement, draw, ordinal, stamp)
    }
    fn source_spawn_child_tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        _: &Ref,
        position: MovementLocalPosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.source_spawn_tile_allowed(runtime, parent, position, stamp)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "source placement reads the exact child and original occurrence"
    )]
    fn source_spawn_child_requested_center(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        _: &Ref,
        placement: &SourceSpawnPlacement,
        draw: u64,
        ordinal: usize,
        stamp: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        self.source_spawn_requested_center(runtime, parent, placement, draw, ordinal, stamp)
    }
    fn current_native(&self) -> Option<&crate::content::native_gameplay::NativeGameplayState> {
        None
    }
    /// Original Lua createMonster position, before the placement owner's fallback.
    /// A selected fallback is never evidence for a forced requested center.
    fn source_spawn_requested_center(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        placement: &SourceSpawnPlacement,
        _: u64,
        _: usize,
        _: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        match placement {
            SourceSpawnPlacement::Subject => runtime
                .read_actor_position(parent)
                .ok()
                .map(|p| p.position()),
            _ => None,
        }
    }
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
pub enum Kind {
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
pub struct Child {
    pub creature: Ref,
    pub health: i64,
    source: Option<crate::content::native_monster_callbacks::QualifiedNativeCallbackSpawn>,
}
impl Child {
    pub fn native_source(
        &self,
    ) -> Option<&crate::content::native_monster_callbacks::QualifiedNativeCallbackSpawn> {
        self.source.as_ref()
    }
}
#[derive(Clone)]
pub struct SourceCallbackSpawn {
    pub kind: Kind,
    pub creature: String,
    pub parent: Ref,
    pub index: usize,
    pub list: ScheduleList,
    pub content: [u8; 32],
    pub children: Vec<Child>,
    anchors: Vec<serde_json::Value>,
    pub range: u16,
    pub magnitude: Option<crate::content::ProjectV2Magnitude>,
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
pub fn fixture() -> Result<Witness, AttackError> {
    serde_json::from_str(include_str!(
        "../../../../../../apps/game-server/src/source_callback_spawn_fixture.json"
    ))
    .map_err(|_| AttackError::InvalidSource)
}
fn native_fixture() -> Result<&'static Witness, AttackError> {
    static FIXTURE: std::sync::OnceLock<Option<Witness>> = std::sync::OnceLock::new();
    FIXTURE
        .get_or_init(|| fixture().ok())
        .as_ref()
        .ok_or(AttackError::InvalidSource)
}
impl SourceCallbackSpawn {
    pub fn qualify_native(
        native: &crate::content::native_gameplay::NativeGameplayState,
        caster: &Ref,
        index: usize,
        list: ScheduleList,
    ) -> Result<Option<Self>, AttackError> {
        let Some(proof) = super::native_witness::qualify(native, caster, index, list)? else {
            return Ok(None);
        };
        let w = native_fixture()?;
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
        if proof.entry().ability.key != c.parent || proof.entry().encounter.key != c.encounter {
            return Err(AttackError::InvalidSource);
        }
        let kind = kind(
            caster
                .key
                .strip_prefix("oteryn:creature.")
                .ok_or(AttackError::InvalidSource)?,
            c.parent
                .strip_prefix("oteryn:ability.spell.")
                .ok_or(AttackError::InvalidSource)?,
        )
        .ok_or(AttackError::InvalidSource)?;
        let mut children = Vec::new();
        for name in child_names(kind) {
            let target = Ref {
                family: Family::Creature,
                key: format!("oteryn:creature.{name}"),
                revision: "definition-r1".into(),
            };
            let body = super::native_witness::native_creature(native, &target)?;
            let health = i64::try_from(body.health.ok_or(AttackError::InvalidSource)?)
                .map_err(|_| AttackError::NumericOverflow)?;
            if health <= 0 {
                return Err(AttackError::InvalidSource);
            }
            let source = native
                .callback_spawn_source(
                    caster,
                    index,
                    if list == ScheduleList::Attack {
                        crate::content::native_monster_callbacks::NativeCallbackList::Attack
                    } else {
                        crate::content::native_monster_callbacks::NativeCallbackList::Defence
                    },
                    &target,
                )
                .map_err(|_| AttackError::InvalidSource)?
                .ok_or(AttackError::InvalidSource)?;
            children.push(Child {
                creature: target,
                health,
                source: Some(source),
            });
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == *caster)
            .ok_or(AttackError::InvalidSource)?;
        let Data::Behavior(behavior) = &record
            .behavior
            .as_ref()
            .ok_or(AttackError::InvalidSource)?
            .data
        else {
            return Err(AttackError::InvalidSource);
        };
        let entries = if list == ScheduleList::Attack {
            &behavior.attacks
        } else {
            &behavior.defenses
        };
        let entry = entries.get(index).ok_or(AttackError::InvalidSource)?;
        if serde_json::to_value(entry).map_err(|_| AttackError::InvalidSource)? != c.entry {
            return Err(AttackError::InvalidSource);
        }
        Self::from_reviewed_case(w, c, caster, entry, children, proof.content()).map(Some)
    }
    pub fn anchors(&self) -> &[serde_json::Value] {
        &self.anchors
    }
    pub fn ability(&self) -> &Ref {
        &self.parent
    }
    pub fn qualify(
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
                source: None,
            });
        }
        Self::from_reviewed_case(&w, c, caster, entry, children, content).map(Some)
    }
    fn from_reviewed_case(
        w: &Witness,
        c: &Case,
        caster: &Ref,
        entry: &crate::content::ProjectV2AbilitySchedule,
        children: Vec<Child>,
        content: [u8; 32],
    ) -> Result<Self, AttackError> {
        let kind = kind(
            caster
                .key
                .strip_prefix("oteryn:creature.")
                .ok_or(AttackError::InvalidSource)?,
            c.parent
                .strip_prefix("oteryn:ability.spell.")
                .ok_or(AttackError::InvalidSource)?,
        )
        .ok_or(AttackError::InvalidSource)?;
        let index = c.index;
        let list = match c.phase.as_str() {
            "attacks" => ScheduleList::Attack,
            "defenses" => ScheduleList::Defence,
            _ => return Err(AttackError::InvalidSource),
        };
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
        Ok(Self {
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
        })
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
pub fn qualified_native_children(d: &ProjectV2Draft) -> Result<Vec<Ref>, AttackError> {
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
pub struct Pending {
    token: u64,
    source: SourceCallbackSpawn,
    proposal: ProfileAbilityProposal,
}
impl Pending {
    pub fn source(&self) -> &SourceCallbackSpawn {
        &self.source
    }
    pub fn proposal(&self) -> &ProfileAbilityProposal {
        &self.proposal
    }
}
pub struct Memo {
    actor: ExactActorRef,
    ability: Ref,
    sequence: u64,
    at: u64,
    proposal: ProfileAbilityProposal,
}
#[derive(Debug)]
pub struct SpawnChildReceipt {
    pub creature: Ref,
    pub result: Result<ExactActorRef, AttackError>,
    pub intrinsic: bool,
    pub cleanup: Option<Result<(), oteryn_gameplay_domain::domain::carrier_values::CarrierError>>,
}
#[derive(Debug)]
pub struct CallbackSpawnReceipt {
    pub actor: ExactActorRef,
    pub ability: Ref,
    pub requested: u32,
    pub children: Vec<SpawnChildReceipt>,
    pub remove: Option<Result<(), oteryn_gameplay_domain::domain::carrier_values::CarrierError>>,
    pub flags: &'static [&'static str],
}
pub struct CallbackSpawnOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Timer, Work>,
    pub pending: Vec<Pending>,
    pub memos: Vec<Memo>,
    next: u64,
    // gaz_functions.lua owns this across source boss incarnations. Keep it in
    // the existing runtime-scope callback owner, never in an actor memo.
    gaz_minions_now: u32,
    heart_counts: [i64; 4],
    heart_next_allowed: [u64; 2],
    speech: Vec<
        oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech,
    >,
    speech_errors: Vec<(ExactActorRef, AttackError)>,
}
impl CallbackSpawnOwner {
    pub fn new_current(r: &ChannelRuntimeV1) -> Result<Self, AttackError> {
        Self::new(r, r.owner_fence().map_err(|_| AttackError::StaleOwner)?)
    }
    pub fn schedule_current(
        &mut self,
        r: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        source: &SourceCallbackSpawn,
        proposal: &ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<bool, AttackError> {
        self.schedule(
            r,
            r.owner_fence().map_err(|_| AttackError::StaleOwner)?,
            stamp,
            source,
            proposal,
            now,
        )
    }
    pub fn new(r: &ChannelRuntimeV1, f: &ScopeRuntimeFence) -> Result<Self, AttackError> {
        let b = r.binding();
        let scope = b.runtime_scope();
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
            gaz_minions_now: 2,
            heart_counts: [0; 4],
            heart_next_allowed: [0; 2],
            speech: Vec::new(),
            speech_errors: Vec::new(),
        })
    }
    pub fn take_source_speech(
        &mut self,
    ) -> Vec<
        oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech,
    > {
        std::mem::take(&mut self.speech)
    }
    pub fn take_speech_errors(&mut self) -> Vec<(ExactActorRef, AttackError)> {
        std::mem::take(&mut self.speech_errors)
    }
    fn current(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> bool {
        let b = r.binding();
        self.scope == b.runtime_scope()
            && self.generation == b.scope_generation()
            && f.is_current_for_scope(self.scope, self.generation)
            && f.accepts_stamp(stamp)
    }
    fn current_bound(
        &self,
        r: &ChannelRuntimeV1,
        f: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
    ) -> bool {
        match f {
            Some(f) => self.current(r, f, stamp),
            None => r.owner_fence().is_ok_and(|f| self.current(r, f, stamp)),
        }
    }
    pub fn schedule(
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
        oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(
            r, p,
        )?;
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
        let delay = match s.kind {
            Kind::GeneratorRelease => 14_000_000,
            Kind::Mazoran => 3_000_000,
            _ => 0,
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
    pub fn next_due_time(&self) -> Option<u64> {
        self.timers.earliest_due().map(SemanticTimeMicros::get)
    }
    pub fn pending_due(&self, pending: &Pending) -> Option<u64> {
        self.timers
            .occurrence_due(Timer, Work(pending.token))
            .map(SemanticTimeMicros::get)
    }
    pub fn draw(
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
        self.execute_bound(r, Some(f), stamp, p, reader, now)
    }
    fn execute_bound(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        p: Pending,
        reader: &mut impl SourceSpawnFacts,
        now: SemanticTimeMicros,
    ) -> Result<CallbackSpawnReceipt, AttackError> {
        let s = &p.source;
        if !self.current_bound(r, f, stamp) {
            return Err(AttackError::StaleOwner);
        }
        if !s.current(r, p.proposal.issuer) {
            return Err(AttackError::StaleIssuer);
        }
        let actor = p.proposal.issuer;
        let total = r.native_summon_count(actor, None);
        let draw = Self::draw(s.content, &p.proposal, "selection")?;
        if !self
            .memos
            .iter()
            .any(|m| m.actor == actor && m.ability == s.parent)
        {
            return Err(AttackError::InvalidPlan);
        }
        let heart_index = match s.kind {
            Kind::Destruction => Some(0),
            Kind::Hunger => Some(1),
            Kind::Rage => Some(2),
            Kind::Devourer => Some(3),
            _ => None,
        };
        let counter = heart_index.map_or(0, |index| self.heart_counts[index]);
        let available = match s.kind {
            Kind::Destruction => self.heart_next_allowed[0] <= now.get(),
            Kind::Hunger => self.heart_next_allowed[1] <= now.get(),
            _ => true,
        };
        let mut requests: Vec<(&str, u32, bool, SourceSpawnPlacement, u32)> = Vec::new();
        let mut gaz_increment = false;
        match s.kind {
            Kind::Gaz => {
                let center = r
                    .read_actor_position(actor)
                    .map_err(|_| AttackError::StaleIssuer)?;
                let mut nearby = 0_u32;
                for candidate in r.visible_entities().creatures {
                    // The qualified caster is Gaz, not its named minion. Name
                    // lookup is definition data, independent of companion linkage.
                    if candidate.actor == actor {
                        continue;
                    }
                    let position = r
                        .read_actor_position(candidate.actor)
                        .map_err(|_| AttackError::StaleTarget)?;
                    if position.context() != center.context()
                        || position.position().floor != center.position().floor
                        || position.position().x.abs_diff(center.position().x) > 25
                        || position.position().y.abs_diff(center.position().y) > 25
                    {
                        continue;
                    }
                    let identity = std::str::from_utf8(
                        r.current_live_creature_identity(candidate.actor)
                            .map_err(|_| AttackError::StaleTarget)?,
                    )
                    .map_err(|_| AttackError::InvalidSource)?;
                    let policy = r
                        .companion_policy(identity)
                        .map_err(|_| AttackError::MissingCombatFacts)?;
                    if policy.display_name == "Minion of Gaz'haragoth" {
                        nearby = nearby.checked_add(1).ok_or(AttackError::NumericOverflow)?;
                    }
                }
                let count = if nearby >= 7 {
                    0
                } else if nearby < self.gaz_minions_now {
                    self.gaz_minions_now - nearby
                } else if crate::spell::uniform_draw(draw, 0, 100) < 25 {
                    1
                } else {
                    0
                };
                // Source calls newMinion:setSummon(unbound sum), which returns
                // false. It never binds these children to the caster.
                requests.push((
                    "minion_of_gaz_haragoth",
                    count,
                    false,
                    SourceSpawnPlacement::Subject,
                    7,
                ));
                if nearby < 7 && nearby >= self.gaz_minions_now && count == 1 {
                    // Publish only after all fallible preparation, below.
                    gaz_increment = true;
                }
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
        let heart_update = if let Some(index) = heart_index.filter(|_| requested > 0) {
            let count = self.heart_counts[index]
                .checked_add(1)
                .ok_or(AttackError::NumericOverflow)?;
            let next = if index < 2 {
                Some(
                    now.get()
                        .checked_add(15_000_000)
                        .ok_or(AttackError::NumericOverflow)?,
                )
            } else {
                None
            };
            Some((index, count, next))
        } else {
            None
        };
        let mut output = CallbackSpawnReceipt {
            actor,
            ability: s.parent.clone(),
            requested,
            children: Vec::new(),
            remove: None,
            flags: &[
                "PROJECT_NATIVE_SUMMON_CAP16_SCOPE64",
                "PROJECT_CURRENT_MAP_PLACEMENT_PROVIDER",
                "SOURCE_SCOPE_SHARED_SIGNED_HEART_COUNTERS",
            ],
        };
        output
            .children
            .try_reserve_exact(requested as usize)
            .map_err(|_| AttackError::LedgerFull)?;
        if s.children.iter().any(|child| child.source.is_some())
            && !reader
                .current_native()
                .is_some_and(|native| native.source_digest() == s.content)
        {
            return Err(AttackError::MissingCombatFacts);
        }
        // Qualify and preallocate all immutable admission data before physical work.
        // Placement itself reads the canonical census immediately before EACH
        // source creation; prior children participate in the next queryAdd.
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
                placements.push((child.clone(), *intrinsic, spec, location.clone(), ordinal));
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
        let gaz_next = if gaz_increment {
            Some(
                self.gaz_minions_now
                    .checked_add(1)
                    .ok_or(AttackError::NumericOverflow)?,
            )
        } else {
            None
        };
        let mut attempted = false;
        for (child, intrinsic, spec, location, ordinal) in placements {
            let requested_center = if child.source.is_some() {
                reader.source_spawn_child_requested_center(
                    r,
                    actor,
                    &child.creature,
                    &location,
                    draw,
                    ordinal,
                    stamp,
                )
            } else {
                None
            };
            let at = reader.source_spawn_child_position(
                r,
                actor,
                &child.creature,
                &location,
                draw,
                ordinal,
                stamp,
            );
            let allowed = at.and_then(|at| {
                reader.source_spawn_child_tile_allowed(r, actor, &child.creature, at, stamp)
            });
            let unavailable = (child.source.is_some() && requested_center.is_none())
                || at.is_none()
                || allowed.is_none();
            if unavailable && !attempted {
                // No source attempt or physical write has occurred: preserve
                // the outer missing-facts refusal, including legacy callers.
                return Err(AttackError::MissingCombatFacts);
            }
            if !unavailable {
                if !attempted && let Some(next) = gaz_next {
                    self.gaz_minions_now = next;
                }
                attempted = true;
                if s.kind == Kind::Gaz {
                    match oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech::spawn(r,actor,&s.creature) {
                        Ok(Some(line))=>self.speech.push(line),
                        Err(error)=>self.speech_errors.push((actor,error)),
                        Ok(None)=>{},
                    }
                }
            }
            let mut cleanup = None;
            let result = if child.source.is_some() && requested_center.is_none() {
                Err(AttackError::MissingCombatFacts)
            } else if let (Some(at), Some(allowed)) = (at, allowed) {
                if !allowed {
                    Err(AttackError::OutOfRange)
                } else {
                    match spec {
                        Admission::Intrinsic(spec) => {
                            let admitted = if let Some(source) = child.source.as_ref() {
                                let native = reader
                                    .current_native()
                                    .ok_or(AttackError::MissingCombatFacts)?;
                                let center =
                                    requested_center.ok_or(AttackError::MissingCombatFacts)?;
                                match f {
                                    Some(f) => r.admit_native_callback_intrinsic(
                                        native,
                                        f,
                                        stamp,
                                        actor,
                                        &spec,
                                        source,
                                        center,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                    None => r.admit_current_native_callback_intrinsic(
                                        native,
                                        stamp,
                                        actor,
                                        &spec,
                                        source,
                                        center,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                }
                            } else {
                                match f {
                                    Some(f) => r.admit_native_summon(f, stamp, actor, &spec, at),
                                    None => r.admit_current_native_summon(stamp, actor, &spec, at),
                                }
                            };
                            match admitted {
                                Err(_) => Err(AttackError::InvalidPlan),
                                Ok(new) => {
                                    match r.install_creature_policy(new, &child.creature.key) {
                                        Ok(()) => Ok(new),
                                        Err(_) => {
                                            cleanup = Some(match f {
                                        Some(f) => r.retire_native_summon_registration_failure(
                                            f, stamp, new,
                                        ),
                                        None => r
                                            .retire_current_native_summon_registration_failure(
                                                stamp, new,
                                            ),
                                    });
                                            Err(AttackError::InvalidPlan)
                                        }
                                    }
                                }
                            }
                        }
                        Admission::Masterless(spec) => {
                            if let Some(source) = child.source.as_ref() {
                                let native = reader
                                    .current_native()
                                    .ok_or(AttackError::MissingCombatFacts)?;
                                let center =
                                    requested_center.ok_or(AttackError::MissingCombatFacts)?;
                                match f {
                                    Some(f) => r.admit_native_callback_masterless(
                                        native,
                                        f,
                                        stamp,
                                        actor,
                                        &spec,
                                        source,
                                        center,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                    None => r.admit_current_native_callback_masterless(
                                        native,
                                        stamp,
                                        actor,
                                        &spec,
                                        source,
                                        center,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                }
                            } else {
                                match f {
                                    Some(f) => r.admit_native_encounter_masterless(
                                        f,
                                        stamp,
                                        actor,
                                        &spec,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                    None => r.admit_current_native_encounter_masterless(
                                        stamp,
                                        actor,
                                        &spec,
                                        at,
                                        |_, _| Some(allowed),
                                    ),
                                }
                            }
                            .map_err(|_| AttackError::InvalidPlan)
                        }
                    }
                }
            } else {
                Err(AttackError::MissingCombatFacts)
            };
            if let Ok(new) = &result {
                let line = match s.kind {
                    Kind::Tenebris => oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech::spawn(
                        r,
                        *new,
                        &child.creature.key,
                    ),
                    Kind::Maxxen => {
                        oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech::generator_warning(r, *new)
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
            let failed = result.is_err();
            let unavailable = matches!(result, Err(AttackError::MissingCombatFacts));
            output.children.push(SpawnChildReceipt {
                creature: child.creature,
                result,
                intrinsic,
                cleanup,
            });
            if unavailable
                || (failed
                    && matches!(
                        s.kind,
                        Kind::Mazoran | Kind::Plagirath | Kind::Razzagorn | Kind::Shulgrax
                    ))
            {
                // Each donor loop returns immediately when createMonster returns nil.
                break;
            }
        }
        // Lua increments these shared counters after a createMonster attempt, including
        // a proved nil result. Unavailable facts do not constitute a source attempt.
        if let Some((index, count, next)) = heart_update
            && output
                .children
                .iter()
                .any(|child| !matches!(child.result, Err(AttackError::MissingCombatFacts)))
        {
            self.heart_counts[index] = count;
            if let Some(next) = next {
                self.heart_next_allowed[index] = next;
            }
        }
        if s.kind == Kind::GeneratorRelease {
            match oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackSpeech::spawn(r, actor, &s.creature) {
                Ok(Some(line)) => self.speech.push(line),
                Err(e) => self.speech_errors.push((actor, e)),
                Ok(None) => {}
            }
        }
        if s.kind == Kind::GeneratorRelease {
            output.remove = Some(match f {
                Some(f) => {
                    r.remove_native_encounter_subject(f, stamp, actor, &s.creature, s.content)
                }
                None => {
                    r.remove_current_native_encounter_subject(stamp, actor, &s.creature, s.content)
                }
            });
        }
        Ok(output)
    }
    pub fn drain_current(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        reader: &mut impl SourceSpawnFacts,
        clock: &impl OwnerClock,
    ) -> Result<Vec<Result<CallbackSpawnReceipt, AttackError>>, AttackError> {
        self.drain_current_through(r, stamp, reader, clock, clock.now().get())
    }
    /// Restrict eligibility by original deadlines while using the actual owner
    /// clock for all execution, map facts and child admission.
    pub fn drain_current_through(
        &mut self,
        r: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        reader: &mut impl SourceSpawnFacts,
        clock: &impl OwnerClock,
        eligible_through_us: u64,
    ) -> Result<Vec<Result<CallbackSpawnReceipt, AttackError>>, AttackError> {
        if !self.current_bound(r, None, stamp) {
            return Err(AttackError::StaleOwner);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(64)
            .map_err(|_| AttackError::LedgerFull)?;
        let eligible = self
            .pending
            .iter()
            .filter_map(|pending| {
                self.pending_due(pending)
                    .filter(|due| *due <= eligible_through_us)
                    .map(|_| Work(pending.token))
            })
            .collect::<Vec<_>>();
        let due = self.timers.drain_due_matching(
            clock,
            r.owner_fence().map_err(|_| AttackError::StaleOwner)?,
            |actor| r.contains_live_creature(actor),
            |_, occurrence| eligible.contains(&occurrence),
        );
        for timer in due {
            if let Some(index) = self
                .pending
                .iter()
                .position(|p| p.token == timer.occurrence.0)
            {
                let pending = self.pending.remove(index);
                out.push(self.execute_bound(r, None, stamp, pending, reader, clock.now()));
            }
        }
        Ok(out)
    }
    pub fn drain(
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
pub fn hex(b: [u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
