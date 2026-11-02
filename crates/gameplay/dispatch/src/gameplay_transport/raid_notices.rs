//! Transient raid announcements use the existing Channel session owner and
//! selected server-message domain. A durable step is acknowledged only after
//! one atomic publication to the currently connected recipient snapshot.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use crate::durability::DurabilityError;
use crate::durability::raid_runner::{CommittedRaidAction, CommittedRaidStep, RaidStepIdentity};
use crate::foundation::GameSessionId;
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_encounter_rules::raid::{MAX_ANNOUNCEMENTS, MAX_CANDIDATES};
use oteryn_protocol_oteryn::zones_nologout::{
    CAPABILITY_ZONES_NOLOGOUT_V1, ServerMessage, encode_server_message,
};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

// Existing GameplayListenerConfig / ListenerLimits registration. A recipient
// is a currently serving connection, never an unbounded historical session.
const MAX_RECIPIENTS: usize = 256;
const MAX_RECEIPTS: usize = MAX_CANDIDATES * MAX_ANNOUNCEMENTS;
type NoticeKey = ([u8; 16], u64, u32);
type ConnectionRoot = (GameSessionId, u64);

struct Recipient {
    connection_generation: u64,
    connection_root: ConnectionRoot,
    queue: VecDeque<Arc<ServerMessage>>,
}

#[derive(Default)]
struct NoticeState {
    recipients: HashMap<GameSessionId, Recipient>,
    published: BTreeMap<NoticeKey, Arc<ServerMessage>>,
}

#[derive(Default)]
pub(super) struct ServerMessageHub {
    state: Mutex<NoticeState>,
    changed: tokio::sync::Notify,
}

impl ServerMessageHub {
    pub(super) fn join(&self, session: GameSessionId, connection_generation: u64) -> bool {
        if connection_generation == 0 {
            return false;
        }
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if let Some(current) = state.recipients.get(&session) {
            if current.connection_generation > connection_generation {
                return false;
            }
            if current.connection_generation == connection_generation {
                return true;
            }
        }
        if !state.recipients.contains_key(&session) && state.recipients.len() >= MAX_RECIPIENTS {
            return false;
        }
        let connection_root = (session, connection_generation);
        if state
            .recipients
            .values()
            .any(|recipient| recipient.connection_root == connection_root)
        {
            return false;
        }
        state.recipients.insert(
            session,
            Recipient {
                connection_generation,
                connection_root,
                queue: VecDeque::new(),
            },
        );
        true
    }

