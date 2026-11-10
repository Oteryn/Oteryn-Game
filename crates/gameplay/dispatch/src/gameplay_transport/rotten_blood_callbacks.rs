//! Pinned Rotten callbacks consumed only by actual reserved native thinks.
use super::actor_spell::ChannelSpellStates;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, ThinkOccurrence,
};
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounters::bosses::rotten_blood::{
    RoomPosition, area_icons, elder_bloodjaw_think as elder, ichgahal_think as ich,
    murcion_think as mur,
};
use oteryn_gameplay_entities::carrier::rotten_blood::RottenBloodSpawnSource;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, sync::Arc};
mod advanced;
pub(super) use advanced::bak_preparation::before_drive as prepare_bak_first_think;
pub(super) use advanced::step::step_in;
mod mushroom;

type BossSpawnPlan = (&'static str, RoomPosition, u8, bool);
type BossSummonChange = (Option<BossSpawnPlan>, Option<u64>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceDue {
    at: u64,
    kind: SourceDueKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceDueKind {
    Effects,
    Mushroom,
    Advanced(advanced::DueKind),
}

#[derive(Debug, Clone)]
enum SourcePrefixBoundary {
    Death(oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathNotice),
    Profile {
        original: Arc<super::monster_profile_cycle::retained::PendingProfilePass>,
        due: u64,
    },
}
#[derive(Debug, Clone)]
struct SourcePrefix {
    boundary: SourcePrefixBoundary,
    through: u64,
}
impl SourcePrefix {
    fn current(&self, runtime: &ChannelRuntimeV1, states: &ChannelSpellStates) -> bool {
        match &self.boundary {
            SourcePrefixBoundary::Death(notice) => {
                runtime.next_source_creature_death().ok().flatten().as_ref() == Some(notice)
            }
            SourcePrefixBoundary::Profile { original, due } => {
                runtime.next_source_creature_death().ok() == Some(None)
                    && states
                        .native_profile_pass
                        .pending()
                        .is_some_and(|current| Arc::ptr_eq(&current, original))
                    // next_due validates the original batch's current owner
                    // and every content/frame pin before exposing its clock.
                    && states.native_profile_pass.next_due(runtime).ok() == Some(Some(*due))
            }
        }
    }
}

/// Ordinary detached entry points use the same source-death minimum as the
/// native owner. A retained Profile is also a bounded, immutable prefix.
fn source_prefix(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    native: &NativeGameplayState,
    kind: advanced::DueKind,
    now: u64,
) -> Result<Option<SourcePrefix>, ()> {
    let Some(notice) = runtime.next_source_creature_death().map_err(|_| ())? else {
        let Some(profile) = states.native_profile_pass.next_due(runtime)? else {
            return Ok(None);
        };
        let selected = states
            .rotten_blood_callbacks
            .next_source_due(runtime)?
            .ok_or(())?;
        if selected.kind != SourceDueKind::Advanced(kind)
            || selected.at > profile
            || selected.at > now
            || super::source_death::next_source_work(runtime, states, native)?
                != Some((
                    super::source_death::SourceWorkFamily::RottenBlood,
                    selected.at,
                ))
        {
            return Err(());
        }
        let original = states.native_profile_pass.pending().ok_or(())?;
        return Ok(Some(SourcePrefix {
            boundary: SourcePrefixBoundary::Profile {
                original,
                due: profile,
            },
            through: selected.at,
        }));
    };
    let cutoff = notice.enqueued_at_us().ok_or(())?;
    let selected = states
        .rotten_blood_callbacks
        .next_source_due(runtime)?
        .ok_or(())?;
    if selected.kind != SourceDueKind::Advanced(kind) || selected.at > cutoff || selected.at > now {
        return Err(());
    }
    if states
        .native_callbacks
        .next_source_due_time()?
        .into_iter()
        .chain(states.soul_war_callbacks.next_native_due())
        .chain(
            states
                .soul_war_callbacks
                .next_splinter_due(runtime, native)?,
        )
        .chain(states.spite_global.next_due_us())
        .chain(states.primal_callbacks.next_source_due_time()?)
        .chain(states.native_profile_pass.next_due(runtime)?)
        .any(|due| due < selected.at && due <= now)
    {
        return Err(());
    }
    Ok(Some(SourcePrefix {
        boundary: SourcePrefixBoundary::Death(notice),
        through: selected.at,
    }))
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Consume one selected physical Item continuation outside the runtime lock.
    /// FIRST-think publication keeps its existing Source-before-Instance lanes
    /// and is admitted by its separate entry point before this permit is held.
    pub(super) async fn drain_detached_rotten_before_source_death(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> Result<bool, ()> {
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let kind = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            if !runtime.has_pending_source_creature_deaths()
                && !states.native_profile_pass.has_pending()
            {
                return Ok(false);
            }
            let Some(SourceDue {
                kind: SourceDueKind::Advanced(kind),
                at,
            }) = states.rotten_blood_callbacks.next_source_due(&runtime)?
            else {
                return Ok(false);
            };
            if !matches!(
                kind,
                advanced::DueKind::PortalItem | advanced::DueKind::Footprint
            ) || super::source_death::next_source_work(&runtime, &states, native)?
                != Some((super::source_death::SourceWorkFamily::RottenBlood, at))
                || source_prefix(&runtime, &states, native, kind, now).is_err()
            {
                return Ok(false);
            }
            kind
        };
        match kind {
            advanced::DueKind::PortalItem => self.resolve_rotten_portal_callbacks(permit).await,
            advanced::DueKind::Footprint => {
                self.resolve_rotten_movement_callbacks(permit).await;
            }
            _ => return Err(()),
        }
        Ok(true)
    }
}

#[cfg(test)]
pub(super) struct MassiveStateForTest {
    pub(super) deadlines: Vec<(u8, u64)>,
    pub(super) paused: bool,
    pub(super) commits: usize,
}

#[cfg(test)]
pub(super) struct MushroomProgressForTest {
    pub(super) actor: ExactActorRef,
    pub(super) original_due: u64,
    pub(super) cursor: usize,
    pub(super) total: usize,
    pub(super) players: Vec<(ExactActorRef, GameSessionId)>,
}

#[cfg(test)]
pub(super) struct PortalClockForTest {
    pub(super) cycle_due: Option<u64>,
    pub(super) think_sequence: Option<u64>,
    pub(super) callbacks: Vec<(u64, u64, bool)>,
}

#[derive(Debug)]
struct Boss {
    actor: ExactActorRef,
    source: Arc<RottenBloodSpawnSource>,
    key: String,
    last: Option<ThinkOccurrence>,
    murcion: mur::MurcionThinkOwner,
    ichgahal: ich::IchgahalThinkOwner,
    wave: Vec<(u64, ExactActorRef)>,
    next_id: u64,
}
#[derive(Debug, Clone)]
enum Effect {
    Spawn {
        source: Arc<RottenBloodSpawnSource>,
        parent: ExactActorRef,
        sequence: u64,
        ordinal: u64,
        key: &'static str,
        around: RoomPosition,
        offset: u8,
        close: bool,
        wave: bool,
    },
    Remove {
        actor: ExactActorRef,
    },
}
#[derive(Debug, Default)]
pub(super) struct RottenBloodCallbacks {
    pub(super) advanced: advanced::AdvancedCallbacks,
    mushrooms: mushroom::Mushrooms,
    bosses: Vec<Boss>,
    pending: VecDeque<(u64, Effect)>,
    pending_due: Option<u64>,
    current_taints: Vec<crate::durability::quest_state::rotten_blood::CurrentRottenBloodTaintFacts>,
    prepared_bak: Option<crate::durability::quest_state::rotten_flame::PreparedBakEssences>,
    read_stamp: Option<crate::foundation::RuntimeWorkStamp>,
    read_context: Option<ReadContext>,
}
type ReadContext = (
    crate::foundation::RuntimeScopeRefV1,
    crate::foundation::ScopeOwnershipGeneration,
    u64,
    [u8; 32],
    [u8; 32],
    [u8; 32],
    [u8; 32],
);
fn context(runtime: &ChannelRuntimeV1) -> ReadContext {
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
        runtime.content_pin().activation_sequence(),
        runtime.content_pin().server_artifact_digest(),
        runtime.content_pin().client_artifact_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.map_revision_digest(),
    )
}
impl RottenBloodCallbacks {
    fn next_source_due(&self, runtime: &ChannelRuntimeV1) -> Result<Option<SourceDue>, ()> {
        Ok(self
            .pending
            .front()
            .map(|(due, _)| SourceDue {
                at: *due,
                kind: SourceDueKind::Effects,
            })
            .into_iter()
            .chain(self.mushrooms.next_due_time().map(|due| SourceDue {
                at: due,
                kind: SourceDueKind::Mushroom,
            }))
            .chain(
                self.advanced
                    .next_source_due(runtime)?
                    .map(|(at, kind)| SourceDue {
                        at,
                        kind: SourceDueKind::Advanced(kind),
                    }),
            )
            .min_by_key(|work| work.at))
    }
    /// Original source occurrence time, including committed onMove records not
    /// yet wrapped by the Item consumer. Retry wakes are scheduling hints only.
    pub(super) fn next_source_due_time(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<u64>, ()> {
        Ok(self.next_source_due(runtime)?.map(|work| work.at))
    }
    #[cfg(test)]
    pub(super) fn pause_mushroom_after_player_commit_for_test(&mut self) {
        self.mushrooms.pause_after_player_commit_for_test();
    }
    #[cfg(test)]
    pub(super) fn mushroom_progress_for_test(&self) -> Option<MushroomProgressForTest> {
        self.mushrooms.progress_for_test()
    }
    #[cfg(test)]
    pub(super) fn portal_clock_for_test(&self, actor: ExactActorRef) -> Option<PortalClockForTest> {
        self.advanced.portal_clock_for_test(actor)
    }
    pub(super) fn critical_scope_pending(&self) -> bool {
        self.advanced.critical_scope_pending()
    }
    pub(super) fn pending_for_player(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.advanced.pending_for_player(actor, session)
    }
    #[cfg(test)]
    pub(super) fn actual_massive_commit_for_test(
        &self,
        actor: ExactActorRef,
        phase: u8,
    ) -> Option<(u32, u32, u32)> {
        self.advanced.massive_commit_for_test(actor, phase)
    }
    #[cfg(test)]
    pub(super) fn massive_state_for_test(
        &self,
        centre: MovementLocalPosition,
        actor: ExactActorRef,
    ) -> MassiveStateForTest {
        self.advanced.massive_state_for_test(pos(centre), actor)
    }
    #[cfg(test)]
    pub(super) fn actual_agony_commit_for_test(
        &self,
        actor: ExactActorRef,
    ) -> Option<(u32, u32, u32)> {
        self.advanced
            .agony_commit(actor)
            .map(|commit| (commit.before, commit.after, commit.amount))
    }
    // PARTIAL: the copied Bakragore death/loot handler will consume this
    // current-owner count; a test-only gate would hide the missing live caller.
    #[allow(
        dead_code,
        reason = "PARTIAL Rotten Blood: copied Bakragore death-loot consumer is pending"
    )]
    pub(super) fn actual_bakragore_essence_count_at_death(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
    ) -> Option<u8> {
        if self.read_context != Some(context(runtime))
            || runtime.owner_fence().is_err()
            || !runtime
                .companion_snapshot_including_dead(target)
                .is_ok_and(|s| {
                    s.health == 0
                        && s.state.master.is_none()
                        && s.state.policy.definition_key == "oteryn:creature.bakragore"
                })
            || runtime.native_summon_master(target).is_some()
        {
            return None;
        }
        self.advanced
            .actual_bakragore_essence_count(target)
            .filter(|n| *n <= 4)
    }
    pub(super) fn next_due_time(&self) -> Option<u64> {
        self.pending_due
            .into_iter()
            .chain(self.advanced.next_due_time())
            .chain(self.mushrooms.next_due_time())
            .min()
    }
    pub(super) fn has_native_work(&self) -> bool {
        !self.pending.is_empty()
            || self.advanced.next_due_time().is_some()
            || self.mushrooms.next_due_time().is_some()
    }
    pub(super) fn has_native_due(&self, now: u64) -> bool {
        self.next_due_time().is_some_and(|due| due <= now)
    }
    pub(super) fn taint(&self, actor: ExactActorRef, session: GameSessionId) -> Option<u8> {
        self.current_taints
            .iter()
            .find(|f| f.actor == actor && f.session == session)
            .map(|f| area_icons::donor_taint_level(&f.state))
    }
}
pub(super) async fn refresh_taints(
    root: &crate::durability::DurabilityRoot,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    map: &mut CurrentMonsterMap<'_, '_>,
) -> Result<(), ()> {
    let binding = context(runtime);
    if states.rotten_blood_callbacks.read_context != Some(binding) {
        states.rotten_blood_callbacks = RottenBloodCallbacks {
            read_context: Some(binding),
            ..Default::default()
        };
    }
    states.rotten_blood_callbacks.read_stamp = None;
    let census = runtime.positioned_melee_census().map_err(|_| ())?;
    let players = census
        .into_iter()
        .filter_map(|(a, _, s)| s.map(|s| (a, s)))
        .collect::<Vec<_>>();
    let facts = map
        .current_rotten_blood_taints(root, runtime, &players)
        .await
        .ok_or(())?;
    let prepared_bak = map.current_bak_preparation(runtime).await.ok_or(())?;
    let stamp = runtime.issue_owner_work().map_err(|_| ())?;
    states.rotten_blood_callbacks.prepared_bak = prepared_bak;
    states.rotten_blood_callbacks.current_taints = facts;
    states.rotten_blood_callbacks.read_stamp = Some(stamp);
    Ok(())
}
fn at(p: RoomPosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn pos(p: MovementLocalPosition) -> RoomPosition {
    RoomPosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn near(a: RoomPosition, b: RoomPosition, r: u32) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= r && a.y.abs_diff(b.y) <= r
}
fn draw(
    native: &NativeGameplayState,
    actor: ExactActorRef,
    sequence: u64,
    ordinal: u64,
    bound: u32,
) -> Result<u32, ()> {
    if bound == 0 {
        return Err(());
    }
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:rotten:source:v1")
            .chain_update(native.source_digest())
            .chain_update(actor.placement_identity())
            .finalize()
            .into(),
    );
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&sequence.to_be_bytes());
    bytes[8..].copy_from_slice(&ordinal.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let limit = u64::MAX - u64::MAX % u64::from(bound);
    for n in 0..64 {
        let value = deterministic_decision_u64(&root, occurrence, "rotten_source_draw", n)
            .map_err(|_| ())?;
        if value < limit {
            return Ok((value % u64::from(bound)) as u32);
        }
    }
    Err(())
}

/// Deferred-death barrier port. A due callback remains pending until its
/// original-deadline consumer has been qualified; this port never reserves a
/// fresh think or advances the owner with a synthetic time.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive_due_before(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::attack::ChannelAttackStates,
    notices: &super::raid_notices::ServerMessageHub,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    eligible_through_us: u64,
    now: u64,
) -> Result<Option<u64>, ()> {
    let fence = runtime.owner_fence().map_err(|_| ())?;
    if !native.qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
        || states.rotten_blood_callbacks.read_context != Some(context(runtime))
        || !states
            .rotten_blood_callbacks
            .read_stamp
            .is_some_and(|stamp| fence.accepts_stamp(stamp))
        || room.frame_binding().digest() != runtime.content_pin().frame_binding_digest()
        || room.map_revision_digest() != runtime.map_revision_digest()
    {
        return Err(());
    }
    let selected = states.rotten_blood_callbacks.next_source_due(runtime)?;
    if let Some(selected) = selected.filter(|work| work.at <= eligible_through_us && work.at <= now)
    {
        match selected.kind {
            SourceDueKind::Effects => {
                drain_pending_through(runtime, states, native, map, selected.at, now).await?
            }
            SourceDueKind::Mushroom => {
                mushroom::drive_one_due(
                    runtime,
                    states,
                    attack,
                    room,
                    native,
                    map,
                    selected.at,
                    now,
                )
                .await?
            }
            SourceDueKind::Advanced(_) => {
                advanced::drive_due_through(
                    runtime,
                    states,
                    attack,
                    notices,
                    room,
                    native,
                    map,
                    selected.at,
                    now,
                )
                .await?
            }
        }
    }
    states.rotten_blood_callbacks.next_source_due_time(runtime)
}

