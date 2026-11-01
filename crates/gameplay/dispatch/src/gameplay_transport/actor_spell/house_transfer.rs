//! Move the existing actor's data across one committed native House handoff.
//! The new GameSession has its own ingress; clocks, vitals and owned values do
//! not become a new login. Saved CommandRefs and unresolved writers never move.
use super::*;
use crate::foundation::{CharacterLease, RuntimeScopeRefV1};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::attack::ChannelAttackStates;

/// Prepared from the actual current source, or the retained historical source
/// of an authenticated native recovery, while its runtime/state locks are held.
/// Publication must independently prove the native successor lease and session.
pub(in crate::gameplay_transport) struct HouseTransferViews {
    actor: ExactActorRef,
    source: GameSessionId,
    destination: GameSessionId,
    lease: CharacterLease,
    scope: RuntimeScopeRefV1,
    revision: Option<u64>,
}

fn current_owner(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<(CharacterLease, RuntimeScopeRefV1)> {
    if let Ok(house) = runtime.house_actor_state(actor, session) {
        if runtime
            .house_player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
            || runtime.house_player_writes_fenced(actor, session).ok()?
        {
            return None;
        }
        return Some((house.lease, house.scope));
    }
    if runtime
        .player_control_facts(actor, session)
        .ok()?
        .control_loss
        .is_some()
    {
        return None;
    }
    let binding = runtime.binding();
    Some((
        runtime.bound_attacker_lease(actor, session).ok()?,
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
    ))
}

impl ChannelSpellStates {
    fn house_transfer_data_ready(
        &self,
        attack: &ChannelAttackStates,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        source != destination
            && !self.is_dead(actor)
            && !self.has_pending_spell_commit(actor, source)
            && !self
                .actors
                .iter()
                .any(|(a, s, state)| *a == actor && *s == source && state.has_training_to_flush())
            && !self.actors.iter().any(|(_, s, _)| *s == destination)
            && !self
                .gourmet_checkpoints
                .iter()
                .any(|(_, s, _, _)| *s == destination)
            && !self.charm_states.iter().any(|(_, s, _)| *s == destination)
            && !self.pvp_locks.iter().any(|(_, s, _)| *s == destination)
            && self.house_stamina_transfer_ready(actor, source, destination)
            && self.house_prey_transfer_ready(actor, source, destination)
            && self
                .item_uses
                .house_transfer_ready(actor, source, destination)
            && attack.house_transfer_ready(destination)
    }

    pub(in crate::gameplay_transport) fn prepare_house_transfer_views(
        &self,
        runtime: &ChannelRuntimeV1,
        attack: &ChannelAttackStates,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> Option<HouseTransferViews> {
        let (lease, scope) = current_owner(runtime, actor, source)?;
        if !self.house_transfer_data_ready(attack, actor, source, destination) {
            return None;
        }
        Some(HouseTransferViews {
            actor,
            source,
            destination,
            lease,
            scope,
            revision: self
                .actors
                .iter()
                .find(|(a, s, _)| *a == actor && *s == source)
                .map(|(_, _, state)| state.revision()),
        })
    }

    /// Only the authenticated native recovery caller can supply this committed
    /// edge. A retained source is historical data, possibly carrying the exact
    /// pre-COMMIT transition fence, not renewed source write authority.
    pub(in crate::gameplay_transport) fn prepare_recovered_house_transfer_views(
        &self,
        runtime: &ChannelRuntimeV1,
        attack: &ChannelAttackStates,
        recovered: &crate::durability::house_transfer::HouseTransferRecovery,
    ) -> Option<Option<HouseTransferViews>> {
        let destination = GameSessionId::decode(&recovered.ack.session).ok()?;
        let source = recovered.source;
        let binding = runtime.binding();
        if recovered.started
            || source == destination
            || recovered.ack.world != *binding.world_id().as_bytes()
            || recovered.ack.channel != *binding.channel_id().as_bytes()
            || recovered.current.world_id != binding.world_id()
        {
            return None;
        }
        // load() obtains the source lease from the immutable transfer receipt
        // and constructs this exact successor. restore_transfer_actor repeats
        // the old carrier lease comparison before any runtime rebind.
        let lease = CharacterLease::new(
            recovered.current.character_id,
            recovered.ack.lease_generation.checked_sub(1)?,
        )
        .ok()?;
        let (actor, scope) =
            if let Some((actor, house)) = runtime.house_player_for_session(source).ok()? {
                if recovered.ack.house.is_some() || house.lease != lease {
                    return None;
                }
                (actor, house.scope)
            } else if let Some((actor, _)) = runtime.positioned_player_for_session(source).ok()? {
                recovered.ack.house.as_ref()?;
                // Unlike ordinary prepare, this read deliberately accepts the
                // retained transition fence; the native recovery owner, not this
                // value token, proves whether the committed successor may bind.
                runtime.player_control_facts(actor, source).ok()?;
                (
                    actor,
                    RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                )
            } else {
                // Cold reconstruction and an already installed destination keep
                // their existing read-only restore path. Never invent a source.
                return Some(None);
            };
        if !self.house_transfer_data_ready(attack, actor, source, destination) {
            return None;
        }
        Some(Some(HouseTransferViews {
            actor,
            source,
            destination,
            lease,
            scope,
            revision: self
                .actors
                .iter()
                .find(|(a, s, _)| *a == actor && *s == source)
                .map(|(_, _, state)| state.revision()),
        }))
    }

    pub(in crate::gameplay_transport) fn commit_house_transfer_views(
        &mut self,
        runtime: &ChannelRuntimeV1,
        attack: &mut ChannelAttackStates,
        prepared: HouseTransferViews,
    ) -> Option<()> {
        let HouseTransferViews {
            actor,
            source,
            destination,
            lease,
            scope,
            revision,
        } = prepared;
        let (next_lease, next_scope) = current_owner(runtime, actor, destination)?;
        if next_lease.character_id() != lease.character_id()
            || Some(next_lease.generation()) != lease.generation().checked_add(1)
            || next_scope.world_id() != scope.world_id()
            || next_scope == scope
            || !matches!(
                (scope, next_scope),
                (
                    RuntimeScopeRefV1::Channel { .. },
                    RuntimeScopeRefV1::Instance { .. }
                ) | (
                    RuntimeScopeRefV1::Instance { .. },
                    RuntimeScopeRefV1::Channel { .. }
                )
            )
            || current_owner(runtime, actor, source).is_some()
            || !self.house_transfer_data_ready(attack, actor, source, destination)
            || self
                .actors
                .iter()
                .find(|(a, s, _)| *a == actor && *s == source)
                .map(|(_, _, state)| state.revision())
                != revision
        {
            return None;
        }
        // All checks precede the infallible in-place moves under the same owner
        // locks. No durable occurrence, timer, epoch or remaining time is reset.
        for (a, s, _) in &mut self.actors {
            if *a == actor && *s == source {
                *s = destination;
            }
        }
        for (a, s, _, _) in &mut self.gourmet_checkpoints {
            if *a == actor && *s == source {
                *s = destination;
            }
        }
        for (a, s, _) in &mut self.charm_states {
            if *a == actor && *s == source {
                *s = destination;
            }
        }
        for (a, s, _) in &mut self.pvp_locks {
            if *a == actor && *s == source {
                *s = destination;
            }
        }
        self.move_house_stamina(actor, source, destination);
        self.move_house_prey(actor, source, destination);
        self.item_uses.move_house(actor, source, destination);
        attack.move_house(actor, source, destination);
        // spell_timers/presentations and source occurrence memos retain their
        // immutable original CommandRef and scope. Their existing native due
        // checks decide whether they remain applicable; this cannot retarget a
        // cast or synthesize a Familiar login (Q17 is a separate owner decision).
        Some(())
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Finish the existing source checkpoints before entering the exclusive
    /// handoff lane. Unknown outcomes retain that source and its original work.
    pub(in crate::gameplay_transport) async fn prepare_house_transfer_checkpoints(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if !crate::gameplay_transport::item_use::gourmet::checkpoint(self, actor, session, true)
            .await
        {
            return false;
        }
        let source_scope = {
            let runtime = self.runtime.lock().await;
            let Some((_, scope)) = current_owner(&runtime, actor, session) else {
                return false;
            };
            scope
        };
        if matches!(source_scope, RuntimeScopeRefV1::Channel { .. }) {
            matches!(
                self.save_spell_training(actor, session, true).await,
                TrainingSave::Saved | TrainingSave::NotApplicable
            )
        } else {
            // No generic training/Prey writer is admitted in a House. The
            // in-lane preflight requires its carried source work to be clean.
            true
        }
    }
}
