//! Own-actor cooldowns captured at the state installation boundary.
use super::actor_spell::{ActorVitals, ChannelSpellStates};
use super::connection::AdmittedSession;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::{Cooldowns, SpellBook, cast::PlayerSpellState};
use oteryn_protocol_oteryn::presentation as wire;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

type Key = (wire::ActorCooldownKind, u32);
pub type Vitals = (u64, ActorVitals);
pub type JoinVitals = (Option<Vitals>, Option<CooldownFeed>);

/// One transport's receipt mailbox. The owner coalesces changes by catalogue key,
/// so a stalled connection cannot grow an unbounded queue.
#[derive(Debug)]
pub struct CooldownFeed {
    snapshot: BTreeMap<Key, u64>,
    pending: Arc<Mutex<Option<CommittedCooldowns>>>,
    connection_generation: u64,
}

#[derive(Debug, Default)]
pub(super) struct CommittedCooldowns {
    pub(super) deadlines: Option<BTreeMap<Key, u64>>,
    pub(super) rescued: Option<Vitals>,
}

#[derive(Debug)]
pub(super) struct Subscriber {
    fence: CurrentCharacterItemFence,
    spell_indices: BTreeMap<String, u32>,
    pending: Weak<Mutex<Option<CommittedCooldowns>>>,
    sparse: bool,
}

impl CooldownFeed {
    pub(super) fn take_snapshot(&mut self) -> Option<BTreeMap<Key, u64>> {
        let pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.as_ref()?;
        Some(std::mem::take(&mut self.snapshot))
    }

    pub(super) fn take(
        &self,
        connection_generation: u64,
        cast_revision: Option<u64>,
    ) -> Option<CommittedCooldowns> {
        if self.connection_generation != connection_generation {
            return None;
        }
        let mut mailbox = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let pending = mailbox.as_mut()?;
        // A rescue at or after the paid cast can satisfy its post-result
        // vitals requirement. Leave the whole receipt for the spell follow-up.
        if let Some(cast_revision) = cast_revision
            && pending
                .rescued
                .is_some_and(|(revision, _)| revision >= cast_revision)
        {
            return None;
        }
        Some(std::mem::take(pending))
    }
}

impl Subscriber {
    fn invalidate(&self) {
        if let Some(mailbox) = self.pending.upgrade() {
            *mailbox.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    fn keys(&self, cooldowns: &Cooldowns) -> BTreeMap<Key, u64> {
        static GROUPS: OnceLock<Vec<String>> = OnceLock::new();
        let groups = GROUPS.get_or_init(|| {
            let value: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/spell-authoring/cooldown-groups.json"
            ))
            .unwrap_or_default();
            value["groups"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|group| group["key"].as_str().map(str::to_owned))
                .collect()
        });
        cooldowns
            .spells
            .iter()
            .filter_map(|(key, expiry)| {
                Some((
                    (
                        wire::ActorCooldownKind::Spell,
                        *self.spell_indices.get(key)?,
                    ),
                    expiry.get(),
                ))
            })
            .chain(cooldowns.groups.iter().filter_map(|(key, expiry)| {
                let ordinal = groups.iter().position(|known| known == key)? + 1;
                Some((
                    (wire::ActorCooldownKind::Group, u32::try_from(ordinal).ok()?),
                    expiry.get(),
                ))
            }))
            .collect()
    }
}

/// The connection's admission fence is compared with independent runtime custody.
fn current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> bool {
    let session = fence.game_session_id;
    runtime.owner_fence().is_ok()
        && if let Ok(house) = runtime.house_actor_state(actor, session) {
            house.scope == fence.runtime_scope
                && house.generation == fence.scope_ownership_generation
                && house.lease.character_id().as_bytes() == fence.character_id.as_bytes()
                && house.lease.generation() == fence.character_lease_generation
                && runtime
                    .house_player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
        } else {
            let binding = runtime.binding();
            binding.runtime_scope() == fence.runtime_scope
                && binding.scope_generation() == fence.scope_ownership_generation
                && runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                && runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
        }
}

impl ChannelSpellStates {
    pub(super) fn replace_cooldown_transport(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
    ) {
        let key = actor.placement_identity();
        if self
            .cooldown_generations
            .get(&key)
            .is_some_and(|(held_actor, held_session, current)| {
                *held_actor == actor && *held_session == session && *current >= generation
            })
        {
            // A durable recovery receipt can replay for the same transport.
            return;
        }
        self.cooldown_generations
            .insert(key, (actor, session, generation));
        if let Some(subscriber) = self.cooldown_subscribers.remove(&key) {
            subscriber.invalidate();
        }
    }

    pub(super) fn current_cooldown_transport(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
    ) -> bool {
        let current = self
            .cooldown_generations
            .get(&actor.placement_identity())
            .filter(|(held_actor, held_session, _)| {
                *held_actor == actor && *held_session == session
            })
            .map_or(
                super::connection::SessionContinuity::FRESH.connection_generation,
                |(_, _, generation)| *generation,
            );
        generation == current
    }