/// Executes only reserved source occurrences. Queued effects stay in the existing
/// owner across await/cancellation; no successful physical spawn is replayed.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::attack::ChannelAttackStates,
    notices: &super::raid_notices::ServerMessageHub,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<Option<u64>, ()> {
    if !native.qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest()) {
        return Ok(None);
    }
    let fence = runtime.owner_fence().map_err(|_| ())?;
    if states.rotten_blood_callbacks.read_context != Some(context(runtime)) {
        return Ok(None);
    }
    if !states
        .rotten_blood_callbacks
        .read_stamp
        .is_some_and(|s| fence.accepts_stamp(s))
    {
        return Ok(None);
    }
    if states
        .rotten_blood_callbacks
        .advanced
        .critical_scope_pending()
    {
        return Err(());
    }
    drain_pending(runtime, states, native, map, now).await?;
    mushroom::drive(runtime, states, attack, room, native, map, occurrences, now).await?;
    for occurrence in occurrences {
        let actor = occurrence.actor;
        let Some(key) = super::soul_war_callbacks::source_actor(runtime, native, actor) else {
            continue;
        };
        if !matches!(key.as_str(), mur::MURCION_KEY | ich::ICHGAHAL_KEY)
            || !runtime.contains_live_creature(actor)
        {
            continue;
        }
        let source_room = room.pinned_activity_source_room();
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
        if !matches!(
            (key.as_str(), source_room),
            (mur::MURCION_KEY, Some(Room::Murcion)) | (ich::ICHGAHAL_KEY, Some(Room::Ichgahal))
        ) {
            continue;
        }
        if !states
            .rotten_blood_callbacks
            .bosses
            .iter()
            .any(|b| b.actor == actor)
        {
            let source = Arc::new(
                runtime
                    .qualify_rotten_blood_spawn_source(actor, native)
                    .map_err(|_| ())?,
            );
            states.rotten_blood_callbacks.bosses.push(Boss {
                actor,
                source,
                key: key.clone(),
                last: None,
                murcion: Default::default(),
                ichgahal: Default::default(),
                wave: Vec::new(),
                next_id: 1,
            });
        }
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        let boss_position = pos(runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position());
        let center = RoomPosition {
            x: 33008,
            y: if key == mur::MURCION_KEY {
                32367
            } else {
                32334
            },
            floor: 15,
        };
        let mut maximum = 0;
        for (a, p, s) in &census {
            if let Some(session) = s
                && near(center, pos(p.position()), 15)
            {
                maximum = maximum.max(
                    states
                        .rotten_blood_callbacks
                        .taint(*a, *session)
                        .ok_or(())?,
                );
            }
        }
        let nearby = |child: &str, center: RoomPosition, range: u32| -> u8 {
            census
                .iter()
                .filter(|(a, p, s)| {
                    s.is_none()
                        && near(center, pos(p.position()), range)
                        && runtime.contains_live_creature(*a)
                        && super::soul_war_callbacks::source_actor(runtime, native, *a).as_deref()
                            == Some(child)
                })
                .count()
                .min(255) as u8
        };
        let mushrooms = nearby(
            mur::MUSHROOM_KEY,
            boss_position,
            if key == mur::MURCION_KEY { 16 } else { 14 },
        );
        let jaws = nearby(mur::ELDER_BLOODJAW_KEY, boss_position, 15);
        let owner = &mut states.rotten_blood_callbacks;
        let boss = owner
            .bosses
            .iter_mut()
            .find(|b| b.actor == actor)
            .ok_or(())?;
        if boss.last.is_some_and(|p| p.sequence >= occurrence.sequence) {
            continue;
        }
        let interval = 1000; // Crystal Monster::onThink passes the creature-check interval, not wall-clock drift.
        let mut waved = false;
        let mut interval_draw = || {
            waved = true;
            draw(native, actor, occurrence.sequence, 0, 1001).map_or(0, |n| 8000 + u64::from(n))
        };
        let effects: Vec<BossSummonChange> = if key == mur::MURCION_KEY {
            boss.murcion
                .think(
                    &mur::MurcionThinkFacts {
                        boss: boss_position,
                        interval_ms: interval,
                        nearby_mushrooms: mushrooms,
                        nearby_bloodjaws: jaws,
                        bloodjaw_target: elder::bloodjaw_target_count(maximum),
                        mechanics_disabled: false,
                        bloodjaw_spawn_paused: false,
                    },
                    &mut interval_draw,
                )
                .map_err(|_| ())?
                .into_iter()
                .map(|e| match e {
                    mur::MurcionEffect::Spawn {
                        creature_key,
                        around,
                        placement,
                    } => {
                        let (offset, close) = match placement {
                            mur::Placement::Close { offset_tiles } => (offset_tiles, true),
                            mur::Placement::Random { offset_tiles } => (offset_tiles, false),
                        };
                        (Some((creature_key, around, offset, close)), None)
                    }
                    mur::MurcionEffect::RemoveSummon { actor } => (None, Some(actor)),
                })
                .collect()
        } else {
            boss.ichgahal
                .think(
                    &ich::IchgahalThinkFacts {
                        boss: boss_position,
                        interval_ms: interval,
                        nearby_mushrooms: mushrooms,
                        nearby_bloodjaws: jaws,
                        bloodjaw_target: elder::bloodjaw_target_count(maximum),
                        mechanics_disabled: false,
                        bloodjaw_spawn_paused: false,
                    },
                    &mut interval_draw,
                )
                .map_err(|_| ())?
                .into_iter()
                .map(|e| match e {
                    ich::IchgahalEffect::Spawn {
                        creature_key,
                        around,
                        placement,
                    } => {
                        let (offset, close) = match placement {
                            ich::Placement::Close { offset_tiles } => (offset_tiles, true),
                            ich::Placement::Random { offset_tiles } => (offset_tiles, false),
                        };
                        (Some((creature_key, around, offset, close)), None)
                    }
                    ich::IchgahalEffect::RemoveSummon { actor } => (None, Some(actor)),
                })
                .collect()
        };
        let count = effects.len();
        for (index, (spawn, remove)) in effects.into_iter().enumerate() {
            if let Some((key, around, offset, close)) = spawn {
                owner.pending.push_back((
                    now,
                    Effect::Spawn {
                        source: boss.source.clone(),
                        parent: actor,
                        sequence: occurrence.sequence,
                        ordinal: index as u64 + 1,
                        key,
                        around,
                        offset,
                        close,
                        wave: waved && index + 3 >= count,
                    },
                ));
            }
            if let Some(id) = remove
                && let Some((_, actor)) = boss.wave.iter().find(|(n, _)| *n == id)
            {
                owner
                    .pending
                    .push_back((now, Effect::Remove { actor: *actor }));
                boss.wave.retain(|(n, _)| *n != id);
            }
        }
        boss.last = Some(*occurrence);
    }
    drain_pending(runtime, states, native, map, now).await?;
    let advanced_due = advanced::drive(
        runtime,
        states,
        attack,
        notices,
        room,
        native,
        map,
        occurrences,
        now,
    )
    .await?;
    Ok(advanced_due
        .into_iter()
        .chain(states.rotten_blood_callbacks.mushrooms.next_due_time())
        .min())
}

