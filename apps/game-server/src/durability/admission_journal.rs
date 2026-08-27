use crate::durability::{DurabilityError, schema};
use oteryn_game_server::foundation::{
    PendingCommandDispositionV1, ProtectionEntitlementV1, ReconnectCommitDispositionV1,
    ReconnectCommitRequestV1, ReconnectDurabilityRecordV1,
    ReconnectDurableReconciliationSnapshotV1, ReconnectPrepareDispositionV1,
    ReconnectPrepareRequestV1, ReconnectProofV1, RuntimeScopeRefV1,
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
const CHANNEL_SCOPE: i16 = 1;
const INSTANCE_SCOPE: i16 = 2;
const PENDING_ORIGINAL: i16 = 1;
const TERMINAL_OUTCOME_RETAINED: i16 = 2;
type ScopeStorage = (i16, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>);

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
             ) VALUES (encode($1, 'hex')::uuid, $2, $3, $4, $5, $3)\
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
                scope_ownership_generation, current_generation, current_transport_ref, session_state, \
                attempt_count, prepared_attempt_ref \
             FROM game_durability_reconnect_sessions \
             WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
        )
        .bind(session_id.as_slice())
        .fetch_one(&mut *transaction)
        .await?;
        let existing = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
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
            if !attempt_binding_is_valid(&mut transaction, record).await? {
                return Err(DurabilityError::InvalidStoredState);
            }
            return disposition_for_existing(existing.try_get("state")?);
        }

        let count: i16 = session.try_get("attempt_count")?;
        if count >= MAX_ATTEMPTS_PER_EPOCH {
            return Ok(ReconnectPrepareDispositionV1::AttemptCapacityExceeded);
        }

        let is_current = session.try_get::<i64, _>("control_loss_epoch")? == epoch
            && session.try_get::<i64, _>("predecessor_generation")? == predecessor
            && session.try_get::<i64, _>("character_lease_generation")? == character_lease
            && session.try_get::<i64, _>("scope_ownership_generation")? == scope_generation
            && session.try_get::<i64, _>("current_generation")? == predecessor
            && session
                .try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
                .is_none()
            && session.try_get::<i16, _>("session_state")? == RECONNECTABLE;
        if !is_current || database_now(&mut transaction).await? > prepared_deadline {
            insert_attempt(&mut transaction, record, &encoded_record, STALE_TERMINAL).await?;
            increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
            transaction.commit().await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
        }

        let incumbent: Option<Vec<u8>> = session.try_get("prepared_attempt_ref")?;
        if incumbent.is_some() {
            insert_attempt(
                &mut transaction,
                record,
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
             VALUES ($1, encode($2, 'hex')::uuid, $3) \
             ON CONFLICT (transport_ref) DO NOTHING",
        )
        .bind(transport_ref.as_slice())
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .execute(&mut *transaction)
        .await?;
        if reservation.rows_affected() == 0 {
            insert_attempt(
                &mut transaction,
                record,
                &encoded_record,
                COLLISION_TERMINAL,
            )
            .await?;
            increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
            transaction.commit().await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedTransportRefCollision);
        }

        insert_attempt(&mut transaction, record, &encoded_record, PREPARED).await?;
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
             SET attempt_count = attempt_count + 1, prepared_attempt_ref = $2 \
             WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(ReconnectPrepareDispositionV1::Prepared)
    }

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
        let recovery_grant_nonce = recovery_grant_nonce(record);
        let encoded_record = encode_record(record).to_string();
        let predecessor = i64::try_from(record.connection().predecessor().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let candidate = i64::try_from(record.connection().candidate().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let epoch = i64::try_from(record.continuity().control_loss_epoch().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let character_lease = i64::try_from(record.authority().character_lease_generation())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let scope_generation = i64::try_from(record.authority().scope_ownership_generation().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;

        let mut transaction = self.pool.begin().await?;
        let session = sqlx::query(
            "SELECT control_loss_epoch, predecessor_generation, character_lease_generation, \
                scope_ownership_generation, current_generation, current_transport_ref, \
                session_state, prepared_attempt_ref \
             FROM game_durability_reconnect_sessions \
             WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
        )
        .bind(session_id.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(session) = session else {
            return Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority);
        };
        let attempt = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
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
        if !attempt_binding_is_valid(&mut transaction, record).await? {
            return Err(DurabilityError::InvalidStoredState);
        }
        match attempt.try_get::<i16, _>("state")? {
            COMMITTED => {
                let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
                if session.try_get::<i64, _>("control_loss_epoch")? == epoch
                    && session.try_get::<i64, _>("predecessor_generation")? == predecessor
                    && session.try_get::<i64, _>("character_lease_generation")? == character_lease
                    && session.try_get::<i64, _>("scope_ownership_generation")? == scope_generation
                    && session.try_get::<i64, _>("current_generation")? == candidate
                    && current_ref.as_deref() == Some(transport_ref.as_slice())
                    && session.try_get::<i16, _>("session_state")? == ACTIVE
                    && session
                        .try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
                        .is_none()
                    && recovery_grant_binding_is_valid(
                        &mut transaction,
                        recovery_grant_nonce.as_deref(),
                        session_id.as_slice(),
                        attempt_ref.as_slice(),
                    )
                    .await?
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
        let is_current = session.try_get::<i64, _>("control_loss_epoch")? == epoch
            && session.try_get::<i64, _>("predecessor_generation")? == predecessor
            && session.try_get::<i64, _>("character_lease_generation")? == character_lease
            && session.try_get::<i64, _>("scope_ownership_generation")? == scope_generation
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

        if let Some(recovery_grant_nonce) = recovery_grant_nonce.as_deref() {
            let consumed = sqlx::query(
                "INSERT INTO game_durability_recovery_grant_consumptions (\
                    recovery_grant_nonce, game_session_id, reconnect_attempt_ref\
                 ) VALUES ($1, encode($2, 'hex')::uuid, $3) \
                 ON CONFLICT (recovery_grant_nonce) DO NOTHING",
            )
            .bind(recovery_grant_nonce)
            .bind(session_id.as_slice())
            .bind(attempt_ref.as_slice())
            .execute(&mut *transaction)
            .await?;
            if consumed.rows_affected() != 1 {
                terminalize_stale_commit(
                    &mut transaction,
                    session_id.as_slice(),
                    attempt_ref.as_slice(),
                )
                .await?;
                transaction.commit().await?;
                return Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority);
            }
        }

        let committed = sqlx::query(
            "UPDATE game_durability_reconnect_attempts SET state = $3 \
             WHERE game_session_id = encode($1, 'hex')::uuid \
               AND reconnect_attempt_ref = $2 AND state = $4",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .bind(COMMITTED)
        .bind(PREPARED)
        .execute(&mut *transaction)
        .await?;
        if committed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let advanced = sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
             SET current_generation = $2, current_transport_ref = $3, \
                 session_state = $4, prepared_attempt_ref = NULL \
             WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(session_id.as_slice())
        .bind(candidate)
        .bind(transport_ref.as_slice())
        .bind(ACTIVE)
        .execute(&mut *transaction)
        .await?;
        if advanced.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        transaction.commit().await?;
        Ok(ReconnectCommitDispositionV1::Committed)
    }

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
        let recovery_grant_nonce = recovery_grant_nonce(record);
        let encoded_record = encode_record(record).to_string();
        let epoch = i64::try_from(record.continuity().control_loss_epoch().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let predecessor = i64::try_from(record.connection().predecessor().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let candidate = i64::try_from(record.connection().candidate().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let character_lease = i64::try_from(record.authority().character_lease_generation())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;
        let scope_generation = i64::try_from(record.authority().scope_ownership_generation().get())
            .map_err(|_error| DurabilityError::InvalidStoredState)?;

        let mut transaction = self.pool.begin().await?;
        let session = sqlx::query(
            "SELECT control_loss_epoch, predecessor_generation, character_lease_generation, \
                scope_ownership_generation, current_generation, current_transport_ref, \
                session_state, prepared_attempt_ref \
             FROM game_durability_reconnect_sessions \
             WHERE game_session_id = encode($1, 'hex')::uuid FOR SHARE",
        )
        .bind(session_id.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(session) = session else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let attempt = sqlx::query(
            "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(attempt) = attempt else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if attempt.try_get::<String, _>("record_json")? != encoded_record
            || !attempt_binding_is_valid(&mut transaction, record).await?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let snapshot = match attempt.try_get::<i16, _>("state")? {
            PREPARED => {
                let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
                let prepared_ref: Option<Vec<u8>> = session.try_get("prepared_attempt_ref")?;
                if session.try_get::<i64, _>("control_loss_epoch")? != epoch
                    || session.try_get::<i64, _>("predecessor_generation")? != predecessor
                    || session.try_get::<i64, _>("character_lease_generation")? != character_lease
                    || session.try_get::<i64, _>("scope_ownership_generation")? != scope_generation
                    || session.try_get::<i64, _>("current_generation")? != predecessor
                    || current_ref.is_some()
                    || session.try_get::<i16, _>("session_state")? != RECONNECTABLE
                    || prepared_ref.as_deref() != Some(attempt_ref.as_slice())
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                ReconnectDurableReconciliationSnapshotV1::prepared(record.clone())
            }
            COMMITTED => {
                let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
                let prepared_ref: Option<Vec<u8>> = session.try_get("prepared_attempt_ref")?;
                if session.try_get::<i64, _>("control_loss_epoch")? != epoch
                    || session.try_get::<i64, _>("predecessor_generation")? != predecessor
                    || session.try_get::<i64, _>("character_lease_generation")? != character_lease
                    || session.try_get::<i64, _>("scope_ownership_generation")? != scope_generation
                    || session.try_get::<i64, _>("current_generation")? != candidate
                    || current_ref.as_deref() != Some(transport_ref.as_slice())
                    || session.try_get::<i16, _>("session_state")? != ACTIVE
                    || prepared_ref.is_some()
                    || !recovery_grant_binding_is_valid(
                        &mut transaction,
                        recovery_grant_nonce.as_deref(),
                        session_id.as_slice(),
                        attempt_ref.as_slice(),
                    )
                    .await?
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

fn recovery_grant_nonce(record: &ReconnectDurabilityRecordV1) -> Option<Vec<u8>> {
    match record.proof() {
        ReconnectProofV1::FastReconnect { .. } => None,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce,
        } => Some(recovery_grant_nonce.to_vec()),
    }
}

async fn recovery_grant_binding_is_valid(
    transaction: &mut Transaction<'_, Postgres>,
    recovery_grant_nonce: Option<&[u8]>,
    session_id: &[u8],
    attempt_ref: &[u8],
) -> Result<bool, DurabilityError> {
    let Some(recovery_grant_nonce) = recovery_grant_nonce else {
        return Ok(true);
    };
    let owner = sqlx::query(
        "SELECT uuid_send(game_session_id) AS game_session_id, reconnect_attempt_ref \
         FROM game_durability_recovery_grant_consumptions \
         WHERE recovery_grant_nonce = $1",
    )
    .bind(recovery_grant_nonce)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(owner) = owner else {
        return Ok(false);
    };
    let owner_session: Vec<u8> = owner.try_get("game_session_id")?;
    let owner_attempt: Vec<u8> = owner.try_get("reconnect_attempt_ref")?;
    Ok(owner_session.as_slice() == session_id && owner_attempt.as_slice() == attempt_ref)
}

async fn attempt_binding_is_valid(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
) -> Result<bool, DurabilityError> {
    let identity = record.identity();
    let session_id = identity.game_session_id().as_bytes().to_vec();
    let attempt_ref = identity.reconnect_attempt_ref().to_be_bytes();
    let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) = scope_storage(record);
    let row = sqlx::query(
        "SELECT account_id::text AS account_id, uuid_send(character_id) AS character_id, \
                uuid_send(world_id) AS world_id, runtime_scope_kind, \
                uuid_send(runtime_scope_world_id) AS runtime_scope_world_id, \
                CASE WHEN runtime_scope_channel_id IS NULL THEN NULL \
                     ELSE uuid_send(runtime_scope_channel_id) END AS runtime_scope_channel_id, \
                CASE WHEN runtime_scope_instance_id IS NULL THEN NULL \
                     ELSE uuid_send(runtime_scope_instance_id) END AS runtime_scope_instance_id, \
                fnd02_next_command_id::text AS fnd02_next_command_id \
         FROM game_durability_reconnect_attempts \
         WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
    )
    .bind(session_id.as_slice())
    .bind(attempt_ref.as_slice())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let stored_channel: Option<Vec<u8>> = row.try_get("runtime_scope_channel_id")?;
    let stored_instance: Option<Vec<u8>> = row.try_get("runtime_scope_instance_id")?;
    if row.try_get::<String, _>("account_id")? != identity.account_id()
        || row.try_get::<Vec<u8>, _>("character_id")?.as_slice()
            != identity.character_id().as_bytes().as_slice()
        || row.try_get::<Vec<u8>, _>("world_id")?.as_slice()
            != identity.world_id().as_bytes().as_slice()
        || row.try_get::<i16, _>("runtime_scope_kind")? != scope_kind
        || row
            .try_get::<Vec<u8>, _>("runtime_scope_world_id")?
            .as_slice()
            != scope_world_id.as_slice()
        || stored_channel.as_deref() != scope_channel_id.as_deref()
        || stored_instance.as_deref() != scope_instance_id.as_deref()
        || row.try_get::<String, _>("fnd02_next_command_id")?
            != record.fnd02().next_command_id().get().to_string()
    {
        return Ok(false);
    }

    let stored_pending = sqlx::query(
        "SELECT command_id::text AS command_id, disposition \
         FROM game_durability_reconnect_pending_commands \
         WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2 \
         ORDER BY command_id ASC",
    )
    .bind(session_id.as_slice())
    .bind(attempt_ref.as_slice())
    .fetch_all(&mut **transaction)
    .await?;
    if stored_pending.len() != record.fnd02().pending().len() {
        return Ok(false);
    }
    for (stored, expected) in stored_pending.iter().zip(record.fnd02().pending()) {
        if stored.try_get::<String, _>("command_id")? != expected.command_id().get().to_string()
            || stored.try_get::<i16, _>("disposition")?
                != pending_disposition(expected.disposition())
        {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn database_now(transaction: &mut Transaction<'_, Postgres>) -> Result<i64, DurabilityError> {
    let row = sqlx::query("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT AS now")
        .fetch_one(&mut **transaction)
        .await?;
    row.try_get("now").map_err(DurabilityError::from)
}

async fn insert_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    encoded_record: &str,
    state: i16,
) -> Result<(), DurabilityError> {
    let identity = record.identity();
    let session_id = identity.game_session_id().as_bytes().to_vec();
    let attempt_ref = identity.reconnect_attempt_ref().to_be_bytes();
    let epoch = i64::try_from(record.continuity().control_loss_epoch().get())
        .map_err(|_error| DurabilityError::InvalidStoredState)?;
    let transport_ref = record.connection().transport_ref().to_bytes();
    let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) = scope_storage(record);
    let next_command_id = record.fnd02().next_command_id().get().to_string();

    sqlx::query(
        "INSERT INTO game_durability_reconnect_attempts (\
            game_session_id, reconnect_attempt_ref, control_loss_epoch, transport_ref, \
            account_id, character_id, world_id, runtime_scope_kind, runtime_scope_world_id, \
            runtime_scope_channel_id, runtime_scope_instance_id, fnd02_next_command_id, \
            record_json, state\
         ) VALUES (\
            encode($1, 'hex')::uuid, $2, $3, $4, $5::text::uuid, \
            encode($6, 'hex')::uuid, encode($7, 'hex')::uuid, $8, \
            encode($9, 'hex')::uuid, encode($10, 'hex')::uuid, \
            encode($11, 'hex')::uuid, $12::text::numeric(20, 0), $13, $14\
         )",
    )
    .bind(session_id.as_slice())
    .bind(attempt_ref.as_slice())
    .bind(epoch)
    .bind(transport_ref.as_slice())
    .bind(identity.account_id())
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(identity.world_id().as_bytes().as_slice())
    .bind(scope_kind)
    .bind(scope_world_id.as_slice())
    .bind(scope_channel_id.as_deref())
    .bind(scope_instance_id.as_deref())
    .bind(&next_command_id)
    .bind(encoded_record)
    .bind(state)
    .execute(&mut **transaction)
    .await?;

    for pending in record.fnd02().pending() {
        let command_id = pending.command_id().get().to_string();
        sqlx::query(
            "INSERT INTO game_durability_reconnect_pending_commands (\
                game_session_id, reconnect_attempt_ref, command_id, disposition\
             ) VALUES (encode($1, 'hex')::uuid, $2, $3::text::numeric(20, 0), $4)",
        )
        .bind(session_id.as_slice())
        .bind(attempt_ref.as_slice())
        .bind(&command_id)
        .bind(pending_disposition(pending.disposition()))
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

fn scope_storage(record: &ReconnectDurabilityRecordV1) -> ScopeStorage {
    match record.identity().runtime_scope() {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => (
            CHANNEL_SCOPE,
            world_id.as_bytes().to_vec(),
            Some(channel_id.as_bytes().to_vec()),
            None,
        ),
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => (
            INSTANCE_SCOPE,
            world_id.as_bytes().to_vec(),
            None,
            Some(instance_id.to_vec()),
        ),
    }
}

const fn pending_disposition(disposition: PendingCommandDispositionV1) -> i16 {
    match disposition {
        PendingCommandDispositionV1::PendingOriginal => PENDING_ORIGINAL,
        PendingCommandDispositionV1::TerminalOutcomeRetained => TERMINAL_OUTCOME_RETAINED,
    }
}

async fn increment_attempt_count(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: &[u8],
) -> Result<(), DurabilityError> {
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET attempt_count = attempt_count + 1 \
         WHERE game_session_id = encode($1, 'hex')::uuid",
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
         WHERE game_session_id = encode($1, 'hex')::uuid \
           AND reconnect_attempt_ref = $2 AND state = $4",
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
         WHERE game_session_id = encode($1, 'hex')::uuid AND prepared_attempt_ref = $2",
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
                    PendingCommandDispositionV1::PendingOriginal => "pending_original",
                    PendingCommandDispositionV1::TerminalOutcomeRetained => "terminal_outcome_retained",
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