    pub(super) fn subscribe_cooldowns(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        admitted: AdmittedSession,
        book: &SpellBook,
    ) -> Option<CooldownFeed> {
        let fence = admitted.item_fence?;
        let session = admitted.game_session_id;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_PRESENTATION_V1)
            || fence.game_session_id != session
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || !self.current_cooldown_transport(actor, session, fence.connection_generation.get())
            || !current(runtime, actor, fence)
        {
            return None;
        }
        let state = if runtime.house_actor_state(actor, session).is_ok() {
            self.house_spell_state(runtime, actor, session, fence)?
        } else {
            self.get(runtime, actor, session)?
        };
        let pending = Arc::new(Mutex::new(Some(CommittedCooldowns::default())));
        let subscriber = Subscriber {
            fence,
            spell_indices: (1..=u32::try_from(book.source_len()).ok()?)
                .filter_map(std::num::NonZeroU32::new)
                .filter_map(|index| {
                    let (spell, _) = book.source_indexed(index)?;
                    Some((spell.key.clone(), index.get()))
                })
                .collect(),
            pending: Arc::downgrade(&pending),
            sparse: admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1),
        };
        let snapshot = subscriber.keys(&state.cooldowns);
        let key = actor.placement_identity();
        self.cooldown_subscribers
            .retain(|_, subscriber| subscriber.pending.strong_count() != 0);
        if self.cooldown_subscribers.get(&key).is_some_and(|existing| {
            existing.fence.game_session_id == session
                && existing.fence.connection_generation >= fence.connection_generation
        }) {
            return None;
        }
        if let Some(displaced) = self.cooldown_subscribers.insert(key, subscriber) {
            displaced.invalidate();
        }
        Some(CooldownFeed {
            snapshot,
            pending,
            connection_generation: fence.connection_generation.get(),
        })
    }

    pub(super) fn committed_cooldowns(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        before: &PlayerSpellState,
        after: &PlayerSpellState,
    ) {
        let activation = after.wheel_gift_activation();
        let rescued = activation
            .filter(|value| Some(*value) != before.wheel_gift_activation())
            .map(|_| (after.revision(), after.vitals()));
        self.record_cooldown_changes(
            runtime,
            actor,
            session,
            &before.cooldowns,
            &after.cooldowns,
            rescued,
        );
    }

    pub(super) fn record_cooldown_changes(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        before: &Cooldowns,
        after: &Cooldowns,
        rescued: Option<Vitals>,
    ) {
        if before == after {
            return;
        }
        let Some(subscriber) = self.cooldown_subscribers.get(&actor.placement_identity()) else {
            return;
        };
        if subscriber.fence.game_session_id != session
            || !self.current_cooldown_transport(
                actor,
                session,
                subscriber.fence.connection_generation.get(),
            )
            || !current(runtime, actor, subscriber.fence)
        {
            return;
        }
        let Some(pending) = subscriber.pending.upgrade() else {
            return;
        };
        let before = subscriber.keys(before);
        let after = subscriber.keys(after);
        let mut mailbox = pending.lock().unwrap_or_else(|e| e.into_inner());
        let Some(pending) = mailbox.as_mut() else {
            return;
        };
        if before != after {
            if subscriber.sparse {
                let changes = pending.deadlines.get_or_insert_with(BTreeMap::new);
                for (key, deadline) in &after {
                    if before.get(key) != Some(deadline) {
                        changes.insert(*key, *deadline);
                    }
                }
                for key in before.keys().filter(|key| !after.contains_key(key)) {
                    changes.insert(*key, 0);
                }
            } else {
                pending.deadlines = Some(after);
            }
        }
        if rescued.is_some() {
            pending.rescued = rescued;
        }
    }
}

pub(super) fn payload(
    deadlines: &BTreeMap<Key, u64>,
    now_us: u64,
    complete: bool,
) -> Result<Vec<u8>, wire::PresentationWireError> {
    // The existing codec subtracts server_now_ms. A zero base lets deltas send
    // remaining time directly; complete views retain absolute expiries.
    let server_now_ms = if complete {
        (now_us / 1000).min(wire::MAX_CLOCK_MS)
    } else {
        0
    };
    let entries = deadlines
        .iter()
        .filter_map(|(&(kind, id), &deadline)| {
            if complete && deadline <= now_us {
                return None;
            }
            Some(wire::ActorCooldown {
                kind,
                id,
                expires_at_ms: (if complete {
                    deadline
                } else {
                    deadline.saturating_sub(now_us)
                })
                .div_ceil(1000)
                .min(wire::MAX_CLOCK_MS),
            })
        })
        .collect();
    wire::encode_actor_cooldowns(&wire::ActorCooldowns {
        server_now_ms,
        rtt_ms: None,
        entries,
    })
}

#[cfg(test)]
pub(super) struct TestReceipt {
    pending: Arc<Mutex<Option<CommittedCooldowns>>>,
    rescued: std::cell::Cell<Option<Vitals>>,
}

#[cfg(test)]
impl Default for TestReceipt {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(Some(CommittedCooldowns::default()))),
            rescued: Default::default(),
        }
    }
}

#[cfg(test)]
impl TestReceipt {
    pub(super) fn feed(&self, admitted: AdmittedSession) -> CooldownFeed {
        CooldownFeed {
            snapshot: BTreeMap::new(),
            pending: Arc::clone(&self.pending),
            connection_generation: admitted.continuity.connection_generation,
        }
    }

    pub(super) fn rescue(&self, vitals: Vitals) -> Option<()> {
        self.rescued.set(Some(vitals));
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let pending = pending.as_mut()?;
        pending.rescued = Some(vitals);
        pending
            .deadlines
            .get_or_insert_with(BTreeMap::new)
            .insert((wire::ActorCooldownKind::Spell, 1), 30_000_000);
        Some(())
    }

    pub(super) fn rescued(&self) -> Option<Vitals> {
        self.rescued.get()
    }
}
