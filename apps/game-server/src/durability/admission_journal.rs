use crate::durability::{DurabilityError, schema};
use oteryn_game_server::foundation::{
    ProtectionEntitlementV1, ReconnectCommitDispositionV1, ReconnectCommitRequestV1,
    ReconnectDurabilityRecordV1, ReconnectDurableReconciliationSnapshotV1,
    ReconnectPrepareDispositionV1, ReconnectPrepareRequestV1, ReconnectProofV1, RuntimeScopeRefV1,
};
use serde_json::json;
use sqlx::{PgPool, Postgres, Row, Transaction};

const PREPARED: i16 = 1;
const COLLISION_TERMINAL: i16 = 2;
const CONCURRENT_TERMINAL: i16 = 3;
const STALE_TERMINAL: i16 = 4;
const COMMITTED: i16 = 5;
const RECONNECTABLE: i16 = 1;
const ACTIVE: i16 = 2;
const MAX_ATTEMPTS_PER_EPOCH: i16 = 8;

/// Asynchronous PostgreSQL worker for the Foundation V1 split-phase port.
///
/// It is intentionally not an implementation of Foundation's synchronous
/// compatibility trait: callers submit the V1 request and later feed the typed
/// result back into Foundation as a fresh normalized input.
#[derive(Clone)]
pub struct AdmissionReconnectJournal {
    pool: PgPool,
}

impl AdmissionReconnectJournal {
    pub async fn connect_runtime(database_url: &str) -> Result<Self, DurabilityError> {
        Ok(Self {
            pool: schema::connect_runtime(database_url).await?,
        })
    }

    /// Atomically creates the exact PREPARED record and its transport-ref
    /// reservation, or records the exact durable terminal classification.
    pub async fn prepare(
        &self,
        request: &ReconnectPrepareRequestV1,
    ) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
        let record = request.record();
        let session_id = record.identity().game_session_id().as_bytes().to_vec();
        let attempt_ref = record
            .identity()
            .reconnect_attempt_ref()
            .to_be_bytes()
            .to_vec();
        let transport_ref = record.connection().transport_ref().to_bytes().to_vec();
        let epoch = i64::try_from(record.continuity().control_loss_epoch().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let predecessor = i64::try_from(record.connection().predecessor().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let character_lease = i64::try_from(record.authority().character_lease_generation())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let scope_generation = i64::try_from(record.authority().scope_ownership_generation().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let prepared_deadline = record.continuity().prepared_deadline();
        let encoded_record = encode_record(record).to_string();

        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions (\
                game_session_id, control_loss_epoch, predecessor_generation, \
                character_lease_generation, scope_ownership_generation, current_generation\
             ) VALUES ($1, $2, $3, $4, $5, $3)\
             ON CONFLICT (game_session_id) DO NOTHING",
        )
        .bind(session_id.as_slice())
        .bind(epoch)
        .bind(predecessor)
        .bind(character_lease)
        .bind(scope_generation)
        .execute(&mut *transaction)
        .await?;

        let session = sqlx::query(
            "SELECT control_loss_epoch, predecessor_generation, character_lease_generation, \
                scope_ownership_generation, attempt_count, prepared_attempt_ref \
             FROM game_durability_reconnect_sessions \
             WHERE game_session_id = $1 FOR UPDATE",
        )
        .bind(session_id.as_slice())
        .fetch_one(&mut *transaction)
        .await?;
        // The session row is our per-session transaction mutex. Check the
        // idempotency key only after it is held, otherwise two concurrent
        // identical requests could both observe an empty attempt table.
        let existing = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = $1 AND reconnect_attempt_ref = $2",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(existing) = existing {
            let stored_record: String = existing.try_get("record_json")?;
            if stored_record != encoded_record {
                return Ok(ReconnectPrepareDispositionV1::IdempotencyConflict);
            }
            return disposition_for_existing(existing.try_get("state")?);
        }
        let is_current = session.try_get::<i64, _>("control_loss_epoch")? == epoch
            && session.try_get::<i64, _>("predecessor_generation")? == predecessor
            && session.try_get::<i64, _>("character_lease_generation")? == character_lease
            && session.try_get::<i64, _>("scope_ownership_generation")? == scope_generation;
        if !is_current || database_now(&mut transaction).await? > prepared_deadline {
            insert_terminal(
                &mut transaction,
                session_id.as_slice(),
                attempt_ref.as_slice(),
                epoch,
                transport_ref.as_slice(),
                &encoded_record,
                STALE_TERMINAL,
            )
            .await?;
            transaction.commit().await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
        }

        let count: i16 = session.try_get("attempt_count")?;
        if count >= MAX_ATTEMPTS_PER_EPOCH {
            return Ok(ReconnectPrepareDispositionV1::AttemptCapacityExceeded);
        }
        let incumbent: Option<Vec<u8>> = session.try_get("prepared_attempt_ref")?;
        if incumbent.is_some() {
            insert_terminal(
                &mut transaction,
                session_id.as_slice(),
                attempt_ref.as_slice(),
                epoch,
                transport_ref.as_slice(),
                &encoded_record,
                CONCURRENT_TERMINAL,
            )
            .await?;
            increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
            transaction.commit().await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedConcurrentPrepared);
        }

