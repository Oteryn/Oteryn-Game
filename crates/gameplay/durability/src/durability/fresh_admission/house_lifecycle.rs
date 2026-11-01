//! Existing admission lifecycle over an authenticated native House handoff.
//! Initial admission and transfer receipts are provenance; only the independently
//! current session, claims, node and named scope authorize a lifecycle change.
use super::*;
use super::super::house_hireling::fence::{assert_house_custody_fence, assert_house_inventory_fence};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use sqlx::{Postgres, Row, Transaction};

/// One immutable committed native House edge, shared by lifecycle and login-clock readers.
pub(super) struct CommittedHousePredecessor {
    pub(super) transfer: [u8; 16],
    pub(super) source: GameSessionId,
    pub(super) source_transport: AuthenticatedTransportRefV1,
    pub(super) source_generation: ConnectionGeneration,
    pub(super) source_lease: u64,
}

impl FreshAdmissionStore {
    /// Retain the actual carrier's node proof for the narrow named-House loss,
    /// recovery and terminal paths. Generic Instance admission stays closed.
    #[must_use]
    pub fn with_house_lifecycle_owner(&self, node: &NodeIncarnationProof) -> Self {
        self.with_house_release_owner(node)
    }

    pub(super) async fn initial_operation_locked(
        &self, tx: &mut Transaction<'_, Postgres>, session: GameSessionId,
    ) -> Result<FreshAdmissionOperationV1> {
        let payload: Option<String> = sqlx::query_scalar(
            "SELECT CASE WHEN octet_length(r.operation_json)<=65536 THEN r.operation_json END \
             FROM game_durability_reconnect_sessions s \
             JOIN game_durability_fresh_admission_receipts r \
               ON r.replay_key=COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) \
             WHERE s.game_session_id=encode($1,'hex')::uuid FOR SHARE OF s,r",
        ).bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        decode_operation(&payload.ok_or(DurabilityError::InvalidStoredState)?, self.maximum_operation_bytes)
    }

