//! Crystal soul_war.lua1398–1452: one shared wall-second gate and a captured
//! Player-only cohort. Retrying an owner turn never repeats completed recipients.
use super::*;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::gameplay_transport::raid_notices::ServerMessageHub;
use oteryn_gameplay_encounters::bosses::soul_war_torment::{Cadence, Warning};

type Binding = (
    RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
    [u8; 32],
    [u8; 32],
    [u8; 32],
    [u8; 32],
    u64,
);
fn binding(runtime: &ChannelRuntimeV1) -> Binding {
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
        runtime.content_pin().server_artifact_digest(),
        runtime.content_pin().client_artifact_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.map_revision_digest(),
        runtime.content_pin().activation_sequence(),
    )
}

/// Authority retained at the actual native callback, not reconstructed from a
/// later source incarnation. Target HP/counters still use their current owner.
#[derive(Debug)]
pub(in crate::gameplay_transport) struct PreparedPulse {
    binding: Binding,
    source: ExactActorRef,
    maximum: u8,
    megalomania_present: bool,
    now: u64,
    targets: Vec<(ExactActorRef, GameSessionId)>,
}
impl PreparedPulse {
    pub(in crate::gameplay_transport) fn capture(
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        source: ExactActorRef,
        now: u64,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        if !qualifies_source(runtime, native)
            || !matches!(
                runtime.binding().runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
            || runtime.companion_snapshot(source).ok()?.health == 0
        {
            return None;
        }
        let key = source_actor(runtime, native, source)?;
        let (maximum, center, _) = source_parameters(&key)?;
        let targets = runtime
            .positioned_melee_census()
            .ok()?
            .into_iter()
            .filter_map(|(actor, at, session)| {
                let position = at.position();
                session
                    .filter(|session| runtime.player_control_facts(actor, *session).is_ok())
                    .filter(|_| {
                        position.floor == center.floor
                            && position.x.abs_diff(center.x) <= 15
                            && position.y.abs_diff(center.y) <= 15
                    })
                    .map(|session| (actor, session))
            })
            .collect();
        Some(Self {
            binding: binding(runtime),
            source,
            maximum,
            megalomania_present: runtime.soul_war_megalomania_name_present().ok()?,
            now,
            targets,
        })
    }
    pub(in crate::gameplay_transport) fn current_target(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.binding == binding(runtime)
            && runtime.owner_fence().is_ok()
            && qualifies_source(runtime, native)
            && self.targets.contains(&(target, session))
            && runtime.player_control_facts(target, session).is_ok()
    }
    pub(in crate::gameplay_transport) fn source(&self) -> ExactActorRef {
        self.source
    }
    pub(in crate::gameplay_transport) fn maximum(&self) -> u8 {
        self.maximum
    }
    pub(in crate::gameplay_transport) fn megalomania_present(&self) -> bool {
        self.megalomania_present
    }
    pub(in crate::gameplay_transport) fn now(&self) -> u64 {
        self.now
    }
}

fn source_parameters(key: &str) -> Option<(u8, MovementLocalPosition, u64)> {
    let (maximum, x, y, interval) = match key {
        "oteryn:creature.symbol_of_hatred" => (30, 33744, 31599, 3_000),
        "oteryn:creature.goshnar_s_megalomania_purple"
        | "oteryn:creature.goshnar_s_megalomania_green"
        | "oteryn:creature.goshnar_s_megalomania_blue" => (36, 33710, 31634, 10_000),
        _ => return None,
    };
    Some((maximum, MovementLocalPosition { x, y, floor: 14 }, interval))
}
#[derive(Debug)]
struct PendingPulse {
    occurrence: ThinkOccurrence,
    pulse: PreparedPulse,
    cursor: usize,
}
#[derive(Debug, Default)]
pub(super) struct TormentCallbacks {
    binding: Option<Binding>,
    clock: Cadence,
    last: Vec<(ExactActorRef, u64)>,
    pending: Option<PendingPulse>,
}
impl TormentCallbacks {
    fn reserve_source(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<(), ()> {
        if self.last.iter().any(|(a, _)| *a == actor) {
            return Ok(());
        }
        self.last
            .retain(|(a, _)| runtime.source_creature_is_registered(*a).unwrap_or(false));
        if self.last.len() >= runtime.actor_slot_capacity() {
            return Err(());
        }
        self.last.try_reserve(1).map_err(|_| ())?;
        self.last.push((actor, 0));
        Ok(())
    }
    fn complete(&mut self, occurrence: ThinkOccurrence) -> Result<(), ()> {
        let entry = self
            .last
            .iter_mut()
            .find(|(a, _)| *a == occurrence.actor)
            .ok_or(())?;
        entry.1 = occurrence.sequence;
        Ok(())
    }
    fn finish(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        messages: &ServerMessageHub,
        execution_now: u64,
    ) -> Result<(), ()> {
        let Some(pending) = &mut self.pending else {
            return Ok(());
        };
        if pending.pulse.binding != binding(runtime) {
            return Err(());
        }
        while let Some(&(actor, session)) = pending.pulse.targets.get(pending.cursor) {
            if runtime.player_control_facts(actor, session).is_ok() {
                let pulse = states
                    .apply_retained_soul_war_torment_pulse(
                        runtime,
                        native,
                        &pending.pulse,
                        actor,
                        session,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            execution_now,
                        ),
                    )
                    .ok_or(())?;
                if pulse.effect
                    && let Ok(at) = runtime.read_actor_position(actor)
                {
                    // Active client-ids.json: CONST_ME_PINK_ENERGY_SPARK = 179.
                    let _ = super::super::soul_war_scene::emit_effect(room, at.position(), 179);
                }
                if let Some(warning) = pulse.warning {
                    let message = match warning {
                        Warning::Begins => {
                            "The dread starts to torment you! Don't let dread level reach critical value!"
                        }
                        Warning::Unbearable => "The dread's torment becomes unbearable!",
                        Warning::Tearing => "The Dread's torment begins to tear you apart!",
                        Warning::Killing => "The dread's torment is killing you!",
                        Warning::Lethal => "The dread's torment is now lethal!",
                    };
                    messages.send_current(
                        session,
                        oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(
                            message.into(),
                        ),
                    );
                }
            }
            pending.cursor += 1;
        }
        let occurrence = pending.occurrence;
        self.complete(occurrence)?;
        self.pending = None;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    messages: &ServerMessageHub,
    now: u64,
    execution_now: u64,
) -> Result<(), ()> {
    use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Role;
    if !matches!(
        room.pinned_activity_source_room(),
        Some(Role::GoshnarsHatred | Role::GoshnarsMegalomania)
    ) {
        return Ok(());
    }
    if !qualifies_source(runtime, native)
        || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || !map.validate_transaction(runtime).await
    {
        return Err(());
    }
    if states
        .soul_war_callbacks
        .torment
        .binding
        .is_some_and(|old| old != binding(runtime))
    {
        return Err(());
    }
    states.soul_war_callbacks.torment.binding = Some(binding(runtime));
    finish_pending(runtime, states, room, native, messages, execution_now)?;
    for &occurrence in occurrences {
        if states
            .soul_war_callbacks
            .torment
            .last
            .iter()
            .any(|(actor, sequence)| *actor == occurrence.actor && *sequence >= occurrence.sequence)
        {
            continue;
        }
        let Some(key) = source_actor(runtime, native, occurrence.actor) else {
            continue;
        };
        let Some((_, _, interval)) = source_parameters(&key) else {
            continue;
        };
        if !runtime
            .companion_snapshot(occurrence.actor)
            .is_ok_and(|s| s.health > 0)
        {
            continue;
        }
        states
            .soul_war_callbacks
            .torment
            .reserve_source(runtime, occurrence.actor)?;
        // State is retained in ChannelSpellStates across the awaited current
        // clock read. Cancellation cannot discard the prior cohort/cadence.
        let wall_ms = map
            .source_observed_seconds(runtime)
            .await
            .ok_or(())?
            .checked_mul(1000)
            .ok_or(())?;
        let mut clock = states.soul_war_callbacks.torment.clock;
        if !clock.claim(wall_ms, interval) {
            states.soul_war_callbacks.torment.complete(occurrence)?;
            continue;
        }
        let pulse = PreparedPulse::capture(runtime, native, occurrence.actor, now).ok_or(())?;
        states.soul_war_callbacks.torment.pending = Some(PendingPulse {
            occurrence,
            pulse,
            cursor: 0,
        });
        states.soul_war_callbacks.torment.clock = clock;
        finish_pending(runtime, states, room, native, messages, execution_now)?;
    }
    Ok(())
}

fn finish_pending(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    messages: &ServerMessageHub,
    execution_now: u64,
) -> Result<(), ()> {
    // This temporary extraction contains no await or early return before restore.
    let mut owner = std::mem::take(&mut states.soul_war_callbacks.torment);
    let result = owner.finish(runtime, states, room, native, messages, execution_now);
    states.soul_war_callbacks.torment = owner;
    result
}
