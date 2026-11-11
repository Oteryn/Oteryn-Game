//! The real Greed callbacks retain their original input and unfinished statements.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{
    GameSessionId, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::monster_profile_cycle::retained::PendingProfilePass;
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal;
use std::collections::VecDeque;
use std::sync::Arc;

mod effects;
#[cfg(test)]
mod retention_test;
mod spawn;

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
    let pin = runtime.content_pin();
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
        pin.server_artifact_digest(),
        pin.client_artifact_digest(),
        pin.frame_binding_digest(),
        runtime.map_revision_digest(),
        pin.activation_sequence(),
    )
}

#[derive(Debug)]
enum Origin {
    Timer,
    Think {
        pass: Arc<PendingProfilePass>,
        occurrence: ThinkOccurrence,
    },
}

#[derive(Debug, Clone, Copy)]
struct SphereRead {
    instance: u64,
    position: RoomPosition,
    west: MovementLocalPosition,
}

#[derive(Debug)]
struct Pending {
    origin: Origin,
    original_due: u64,
    sampled_at: u64,
    boss: ExactActorRef,
    players: Vec<(ExactActorRef, GameSessionId)>,
    effects: VecDeque<GreedEffect>,
    sphere: Option<SphereRead>,
    #[cfg(test)]
    committed: usize,
}

#[derive(Debug, Default)]
pub(super) struct Retention {
    context: Option<(Binding, RuntimeWorkStamp)>,
    pending: Option<Pending>,
    #[cfg(test)]
    pause_after_spawn: Option<(&'static str, Arc<tokio::sync::Notify>)>,
    #[cfg(test)]
    pause_before_think: Option<(ExactActorRef, Arc<tokio::sync::Notify>)>,
    #[cfg(test)]
    pause_after_health: Option<Arc<tokio::sync::Notify>>,
}
impl Retention {
    pub(super) fn capture(runtime: &mut ChannelRuntimeV1) -> Result<Self, ()> {
        Ok(Self {
            context: Some((
                binding(runtime),
                runtime.issue_owner_work().map_err(|_| ())?,
            )),
            ..Self::default()
        })
    }
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.context.is_some_and(|(pin, stamp)| {
            binding(runtime) == pin
                && runtime
                    .owner_fence()
                    .is_ok_and(|fence| fence.accepts_stamp(stamp))
        })
    }
    pub(super) fn next_due_us(&self, clock: &greed_model::GoshnarsGreedRoom) -> Option<u64> {
        self.pending
            .as_ref()
            .map(|pending| pending.original_due)
            .into_iter()
            .chain(clock.next_due_ms().and_then(|ms| ms.checked_mul(1_000)))
            .min()
    }
}

pub(in crate::gameplay_transport) fn next_source_due_time(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
) -> Result<Option<u64>, ()> {
    let owner = &states.soul_war_callbacks;
    let Some(due) = owner.greed_work.next_due_us(&owner.greed) else {
        return Ok(None);
    };
    if !owner.greed_work.current(runtime) {
        return Err(());
    }
    if let Some(Pending {
        origin: Origin::Think { pass, .. },
        original_due,
        ..
    }) = &owner.greed_work.pending
        && (states
            .native_profile_pass
            .pending()
            .is_none_or(|current| !Arc::ptr_eq(pass, &current))
            || states.native_profile_pass.next_due(runtime)? != Some(*original_due))
    {
        return Err(());
    }
    Ok(Some(due))
}

fn current_context(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
) -> bool {
    matches!(
        runtime.binding().runtime_scope(),
        RuntimeScopeRefV1::Instance { .. }
    ) && room.pinned_activity_source_room() == Some(PinnedActivitySourceRoom::GoshnarsGreed)
        && runtime.content_pin().frame_binding_digest() == room.frame_binding().digest()
        && runtime.map_revision_digest() == room.map_revision_digest()
        && states.soul_war_callbacks.qualified_content == Some(native.source_digest())
        && states.soul_war_callbacks.greed_work.current(runtime)
        && qualifies_source(runtime, native)
}

