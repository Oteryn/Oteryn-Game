//! Resolve an unpublished join using independently read receipts and live fences.
use super::*;
use crate::durability::fresh_admission::FreshReconciliation;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::connection::join_recovery::{
    JoinAction, PendingHouseJoin, UnpublishedJoin,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_unpublished_join(
        &self,
        pending: &UnpublishedJoin,
        ended: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        self.root.await_ready().await.map_err(|_| ())?;
        let JoinAction::Fresh {
            request,
            reservation,
            channel_entry_started,
        } = &pending.action
        else {
            return self.reconcile_interrupted_resume(pending).await;
        };
        let original = match store.reconcile(request.operation()).await.map_err(|_| ())? {
            FreshReconciliation::Committed(original) => original,
            FreshReconciliation::Absent | FreshReconciliation::Conflict if !pending.committed => {
                self.rollback_runtime_player(*reservation)
                    .await
                    .map_err(|_| ())?;
                return Ok(ended);
            }
            _ => return Err(()),
        };
        let mut source = pending.source;
        let controller = source.controller.ok_or(())?;
        let current = original.current_session;
        if current.session_state() == GameSessionState::Active {
            if !crate::gameplay_transport::owns_fresh_session(
                current,
                source.game_session_id,
                controller.transport,
            ) {
                return Ok(ended);
            }
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or(())?;
            if source.item_fence.is_some_and(|expected| expected != fence) {
                return Err(());
            }
            let mut runtime = self.runtime.lock().await;
            let actor = match source.runtime_actor {
                Some(actor) => actor,
                None => runtime
                    .commit_fresh_session_at(*reservation, self.owner_now().get())
                    .map_err(|_| ())?,
            };
            source.runtime_actor = Some(actor);
            if fence.runtime_scope != actor.runtime_scope() {
                return Err(());
            }
            runtime
                .bind_attacker_lease(
                    actor,
                    source.game_session_id,
                    current.current_character_lease(),
                )
                .map_err(|_| ())?;
            drop(runtime);
            source.item_fence = Some(fence);
            if *channel_entry_started && source.first_entry == FirstEntryOutcome::RefusedUnavailable
            {
                let repaired = self
                    .finish_channel_entry(source, request)
                    .await
                    .map_err(|_| ())?;
                return Ok(repaired);
            }
            return Ok(AdmittedSession {
                continuity: if ended.game_session_id == source.game_session_id {
                    ended.continuity
                } else {
                    source.continuity
                },
                ..source
            });
        }
        match (&pending.house, current.session_state()) {
            (Some(house), GameSessionState::Terminal)
                if !matches!(house.stage, HouseJoinStage::Prepared) =>
            {
                self.reconcile_unpublished_house(
                    source,
                    house,
                    ended,
                    current.current_character_lease(),
                )
                .await
            }
            _ => Ok(ended),
        }
    }

    async fn reconcile_interrupted_resume(
        &self,
        pending: &UnpublishedJoin,
    ) -> Result<AdmittedSession, ()> {
        use crate::foundation::CompleteReconnectOutcomeV1;
        use crate::gameplay_transport::connection::GraceExpiryResult;
        let mut source = pending.source;
        let inactive = AdmittedSession {
            runtime_actor: None,
            ..source
        };
        let store = FreshAdmissionStore::from_root(self.root.clone());
        match &pending.action {
            JoinAction::Fresh { .. } => Err(()),
            JoinAction::NativeActivity => Ok(inactive),
            JoinAction::CapabilityMismatch {
                epoch,
                activity_store,
            } => {
                let scoped = self.resolve_runtime_view(source).await?;
                let owner = scoped.as_ref().unwrap_or(self);
                let outcome = match activity_store {
                    Some(store) => {
                        owner
                            .release_terminal_with_store(
                                source,
                                crate::gameplay_transport::TerminalRelease::CapabilityMismatch(
                                    *epoch,
                                ),
                                (**store).clone(),
                                false,
                            )
                            .await
                    }
                    None => owner.release_capability_mismatch(source, *epoch).await,
                };
                if outcome == GraceExpiryResult::Released {
                    owner.forget_lost(
                        source.game_session_id,
                        source.continuity.connection_generation,
                    );
                }
                match outcome {
                    GraceExpiryResult::Unknown => Err(()),
                    _ => Ok(inactive),
                }
            }
            JoinAction::Resume {
                identity,
                operation,
                epoch,
                house,
            } => {
                let committed = match operation {
                    Some(operation) => store
                        .reconcile_complete_reconnect(operation)
                        .await
                        .map_err(|_| ())?
                        .is_some_and(|receipt| {
                            matches!(
                                receipt.outcome,
                                CompleteReconnectOutcomeV1::Committed { .. }
                            )
                        }),
                    None => false,
                };
                if !committed {
                    store
                        .abort_complete_reconnect(identity)
                        .await
                        .map_err(|_| ())?;
                    return Ok(inactive);
                }
                let current = store
                    .current_session(source.game_session_id)
                    .await
                    .map_err(|_| ())?;
                if current.session_state() != GameSessionState::Active
                    || current.current_transport()
                        != source.controller.map(|controller| controller.transport)
                    || crate::gameplay_transport::item_fence_of(current) != source.item_fence
                {
                    return Ok(inactive);
                }
                let scoped = self.resolve_runtime_view(source).await?;
                let owner = scoped.as_ref().unwrap_or(self);
                if *house
                    && !owner
                        .read_house_resume(source.runtime_actor.ok_or(())?, source.game_session_id)
                        .await
                        .map_err(|_| ())?
                {
                    return match owner.release_abandoned_session(source).await {
                        GraceExpiryResult::Unknown => Err(()),
                        _ => Ok(inactive),
                    };
                }
                let predecessor = source
                    .continuity
                    .connection_generation
                    .checked_sub(1)
                    .ok_or(())?;
                owner
                    .restore_committed_resume(source, *epoch, *house, predecessor)
                    .await
                    .map_err(|_| ())?;
                self.activity_runtimes.remember(source)?;
                Ok(source)
            }
            JoinAction::NativeHouse(expected) => {
                let current = store
                    .current_session(source.game_session_id)
                    .await
                    .map_err(|_| ())?;
                if current.session_state() != GameSessionState::Active
                    || current.current_transport()
                        != source.controller.map(|controller| controller.transport)
                {
                    return Ok(inactive);
                }
                let fence = crate::gameplay_transport::item_fence_of(current).ok_or(())?;
                let Some(mut recovered) = self
                    .root
                    .house_transfer_recovery(source.game_session_id)
                    .await
                    .map_err(|_| ())?
                else {
                    return Err(());
                };
                if recovered.ack != expected.ack
                    || recovered.source != expected.source
                    || recovered.command != expected.command
                    || recovered.account != expected.account
                    || recovered.started
                    || Some(CurrentCharacterItemFence {
                        connection_generation: source.item_fence.ok_or(())?.connection_generation,
                        ..fence
                    }) != source.item_fence
                {
                    return Err(());
                }
                recovered.ack.connection_generation = fence.connection_generation.get();
                source.continuity.connection_generation = fence.connection_generation.get();
                source.item_fence = Some(fence);
                self.reserve_precondition().await.map_err(|_| ())?;
                self.install_recovered_house(recovered, source)
                    .await
                    .map_err(|_| ())
            }
        }
    }

    async fn reconcile_unpublished_house(
        &self,
        source: AdmittedSession,
        pending: &PendingHouseJoin,
        ended: AdmittedSession,
        source_lease: CharacterLease,
    ) -> Result<AdmittedSession, ()> {
        let request = &pending.request;
        let destination = GameSessionId::decode(&request.ack.session).map_err(|_| ())?;
        let current = FreshAdmissionStore::from_root(self.root.clone())
            .current_session(destination)
            .await
            .map_err(|_| ())?;
        if current.current_transport() != Some(request.transport)
            || current.session_state() != GameSessionState::Active
        {
            // An independently advanced controller owns any further recovery.
            return Ok(ended);
        }
        let receipt = self
            .root
            .house_transfer_recovery(destination)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        let mut expected = request.ack.clone();
        // The House transaction resolves its actual scope generation under lock.
        expected.scope_generation = receipt.ack.scope_generation;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or(())?;
        if receipt.ack != expected
            || receipt.source != source.game_session_id
            || receipt.command != request.command
            || !receipt.admission_restore
            || receipt.started
            || source
                .controller
                .is_none_or(|controller| controller.account_id != receipt.account)
            || fence.game_session_id != destination
            || fence.character_id != request.fence.character_id
            || fence.character_lease_generation != expected.lease_generation
            || fence.connection_generation.get() != expected.connection_generation
            || fence.scope_ownership_generation.get() != expected.scope_generation
            || fence.runtime_scope != pending.placement.scope
        {
            return Err(());
        }
        self.reserve_precondition().await.map_err(|_| ())?;
        let actor = source.runtime_actor.ok_or(())?;
        let mut placement = pending.placement.clone();
        placement.generation = fence.scope_ownership_generation;
        let mut runtime = self.runtime.lock().await;
        let pin = runtime.content_pin();
        if pin.frame_binding_digest() != request.last_position.coordinate_frame_digest
            || pin.server_artifact_digest() != request.last_position.server_artifact_digest
            || runtime.map_revision_digest() != request.last_position.map_revision_digest
        {
            return Err(());
        }
        match runtime.house_actor_state(actor, destination) {
            Ok(held) if held == placement => {}
            Ok(_) => return Err(()),
            Err(_) => runtime
                .restore_admitted_house_actor(
                    actor,
                    source.game_session_id,
                    destination,
                    source_lease,
                    placement,
                )
                .map_err(|_| ())?,
        }
        drop(runtime);
        let admitted = AdmittedSession {
            game_session_id: destination,
            first_entry: FirstEntryOutcome::Positioned,
            item_fence: Some(fence),
            continuity: SessionContinuity {
                house_input_pending: Some(true),
                ..source.continuity
            },
            ..source
        };
        Ok(if ended.game_session_id == destination {
            ended
        } else {
            match &pending.stage {
                HouseJoinStage::Committed(known) => **known,
                _ => admitted,
            }
        })
    }
}