#[allow(clippy::too_many_arguments)]
async fn drain_pending(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
) -> Result<(), ()> {
    drain_pending_through(runtime, states, native, map, now, now).await
}

#[allow(clippy::too_many_arguments)]
async fn drain_pending_through(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
) -> Result<(), ()> {
    if !states.rotten_blood_callbacks.pending.is_empty() {
        states.rotten_blood_callbacks.pending_due = Some(now.saturating_add(1_000_000));
    }
    while let Some((due, effect)) = states.rotten_blood_callbacks.pending.front().cloned() {
        if due > through || due > now {
            return Ok(());
        }
        match effect {
            Effect::Remove { actor } => {
                if runtime.contains_live_creature(actor) {
                    let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                    runtime
                        .remove_current_native_encounter_subject(
                            stamp,
                            actor,
                            mur::MUSHROOM_KEY,
                            native.source_digest(),
                        )
                        .map_err(|_| ())?;
                }
            }
            Effect::Spawn {
                source,
                parent,
                sequence,
                ordinal,
                key,
                around,
                offset,
                close,
                wave,
            } => {
                if let Some(actor) = spawn_child(
                    runtime, states, native, map, &source, parent, sequence, ordinal, key, around,
                    offset, close, now,
                )
                .await?
                    && wave
                {
                    let boss = states
                        .rotten_blood_callbacks
                        .bosses
                        .iter_mut()
                        .find(|b| b.actor == parent)
                        .ok_or(())?;
                    let id = boss.next_id;
                    boss.next_id = id.checked_add(1).ok_or(())?;
                    boss.wave.push((id, actor));
                    if boss.key == mur::MURCION_KEY {
                        boss.murcion.record_wave_summon(id);
                    } else {
                        boss.ichgahal.record_wave_summon(id);
                    }
                }
            }
        }
        states.rotten_blood_callbacks.pending.pop_front();
    }
    states.rotten_blood_callbacks.pending_due = None;
    Ok(())
}

