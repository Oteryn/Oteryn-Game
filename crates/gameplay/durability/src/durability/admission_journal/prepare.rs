//! Private ordered phases of reconnect PREPARE; all share one transaction.

use super::*;

struct PrepareBinding<'a> {
    record: &'a ReconnectDurabilityRecordV1,
    session_id: Vec<u8>,
    attempt_ref: Vec<u8>,
    transport_ref: Vec<u8>,
    epoch: String,
    predecessor: String,
    candidate: String,
    character_lease: String,
    scope_generation: String,
    original_grace_deadline: i64,
    prepared_deadline: i64,
    encoded_record: String,
    scope: ScopeStorage,
}

impl<'a> PrepareBinding<'a> {
    fn new(record: &'a ReconnectDurabilityRecordV1) -> Self {
        let identity = record.identity();
        let session_id = identity.game_session_id().as_bytes().to_vec();
        let attempt_ref = identity.reconnect_attempt_ref().to_be_bytes().to_vec();
        let transport_ref = record.connection().transport_ref().to_bytes().to_vec();
        let epoch = record.continuity().control_loss_epoch().get().to_string();
        let predecessor = record.connection().predecessor().get().to_string();
        let candidate = record.connection().candidate().get().to_string();
        let character_lease = record.authority().character_lease_generation().to_string();
        let scope_generation = record
            .authority()
            .scope_ownership_generation()
            .get()
            .to_string();
        let original_grace_deadline = record.continuity().original_grace_deadline();
        let prepared_deadline = record.continuity().prepared_deadline();
        let encoded_record = encode_record(record).to_string();
        let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) =
            scope_storage(record);
        Self {
            record,
            session_id,
            attempt_ref,
            transport_ref,
            epoch,
            predecessor,
            candidate,
            character_lease,
            scope_generation,
            original_grace_deadline,
            prepared_deadline,
            encoded_record,
            scope: (
                scope_kind,
                scope_world_id,
                scope_channel_id,
                scope_instance_id,
            ),
        }
    }
}

