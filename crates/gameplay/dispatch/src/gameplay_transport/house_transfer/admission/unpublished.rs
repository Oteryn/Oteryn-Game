//! Resolve an unpublished join using independently read receipts and live fences.
use super::*;
use crate::durability::fresh_admission::FreshReconciliation;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::connection::join_recovery::{PendingHouseJoin, UnpublishedJoin};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_unpublished_join(
        &self,
        pending: &UnpublishedJoin,
        ended: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        self.root.await_ready().await.map_err(|_| ())?;
        let FreshReconciliation::Committed(original) =
            store.reconcile(&pending.operation).await.map_err(|_| ())?
        else {
            return Err(());
        };
        let source = pending.source;
        let controller = source.controller.ok_or(())?;
        let actor = source.runtime_actor.ok_or(())?;
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
            if source.item_fence.is_some_and(|expected| expected != fence)
                || fence.runtime_scope != actor.runtime_scope()
            {
                return Err(());
            }
            let mut runtime = self.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    actor,
                    source.game_session_id,
                    current.current_character_lease(),
                )
                .map_err(|_| ())?;
            return Ok(AdmittedSession {
                item_fence: Some(fence),
                ..if ended.game_session_id == source.game_session_id {
                    ended
                } else {
                    source
                }
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