        let reservation = sqlx::query(
            "INSERT INTO game_durability_transport_ref_reservations \
                (transport_ref, game_session_id, reconnect_attempt_ref) \
             VALUES ($1, $2, $3) ON CONFLICT (transport_ref) DO NOTHING",
        )
        .bind(transport_ref.as_slice())
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .execute(&mut *transaction)
        .await?;
        if reservation.rows_affected() == 0 {
            insert_terminal(
                &mut transaction,
                session_id.as_slice(),
                attempt_ref.as_slice(),
                epoch,
                transport_ref.as_slice(),
                &encoded_record,
                COLLISION_TERMINAL,
            )
            .await?;
            increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
            transaction.commit().await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedTransportRefCollision);
        }

        sqlx::query(
            "INSERT INTO game_durability_reconnect_attempts (\
                game_session_id, reconnect_attempt_ref, control_loss_epoch, transport_ref, record_json, state\
             ) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .bind(epoch)
        .bind(transport_ref.as_slice())
        .bind(&encoded_record)
        .bind(PREPARED)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
             SET attempt_count = attempt_count + 1, prepared_attempt_ref = $2 \
             WHERE game_session_id = $1",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(ReconnectPrepareDispositionV1::Prepared)
    }

    /// Atomically installs the prepared controller only while the durable
    /// session still exactly carries the authority fenced by the request.
    pub async fn commit(
        &self,
        request: &ReconnectCommitRequestV1,
    ) -> Result<ReconnectCommitDispositionV1, DurabilityError> {
        let record = request.record();
        let session_id = record.identity().game_session_id().as_bytes().to_vec();
        let attempt_ref = record
            .identity()
            .reconnect_attempt_ref()
            .to_be_bytes()
            .to_vec();
        let transport_ref = record.connection().transport_ref().to_bytes().to_vec();
        let encoded_record = encode_record(record).to_string();
        let predecessor = i64::try_from(record.connection().predecessor().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let candidate = i64::try_from(record.connection().candidate().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let mut transaction = self.pool.begin().await?;
        let session = sqlx::query(
            "SELECT control_loss_epoch, predecessor_generation, character_lease_generation, \
                scope_ownership_generation, current_generation, current_transport_ref, \
                session_state, prepared_attempt_ref \
             FROM game_durability_reconnect_sessions WHERE game_session_id = $1 FOR UPDATE",
        )
        .bind(session_id.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(session) = session else {
            return Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority);
        };
        let attempt = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = $1 AND reconnect_attempt_ref = $2",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(attempt) = attempt else {
            return Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority);
        };
        if attempt.try_get::<String, _>("record_json")? != encoded_record {
            return Ok(ReconnectCommitDispositionV1::IdempotencyConflict);
        }
        match attempt.try_get::<i16, _>("state")? {
            COMMITTED => {
                let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
                if session.try_get::<i64, _>("current_generation")? == candidate
                    && current_ref.as_deref() == Some(transport_ref.as_slice())
                    && session.try_get::<i16, _>("session_state")? == ACTIVE
                {
                    transaction.commit().await?;
                    return Ok(ReconnectCommitDispositionV1::Committed);
                }
                return Err(DurabilityError::InvalidStoredState);
            }
            PREPARED => {}
            COLLISION_TERMINAL | CONCURRENT_TERMINAL | STALE_TERMINAL => {
                transaction.commit().await?;
                return Ok(ReconnectCommitDispositionV1::ExistingTerminal);
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        }
        let is_current = session.try_get::<i64, _>("control_loss_epoch")?
            == i64::try_from(record.continuity().control_loss_epoch().get())
                .map_err(|_error| DurabilityError::InvalidStoredState)?
            && session.try_get::<i64, _>("predecessor_generation")? == predecessor
            && session.try_get::<i64, _>("character_lease_generation")?
                == i64::try_from(record.authority().character_lease_generation())
                    .map_err(|_error| DurabilityError::InvalidStoredState)?
            && session.try_get::<i64, _>("scope_ownership_generation")?
                == i64::try_from(record.authority().scope_ownership_generation().get())
                    .map_err(|_error| DurabilityError::InvalidStoredState)?
            && session.try_get::<i64, _>("current_generation")? == predecessor
            && session
                .try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
                .is_none()
            && session.try_get::<i16, _>("session_state")? == RECONNECTABLE
            && session
                .try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
                .as_deref()
                == Some(attempt_ref.as_slice());
        if !is_current
            || database_now(&mut transaction).await?
                > request.authorization().authorization_deadline()
        {
            terminalize_stale_commit(
                &mut transaction,
                session_id.as_slice(),
                attempt_ref.as_slice(),
            )
            .await?;
            transaction.commit().await?;
            return Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority);
        }
        sqlx::query(
            "UPDATE game_durability_reconnect_attempts SET state = $3 \
             WHERE game_session_id = $1 AND reconnect_attempt_ref = $2 AND state = $4",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .bind(COMMITTED)
        .bind(PREPARED)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
             SET current_generation = $2, current_transport_ref = $3, session_state = $4, prepared_attempt_ref = NULL \
             WHERE game_session_id = $1",
        ).bind(session_id.as_slice()).bind(candidate).bind(transport_ref.as_slice()).bind(ACTIVE)
         .execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(ReconnectCommitDispositionV1::Committed)
    }

    /// Reads one exact persisted attempt and its current durable outcome.
    pub async fn reconcile(
        &self,
        request: &ReconnectPrepareRequestV1,
    ) -> Result<ReconnectDurableReconciliationSnapshotV1, DurabilityError> {
        let record = request.record();
        let session_id = record.identity().game_session_id().as_bytes().to_vec();
        let attempt_ref = record
            .identity()
            .reconnect_attempt_ref()
            .to_be_bytes()
            .to_vec();
        let transport_ref = record.connection().transport_ref().to_bytes().to_vec();
        let encoded_record = encode_record(record).to_string();
        let mut transaction = self.pool.begin().await?;
        let session = sqlx::query(
            "SELECT current_generation, current_transport_ref FROM game_durability_reconnect_sessions \
             WHERE game_session_id = $1 FOR SHARE",
        ).bind(session_id.as_slice()).fetch_optional(&mut *transaction).await?;
        let Some(session) = session else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let attempt = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = $1 AND reconnect_attempt_ref = $2",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(attempt) = attempt else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if attempt.try_get::<String, _>("record_json")? != encoded_record {
            return Err(DurabilityError::InvalidStoredState);
        }
        let snapshot = match attempt.try_get::<i16, _>("state")? {
            PREPARED => ReconnectDurableReconciliationSnapshotV1::prepared(record.clone()),
            COMMITTED => {
                let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
                let candidate = i64::try_from(record.connection().candidate().get())
                    .map_err(|_error| DurabilityError::InvalidStoredState)?;
                if session.try_get::<i64, _>("current_generation")? != candidate
                    || current_ref.as_deref() != Some(transport_ref.as_slice())
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                ReconnectDurableReconciliationSnapshotV1::committed(record.clone())
            }
            COLLISION_TERMINAL | CONCURRENT_TERMINAL | STALE_TERMINAL => {
                ReconnectDurableReconciliationSnapshotV1::terminal(record.clone())
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        transaction.commit().await?;
        Ok(snapshot)
    }
}

