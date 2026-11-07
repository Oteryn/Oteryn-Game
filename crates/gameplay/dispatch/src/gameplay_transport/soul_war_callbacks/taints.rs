//! Current Quest DATA for the source callbacks; no independent HP or taint state.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::content::{PublishedWorldSourceFrame, QualifiedPublishedWorldContext};
use crate::durability::DurabilityRoot;
use crate::durability::quest_state::soul_war::CurrentSoulWarTaintFacts;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};

pub(super) type Binding = (
    RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
    [u8; 32],
    [u8; 32],
    [u8; 32],
);
#[derive(Debug, Clone, Copy)]
pub(super) enum Geometry {
    Boss(Room),
    World(PublishedWorldSourceFrame),
}
impl Geometry {
    pub(super) fn source_position(self, p: MovementLocalPosition) -> Option<RoomPosition> {
        match self {
            Self::Boss(_) => Some(RoomPosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            }),
            Self::World(frame) => {
                let [x, y, z] = frame.source_position((p.x, p.y, p.floor))?;
                Some(RoomPosition {
                    x,
                    y,
                    floor: i16::try_from(z).ok()?,
                })
            }
        }
    }
    pub(super) fn physical_position(self, p: RoomPosition) -> Option<MovementLocalPosition> {
        match self {
            Self::Boss(_) => Some(MovementLocalPosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            }),
            Self::World(frame) => {
                let (x, y, floor) = frame.physical_position([p.x, p.y, i32::from(p.floor)])?;
                Some(MovementLocalPosition { x, y, floor })
            }
        }
    }
    pub(super) fn contains(self, p: MovementLocalPosition) -> bool {
        match self {
            Self::Boss(room) => in_boss(room, p),
            Self::World(_) => self.source_position(p).is_some_and(
                oteryn_gameplay_encounters::soul_war_taint_teleport::in_soul_war_hunting_zone,
            ),
        }
    }
}
#[derive(Debug, Default)]
pub(super) struct CurrentTaints {
    context: Option<(Binding, Geometry)>,
    observed_at: Option<u64>,
    players: Vec<CurrentSoulWarTaintFacts>,
}
pub(super) fn binding(runtime: &ChannelRuntimeV1) -> Binding {
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
        runtime.content_pin().server_artifact_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.map_revision_digest(),
    )
}
pub(super) fn boss_room(room: Room) -> bool {
    matches!(
        room,
        Room::GoshnarsGreed
            | Room::GoshnarsMalice
            | Room::GoshnarsSpite
            | Room::GoshnarsHatred
            | Room::GoshnarsCruelty
            | Room::GoshnarsMegalomania
    )
}
pub(super) fn in_boss(room: Room, p: MovementLocalPosition) -> bool {
    let (x0, y0, x1, y1, z) = match room {
        Room::GoshnarsGreed => (33737, 31658, 33755, 31673, 14),
        Room::GoshnarsMalice => (33699, 31590, 33718, 31607, 14),
        Room::GoshnarsSpite => (33734, 31624, 33751, 31640, 14),
        Room::GoshnarsHatred => (33735, 31592, 33751, 31606, 14),
        Room::GoshnarsCruelty => (33847, 31858, 33864, 31874, 7),
        Room::GoshnarsMegalomania => (33701, 31626, 33719, 31642, 14),
        _ => return false,
    };
    p.floor == z && (x0..=x1).contains(&p.x) && (y0..=y1).contains(&p.y)
}
impl CurrentTaints {
    pub(super) fn for_room(runtime: &ChannelRuntimeV1, room: &QualifiedNativeEntryRoom) -> Self {
        let context = room
            .pinned_activity_source_room()
            .filter(|r| boss_room(*r))
            .filter(|_| {
                runtime.content_pin().frame_binding_digest() == room.frame_binding().digest()
                    && runtime.map_revision_digest() == room.map_revision_digest()
            })
            .map(|r| (binding(runtime), Geometry::Boss(r)));
        Self {
            context,
            ..Default::default()
        }
    }
    pub(super) fn geometry(&self, runtime: &ChannelRuntimeV1) -> Option<Geometry> {
        self.context
            .filter(|(expected, _)| *expected == binding(runtime))
            .map(|(_, geometry)| geometry)
    }
    pub(super) fn current_players(
        &self,
        runtime: &ChannelRuntimeV1,
        now: u64,
    ) -> &[CurrentSoulWarTaintFacts] {
        if self
            .context
            .is_some_and(|(expected, _)| expected == binding(runtime))
            && self.observed_at == Some(now)
        {
            &self.players
        } else {
            &[]
        }
    }
    fn third_policy(
        &self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        now: u64,
    ) -> Option<QualifiedThirdTaintPolicy> {
        let position = runtime.read_actor_position(target).ok()?.position();
        let mut current = None;
        let mut role = None;
        if let Some((expected, room)) = self.context {
            if expected != binding(runtime) {
                return None;
            }
            role = Some(room);
            if self.observed_at == Some(now) {
                current = self
                    .players
                    .iter()
                    .find(|facts| {
                        facts.actor == target
                            && runtime.player_control_facts(target, facts.session).is_ok()
                    })
                    .map(|facts| (facts.session, facts.count));
            }
            if room.contains(position)
                && runtime.contains_live_creature(issuer)
                && current.is_none()
            {
                return None;
            }
        }
        Some(QualifiedThirdTaintPolicy {
            binding: binding(runtime),
            issuer,
            target,
            now,
            role,
            current,
        })
    }
}