pub(super) async fn prepare_pass(
    holder: &mut PoolConnection<Postgres>,
    deadline: Option<Instant>,
    record: &ReconnectDurabilityRecordV1,
    receipt_authorized: bool,
) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
    let binding = PrepareBinding::new(record);
    let record = binding.record;
    let identity = binding.record.identity();
    let session_id = &binding.session_id;
    let attempt_ref = &binding.attempt_ref;
    let epoch = binding.epoch.as_str();
    let predecessor = binding.predecessor.as_str();
    let character_lease = binding.character_lease.as_str();
    let scope_generation = binding.scope_generation.as_str();
    let original_grace_deadline = binding.original_grace_deadline;
    let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) = &binding.scope;
    let mut transaction = begin_pass_transaction(holder, deadline).await?;
    db::lock_admission_domain(&mut transaction, record).await?;
    if super::super::fresh_admission::has_owning_loss_receipt(
        &mut transaction,
        session_id,
        record.continuity().control_loss_epoch().get(),
    )
    .await?
    {
        return Ok(ReconnectPrepareDispositionV1::Unavailable);
    }
    let existing_session: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid)")
               .bind(session_id.as_slice()).fetch_one(&mut *transaction).await?;
    if !existing_session {
        let account_incumbent: bool = sqlx::query_scalar(
                   "SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE account_id = $1::text::uuid AND session_state IN (1, 2) AND game_session_id <> encode($2, 'hex')::uuid AND character_id <> encode($3, 'hex')::uuid)",
               )
               .bind(identity.account_id())
               .bind(session_id.as_slice())
               .bind(identity.character_id().as_bytes().as_slice())
               .fetch_one(&mut *transaction)
               .await?;
        if account_incumbent {
            return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
        }
        let revision = super::super::fresh_admission::unused_session_revision(
            &mut transaction,
            identity.character_id(),
            identity.game_session_id(),
        )
        .await?;
        super::super::fresh_admission::commit_session_use(
            &mut transaction,
            identity.character_id(),
            identity.game_session_id(),
            record.connection().transport_ref().to_bytes(),
            revision,
        )
        .await?;
    }
    let inserted_session = sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions (\
                game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                control_loss_epoch, original_grace_deadline, predecessor_generation, \
                character_lease_generation, scope_ownership_generation, current_generation\
             ) VALUES (\
                encode($1, 'hex')::uuid, $2::text::uuid, encode($3, 'hex')::uuid, \
                encode($4, 'hex')::uuid, $5, encode($6, 'hex')::uuid, \
                encode($7, 'hex')::uuid, encode($8, 'hex')::uuid, \
                $9::text::numeric(20, 0), $10, $11::text::numeric(20, 0), \
                $12::text::numeric(20, 0), $13::text::numeric(20, 0), \
                $11::text::numeric(20, 0)\
             ) ON CONFLICT DO NOTHING",
    )
    .bind(session_id.as_slice())
    .bind(identity.account_id())
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(identity.world_id().as_bytes().as_slice())
    .bind(scope_kind)
    .bind(scope_world_id.as_slice())
    .bind(scope_channel_id.as_deref())
    .bind(scope_instance_id.as_deref())
    .bind(epoch)
    .bind(original_grace_deadline)
    .bind(predecessor)
    .bind(character_lease)
    .bind(scope_generation)
    .execute(&mut *transaction)
    .await?;
    if inserted_session.rows_affected() == 1 {
        ensure_precommit_protection_continuity(&mut transaction, record).await?;
    }

    let Some(mut session) =
        load_session_for_update(&mut transaction, session_id.as_slice()).await?
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    // Fresh ACTIVE sessions have no accepted loss epoch. A legacy
    // PREPARE must not invent one from its proposed record.
    if session
        .try_get::<Option<String>, _>("control_loss_epoch")?
        .is_none()
    {
        return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
    }
    if !session_binding_is_valid(&session, record)? {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipt_backed =
        !receipt_authorized && replacement_receipt_matches_record(&mut transaction, record).await?;
    let existing = sqlx::query(
        "SELECT state, record_json FROM game_durability_reconnect_attempts \
             WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
    )
    .bind(session_id.as_slice())
    .bind(attempt_ref.as_slice())
    .fetch_optional(&mut *transaction)
    .await?;
    if let Some(existing) = existing {
        return resolve_existing_attempt(
            transaction,
            deadline,
            &binding,
            &session,
            existing,
            receipt_backed,
        )
        .await;
    }

    if let Some(retained_for_actor_epoch) =
        lock_actor_epoch_attempt_budget(&mut transaction, record).await?
        && retained_for_actor_epoch >= MAX_ATTEMPTS_PER_EPOCH
    {
        return Ok(ReconnectPrepareDispositionV1::AttemptCapacityExceeded);
    }
    match open_prepare_epoch(&mut transaction, &binding, &mut session).await? {
        PrepareEpochOutcome::Continue => {}
        PrepareEpochOutcome::CapacityExceeded => {
            return Ok(ReconnectPrepareDispositionV1::AttemptCapacityExceeded);
        }
        PrepareEpochOutcome::StaleAttemptRecorded => {
            commit_pass_transaction(transaction, deadline).await?;
            return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
        }
    }
    reserve_prepared_attempt(transaction, deadline, &binding, &session).await
}

