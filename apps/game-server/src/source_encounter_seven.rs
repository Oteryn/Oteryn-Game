//! Seven exact source descriptors; Map-dependent actions fail closed without their real owner.
//! Time phase is PROJECT_SINGLE_ACTOR_PROFILE_SWAP_NOT_GLOBAL_TWO_ACTOR_SWAP.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    ProjectReferenceRecord as Record, ProjectV2AuthoringProfile,
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref, ProjectV2Draft,
    ProjectV2Family as Family, ProjectV2SourceIdentityBinding,
};
use crate::creature_auto_attack::AttackError;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementPositionContext,
    PreparedCreatureProfileTransformation, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence,
    WorldId,
};
use sha2::{Digest, Sha256};
const BASE: &str = "oteryn:creature.the_time_guardian";
const BLAZING: &str = "oteryn:creature.the_blazing_time_guardian";
const FREEZING: &str = "oteryn:creature.the_freezing_time_guardian";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SevenKind {
    BrainHeal,
    Vortex,
    Maxxen,
    Ragiaz,
    TimeGuardian,
    TimeGuardiann,
    Zamulosh,
}
#[derive(serde::Deserialize)]
struct Witness {
    cases: Vec<Case>,
    profiles: Vec<ProjectV2AuthoringProfile>,
    records: Vec<Record>,
    source_identity_bindings: Vec<ProjectV2SourceIdentityBinding>,
}
#[derive(Clone, serde::Deserialize)]
struct Case {
    creature: String,
    ability: String,
    list: String,
    index: usize,
    encounter: String,
}
#[derive(Clone)]
pub(crate) struct SevenSource {
    case: Case,
    kind: SevenKind,
    content: [u8; 32],
    profiles: std::sync::Arc<[ProjectV2AuthoringProfile]>,
    records: std::sync::Arc<[Record]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceMapWork {
    pub(crate) actor: ExactActorRef,
    pub(crate) ability: Ref,
    pub(crate) sequence: u64,
    pub(crate) kind: SevenKind,
    pub(crate) content: [u8; 32],
    pub(crate) source_frame: &'static str,
}
impl SevenSource {
    pub(crate) fn qualify(
        d: &ProjectV2Draft,
        c: &Ref,
        index: usize,
        list: ScheduleList,
        content: [u8; 32],
    ) -> Result<Option<Self>, AttackError> {
        let w: Witness = serde_json::from_str(include_str!("source_encounter_seven_fixture.json"))
            .map_err(|_| AttackError::InvalidSource)?;
        if w.cases.len() != 7 || w.profiles.len() > 512 || w.records.len() > 512 {
            return Err(AttackError::InvalidSource);
        }
        let name = if list == ScheduleList::Attack {
            "attacks"
        } else {
            "defenses"
        };
        let Some(case) = w
            .cases
            .iter()
            .find(|x| x.creature == c.key && x.index == index && x.list == name)
            .cloned()
        else {
            return Ok(None);
        };
        if c.family != Family::Creature || c.revision != "definition-r1" {
            return Err(AttackError::InvalidSource);
        }
        for p in &w.profiles {
            let mut matches = d
                .state
                .authoring_profiles
                .iter()
                .filter(|q| q.target == p.target);
            if matches.next() != Some(p) || matches.next().is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        for record in &w.records {
            if d.core.records.iter().filter(|r| *r == record).count() != 1 {
                return Err(AttackError::InvalidSource);
            }
        }
        for key in [&case.creature, &case.encounter] {
            if !w
                .source_identity_bindings
                .iter()
                .any(|b| b.target.key == *key)
            {
                return Err(AttackError::InvalidSource);
            }
        }
        for b in w
            .source_identity_bindings
            .iter()
            .filter(|b| b.target.key == case.creature || b.target.key == case.encounter)
        {
            let mut matches = d.state.source_identity_bindings.iter().filter(|q| {
                q.target == b.target
                    && q.identity_namespace == b.identity_namespace
                    && q.external_id == b.external_id
            });
            if matches.next() != Some(b)
                || matches.next().is_some()
                || !d
                    .state
                    .sources
                    .iter()
                    .any(|s| s.key == b.source_key && s.revision == b.source_revision)
            {
                return Err(AttackError::InvalidSource);
            }
        }
        let behavior = d
            .core
            .records
            .iter()
            .find_map(|r| match r {
                Record::Creature {
                    identity, behavior, ..
                } if identity.key == c.key && identity.revision == c.revision => Some(behavior),
                _ => None,
            })
            .ok_or(AttackError::InvalidSource)?;
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
        let e = entries.get(index).ok_or(AttackError::InvalidSource)?;
        if e.ability.key != case.ability || e.ability.revision != "definition-r1" {
            return Err(AttackError::InvalidSource);
        }
        let kind = match case.ability.as_str() {
            "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head" => SevenKind::BrainHeal,
            "oteryn:ability.spell.charge_vortex" => SevenKind::Vortex,
            "oteryn:ability.spell.maxxenteleport" => SevenKind::Maxxen,
            "oteryn:ability.spell.ragiaz_transform" => SevenKind::Ragiaz,
            "oteryn:ability.spell.time_guardian" => SevenKind::TimeGuardian,
            "oteryn:ability.spell.time_guardiann" => SevenKind::TimeGuardiann,
            "oteryn:ability.spell.zamulosh_tp" => SevenKind::Zamulosh,
            _ => return Err(AttackError::UnsupportedShape),
        };
        Ok(Some(Self {
            case,
            kind,
            content,
            profiles: w.profiles.into(),
            records: w.records.into(),
        }))
    }
    pub(crate) fn ability(&self) -> Ref {
        Ref {
            family: Family::Ability,
            key: self.case.ability.clone(),
            revision: "definition-r1".into(),
        }
    }
    pub(crate) fn kind(&self) -> SevenKind {
        self.kind
    }
    pub(crate) fn phase(&self) -> ScheduleList {
        if self.case.list == "attacks" {
            ScheduleList::Attack
        } else {
            ScheduleList::Defence
        }
    }
    pub(crate) fn native_profiles(&self) -> &[ProjectV2AuthoringProfile] {
        &self.profiles
    }
    pub(crate) fn native_records(&self) -> &[Record] {
        &self.records
    }
    fn sequence(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        p: &ProfileAbilityProposal,
    ) -> Result<u64, AttackError> {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if r.content_pin().server_artifact_digest() != self.content {
            return Err(AttackError::ContentChanged);
        }
        if !r.contains_live_creature(p.issuer)
            || p.ability != self.ability()
            || p.list != self.phase()
            || p.entry_index != self.case.index
            || p.range_tiles != 0
            || p.magnitude.is_some()
            || p.target != p.issuer
        {
            return Err(AttackError::InvalidSource);
        }
        let atom = format!("actor:{}", hex(p.issuer.placement_identity()));
        if p.intent.proposal_source() != crate::ability::ProposalSource::Ai
            || p.intent.actor() != atom
            || p.intent.candidate_count() != 1
            || p.intent.resolved_targets().len() != 1
            || p.intent.resolved_targets()[0].as_str() != atom
        {
            return Err(AttackError::InvalidPlan);
        }
        let prefix = format!("ai-profile:{}:", hex(p.issuer.placement_identity()));
        let rest = p
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (sequence, suffix) = rest.split_once(':').ok_or(AttackError::InvalidPlan)?;
        let label = if p.list == ScheduleList::Attack {
            "attack"
        } else {
            "defence"
        };
        if suffix != format!("{label}:{}", self.case.index) {
            return Err(AttackError::InvalidPlan);
        }
        sequence.parse().map_err(|_| AttackError::InvalidPlan)
    }
    /// Descriptive source work is not a relocation or top-stack grant. Thalom does not contain these quest anchors.
    pub(crate) fn prepare_map_work(
        &self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        p: &ProfileAbilityProposal,
    ) -> Result<SourceMapWork, AttackError> {
        let sequence = self.sequence(r, f, stamp, p)?;
        if !r.matches_live_creature_identity(p.issuer, self.case.creature.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        Ok(SourceMapWork {
            actor: p.issuer,
            ability: self.ability(),
            sequence,
            kind: self.kind,
            content: self.content,
            source_frame: "SOURCE_GLOBAL_QUEST_COORDINATES_REQUIRE_QUALIFIED_FRAME_ROLE_BINDING",
        })
    }
}
fn hex(v: [u8; 16]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Pin {
    world: WorldId,
    activation: u64,
    digests: [[u8; 32]; 4],
    context: MovementPositionContext,
}
impl Pin {
    fn read(r: &ChannelRuntimeV1) -> Self {
        let p = r.content_pin();
        Self {
            world: r.binding().world_id(),
            activation: p.activation_sequence(),
            digests: [
                p.server_artifact_digest(),
                p.client_artifact_digest(),
                p.frame_binding_digest(),
                p.map_revision_digest(),
            ],
            context: r.pinned_movement_context(),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ReturnFamily;
impl TimerFamily for ReturnFamily {
    fn registered_maximum(self) -> usize {
        1
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ReturnKey {
    actor: ExactActorRef,
    token: u64,
}
struct Pending {
    actor: ExactActorRef,
    token: u64,
    epoch: u64,
    pin: Pin,
    source: SevenSource,
    phase: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimePhaseOutcome {
    pub(crate) actor: ExactActorRef,
    pub(crate) definition: String,
    pub(crate) health: i64,
    pub(crate) epoch: u64,
    pub(crate) returned: bool,
    pub(crate) qualification: &'static str,
}
struct Memo {
    actor: ExactActorRef,
    pin: Pin,
    sequence: u64,
    proof: [u8; 32],
    at: SemanticTimeMicros,
    out: TimePhaseOutcome,
}
pub(crate) struct TimeGuardianOwner {
    timers: OwnerTimerLane<ReturnFamily, ReturnKey>,
    pending: Vec<Pending>,
    memos: Vec<Memo>,
    next: u64,
    last_time: Option<SemanticTimeMicros>,
}
pub(crate) struct PreparedTimeCast {
    actor: ExactActorRef,
    sequence: u64,
    proof: [u8; 32],
    at: SemanticTimeMicros,
    deadline: SemanticTimeMicros,
    pin: Pin,
    source: SevenSource,
    phase: String,
    native: PreparedCreatureProfileTransformation,
    expected_epoch: u64,
    out: TimePhaseOutcome,
    memo_out: TimePhaseOutcome,
}
pub(crate) struct DueTimeReturn {
    actor: ExactActorRef,
    token: u64,
    source: SevenSource,
    native: PreparedCreatureProfileTransformation,
    out: TimePhaseOutcome,
    at: SemanticTimeMicros,
}
impl TimeGuardianOwner {
    pub(crate) fn new(r: &ChannelRuntimeV1) -> Result<Self, AttackError> {
        let b = r.binding();
        Ok(Self {
            timers: OwnerTimerLane::for_generation(
                RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                b.scope_generation(),
                [(
                    ReturnFamily,
                    FamilyPolicy {
                        max_pending: 1,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )],
            )
            .map_err(|_| AttackError::InvalidPlan)?,
            pending: Vec::new(),
            memos: Vec::new(),
            next: 0,
            last_time: None,
        })
    }
    /// The aggregate prepares the replacement source/profile bundle BEFORE passing this plan to commit.
    pub(crate) fn prepare_cast(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        s: &SevenSource,
        p: &ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<Result<PreparedTimeCast, TimePhaseOutcome>, AttackError> {
        let sequence = s.sequence(r, f, stamp, p)?;
        if !matches!(s.kind, SevenKind::TimeGuardian | SevenKind::TimeGuardiann) {
            return Err(AttackError::UnsupportedShape);
        }
        if self.last_time.is_some_and(|last| now < last) {
            return Err(AttackError::InvalidPlan);
        }
        let pin = Pin::read(r);
        let proof: [u8; 32] = Sha256::digest(format!("{:?}:{:?}", p, s.kind).as_bytes()).into();
        self.memos
            .retain(|m| m.pin == pin && r.contains_live_creature(m.actor));
        if let Some(m) = self.memos.iter().find(|m| m.actor == p.issuer) {
            if sequence < m.sequence || now < m.at {
                return Err(AttackError::InvalidPlan);
            }
            if sequence == m.sequence {
                return if m.proof == proof {
                    Ok(Err(m.out.clone()))
                } else {
                    Err(AttackError::InvalidPlan)
                };
            }
        }
        if !r.matches_live_creature_identity(p.issuer, BASE.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        let snapshot = r
            .companion_snapshot(p.issuer)
            .map_err(|_| AttackError::InvalidPlan)?;
        if snapshot.position.floor != 14 {
            return Ok(Err(TimePhaseOutcome {
                actor: p.issuer,
                definition: BASE.into(),
                health: snapshot.health,
                epoch: snapshot.state.lifecycle_epoch,
                returned: false,
                qualification: "SOURCE_FLOOR14_GUARD_NOOP",
            }));
        }
        if self.pending.iter().any(|x| x.actor == p.issuer)
            || self.pending.len() >= 64
            || (self.memos.len() >= 64 && !self.memos.iter().any(|m| m.actor == p.issuer))
        {
            return Err(AttackError::LedgerFull);
        }
        self.pending
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.memos
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        let deadline = SemanticTimeMicros::from_micros(
            now.get()
                .checked_add(30_000_000)
                .ok_or(AttackError::NumericOverflow)?,
        );
        let draw = Sha256::new()
            .chain_update(s.content)
            .chain_update(p.occurrence.id().as_str().as_bytes())
            .finalize();
        let mut bits = [0; 8];
        bits.copy_from_slice(&draw[..8]);
        let phase = if crate::spell::uniform_draw(u64::from_le_bytes(bits), 1, 2) == 1 {
            BLAZING
        } else {
            FREEZING
        }
        .to_owned();
        let native = r
            .prepare_creature_profile_transformation(&snapshot, &phase)
            .map_err(|_| AttackError::InvalidSource)?;
        let out = TimePhaseOutcome {
            actor: p.issuer,
            definition: phase.clone(),
            health: snapshot.health,
            epoch: snapshot
                .state
                .lifecycle_epoch
                .checked_add(1)
                .ok_or(AttackError::NumericOverflow)?,
            returned: false,
            qualification: QUALIFICATION,
        };
        let memo_out = out.clone();
        Ok(Ok(PreparedTimeCast {
            actor: p.issuer,
            sequence,
            proof,
            at: now,
            deadline,
            pin,
            source: s.clone(),
            phase,
            native,
            expected_epoch: snapshot.state.lifecycle_epoch,
            out,
            memo_out,
        }))
    }
    pub(crate) fn planned_definition(plan: &PreparedTimeCast) -> &str {
        &plan.phase
    }
    pub(crate) fn source_for_cast(plan: &PreparedTimeCast) -> &SevenSource {
        &plan.source
    }
    pub(crate) fn commit_cast(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        plan: PreparedTimeCast,
    ) -> Result<TimePhaseOutcome, AttackError> {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if self.last_time.is_some_and(|last| plan.at < last) {
            return Err(AttackError::InvalidPlan);
        }
        if Pin::read(r) != plan.pin
            || !r.matches_live_creature_identity(plan.actor, BASE.as_bytes())
            || r.companion_snapshot(plan.actor)
                .map_err(|_| AttackError::StaleIssuer)?
                .state
                .lifecycle_epoch
                != plan.expected_epoch
        {
            return Err(AttackError::InvalidPlan);
        }
        if self.pending.iter().any(|p| p.actor == plan.actor)
            || self.pending.len() >= 64
            || (self.memos.len() >= 64 && !self.memos.iter().any(|m| m.actor == plan.actor))
        {
            return Err(AttackError::LedgerFull);
        }
        self.pending
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        self.memos
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        let token = self
            .next
            .checked_add(1)
            .ok_or(AttackError::NumericOverflow)?;
        let key = ReturnKey {
            actor: plan.actor,
            token,
        };
        self.timers
            .schedule(f, stamp, ReturnFamily, key, Some(plan.actor), plan.deadline)
            .map_err(|_| AttackError::InvalidPlan)?;
        let after = match r.commit_creature_profile_transformation(plan.native) {
            Ok(after) => after,
            Err(_) => {
                self.timers.cancel(ReturnFamily, key);
                return Err(AttackError::InvalidPlan);
            }
        };
        self.next = token;
        self.last_time = Some(plan.at);
        let out = plan.out;
        self.pending.push(Pending {
            actor: plan.actor,
            token,
            epoch: after.state.lifecycle_epoch,
            pin: plan.pin,
            source: plan.source,
            phase: plan.phase,
        });
        let memo = Memo {
            actor: plan.actor,
            pin: plan.pin,
            sequence: plan.sequence,
            proof: plan.proof,
            at: plan.at,
            out: plan.memo_out,
        };
        if let Some(i) = self.memos.iter().position(|m| m.actor == plan.actor) {
            self.memos[i] = memo
        } else {
            self.memos.push(memo)
        }
        Ok(out)
    }
    /// Current source actor survives phase change; vanished, dead, foreign-content or replaced generations cancel, never teleport a replacement.
    pub(crate) fn due_returns(
        &mut self,
        r: &ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        clock: &impl OwnerClock,
    ) -> Result<Vec<Result<DueTimeReturn, AttackError>>, AttackError> {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) {
            return Err(AttackError::StaleOwner);
        }
        let now = clock.now();
        if self.last_time.is_some_and(|last| now < last) {
            return Err(AttackError::InvalidPlan);
        }
        let pin = Pin::read(r);
        for p in &self.pending {
            if p.pin != pin || !r.contains_live_creature(p.actor) {
                self.timers.cancel(
                    ReturnFamily,
                    ReturnKey {
                        actor: p.actor,
                        token: p.token,
                    },
                );
            }
        }
        self.pending
            .retain(|p| p.pin == pin && r.contains_live_creature(p.actor));
        let mut out = Vec::new();
        out.try_reserve(self.pending.len())
            .map_err(|_| AttackError::LedgerFull)?; // Preview preserves the sole callback until successful physical commit; refused preparation/commit is retryable.
        let mut preview = self.timers.clone();
        let due = preview.drain_due(clock, f, |a| r.contains_live_creature(a));
        for fired in due {
            let key = fired.occurrence;
            let Some(p) = self
                .pending
                .iter()
                .find(|p| p.actor == key.actor && p.token == key.token)
            else {
                out.push(Err(AttackError::InvalidPlan));
                continue;
            };
            let plan = (|| {
                if p.pin != pin || !r.matches_live_creature_identity(p.actor, p.phase.as_bytes()) {
                    return Err(AttackError::ContentChanged);
                }
                let snapshot = r
                    .companion_snapshot(p.actor)
                    .map_err(|_| AttackError::StaleIssuer)?;
                if snapshot.state.lifecycle_epoch != p.epoch {
                    return Err(AttackError::InvalidPlan);
                }
                let native = r
                    .prepare_creature_profile_transformation(&snapshot, BASE)
                    .map_err(|_| AttackError::InvalidSource)?;
                let out = TimePhaseOutcome {
                    actor: p.actor,
                    definition: BASE.into(),
                    health: snapshot.health,
                    epoch: snapshot
                        .state
                        .lifecycle_epoch
                        .checked_add(1)
                        .ok_or(AttackError::NumericOverflow)?,
                    returned: true,
                    qualification: QUALIFICATION,
                };
                Ok(DueTimeReturn {
                    actor: p.actor,
                    token: p.token,
                    source: p.source.clone(),
                    native,
                    out,
                    at: now,
                })
            })();
            out.push(plan);
        }
        Ok(out)
    }
    pub(crate) fn returned_actor(plan: &DueTimeReturn) -> ExactActorRef {
        plan.actor
    }
    pub(crate) fn source_for_return(plan: &DueTimeReturn) -> &SevenSource {
        &plan.source
    }
    pub(crate) fn commit_return(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        plan: DueTimeReturn,
    ) -> Result<TimePhaseOutcome, AttackError> {
        let b = r.binding();
        if !f.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !f.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if self.last_time.is_some_and(|last| plan.at < last) {
            return Err(AttackError::InvalidPlan);
        }
        let Some(p) = self
            .pending
            .iter()
            .find(|p| p.actor == plan.actor && p.token == plan.token)
        else {
            return Err(AttackError::InvalidPlan);
        };
        if Pin::read(r) != p.pin {
            return Err(AttackError::ContentChanged);
        }
        r.commit_creature_profile_transformation(plan.native)
            .map_err(|_| AttackError::InvalidPlan)?;
        self.timers.cancel(
            ReturnFamily,
            ReturnKey {
                actor: plan.actor,
                token: plan.token,
            },
        );
        self.pending
            .retain(|p| p.actor != plan.actor || p.token != plan.token);
        self.last_time = Some(plan.at);
        Ok(plan.out)
    }
}
const QUALIFICATION: &str = "PROJECT_SINGLE_ACTOR_PROFILE_SWAP_NOT_GLOBAL_TWO_ACTOR_SWAP;PROJECT_UNIFORM_DRAW;SOURCE_TWO_DISTINCT_ENTRIES_PRESERVED;SOURCE_30S_DELAY;PRESENTATION_NETWORK_PENDING";

#[cfg(test)]
#[path = "source_encounter_seven_tests.rs"]
pub(crate) mod tests;
