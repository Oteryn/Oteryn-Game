//! Activity-session loss provenance, retained separately from the initial login.
use super::*;
use sqlx::{Postgres, Transaction};

impl FreshAdmissionStore {
    /// Inert history for the sealed native loss observer. This never records a
    /// loss or grants control. An active destination must still have either its
    /// issued activity transport or the exact committed lost-ack recovery.
    pub async fn activity_loss_origin(
        &self,
        session: GameSessionId,
    ) -> Result<Option<ControlLossHistoryV1>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::admission_journal::begin_pass_transaction(holder, deadline)
                            .await?;
                    super::super::db::lock_admission_relations(&mut tx).await?;
                    let current = store.current_session_locked(&mut tx, session).await?;
                    let result = if current.session_state() == GameSessionState::Active
                        && current.current_control_loss_epoch().is_none()
                        && current.current_original_grace_deadline().is_none()
                    {
                        match store.activity_origin_id_locked(&mut tx, current).await? {
                            Some(admission_id)
                                if store
                                    .activity_origin_transport_locked(
                                        &mut tx,
                                        current,
                                        admission_id,
                                    )
                                    .await? =>
                            {
                                Some(ControlLossHistoryV1::ActivityOrigin {
                                    admission_id,
                                    protection: store
                                        .activity_origin_protection_locked(&mut tx, current)
                                        .await?,
                                })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Capture the source owner's exact protection before a handoff terminalizes
    /// its session and attempts. Neither entry nor return activates or rearms it.
    /// There is no request field through which a caller may select this value.
    pub(crate) async fn activity_handoff_protection_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        source: super::super::item_transfer::CurrentCharacterItemFence,
        now: i64,
    ) -> Result<Vec<u8>> {
        let current = self
            .current_session_locked(tx, source.game_session_id)
            .await?;
        if current.current_game_session_id() != source.game_session_id
            || current.current_character_lease().character_id().as_bytes()
                != source.character_id.as_bytes()
            || current.current_character_lease().generation() != source.character_lease_generation
            || current.current_connection_generation() != source.connection_generation
            || current.current_runtime_scope() != source.runtime_scope
            || current.current_scope_generation() != source.scope_ownership_generation
        {
            return Err(DurabilityError::Unavailable);
        }
        let protection = match (
            current.session_state(),
            current.current_control_loss_epoch(),
        ) {
            (GameSessionState::Active, None) => {
                if let Some(id) = self.activity_origin_id_locked(tx, current).await? {
                    if !self
                        .activity_origin_transport_locked(tx, current, id)
                        .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    self.activity_origin_protection_locked(tx, current).await?
                } else {
                    // The existing first-loss owner starts with precisely this
                    // continuity only for the actual original fresh session.
                    if current.current_game_session_id() != current.commit().game_session_id()
                        || current.current_connection_generation()
                            != current.commit().connection_generation()
                        || current.current_transport() != Some(current.commit().initial_transport())
                        || current.current_original_grace_deadline().is_some()
                        || self
                            .session_fresh_operation_locked(tx, source.game_session_id)
                            .await?
                            .is_none()
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    RecoveryProtectionContinuityV1 {
                        usage: RecoveryProtectionUseV1::Unused {
                            entitlement_generation: 1,
                        },
                        rearm: RecoveryProtectionRearmV1::NotRearmed {
                            generation: 1,
                            stable_control_started_at: None,
                            accepted_deadline: None,
                        },
                    }
                }
            }
            (GameSessionState::Active, Some(_)) => {
                let Some(ControlLossHistoryV1::Resumed { protection, .. }) =
                    self.resumed_history_locked(tx, current).await?
                else {
                    return Err(DurabilityError::Unavailable);
                };
                protection
            }
            (GameSessionState::Reconnectable, Some(epoch)) => {
                if let Some((loss, _)) = self
                    .owning_loss_locked(tx, source.game_session_id, epoch)
                    .await?
                {
                    if loss.observation.session.current_game_session_id() != source.game_session_id
                        || loss.observation.session.current_connection_generation()
                            != current.current_connection_generation()
                        || Some(loss.observation.original_grace_deadline)
                            != current.current_original_grace_deadline()
                        || current.current_transport().is_some()
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    loss.observation.protection
                } else {
                    // An offline return inherits a prior loss's deadline but
                    // issues no new loss. Its immutable return is the only
                    // supported missing-owning-loss case.
                    let inherited: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_activity_runtime_returns r ON r.return_id=s.activity_return_id AND r.destination_game_session_id=s.game_session_id WHERE s.game_session_id=encode($1,'hex')::uuid AND s.session_state=1 AND s.current_generation=1 AND s.control_loss_epoch=1 AND s.predecessor_generation=1 AND s.current_transport_ref IS NULL AND s.prepared_attempt_ref IS NULL AND s.attempt_count=0 AND r.destination_transport_ref IS NULL AND game_activity_session_origin_valid(s.game_session_id,s.character_id))")
                        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    if !inherited {
                        return Err(DurabilityError::Unavailable);
                    }
                    self.activity_origin_protection_locked(tx, current).await?
                }
            }
            _ => return Err(DurabilityError::Unavailable),
        };
        checked(protection.validate_at(now))?;
        let mut writer = Writer::new(64);
        write_protection(&mut writer, protection)?;
        Ok(writer.bytes)
    }

    pub(super) async fn activity_origin_protection_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<RecoveryProtectionContinuityV1> {
        if self.activity_origin_id_locked(tx, current).await?.is_none() {
            return Err(DurabilityError::Unavailable);
        }
        let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT CASE WHEN octet_length(activity_recovery_protection)<=64 THEN activity_recovery_protection END FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR SHARE")
            .bind(current.current_game_session_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let bytes = bytes.ok_or(DurabilityError::InvalidStoredState)?;
        let mut reader = Reader::new(&bytes);
        let protection = control_loss_codec::read_protection(&mut reader)?;
        reader.finish()?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut **tx)
                .await?;
        checked(protection.validate_at(now))?;
        let mut canonical = Writer::new(64);
        write_protection(&mut canonical, protection)?;
        if canonical.bytes != bytes {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(protection)
    }

    /// Resolve the immutable original through the destination's retained anchor.
    /// Activity descendants additionally require their complete session-use and
    /// handoff history. This does not reinterpret a new identity as a fresh grant.
    pub(super) async fn session_fresh_operation_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session: GameSessionId,
    ) -> Result<Option<FreshAdmissionOperationV1>> {
        let payload: Option<Option<String>> = sqlx::query_scalar(
            "SELECT CASE WHEN octet_length(r.operation_json)<=65536 THEN r.operation_json END FROM game_durability_reconnect_sessions s JOIN game_durability_fresh_admission_receipts r ON r.replay_key=COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) WHERE s.game_session_id=encode($1,'hex')::uuid FOR SHARE OF s,r")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let operation = decode_operation(
            &payload.ok_or(DurabilityError::InvalidStoredState)?,
            self.maximum_operation_bytes,
        )?;
        if operation.authorization.candidate_session != session {
            let character = checked(operation.authorization.initial_commit())?.character_id();
            if !self
                .committed_activity_origin_in_transaction(tx, character, session)
                .await?
            {
                return Err(DurabilityError::Unavailable);
            }
            let valid: bool = sqlx::query_scalar("SELECT COALESCE(game_activity_session_origin_valid(game_session_id,character_id),false) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(session.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if !valid {
                return Err(DurabilityError::Unavailable);
            }
        }
        Ok(Some(operation))
    }

    pub(super) async fn activity_origin_id_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<[u8; 16]>> {
        let session = current.current_game_session_id();
        if session == current.commit().game_session_id()
            || !self
                .committed_activity_origin_in_transaction(
                    tx,
                    current.commit().character_id(),
                    session,
                )
                .await?
        {
            return Ok(None);
        }
        let value: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(COALESCE(activity_return_id,activity_handoff_id)) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND game_activity_session_origin_valid(game_session_id,character_id)")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.flatten();
        value
            .map(|value| {
                value
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)
            })
            .transpose()
    }