    /// The one physical connection guard retains its original identity across
    /// committed scope handoffs. Resolve that exact root in the bounded current
    /// recipient set, even if cancellation happened before destination serve.
    pub(super) fn leave(&self, session: GameSessionId, connection_generation: u64) {
        if let Ok(mut state) = self.state.lock()
            && let Some(current) = state.recipients.iter().find_map(|(current, recipient)| {
                (recipient.connection_root == (session, connection_generation)).then_some(*current)
            })
        {
            state.recipients.remove(&current);
        }
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut state) = self.state.lock() {
            state.recipients.remove(&session);
        }
    }

    /// A committed scope handoff keeps the same connected controller. Move
    /// only its already registered recipient; recovery or an absent source
    /// never creates a recipient that could receive historical publication.
    pub(super) fn transfer_connection(
        &self,
        source: AdmittedSession,
        destination: AdmittedSession,
    ) -> Result<(), ()> {
        if source.controller.is_none()
            || source.controller != destination.controller
            || !selected(source)
            || !selected(destination)
        {
            return Ok(());
        }
        let generation = source.continuity.connection_generation;
        let next_generation = destination.continuity.connection_generation;
        if source.world_id != destination.world_id
            || source.channel_id != destination.channel_id
            || generation == 0
            || next_generation == 0
            || source.game_session_id == destination.game_session_id
        {
            return Err(());
        }
        let mut state = self.state.lock().map_err(|_| ())?;
        let Some(current) = state.recipients.get(&source.game_session_id) else {
            return Ok(());
        };
        if current.connection_generation != generation
            || state.recipients.contains_key(&destination.game_session_id)
        {
            return Err(());
        }
        let Some(mut recipient) = state.recipients.remove(&source.game_session_id) else {
            return Err(());
        };
        recipient.connection_generation = next_generation;
        state
            .recipients
            .insert(destination.game_session_id, recipient);
        drop(state);
        self.changed.notify_waiters();
        Ok(())
    }

    fn publish(&self, identity: RaidStepIdentity, message: ServerMessage) -> Result<(), ()> {
        // Validate before retaining either the publication or any recipient.
        encode_server_message(&message).map_err(|_| ())?;
        let key = (
            identity.firing_id,
            identity.generation.get(),
            identity.ordinal,
        );
        let mut state = self.state.lock().map_err(|_| ())?;
        if let Some(previous) = state.published.get(&key) {
            return if previous.as_ref() == &message {
                Ok(())
            } else {
                Err(())
            };
        }
        if state.published.len() >= MAX_RECEIPTS
            || state
                .recipients
                .values()
                .any(|recipient| recipient.queue.len() >= MAX_ANNOUNCEMENTS)
        {
            return Err(());
        }
        let message = Arc::new(message);
        for recipient in state.recipients.values_mut() {
            recipient.queue.push_back(message.clone());
        }
        state.published.insert(key, message);
        drop(state);
        self.changed.notify_waiters();
        Ok(())
    }

    /// The caller has received a committed Applied/Replayed acknowledgement.
    /// Subsequent broadcast attempts independently observe the acknowledged
    /// durable effect and cannot republish it. Recipient queues own their Arcs,
    /// so reclaiming this receipt never removes an undelivered message.
    fn acknowledge(&self, identity: RaidStepIdentity) -> Result<(), ()> {
        let key = (
            identity.firing_id,
            identity.generation.get(),
            identity.ordinal,
        );
        self.state.lock().map_err(|_| ())?.published.remove(&key);
        Ok(())
    }

    /// A committed terminal ACK proves earlier effects acknowledged or LOST.
    /// Reclaim receipts whose individual ACK committed but its response was
    /// lost, including a predecessor generation retired by LOST cleanup.
    fn acknowledge_end(&self, identity: RaidStepIdentity) -> Result<(), ()> {
        self.state
            .lock()
            .map_err(|_| ())?
            .published
            .retain(|(firing, _, _), _| *firing != identity.firing_id);
        Ok(())
    }

    pub(super) fn take(
        &self,
        session: GameSessionId,
        connection_generation: u64,
    ) -> Option<ServerMessage> {
        let mut state = self.state.lock().ok()?;
        let recipient = state.recipients.get_mut(&session)?;
        if recipient.connection_generation != connection_generation {
            return None;
        }
        recipient
            .queue
            .pop_front()
            .map(|notice| notice.as_ref().clone())
    }

    pub(super) async fn wait(&self, session: GameSessionId, connection_generation: u64) {
        loop {
            let mut notified = std::pin::pin!(self.changed.notified());
            // Register before inspecting the queue, so a publication between
            // that inspection and awaiting cannot be lost. Cancelled polls are
            // harmless because the retained queue is inspected on every poll.
            notified.as_mut().enable();
            if self.state.lock().is_ok_and(|state| {
                state.recipients.get(&session).is_some_and(|recipient| {
                    recipient.connection_generation == connection_generation
                        && !recipient.queue.is_empty()
                })
            }) {
                return;
            }
            notified.await;
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn acknowledge_raid_notice(&self, identity: RaidStepIdentity) -> Result<(), ()> {
        if identity.channel != self.channel_id {
            return Err(());
        }
        self.states.server_messages.acknowledge(identity)
    }

    pub(super) fn acknowledge_raid_notice_end(&self, identity: RaidStepIdentity) -> Result<(), ()> {
        if identity.channel != self.channel_id {
            return Err(());
        }
        self.states.server_messages.acknowledge_end(identity)
    }

    pub(super) async fn broadcast_raid_notice(&self, step: &CommittedRaidStep) -> Result<(), ()> {
        let CommittedRaidAction::Announcement { text, advance } = &step.action else {
            return Err(());
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        let binding = runtime.binding();
        if binding.world_id() != self.world_id
            || binding.channel_id() != Some(self.channel_id)
            || step.identity.channel != self.channel_id
            || binding.scope_generation() != step.identity.generation
        {
            return Err(());
        }
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            self.movement_cells.index()
        else {
            return Err(());
        };
        let bundle = index.map().overlay.base().digest();
        let message = if *advance {
            ServerMessage::Advance(text.clone())
        } else {
            ServerMessage::Warning(text.clone())
        };
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &*runtime, &permit, step, &message);
        pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::raid_runner::raid_effect_pending_in_transaction;
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                };
                let (owner, runtime, permit, step, message) = context;
                let binding = runtime.binding();
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let _authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    binding.runtime_scope(),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let pending = raid_effect_pending_in_transaction(
                    &mut tx,
                    owner.holder,
                    step,
                    owner.world_id,
                    owner.reset_epoch,
                    bundle,
                )
                .await?;
                if pending {
                    owner
                        .states
                        .server_messages
                        .publish(step.identity, (**message).clone())
                        .map_err(|_| DurabilityError::Unavailable)?;
                }
                // Publication and its receipt are installed while assignment,
                // reset and exact effect locks remain held. An uncertain read
                // transaction rollback cannot duplicate the retained notice.
                tx.rollback()
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                Ok(())
            })
        })
        .await
        .map_err(|_| ())
    }
}

pub(super) fn selected(admitted: AdmittedSession) -> bool {
    admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ZONES_NOLOGOUT_V1)
}

/// One guard spans the physical transport, including scope handoffs. Dropping
/// it removes its current recipient even during control-loss grace or a
/// cancelled handoff write. A resumed connection has a different root, so an
/// older connection's drop cannot erase it.
pub(super) struct ServerMessageConnection<'a, A: FreshAdmissionAuthority> {
    authority: &'a A,
    admitted: AdmittedSession,
}

impl<'a, A: FreshAdmissionAuthority> ServerMessageConnection<'a, A> {
    pub(super) fn join(authority: &'a A, admitted: AdmittedSession) -> Option<Self> {
        authority.server_messages_joined(admitted).then_some(Self {
            authority,
            admitted,
        })
    }
}

impl<A: FreshAdmissionAuthority> Drop for ServerMessageConnection<'_, A> {
    fn drop(&mut self) {
        self.authority.server_messages_left(self.admitted);
    }
}

#[cfg(test)]
#[path = "raid_notices_tests.rs"]
mod tests;
