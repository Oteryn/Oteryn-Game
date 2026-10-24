//! The shared frozen one-item retirement workflow for both admitted causes.
use super::*;

impl DurabilityRoot {
    /// Validate the step and reserve its logical `DECAY_RETIRE` under the
    /// current owner fence, or resume this generation's reservation with the
    /// same identities and budget. An item that already retired resumes its
    /// committed reservation regardless of the fence, so the original result
    /// stays reachable. A refusal writes nothing; this retires nothing.
    pub async fn freeze_decay_retire(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        step: DecayRetireStep,
    ) -> Result<DecayRetireCandidate> {
        check_uuid_v7(&step.corpse_item_instance_id)?;
        check_uuid_v7(&step.item_instance_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let root = self.clone();
        let node = node.clone();
        let reservation = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;
                    if let Some(receipt) = load_receipt(&mut tx, step.item_instance_id).await? {
                        let committed = decode_receipt(&receipt)?;
                        if committed.corpse_item_instance_id != step.corpse_item_instance_id
                            || !world_reset_cause::receipt_matches(&receipt, step.cause)?
                        {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        let generation: String =
                            receipt.try_get("fence_scope_ownership_generation")?;
                        let Some(row) =
                            load_reservation(&mut tx, step.item_instance_id, &generation).await?
                        else {
                            return Err(DurabilityError::InvalidStoredState);
                        };
                        let reservation = decode_reservation(&row)?;
                        tx.commit(deadline).await?;
                        return Ok(Ok(reservation));
                    }
                    let generation = fence.scope_ownership_generation.get().to_string();
                    if let Some(row) =
                        load_reservation(&mut tx, step.item_instance_id, &generation).await?
                    {
                        let reservation = decode_reservation(&row)?;
                        if reservation.corpse_item_instance_id != step.corpse_item_instance_id
                            || reservation.cause != step.cause
                        {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        if !fence_matches(&reservation, &node)
                            || !fence_is_live(&mut tx, &fence, &node).await?
                        {
                            return Ok(Err(DecayRetireError::AuthorityRejected));
                        }
                        if let Err(error) = admit(&root, &node, &mut tx, &fence, step).await? {
                            return Ok(Err(error));
                        }
                        tx.commit(deadline).await?;
                        return Ok(Ok(reservation));
                    }

                    if !fence_is_live(&mut tx, &fence, &node).await? {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let admitted = match admit(&root, &node, &mut tx, &fence, step).await? {
                        Ok(admitted) => admitted,
                        Err(error) => return Ok(Err(error)),
                    };
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id",
                    )
                    .fetch_one(&mut **tx)
                    .await?;
                    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
                    let event_id = uuid_text(row.try_get("event_id")?)?;
                    // The trusted timestamp is the admission's own database
                    // clock reading, already at or after the deadline.
                    let occurred_at_unix_ms = admitted.database_now_unix_ms;
                    let tuple = tx.tuple();
                    let envelope = match audit::encode_decay_retire_event(
                        DecayRetireEventIdentity {
                            event_id,
                            transaction_id,
                            occurred_at_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        decay_message(step, &fence, &admitted),
                        tuple.tuple(),
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let fact = node.fact();
                    let reservation = Reservation {
                        cause: step.cause,
                        corpse_item_instance_id: step.corpse_item_instance_id,
                        fence_generation: fence.scope_ownership_generation.get(),
                        transaction_id,
                        event_id,
                        occurred_at_unix_ms,
                        deadline_unix_ms: admitted.deadline_unix_ms,
                        quantity_before: admitted.quantity_before,
                        placement_ordinal: admitted.placement_ordinal,
                        envelope,
                        fence_node_id: *fact.node_id().as_bytes(),
                        fence_registration_revision: fact.registration_revision(),
                        work_units_used: 0,
                    };
                    insert_reservation(&mut tx, step, &admitted, &reservation, tuple).await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(reservation))
                })
            })
            .await??;
        Ok(DecayRetireCandidate {
            step,
            fence_generation: reservation.fence_generation,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            deadline_unix_ms: reservation.deadline_unix_ms,
            envelope: reservation.envelope,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the reservation in its own
    /// committed transaction before the pass does any work.
    async fn charge_decay_retire_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(DecayRetireError::CapacityExceeded);
        }
        let item = candidate.step.item_instance_id;
        let generation = candidate.fence_generation.to_string();
        let transaction_id = candidate.transaction_id;
        let charged = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let used = sqlx::query(
                        "UPDATE game_item_decay_retire_reservations \
                            SET work_units_used = work_units_used + 1 \
                          WHERE item_instance_id = encode($1,'hex')::uuid \
                            AND fence_scope_ownership_generation = $2::text::numeric(20,0) \
                            AND transaction_id = encode($3,'hex')::uuid \
                            AND work_units_used < $4 RETURNING work_units_used",
                    )
                    .bind(item.as_slice())
                    .bind(&generation)
                    .bind(transaction_id.as_slice())
                    .bind(i16::from(RL08_RETRY_WORK_UNITS_MAX))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(used) = used else {
                        let reserved = sqlx::query(
                            "SELECT 1 FROM game_item_decay_retire_reservations \
                              WHERE item_instance_id = encode($1,'hex')::uuid \
                                AND fence_scope_ownership_generation = $2::text::numeric(20,0) \
                                AND transaction_id = encode($3,'hex')::uuid",
                        )
                        .bind(item.as_slice())
                        .bind(&generation)
                        .bind(transaction_id.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                        return Ok(Err(if reserved.is_some() {
                            DecayRetireError::CapacityExceeded
                        } else {
                            DecayRetireError::ConflictingCandidate
                        }));
                    };
                    let used = u8::try_from(used.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(used))
                })
            })
            .await?;
        match charged {
            Ok(used) => {
                candidate.work_units_used = used;
                Ok(())
            }
            Err(DecayRetireError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(DecayRetireError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen step. An item that already retired returns its
    /// original terminal result without reacquiring owner authority. A new
    /// retirement commits only under the reservation's live fence and with
    /// the authoritative before-state still admitting it now.
    pub async fn commit_decay_retire(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<DecayRetireOutcome> {
        // SPELL-LOCK-2 §1.2: the DELETE fires the key-33 owner lock of the corpse's Channel, so
        // the lane is that Channel's and is held to the end of the transaction.
        permit.check_channel(fence.world_id, fence.channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.charge_decay_retire_work_unit(recovery.clone(), candidate)
            .await?;
        let step = candidate.step;
        let generation = candidate.fence_generation;
        let transaction_id = candidate.transaction_id;
        let event_id = candidate.event_id;
        let occurred_at_unix_ms = candidate.occurred_at_unix_ms;
        let envelope = candidate.envelope.clone();
        let root = self.clone();
        let node = node.clone();
        let mut context = permit;

        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let permit: &SpellLanePermit = permit;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;

                    if let Some(row) = load_receipt(&mut tx, step.item_instance_id).await? {
                        let committed = decode_receipt(&row)?;
                        if committed.corpse_item_instance_id != step.corpse_item_instance_id
                            || !world_reset_cause::receipt_matches(&row, step.cause)?
                        {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        tx.commit(deadline).await?;
                        return Ok(Ok(DecayRetireOutcome::AlreadyCommitted(committed)));
                    }

                    if generation != fence.scope_ownership_generation.get() {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let Some(row) =
                        load_reservation(&mut tx, step.item_instance_id, &generation.to_string())
                            .await?
                    else {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    };
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != transaction_id
                        || reservation.event_id != event_id
                        || reservation.occurred_at_unix_ms != occurred_at_unix_ms
                        || reservation.corpse_item_instance_id != step.corpse_item_instance_id
                        || reservation.cause != step.cause
                        || reservation.envelope != envelope
                    {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    if identity_reused(&mut tx, transaction_id, event_id).await? {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    if !fence_matches(&reservation, &node)
                        || !fence_is_live(&mut tx, &fence, &node).await?
                    {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let admitted = match admit(&root, &node, &mut tx, &fence, step).await? {
                        Ok(admitted) => admitted,
                        Err(error) => return Ok(Err(error)),
                    };
                    // The frozen event bytes describe exactly this before-state.
                    if admitted.quantity_before != reservation.quantity_before
                        || admitted.placement_ordinal != reservation.placement_ordinal
                        || admitted.deadline_unix_ms != reservation.deadline_unix_ms
                    {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    let tuple = match tx.frozen_tuple(&reservation.envelope) {
                        Ok(tuple) => tuple,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    // Before-state equality must not reinterpret the frozen event
                    // using the current process's build identity after a restart.
                    let (frozen_event, frozen_retirement) =
                        audit::decode_decay_retire_envelope(&reservation.envelope)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if frozen_retirement != decay_message(step, &fence, &admitted)
                        || frozen_event.event_id != event_id
                        || frozen_event.occurred_at_unix_ms != occurred_at_unix_ms
                        || frozen_event
                            .transaction_event
                            .as_ref()
                            .is_none_or(|reference| reference.transaction_id != transaction_id)
                    {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    let committed =
                        apply_retire(&mut tx, permit, step, &admitted, &reservation, tuple).await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(DecayRetireOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome. The cause lock waits for any in-flight
    /// attempt, so `None` proves nothing retired this item yet. Never
    /// reacquires owner authority.
    pub async fn reconcile_decay_retire(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<Option<CommittedDecayRetire>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.charge_decay_retire_work_unit(recovery.clone(), candidate)
            .await?;
        let step = candidate.step;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;
                    let Some(row) = load_receipt(&mut tx, step.item_instance_id).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let committed = decode_receipt(&row)?;
                    if committed.corpse_item_instance_id != step.corpse_item_instance_id
                        || !world_reset_cause::receipt_matches(&row, step.cause)?
                    {
                        return Ok(Err(DecayRetireError::ConflictingCause));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// D135 recovery query, run at scope (re)admission and after a corpse
    /// MINT commits: every corpse still live on Ground in this scope (live
    /// `game_item_ground_locations` joined to a `CORPSE_MATERIALIZATION`
    /// receipt, `lifecycle = 1`; never a `Container` row and never a retired
    /// corpse) with its durable deadline, oldest first. A partly drained
    /// corpse appears exactly like a fresh one. Reads only; grants nothing.
    pub async fn read_corpse_decay_schedule(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world_id: WorldId,
        channel_id: ChannelId,
    ) -> Result<CorpseDecaySchedule> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let database_now_unix_ms: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let rows = sqlx::query(
                        "SELECT g.item_instance_id::text, r.materialized_at \
                           FROM game_item_ground_locations g \
                           JOIN game_item_mint_receipts r ON r.item_instance_id = g.item_instance_id \
                           JOIN game_item_instances i ON i.item_instance_id = g.item_instance_id \
                          WHERE g.world_id = encode($1,'hex')::uuid \
                            AND g.channel_id = encode($2,'hex')::uuid \
                            AND r.loot_purpose_key = $3 AND r.draw_ordinal = 0 \
                            AND i.lifecycle = 1 \
                          ORDER BY r.materialized_at, g.item_instance_id \
                          LIMIT $4",
                    )
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
                    .bind(i64::try_from(COMBAT01_CORPSES_PER_SCOPE + 1).unwrap_or(i64::MAX))
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > COMBAT01_CORPSES_PER_SCOPE {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut corpses = Vec::with_capacity(rows.len());
                    for row in &rows {
                        let materialized_at_unix_ms: Option<i64> =
                            row.try_get("materialized_at")?;
                        let materialized_at_unix_ms =
                            materialized_at_unix_ms.ok_or(DurabilityError::InvalidStoredState)?;
                        corpses.push(CorpseDecayDeadline {
                            corpse_item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                            materialized_at_unix_ms,
                            decay_at_unix_ms: corpse_decay_at_unix_ms(materialized_at_unix_ms)
                                .ok_or(DurabilityError::InvalidStoredState)?,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CorpseDecaySchedule {
                        database_now_unix_ms,
                        corpses,
                    }))
                })
            })
            .await?
    }

    /// Drain one decayed corpse from durable state alone: one step per entry
    /// still live in its container (in ordinal order, each its own freeze and
    /// commit), then the corpse's own step. An entry that is no longer in the
    /// corpse by the time its step runs (picked up) is skipped, never retired.
    /// Resuming after a partial drain simply finds fewer live entries.
    pub async fn retire_decayed_corpse(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        corpse_item_instance_id: [u8; 16],
    ) -> Result<CorpseDecayReport> {
        check_uuid_v7(&corpse_item_instance_id)?;
        permit.check_channel(fence.world_id, fence.channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let live_entries = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(
                        "SELECT item_instance_id::text FROM game_item_corpse_container_entries \
                          WHERE parent_item_instance_id = encode($1,'hex')::uuid \
                          ORDER BY placement_ordinal LIMIT $2",
                    )
                    .bind(corpse_item_instance_id.as_slice())
                    .bind(i64::try_from(CORPSE_CONTAINER_ENTRIES_MAX + 1).unwrap_or(i64::MAX))
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > CORPSE_CONTAINER_ENTRIES_MAX {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let entries = rows
                        .iter()
                        .map(|row| uuid_text(row.try_get("item_instance_id")?))
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok::<_, DecayRetireError>(entries))
                })
            })
            .await??;
        let mut entries = Vec::with_capacity(live_entries.len());
        for item in live_entries {
            let step = DecayRetireStep::entry(corpse_item_instance_id, item);
            match self.retire_step(permit, authority, node, fence, step).await {
                Ok(committed) => entries.push(committed),
                Err(DecayRetireError::Refused(DecayRetireRefusal::NotInCorpse)) => {}
                Err(error) => return Err(error),
            }
        }
        let corpse = self
            .retire_step(
                permit,
                authority,
                node,
                fence,
                DecayRetireStep::corpse(corpse_item_instance_id),
            )
            .await?;
        Ok(CorpseDecayReport { entries, corpse })
    }

    async fn retire_step(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        step: DecayRetireStep,
    ) -> Result<CommittedDecayRetire> {
        let mut candidate = self
            .freeze_decay_retire(authority, node, fence, step)
            .await?;
        Ok(self
            .commit_decay_retire(permit, authority, node, fence, &mut candidate)
            .await?
            .into_committed())
    }
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    step: DecayRetireStep,
    admitted: &Admitted,
    reservation: &Reservation,
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_item_decay_retire_reservations(item_instance_id, \
           fence_scope_ownership_generation, corpse_item_instance_id, world_id, channel_id, \
           transaction_id, event_id, quantity_before, placement_ordinal, deadline, occurred_at, \
           envelope, fence_holder_node_id, fence_holder_registration_revision, \
           work_units_used, reserved_at, type2_schema_revision, cause_kind, reset_epoch, retirement_binding, target_digest, ground_root_item_instance_id) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), CASE WHEN $16='CORPSE_DECAY' THEN encode($3,'hex')::uuid END, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, \
           encode($7,'hex')::uuid, $8, $9::text::numeric(20,0), CASE WHEN $16='CORPSE_DECAY' THEN $10 ELSE NULL END, $11, $12, \
           encode($13,'hex')::uuid, $14::text::numeric(20,0), 0, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint, $15, $16, $17::text::numeric, $18, $19, CASE WHEN $16='WORLD_RESET' THEN encode($3,'hex')::uuid END)",
    )
    .bind(step.item_instance_id.as_slice())
    .bind(reservation.fence_generation.to_string())
    .bind(step.corpse_item_instance_id.as_slice())
    .bind(admitted.world_id.as_slice())
    .bind(admitted.channel_id.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(i64::from(reservation.quantity_before))
    .bind(
        reservation
            .placement_ordinal
            .map(|ordinal| ordinal.to_string()),
    )
    .bind(reservation.deadline_unix_ms)
    .bind(reservation.occurred_at_unix_ms)
    .bind(reservation.envelope.as_slice())
    .bind(reservation.fence_node_id.as_slice())
    .bind(reservation.fence_registration_revision.to_string())
    .bind(i16::try_from(tuple.schema_revision()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(reservation.cause.tag())
    .bind(reservation.cause.epoch())
    .bind(reservation.cause.binding())
    .bind(reservation.cause.target())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