/// Source getClosePosition (ten attempts then centre) followed by the exact C++
/// default Map::placeCreature eight-neighbour fallback, sharing the current map owner.
#[allow(clippy::too_many_arguments)]
pub(super) async fn spawn_child(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &RottenBloodSpawnSource,
    parent: ExactActorRef,
    sequence: u64,
    ordinal: u64,
    child: &str,
    around: RoomPosition,
    offset: u8,
    close: bool,
    now: u64,
) -> Result<Option<ExactActorRef>, ()> {
    let mut center = at(around);
    for attempt in 0..if close { 10 } else { 1 } {
        let bound = 2 * u32::from(offset) + 1;
        let n = ordinal
            .checked_mul(1024)
            .and_then(|n| n.checked_add(attempt * 2))
            .ok_or(())?;
        let candidate = MovementLocalPosition {
            x: center.x + draw(native, parent, sequence, n, bound)? as i32 - i32::from(offset),
            y: center.y + draw(native, parent, sequence, n + 1, bound)? as i32 - i32::from(offset),
            floor: center.floor,
        };
        map.read(runtime, candidate).await;
        if !close || map.source_is_walkable(runtime, candidate).ok_or(())? {
            center = candidate;
            break;
        }
    }
    let mut n = 0;
    let plan = oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan::prepare(
        center,
        false,
        false,
        |bound| {
            n += 1;
            draw(
                native,
                parent,
                sequence,
                ordinal.saturating_mul(1024).saturating_add(64 + n),
                bound,
            )
            .unwrap_or(bound)
        },
    )
    .map_err(|_| ())?;
    for p in plan.candidates() {
        map.read(runtime, *p).await;
        map.source_placement_cell(runtime, child, *p).ok_or(())?;
    }
    if !map.validate_transaction(runtime).await {
        return Err(());
    }
    let Some(destination) = plan
        .select(
            |p| map.source_placement_cell(runtime, child, p),
            |_, _| Some(true),
        )
        .map_err(|_| ())?
    else {
        return Ok(None);
    };
    let actor =
        match runtime.spawn_rotten_blood_child_from_source(source, child, destination, |r, p| {
            map.view()
                .matches(r)
                .then(|| map.source_placement_cell(r, child, p).map(|c| c.query_add))
                .flatten()
        }) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == child)
        .ok_or(())?;
    if runtime.install_creature_policy(actor, child).is_err()
        || states
            .monster_melee
            .creature_ai
            .admit_native_creature(runtime, native, actor, &record.profile.target, now)
            .is_err()
    {
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        runtime
            .remove_current_native_encounter_subject(stamp, actor, child, native.source_digest())
            .map_err(|_| ())?;
        return Err(());
    }
    Ok(Some(actor))
}
