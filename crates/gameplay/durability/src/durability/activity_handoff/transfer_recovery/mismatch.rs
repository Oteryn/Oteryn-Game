//! D449 for an unused offline Activity return. The normal terminal writer
//! repeats this sealed proof in its deciding transaction; discovery alone
//! cannot release a Character or consume a recovery credential.
use super::*;
use crate::foundation::GameSessionAuthoritySnapshot;

#[derive(Clone)]
pub(crate) struct OfflineActivityMismatchRelease {
    root: DurabilityRoot,
    recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    candidate: ActivityTransferRecovery,
    verified: VerifiedRecoveryDurabilityFactsV2,
    transport: AuthenticatedTransportRefV1,
    capabilities: CurrentCapabilityCheck,
}

impl ActivityHandoffOwner {
    /// Bind an authenticated refusal to the existing terminal-release owner.
    /// This creates no authority: every field is independently checked again
    /// at the physical release transaction, after native release preflight.
    pub fn offline_return_mismatch_store(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: ActivityTransferRecovery,
        verified: VerifiedRecoveryDurabilityFactsV2,
        transport: AuthenticatedTransportRefV1,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<FreshAdmissionStore> {
        if candidate.started
            || candidate.has_owning_loss
            || candidate.return_id.is_none()
            || candidate.connection_generation != 1
            || !matches!(candidate.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(DurabilityError::Unavailable);
        }
        let proof = OfflineActivityMismatchRelease {
            root: self.root.clone(),
            recovery: authority
                .record_for(&self.root)
                .map_err(|_| DurabilityError::Unavailable)?,
            node: node.clone(),
            candidate,
            verified,
            transport,
            capabilities,
        };
        Ok(FreshAdmissionStore::from_root(self.root.clone()).with_offline_activity_mismatch(proof))
    }
}

impl OfflineActivityMismatchRelease {
    pub(crate) async fn validate(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<bool> {
        assert_recovery_fence(tx, &self.recovery).await?;
        if snapshot.current_game_session_id() != self.candidate.current_session
            || snapshot.session_state() != GameSessionState::Reconnectable
            || snapshot.current_transport().is_some()
            || snapshot.current_connection_generation().get() != 1
            || snapshot
                .current_control_loss_epoch()
                .is_none_or(|epoch| epoch.get() != 1)
            || !prove_current_incarnation(tx, &self.node).await?
        {
            return Ok(false);
        }
        sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(self.candidate.current.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx).await?;
        let Some(current) = load(tx, &self.root, self.candidate.current_session).await? else {
            return Ok(false);
        };
        if current.started
            || current.has_owning_loss
            || current.connection_generation != self.candidate.connection_generation
            || !self.candidate.same_handoff(&current)
        {
            return Ok(false);
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut **tx)
                .await?;
        if snapshot
            .current_original_grace_deadline()
            .is_none_or(|deadline| deadline <= now)
        {
            return Ok(false);
        }
        let audit = self.verified.audit();
        let subject = RecoveryEvidenceSubject::new(&audit.account_id, &audit.signing.key_id)?;
        if decide_with_registered_recovery(tx, &self.node, &subject, |source| {
            self.verified.revalidate(
                now,
                &RecoveryDurabilityTrustContextV2::from_owning_source(source),
                &current.current,
            )
        })
        .await?
        .is_err()
        {
            return Ok(false);
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        if !assert_runtime_scope_fence(
            tx,
            current.runtime_scope,
            current.scope_generation,
            &self.node,
        )
        .await?
            || !store
                .check_current_runtime_capabilities(
                    tx,
                    current.runtime_scope,
                    current.scope_generation.get(),
                    &self.capabilities,
                )
                .await?
            || assert_item_only_fence(tx, &current.fence()?, &self.node)
                .await?
                .is_err()
        {
            return Ok(false);
        }
        // An unused offline successor has no attempt, input or independent loss.
        // Pending placement and already consumed credentials stay with their owners.
        let eligible: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s \
             JOIN game_activity_runtime_returns r ON r.return_id=s.activity_return_id \
             WHERE s.game_session_id=encode($1,'hex')::uuid AND s.attempt_count=0 \
               AND s.prepared_attempt_ref IS NULL AND s.predecessor_generation=1 \
               AND r.destination_game_session_id=s.game_session_id AND r.destination_transport_ref IS NULL) \
             AND NOT EXISTS(SELECT 1 FROM game_activity_transfer_recoveries WHERE game_session_id=encode($1,'hex')::uuid) \
             AND NOT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($2,'hex')::uuid) \
             AND NOT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($2,'hex')::uuid) \
             AND NOT EXISTS(SELECT 1 FROM game_durability_recovery_grant_consumptions WHERE recovery_grant_nonce=$3) \
             AND NOT EXISTS(SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref=$4)")
            .bind(current.current_session.as_bytes().as_slice())
            .bind(current.current.character_id.as_bytes().as_slice())
            .bind(audit.grant_nonce.as_slice())
            .bind(self.transport.to_bytes().as_slice())
            .fetch_one(&mut **tx).await?;
        Ok(eligible)
    }
}