async fn resolve_existing_attempt(
    mut transaction: Transaction<'_, Postgres>,
    deadline: Option<Instant>,
    binding: &PrepareBinding<'_>,
    session: &PgRow,
    existing: PgRow,
    receipt_backed: bool,
) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
    let record = binding.record;
    let session_id = &binding.session_id;
    let attempt_ref = &binding.attempt_ref;
    let transport_ref = &binding.transport_ref;
    let epoch = binding.epoch.as_str();
    let predecessor = binding.predecessor.as_str();
    let candidate = binding.candidate.as_str();
    let character_lease = binding.character_lease.as_str();
    let scope_generation = binding.scope_generation.as_str();
    let original_grace_deadline = binding.original_grace_deadline;
    let prepared_deadline = binding.prepared_deadline;
    let encoded_record = binding.encoded_record.as_str();
    if receipt_backed {
        return Err(DurabilityError::InvalidStoredState);
    }
    let stored_record: String = existing.try_get("record_json")?;
    if stored_record != encoded_record {
        return Ok(ReconnectPrepareDispositionV1::IdempotencyConflict);
    }
    if !attempt_binding_is_valid(&mut transaction, record).await? {
        return Err(DurabilityError::InvalidStoredState);
    }
    let state: i16 = existing.try_get("state")?;
    if state == COMMITTED
        && !committed_protection_binding_is_valid(&mut transaction, record).await?
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if state == PREPARED
        && !precommit_protection_binding_is_valid(&mut transaction, record, false).await?
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if state == PREPARED && database_now(&mut transaction).await? > prepared_deadline {
        if session
            .try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
            .as_deref()
            != Some(attempt_ref.as_slice())
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        terminalize_prepared_attempt(
            &mut transaction,
            session_id.as_slice(),
            attempt_ref.as_slice(),
        )
        .await?;
        commit_pass_transaction(transaction, deadline).await?;
        return Ok(ReconnectPrepareDispositionV1::ExistingTerminal);
    }
    if state == COMMITTED {
        let current_ref: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
        let recovery_grant_nonce = recovery_grant_nonce(record);
        let committed_current = session.try_get::<String, _>("control_loss_epoch")? == epoch
            && session.try_get::<i64, _>("original_grace_deadline")? == original_grace_deadline
            && session.try_get::<String, _>("predecessor_generation")? == predecessor
            && session.try_get::<String, _>("character_lease_generation")? == character_lease
            && session.try_get::<String, _>("scope_ownership_generation")? == scope_generation
            && session.try_get::<String, _>("current_generation")? == candidate
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
            && active_committed_binding_is_valid(&mut transaction, session_id.as_slice(), session)
                .await?;
        if !committed_current {
            return Err(DurabilityError::InvalidStoredState);
        }
        commit_pass_transaction(transaction, deadline).await?;
        return Ok(ReconnectPrepareDispositionV1::Ambiguous);
    }
    disposition_for_existing(state)
}

enum PrepareEpochOutcome {
    Continue,
    CapacityExceeded,
    StaleAttemptRecorded,
}

async fn open_prepare_epoch(
    transaction: &mut Transaction<'_, Postgres>,
    binding: &PrepareBinding<'_>,
    session: &mut PgRow,
) -> Result<PrepareEpochOutcome, DurabilityError> {
    let record = binding.record;
    let session_id = &binding.session_id;
    let epoch = binding.epoch.as_str();
    let predecessor = binding.predecessor.as_str();
    let character_lease = binding.character_lease.as_str();
    let scope_generation = binding.scope_generation.as_str();
    let original_grace_deadline = binding.original_grace_deadline;
    let prepared_deadline = binding.prepared_deadline;
    let encoded_record = binding.encoded_record.as_str();
    let current_epoch: String = session.try_get("control_loss_epoch")?;
    if current_epoch != epoch {
        let previously_used: bool = sqlx::query_scalar(
            "SELECT EXISTS (\
                    SELECT 1 FROM game_durability_reconnect_attempts \
                    WHERE game_session_id = encode($1, 'hex')::uuid \
                      AND control_loss_epoch = $2::text::numeric(20, 0)\
                 )",
        )
        .bind(session_id.as_slice())
        .bind(epoch)
        .fetch_one(&mut **transaction)
        .await?;
        if previously_used {
            let retained_for_epoch: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND control_loss_epoch = $2::text::numeric(20, 0)",
            )
            .bind(session_id.as_slice())
            .bind(epoch)
            .fetch_one(&mut **transaction)
            .await?;
            if retained_for_epoch >= i64::from(MAX_ATTEMPTS_PER_EPOCH) {
                return Ok(PrepareEpochOutcome::CapacityExceeded);
            }
            insert_attempt(transaction, record, encoded_record, STALE_TERMINAL).await?;
            return Ok(PrepareEpochOutcome::StaleAttemptRecorded);
        }

        let active_shape_matches = session.try_get::<String, _>("current_generation")?
            == predecessor
            && session.try_get::<String, _>("character_lease_generation")? == character_lease
            && session.try_get::<String, _>("scope_ownership_generation")? == scope_generation
            && session
                .try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
                .is_some()
            && session.try_get::<i16, _>("session_state")? == ACTIVE
            && session
                .try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
                .is_none();
        let active_binding_valid = if active_shape_matches {
            active_committed_binding_is_valid(transaction, session_id.as_slice(), session).await?
        } else {
            false
        };
        if active_shape_matches && !active_binding_valid {
            return Err(DurabilityError::InvalidStoredState);
        }
        let can_open_new_epoch = active_shape_matches && active_binding_valid;
        if !can_open_new_epoch || database_now(transaction).await? > prepared_deadline {
            insert_attempt(transaction, record, encoded_record, STALE_TERMINAL).await?;
            return Ok(PrepareEpochOutcome::StaleAttemptRecorded);
        }

        ensure_precommit_protection_continuity(transaction, record).await?;

        let opened = sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
                 SET control_loss_epoch = $2::text::numeric(20, 0), \
                     original_grace_deadline = $3, \
                     predecessor_generation = $4::text::numeric(20, 0), \
                     current_generation = $4::text::numeric(20, 0), \
                     current_transport_ref = NULL, session_state = $5, \
                     attempt_count = 0, prepared_attempt_ref = NULL \
                 WHERE game_session_id = encode($1, 'hex')::uuid \
                   AND current_generation = $4::text::numeric(20, 0) \
                   AND session_state = $6 AND current_transport_ref IS NOT NULL \
                   AND prepared_attempt_ref IS NULL",
        )
        .bind(session_id.as_slice())
        .bind(epoch)
        .bind(original_grace_deadline)
        .bind(predecessor)
        .bind(RECONNECTABLE)
        .bind(ACTIVE)
        .execute(&mut **transaction)
        .await?;
        if opened.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let Some(refreshed_session) =
            load_session_for_update(transaction, session_id.as_slice()).await?
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        *session = refreshed_session;
        if !session_binding_is_valid(session, record)? {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(PrepareEpochOutcome::Continue)
}

