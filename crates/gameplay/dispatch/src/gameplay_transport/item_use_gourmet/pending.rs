//! A native USE whose outcome is unknown retains its exact prepared successor.
//! The shared pending-spell gate freezes conflicting owner writes until replay.
use super::*;
use crate::spell::cast::PlayerSpellState;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PendingUse {
    pub(in crate::gameplay_transport) actor: ExactActorRef,
    pub(in crate::gameplay_transport) session: GameSessionId,
    fence: CurrentCharacterItemFence,
    request: GourmetUseRequest,
    prepared: PlayerSpellState,
    before_revision: u64,
}

impl PendingUse {
    pub(in crate::gameplay_transport) fn new(
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
        request: GourmetUseRequest,
        prepared: PlayerSpellState,
        before_revision: u64,
    ) -> Self {
        Self {
            actor,
            session,
            fence,
            request,
            prepared,
            before_revision,
        }
    }

    /// Park before awaiting the native writer. Cancellation therefore leaves the
    /// same item occurrence and successor available to the next owner pass.
    pub(in crate::gameplay_transport) fn park(self, states: &mut ChannelSpellStates) -> bool {
        if states
            .pending_gourmet
            .iter()
            .any(|p| p.actor == self.actor && p.session == self.session)
            || !states.actors.iter().any(|(actor, session, state)| {
                *actor == self.actor
                    && *session == self.session
                    && state.revision() == self.before_revision
            })
            || self.fence.game_session_id != self.session
            || self.prepared.revision() != self.before_revision.saturating_add(1)
        {
            return false;
        }
        if states.pending_gourmet.try_reserve(1).is_err() {
            return false;
        }
        states.pending_gourmet.push(self);
        true
    }
}

/// The caller holds the runtime and spell owner locks. Only a known commit
/// installs the retained successor, exactly once; uncertainty leaves the marker.
pub(super) fn finish(
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    result: Result<ItemUseOutcome, ItemUseError>,
    now_us: u64,
) -> Option<UseDisposition> {
    let index = states
        .pending_gourmet
        .iter()
        .position(|p| p.actor == actor && p.session == session)?;
    let pending = &states.pending_gourmet[index];
    let disposition = match result {
        Ok(_) => UseDisposition::Committed,
        Err(ItemUseError::Cooldown) => UseDisposition::Exhausted,
        Err(ItemUseError::StaleState) => UseDisposition::StaleState,
        Err(ItemUseError::Unavailable(_)) => return None,
        Err(_) => UseDisposition::Rejected,
    };
    let state_index = states
        .actors
        .iter()
        .position(|(a, s, _)| *a == actor && *s == session)?;
    if states.actors[state_index].2.revision() != pending.before_revision {
        return None;
    }
    let pending = states.pending_gourmet.swap_remove(index);
    if disposition == UseDisposition::Committed {
        let saved = gourmet_conditions(&pending.prepared, now_us);
        states.actors[state_index].2 = pending.prepared;
        remember(states, actor, session, saved, now_us);
        // If an unknown outcome lasted until expiry, persist the clear rather
        // than abandoning the receipt's older positive duration on disk.
        if saved == GourmetConditions::default() && pending.request.conditions != saved {
            states.gourmet_checkpoints.push((actor, session, saved, 0));
        }
    }
    if let Some((_, _, gate)) = states
        .item_uses
        .0
        .iter_mut()
        .find(|(a, s, _)| *a == actor && *s == session)
    {
        gate.unspendable
            .retain(|id| *id != pending.request.item.source_item_instance_id);
        gate.settle(pending.request.item.command_id, disposition, &[]);
    }
    Some(disposition)
}

pub(in crate::gameplay_transport) async fn resolve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    if !owner
        .states
        .spell_states
        .lock()
        .await
        .pending_gourmet
        .iter()
        .any(|p| p.actor == actor && p.session == session)
    {
        return true;
    }
    let Some(_permit) = owner.spell_lane_permit().await else {
        return false;
    };
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let Some(pending) = states
        .pending_gourmet
        .iter()
        .find(|p| p.actor == actor && p.session == session)
        .cloned()
    else {
        return true;
    };
    if !current(&runtime, actor, session, pending.fence, true) {
        return false;
    }
    // Same CommandRef, immutable source, source quantity and prepared condition
    // snapshot. The native owner first returns any committed receipt; otherwise
    // it rechecks current authority and executes at most one physical burn.
    let result = owner
        .root
        .commit_gourmet_use(
            owner.character,
            owner.holder,
            pending.fence,
            pending.request,
        )
        .await;
    finish(&mut states, actor, session, result, owner.owner_now().get()).is_some()
}