    /// Current named custody only. Loss/terminal history must remain writable
    /// after ACL revocation; granting a recovered controller also requires ACL.
    pub(super) async fn house_lifecycle_fence_locked(
        &self, tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, require_access: bool,
    ) -> Result<Option<ChannelId>> {
        if !matches!(current.current_runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
            return Ok(None);
        }
        let Some(node) = &self.house_release_node else { return Ok(None); };
        let origin: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT uuid_send(origin_channel_id) FROM game_durability_reconnect_sessions \
             WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_kind=2 \
               AND runtime_scope_house_key IS NOT NULL FOR SHARE",
        ).bind(current.current_game_session_id().as_bytes().as_slice())
            .fetch_optional(&mut **tx).await?.flatten();
        let Some(origin) = origin else { return Ok(None); };
        let origin = checked(ChannelId::decode(&origin))?;
        let fence = CurrentCharacterItemFence {
            game_session_id: current.current_game_session_id(),
            character_id: checked(crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()))?,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
        };
        let valid = if require_access {
            assert_house_inventory_fence(tx, node, &fence, *origin.as_bytes()).await?
        } else {
            assert_house_custody_fence(tx, node, &fence, *origin.as_bytes()).await?.is_some()
        };
        Ok(valid.then_some(origin))
    }

    /// Compatibility publication for the real scope. A House uses the same
    /// origin publication as native transfer recovery, after independently
    /// proving its current House authority. The Channel generation is never
    /// substituted into the House session or its recovery source observation.
    pub(super) async fn recovery_runtime_locked(
        &self, tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<AdmissionAuthorityPublicationChangeV1>> {
        let scope = match current.current_runtime_scope() {
            RuntimeScopeRefV1::Channel { .. } => current.current_runtime_scope(),
            RuntimeScopeRefV1::Instance { .. } => {
                let Some(origin) = self.house_lifecycle_fence_locked(tx, current, true).await? else {
                    return Ok(None);
                };
                RuntimeScopeRefV1::channel(current.commit().world_id(), origin)
            }
        };
        let publication = self.guards.load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope)).await?;
        let Some(AdmissionAuthorityGuardStateV1::Runtime { ownership_generation, ready: true, .. })
            = publication.as_ref().map(|row| &row.state) else { return Ok(None); };
        if scope == current.current_runtime_scope()
            && *ownership_generation != current.current_scope_generation().get() { return Ok(None); }
        Ok(publication)
    }

    pub async fn recovery_runtime_publication(
        &self, session: GameSessionId,
    ) -> Result<Option<AdmissionAuthorityPublicationChangeV1>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::admission_journal::begin_pass_transaction(holder, deadline).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let current = store.current_session_locked(&mut tx, session).await?;
            let result = store.recovery_runtime_locked(&mut tx, current).await?;
            super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }

    /// The entry marker is not a recovered loss. Only the actual immutable
    /// transfer chain can establish this third origin, retaining protection
    /// from the last real recovery rather than minting fresh protection.
    pub async fn transferred_loss_history(&self, session: GameSessionId) -> Result<Option<ControlLossHistoryV1>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::admission_journal::begin_pass_transaction(holder, deadline).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let current = store.current_session_locked(&mut tx, session).await?;
            let result = store.transferred_loss_history_locked(&mut tx, current).await?;
            super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }

    pub(super) async fn transferred_loss_history_locked(
        &self, tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<ControlLossHistoryV1>> {
        let (GameSessionState::Active, Some(entry_epoch), Some(mut transport)) = (
            current.session_state(), current.current_control_loss_epoch(), current.current_transport(),
        ) else { return Ok(None); };
        if entry_epoch.get() != 1 || current.current_game_session_id() == current.commit().game_session_id() {
            return Ok(None);
        }
        let mut cursor = current;
        let mut first_transfer = None;
        // A native transfer can only target an unused session and terminalizes
        // its source. The visited set rejects malformed cycles; the existing
        // semantic-pass deadline bounds this historical walk and all SQL work.
        let mut seen = std::collections::BTreeSet::new();
        let protection = loop {
            let session = cursor.current_game_session_id();
            if !seen.insert(*session.as_bytes()) { return Err(DurabilityError::InvalidStoredState); }
            let Some(link) = self.committed_house_predecessor_locked(tx, cursor).await? else { return Ok(None); };
            first_transfer.get_or_insert(link.transfer);
            let source_transport = link.source_transport;
            // Initial entry keeps the authentic source transport. Early lost-
            // barrier recovery has its own retained nonce/transport reservation.
            if cursor.current_connection_generation().get() == 1 {
                if transport != source_transport { return Ok(None); }
            } else {
                let generation = cursor.current_connection_generation().get().to_string();
                let recovery: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts r \
                     JOIN game_durability_transport_ref_reservations v ON v.game_session_id=encode($1,'hex')::uuid \
                       AND v.transport_ref=$2 AND v.reservation_owner=1 \
                     WHERE substring(r.operation_key FROM 1 FOR 26)=convert_to('house-transfer-recovery-v1','UTF8') \
                       AND octet_length(r.operation_json)<=2048 \
                       AND r.operation_json::jsonb->'session'=to_jsonb($3::integer[]) \
                       AND r.operation_json::jsonb->'transport'=to_jsonb($4::integer[]) \
                       AND r.operation_json::jsonb->>'generation'=$5)",
                ).bind(session.as_bytes().as_slice()).bind(transport.to_bytes().as_slice())
                 .bind(session.as_bytes().iter().map(|v| i32::from(*v)).collect::<Vec<_>>())
                 .bind(transport.to_bytes().iter().map(|v| i32::from(*v)).collect::<Vec<_>>())
                 .bind(generation).fetch_one(&mut **tx).await?;
                if !recovery { return Ok(None); }
            }
            // This marker cannot alias any real committed owning-loss epoch.
            if self.owning_loss_locked(tx, session, checked(ControlLossEpochRefV1::new(1))?).await?.is_some() {
                return Ok(None);
            }
            let source = link.source;
            let prior = self.current_session_locked(tx, source).await?;
            if prior.commit() != current.commit()
                || prior.session_state() != GameSessionState::Terminal
                || prior.current_connection_generation() != link.source_generation
                || prior.current_character_lease().generation() != link.source_lease {
                return Err(DurabilityError::InvalidStoredState);
            }
            if let Some(epoch) = prior.current_control_loss_epoch() {
                if let Some(protection) = self.retained_protection_locked(tx, source, epoch, source_transport).await? {
                    break protection;
                }
                if epoch.get() != 1 { return Ok(None); }
                cursor = prior;
                transport = source_transport;
            } else {
                if prior.current_game_session_id() != prior.commit().game_session_id()
                    || prior.current_connection_generation() != prior.commit().connection_generation()
                    || source_transport != prior.commit().initial_transport() { return Ok(None); }
                break RecoveryProtectionContinuityV1 {
                    usage: RecoveryProtectionUseV1::Unused { entitlement_generation: 1 },
                    rearm: RecoveryProtectionRearmV1::NotRearmed {
                        generation: 1, stable_control_started_at: None, accepted_deadline: None,
                    },
                };
            }
        };
        Ok(Some(ControlLossHistoryV1::ScopeTransfer {
            transfer: first_transfer.ok_or(DurabilityError::InvalidStoredState)?, entry_epoch, protection,
        }))
    }


    /// History only: this edge never grants current input or write authority.
    /// Both readers hold admission relation locks and separately prove current custody.
    pub(super) async fn committed_house_predecessor_locked(
        &self, tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<CommittedHousePredecessor>> {
        let session = current.current_game_session_id();
        let Some(row) = sqlx::query(
            "SELECT uuid_send(t.transfer_id) AS transfer,uuid_send(t.source_game_session_id) AS source, \
                t.source_transport,t.source_connection_generation::text AS source_generation, \
                t.source_character_lease_generation::text AS source_lease \
             FROM game_house_transfer_receipts t \
             JOIN game_durability_reconnect_sessions d ON d.game_session_id=t.destination_game_session_id \
             JOIN game_durability_reconnect_sessions s ON s.game_session_id=t.source_game_session_id \
             WHERE t.destination_game_session_id=encode($1,'hex')::uuid AND t.state=2 \
               AND t.character_id=d.character_id AND t.account_id=d.account_id AND t.world_id=d.world_id \
               AND s.character_id=d.character_id AND s.account_id=d.account_id AND s.world_id=d.world_id \
               AND s.session_state=3 AND s.current_transport_ref IS NULL \
               AND s.current_generation=t.source_connection_generation \
               AND s.character_lease_generation=t.source_character_lease_generation \
               AND d.character_lease_generation=t.source_character_lease_generation+1 \
               AND d.scope_ownership_generation=t.destination_scope_generation \
               AND d.initial_fresh_replay_key=COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) \
               AND ((t.direction=1 AND d.runtime_scope_kind=2 AND d.runtime_scope_house_key=t.house_key \
                 AND d.origin_channel_id=t.origin_channel_id \
                 AND d.runtime_scope_instance_id=game_house_scope_instance_id(t.world_id,t.house_key)) \
                OR (t.direction=2 AND d.runtime_scope_kind=1 AND d.runtime_scope_channel_id=t.origin_channel_id)) \
             FOR SHARE OF t,s,d",
        ).bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else { return Ok(None); };
        Ok(Some(CommittedHousePredecessor {
            transfer: checked(row.try_get::<Vec<u8>,_>("transfer")?.try_into())?,
            source: checked(GameSessionId::decode(&row.try_get::<Vec<u8>,_>("source")?))?,
            source_transport: checked(AuthenticatedTransportRefV1::decode(&row.try_get::<Vec<u8>,_>("source_transport")?))?,
            source_generation: checked(ConnectionGeneration::new(checked(row.try_get::<String,_>("source_generation")?.parse())?))?,
            source_lease: checked(row.try_get::<String,_>("source_lease")?.parse())?,
        }))
    }

    async fn retained_protection_locked(
        &self, tx: &mut Transaction<'_, Postgres>, session: GameSessionId,
        epoch: ControlLossEpochRefV1, transport: AuthenticatedTransportRefV1,
    ) -> Result<Option<RecoveryProtectionContinuityV1>> {
        let budget = self.recovery_budget_locked(tx, session, epoch).await?;
        let Some(committed) = budget.entries().iter().find(|entry|
            entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed && entry.transport == transport)
            else { return Ok(None); };
        let mut key = complete_reconnect_prefix(COMPLETE_COMMIT_KEY, session, epoch.get());
        key.extend_from_slice(&committed.attempt.to_be_bytes());
        let committed_at: i64 = sqlx::query_scalar(
            "SELECT decided_at FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1 FOR SHARE",
        ).bind(key).fetch_one(&mut **tx).await?;
        let (loss, _) = self.owning_loss_locked(tx, session, epoch).await?.ok_or(DurabilityError::InvalidStoredState)?;
        Ok(Some(checked(loss.observation.protection.after_complete_reconnect(committed_at))?))
    }
}