async fn reserve_prepared_attempt(
    mut transaction: Transaction<'_, Postgres>,
    deadline: Option<Instant>,
    binding: &PrepareBinding<'_>,
    session: &PgRow,
) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
    let record = binding.record;
    let session_id = &binding.session_id;
    let attempt_ref = &binding.attempt_ref;
    let transport_ref = &binding.transport_ref;
    let epoch = binding.epoch.as_str();
    let predecessor = binding.predecessor.as_str();
    let character_lease = binding.character_lease.as_str();
    let scope_generation = binding.scope_generation.as_str();
    let original_grace_deadline = binding.original_grace_deadline;
    let prepared_deadline = binding.prepared_deadline;
    let encoded_record = binding.encoded_record.as_str();
    let count: i16 = session.try_get("attempt_count")?;
    if count >= MAX_ATTEMPTS_PER_EPOCH {
        return Ok(ReconnectPrepareDispositionV1::AttemptCapacityExceeded);
    }

    let is_current = session.try_get::<String, _>("control_loss_epoch")? == epoch
        && session.try_get::<i64, _>("original_grace_deadline")? == original_grace_deadline
        && session.try_get::<String, _>("predecessor_generation")? == predecessor
        && session.try_get::<String, _>("character_lease_generation")? == character_lease
        && session.try_get::<String, _>("scope_ownership_generation")? == scope_generation
        && session.try_get::<String, _>("current_generation")? == predecessor
        && session
            .try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
            .is_none()
        && session.try_get::<i16, _>("session_state")? == RECONNECTABLE;
    if !is_current || database_now(&mut transaction).await? > prepared_deadline {
        insert_attempt(&mut transaction, record, encoded_record, STALE_TERMINAL).await?;
        increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
        commit_pass_transaction(transaction, deadline).await?;
        return Ok(ReconnectPrepareDispositionV1::RejectedStaleAuthority);
    }
    ensure_precommit_protection_continuity(&mut transaction, record).await?;

    let incumbent: Option<Vec<u8>> = session.try_get("prepared_attempt_ref")?;
    if incumbent.is_some() {
        insert_attempt(
            &mut transaction,
            record,
            encoded_record,
            CONCURRENT_TERMINAL,
        )
        .await?;
        increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
        commit_pass_transaction(transaction, deadline).await?;
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
        insert_attempt(&mut transaction, record, encoded_record, COLLISION_TERMINAL).await?;
        increment_attempt_count(&mut transaction, session_id.as_slice()).await?;
        commit_pass_transaction(transaction, deadline).await?;
        return Ok(ReconnectPrepareDispositionV1::RejectedTransportRefCollision);
    }

    insert_attempt(&mut transaction, record, encoded_record, PREPARED).await?;
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
             SET attempt_count = attempt_count + 1, prepared_attempt_ref = $2 \
             WHERE game_session_id = encode($1, 'hex')::uuid",
    )
    .bind(session_id.as_slice())
    .bind(attempt_ref.as_slice())
    .execute(&mut *transaction)
    .await?;
    commit_pass_transaction(transaction, deadline).await?;
    Ok(ReconnectPrepareDispositionV1::Prepared)
}