async fn database_now(transaction: &mut Transaction<'_, Postgres>) -> Result<i64, DurabilityError> {
    let row = sqlx::query("SELECT EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT AS now")
        .fetch_one(&mut **transaction)
        .await?;
    row.try_get("now").map_err(DurabilityError::from)
}

async fn insert_terminal(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: &[u8],
    attempt_ref: &[u8],
    epoch: i64,
    transport_ref: &[u8],
    encoded_record: &str,
    state: i16,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_durability_reconnect_attempts (\
            game_session_id, reconnect_attempt_ref, control_loss_epoch, transport_ref, record_json, state\
         ) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(attempt_ref)
    .bind(epoch)
    .bind(transport_ref)
    .bind(encoded_record)
    .bind(state)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn increment_attempt_count(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: &[u8],
) -> Result<(), DurabilityError> {
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET attempt_count = attempt_count + 1 WHERE game_session_id = $1",
    )
    .bind(session_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn terminalize_stale_commit(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: &[u8],
    attempt_ref: &[u8],
) -> Result<(), DurabilityError> {
    let terminalized = sqlx::query(
        "UPDATE game_durability_reconnect_attempts SET state = $3 \
         WHERE game_session_id = $1 AND reconnect_attempt_ref = $2 AND state = $4",
    )
    .bind(session_id)
    .bind(attempt_ref)
    .bind(STALE_TERMINAL)
    .bind(PREPARED)
    .execute(&mut **transaction)
    .await?;
    if terminalized.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = NULL \
         WHERE game_session_id = $1 AND prepared_attempt_ref = $2",
    )
    .bind(session_id)
    .bind(attempt_ref)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn disposition_for_existing(state: i16) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
    match state {
        PREPARED => Ok(ReconnectPrepareDispositionV1::ExistingPrepared),
        COLLISION_TERMINAL | CONCURRENT_TERMINAL | STALE_TERMINAL => {
            Ok(ReconnectPrepareDispositionV1::ExistingTerminal)
        }
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

fn encode_record(record: &ReconnectDurabilityRecordV1) -> serde_json::Value {
    let identity = record.identity();
    let connection = record.connection();
    let authority = record.authority();
    let continuity = record.continuity();
    let scope = match identity.runtime_scope() {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => json!({
            "kind": "channel",
            "world_id": world_id.as_bytes(),
            "channel_id": channel_id.as_bytes(),
        }),
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => json!({
            "kind": "instance",
            "world_id": world_id.as_bytes(),
            "instance_id": instance_id,
        }),
    };
    let protection = match continuity.protection_entitlement() {
        ProtectionEntitlementV1::Unused => json!({ "state": "unused" }),
        ProtectionEntitlementV1::Fenced { generation } => {
            json!({ "state": "fenced", "generation": generation })
        }
    };
    let proof = match record.proof() {
        ReconnectProofV1::FastReconnect {
            reconnect_proof_generation,
        } => json!({ "class": "fast_reconnect", "generation": reconnect_proof_generation }),
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce,
        } => {
            json!({ "class": "reauthenticated_recovery", "recovery_grant_nonce": recovery_grant_nonce })
        }
    };
    let fnd02 = record.fnd02();
    let compatibility = record.compatibility();
    json!({
        "version": record.version(),
        "identity": {
            "game_session_id": identity.game_session_id().as_bytes(),
            "reconnect_attempt_ref": identity.reconnect_attempt_ref().to_be_bytes(),
            "account_id": identity.account_id(),
            "character_id": identity.character_id().as_bytes(),
            "world_id": identity.world_id().as_bytes(),
            "runtime_scope": scope,
        },
        "connection": {
            "predecessor_generation": connection.predecessor().get(),
            "candidate_generation": connection.candidate().get(),
            "transport_ref": connection.transport_ref().to_bytes(),
        },
        "authority": {
            "character_lease_generation": authority.character_lease_generation(),
            "scope_ownership_generation": authority.scope_ownership_generation().get(),
            "expected_session_state": "reconnectable",
            "expected_no_current_controller": true,
        },
        "continuity": {
            "control_loss_epoch": continuity.control_loss_epoch().get(),
            "original_grace_deadline": continuity.original_grace_deadline(),
            "prepared_deadline": continuity.prepared_deadline(),
            "protection_entitlement": protection,
        },
        "proof": proof,
        "fnd02": {
            "next_command_id": fnd02.next_command_id().get(),
            "pending": fnd02.pending().iter().map(|pending| json!({
                "command_id": pending.command_id().get(),
                "disposition": match pending.disposition() {
                    oteryn_game_server::foundation::PendingCommandDispositionV1::PendingOriginal => "pending_original",
                    oteryn_game_server::foundation::PendingCommandDispositionV1::TerminalOutcomeRetained => "terminal_outcome_retained",
                },
            })).collect::<Vec<_>>(),
            "server_sequence": fnd02.server_sequence(),
            "domain_revisions": fnd02.domain_revisions().iter().map(|revision| json!({
                "domain_id": revision.domain_id(),
                "revision": revision.revision(),
            })).collect::<Vec<_>>(),
        },
        "compatibility": {
            "protocol_major": compatibility.protocol_major(),
            "transport_profile": compatibility.transport_profile(),
            "ruleset_revision": compatibility.ruleset_revision(),
            "content_revision": compatibility.content_revision(),
            "map_revision": compatibility.map_revision(),
            "world_policy_revision": compatibility.world_policy_revision(),
            "account_security_generation": compatibility.account_security_generation(),
            "platform_security_evidence": encode_evidence(compatibility.platform_security_evidence()),
            "proof_trust_evidence": encode_evidence(compatibility.proof_trust_evidence()),
            "credential_expiration": compatibility.credential_expiration(),
        },
    })
}

fn encode_evidence(
    evidence: &oteryn_game_server::foundation::AuthorityEvidenceFenceV1,
) -> serde_json::Value {
    json!({
        "authority": evidence.authority(),
        "purpose": evidence.purpose(),
        "scope": evidence.scope(),
        "source_revision": evidence.source_revision(),
        "decision_identity": evidence.decision_identity(),
        "source_observed_at": evidence.source_observed_at(),
    })
}