// Physical DATA only: the caller still proves the source geometry and each
// canonical current Character in the independently owned SQL transaction.
pub(in crate::gameplay_transport) fn source_player_census(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
) -> Result<Vec<(ExactActorRef, GameSessionId)>, ()> {
    Ok(runtime
        .positioned_melee_census()
        .map_err(|_| ())?
        .into_iter()
        .filter_map(|(actor, _, session)| {
            if states.is_dead(actor) {
                return None;
            }
            session
                .filter(|session| runtime.player_control_facts(actor, *session).is_ok())
                .map(|session| (actor, session))
        })
        .collect())
}

#[allow(
    clippy::too_many_arguments,
    reason = "one source turn borrows current authority, native map and published World proof"
)]
pub(in crate::gameplay_transport) async fn refresh(
    root: &DurabilityRoot,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
    published: Option<&QualifiedPublishedWorldContext>,
) -> Result<(), ()> {
    let mut next = CurrentTaints::for_room(runtime, room);
    if next.context.is_none()
        && let Some(proof) = published.filter(|proof| proof.matches(room, runtime.content_pin()))
    {
        next.context = Some((binding(runtime), Geometry::World(proof.source_frame())));
    }
    if next.context.is_none() {
        states.soul_war_callbacks.taints = next;
        return Ok(());
    }
    if !qualifies_source(runtime, native) {
        // No source rule was qualified: leave ordinary combat independent of
        // this optional context. A qualified encounter with lost authority
        // instead retains its geometry and refuses only its source hit below.
        next.context = None;
        states.soul_war_callbacks.taints = next;
        return Ok(());
    }
    let actors = source_player_census(runtime, states)?
        .into_iter()
        .filter(|(actor, _)| {
            runtime.read_actor_position(*actor).is_ok_and(|position| {
                next.context
                    .is_some_and(|(_, geometry)| geometry.contains(position.position()))
            })
        })
        .collect::<Vec<_>>();
    let Some(players) = map.current_soul_war_taints(root, runtime, &actors).await else {
        states.soul_war_callbacks.taints = next;
        return Ok(());
    };
    next.observed_at = Some(now);
    next.players = players;
    if let Some((_, geometry)) = next.context {
        super::player_source::publish_reflection_sources(
            runtime,
            states,
            geometry,
            &next.players,
            now,
        )?;
    }
    states.soul_war_callbacks.taints = next;
    Ok(())
}

/// A private-construction numerical policy, copied into the current source
/// reader before its mutable HP commit. It grants no actor, damage or SQL write.
#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct QualifiedThirdTaintPolicy {
    binding: Binding,
    issuer: ExactActorRef,
    target: ExactActorRef,
    now: u64,
    role: Option<Geometry>,
    current: Option<(GameSessionId, u8)>,
}
impl QualifiedThirdTaintPolicy {
    #[allow(
        clippy::too_many_arguments,
        reason = "existing component policy receives the exact opaque retained callback stage"
    )]
    pub(in crate::gameplay_transport) fn apply_retained(
        &self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        now: u64,
        raw: u32,
        caster: &oteryn_gameplay_entities::NativeRetainedCallbackCaster,
        stage: &crate::content::native_monster_callbacks::QualifiedNativeCallbackStage,
    ) -> Option<u32> {
        if !caster.matches_stage(runtime, issuer, stage) {
            return None;
        }
        self.apply_current(runtime, issuer, target, now, raw, true)
    }
    pub(in crate::gameplay_transport) fn apply(
        &self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        now: u64,
        raw: u32,
    ) -> Option<u32> {
        self.apply_current(runtime, issuer, target, now, raw, false)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one current component policy preserves ordinary and sealed retained issuer validation"
    )]
    fn apply_current(
        &self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        now: u64,
        raw: u32,
        retained: bool,
    ) -> Option<u32> {
        if runtime.owner_fence().is_err()
            || self.binding != binding(runtime)
            || self.issuer != issuer
            || self.target != target
            || self.now != now
            || (!retained && !runtime.borrow_exact_actor_lookup().contains(issuer))
            || !runtime.borrow_exact_actor_lookup().contains(target)
        {
            return None;
        }
        let Some(room) = self.role else {
            return Some(raw);
        };
        if !room.contains(runtime.read_actor_position(target).ok()?.position())
            || (!retained && !runtime.contains_live_creature(issuer))
        {
            return Some(raw);
        }
        let (session, count) = self.current?;
        runtime.player_control_facts(target, session).ok()?;
        if count < 3 {
            return Some(raw);
        }
        // Signed Lua -n+ceil(-n*.15) means n+floor(n*.15), before block/armor.
        u32::try_from(u64::from(raw) + u64::from(raw) * 15 / 100).ok()
    }
}
pub(in crate::gameplay_transport) fn third_policy(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    issuer: ExactActorRef,
    target: ExactActorRef,
    now: u64,
) -> Option<QualifiedThirdTaintPolicy> {
    states
        .soul_war_callbacks
        .taints
        .third_policy(runtime, issuer, target, now)
}

#[cfg(test)]
#[path = "taints_retained_tests.rs"]
mod retained_tests;