    pub(super) async fn activity_origin_transport_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        admission_id: [u8; 16],
    ) -> Result<bool> {
        let Some(transport) = current.current_transport() else {
            return Ok(false);
        };
        sqlx::query_scalar(r#"
            SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
                WHERE s.game_session_id=encode($1,'hex')::uuid AND s.session_state=2
                  AND s.current_generation=$2::text::numeric AND s.current_transport_ref=$3
                  AND COALESCE(s.activity_return_id,s.activity_handoff_id)=encode($4,'hex')::uuid
                  AND s.control_loss_epoch IS NULL AND s.original_grace_deadline IS NULL
                  AND s.prepared_attempt_ref IS NULL AND s.attempt_count=0
                  AND game_activity_session_origin_valid(s.game_session_id,s.character_id)
                  AND ((s.current_generation=1 AND EXISTS(
                    SELECT 1 FROM game_durability_transport_ref_reservations t
                    WHERE t.transport_ref=$3 AND t.game_session_id=s.game_session_id AND t.reservation_owner=3
                      AND t.activity_handoff_id IS NOT DISTINCT FROM s.activity_handoff_id
                      AND t.activity_return_id IS NOT DISTINCT FROM s.activity_return_id))
                    OR EXISTS(SELECT 1 FROM game_activity_transfer_recoveries r
                      JOIN game_durability_transport_ref_reservations t ON t.transport_ref=r.transport_ref
                      JOIN game_durability_recovery_grant_consumptions g ON g.recovery_grant_nonce=r.recovery_grant_nonce
                      WHERE r.game_session_id=s.game_session_id AND r.connection_generation=s.current_generation
                        AND r.transport_ref=$3 AND COALESCE(r.return_id,r.handoff_id)=encode($4,'hex')::uuid
                        AND t.game_session_id=s.game_session_id AND t.reservation_owner=1
                        AND t.reconnect_attempt_ref=r.reconnect_attempt_ref
                        AND g.game_session_id=s.game_session_id AND g.reconnect_attempt_ref=r.reconnect_attempt_ref)))
        "#).bind(current.current_game_session_id().as_bytes().as_slice())
            .bind(current.current_connection_generation().get().to_string()).bind(transport.to_bytes().as_slice())
            .bind(admission_id.as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)
    }
}