/// These copied Lua handlers only update room counters and schedule addEvent.
/// Their mutation and the caller's canonical cursor ACK have no intervening await.
#[allow(
    clippy::too_many_arguments,
    reason = "the existing copied death seal, event and current source owners must be checked together"
)]
pub(in crate::gameplay_transport) fn consume_death(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    begin: &SourceCreatureDeathSeal,
    event: &crate::content::QualifiedNativeDeathEvent,
    action_now_us: u64,
) -> Result<bool, ()> {
    let (minion, sphere) = match event.name() {
        "GreedMonsterDeath" => (
            Some(GreedMinion::from_creature_key(begin.definition_key()).ok_or(())?),
            false,
        ),
        "SoulSphereDeath" if begin.definition_key() == greed_model::SOUL_SPHERE => (None, true),
        "SoulSphereDeath" => return Err(()),
        _ => return Ok(false),
    };
    if !current_context(runtime, states, room, native)
        || !begin.current_for(runtime)
        || runtime
            .source_creature_death_event_index(begin)
            .map_err(|_| ())?
            .and_then(|index| begin.events().get(index))
            != Some(event)
    {
        return Err(());
    }
    let owner = &mut states.soul_war_callbacks;
    // An actor may have died before a census observed it. The actual copied
    // event proves this death; adding its local association must not run onSpawn.
    let index = match owner
        .observed
        .iter()
        .position(|entry| entry.actor == begin.actor())
    {
        Some(index) => index,
        None => {
            owner.next_id = owner.next_id.checked_add(1).ok_or(())?;
            owner.observed.push(Observed {
                actor: begin.actor(),
                id: owner.next_id,
                minion,
                sphere,
                death_observed: false,
                death_at_ms: std::sync::Mutex::new(None),
                last_think: None,
            });
            owner.observed.len() - 1
        }
    };
    let entry = &mut owner.observed[index];
    if entry.minion != minion || entry.sphere != sphere {
        return Err(());
    }
    if !entry.death_observed {
        if let Some(minion) = minion {
            owner
                .greed
                .minion_died(entry.id, minion, action_now_us / 1_000);
        } else {
            owner.greed.sphere_died(entry.id);
        }
        entry.death_observed = true;
        *entry
            .death_at_ms
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
    Ok(true)
}

/// Advances only actual retained callbacks selected by their original deadline.
/// Current reads and physical writes keep the real execution clock.
#[allow(
    clippy::too_many_arguments,
    reason = "source eligibility cutoff is independent from the current physical execution clock"
)]
pub(in crate::gameplay_transport) async fn drive_due_through(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    execution_now: u64,
) -> Result<(), ()> {
    if next_source_due_time(runtime, states)?.is_none() {
        return Ok(());
    }
    if !current_context(runtime, states, room, native) {
        return Err(());
    }
    let through = through.min(execution_now);
    while next_source_due_time(runtime, states)?.is_some_and(|due| due <= through) {
        let owner = &mut states.soul_war_callbacks;
        if owner.greed_work.pending.is_none() {
            let boss = owner.boss.ok_or(())?;
            let (due, effect) = owner
                .greed
                .take_next_due_through(through / 1_000)
                .ok_or(())?;
            owner.greed_work.pending = Some(Pending {
                origin: Origin::Timer,
                original_due: due.checked_mul(1_000).ok_or(())?,
                sampled_at: execution_now,
                boss,
                players: Vec::new(),
                effects: [effect].into(),
                sphere: None,
                #[cfg(test)]
                committed: 0,
            });
        }
        effects::drain(runtime, states, room, native, map, execution_now).await?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the ordinary source callback receives the existing native Think batch and current owners"
)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    execution_now: u64,
) -> Result<(), ()> {
    if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsGreed)
        || !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        )
    {
        return Ok(());
    }
    if !current_context(runtime, states, room, native) {
        return Err(());
    }
    let boss = states.soul_war_callbacks.boss.ok_or(())?;
    let pass = states.native_profile_pass.pending().ok_or(())?;
    let original_due = states.native_profile_pass.next_due(runtime)?.ok_or(())?;
    drive_due_through(
        runtime,
        states,
        room,
        native,
        map,
        original_due,
        execution_now,
    )
    .await?;

    for (actor, position, session) in runtime.positioned_actor_census().map_err(|_| ())? {
        if session.is_some()
            || !in_greed(position.position())
            || !runtime.contains_live_creature(actor)
        {
            continue;
        }
        let Some(key) = source_actor(runtime, native, actor) else {
            continue;
        };
        let minion = GreedMinion::from_creature_key(&key);
        let sphere = key == greed_model::SOUL_SPHERE;
        if minion.is_some() || sphere || actor == boss {
            observe(&mut states.soul_war_callbacks, actor, minion, sphere)?;
        }
    }
    for &occurrence in occurrences {
        let Some(index) = states.soul_war_callbacks.observed.iter().position(|entry| {
            entry.actor == occurrence.actor
                && !entry.death_observed
                && entry.last_think != Some(occurrence)
        }) else {
            continue;
        };
        if !runtime.contains_live_creature(occurrence.actor) {
            continue;
        }
        #[cfg(test)]
        if let Some(notify) = retention_test::before_think(states, occurrence) {
            notify.notify_one();
            std::future::pending::<()>().await;
        }
        let players = runtime
            .positioned_actor_census()
            .map_err(|_| ())?
            .into_iter()
            .filter_map(|(actor, position, session)| {
                session
                    .filter(|_| in_greed(position.position()) && !states.is_dead(actor))
                    .map(|session| (actor, session))
            })
            .collect::<Vec<_>>();
        let owner = &mut states.soul_war_callbacks;
        let entry = &owner.observed[index];
        let mut sphere = None;
        let interval = crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS;
        let effects = if occurrence.actor == boss {
            owner.greed.boss_think(interval)
        } else if let Some(minion) = entry.minion {
            let player_ids = players
                .iter()
                .map(|(actor, _)| u64::from(actor.0.actor_local_id.0))
                .collect::<Vec<_>>();
            let mut ordinal = 0;
            owner
                .greed
                .minion_think(
                    entry.id,
                    minion,
                    interval,
                    execution_now / 1_000,
                    &player_ids,
                    || {
                        ordinal += 1;
                        soul_damage(
                            native.source_digest(),
                            occurrence.actor,
                            occurrence.sequence,
                            ordinal,
                        )
                    },
                )
                .map_err(|_| ())?
        } else if entry.sphere {
            let position = runtime
                .read_actor_position(occurrence.actor)
                .map_err(|_| ())?
                .position();
            sphere = Some(SphereRead {
                instance: entry.id,
                position: RoomPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                west: MovementLocalPosition {
                    x: position.x.checked_sub(1).ok_or(())?,
                    ..position
                },
            });
            Vec::new()
        } else {
            Vec::new()
        };
        // Retain the exact original occurrence and all remaining statements
        // synchronously beside its room clock/last_think advancement.
        owner.greed_work.pending = Some(Pending {
            origin: Origin::Think {
                pass: Arc::clone(&pass),
                occurrence,
            },
            original_due,
            sampled_at: execution_now,
            boss,
            players,
            effects: effects.into(),
            sphere,
            #[cfg(test)]
            committed: 0,
        });
        owner.observed[index].last_think = Some(occurrence);
        effects::drain(runtime, states, room, native, map, execution_now).await?;
    }
    states.soul_war_callbacks.observed.retain(|entry| {
        runtime
            .companion_snapshot_including_dead(entry.actor)
            .is_ok()
    });
    Ok(())
}
