//! Strict historical fresh-operation storage. No decoded value is a live capability.
//! Runtime ceilings follow the accepted DFR registry; codec bounds remain explicit.
use super::DurabilityError;
use super::admission_authority_guards::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use oteryn_game_server::foundation::admission_authority_publication::*;
use oteryn_game_server::foundation::fnd04_verifier::*;
use oteryn_game_server::foundation::fresh_admission_durability::*;
use oteryn_game_server::foundation::*;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, DurabilityError>;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    version: u8,
    payload: &'a str,
}

fn write_current(w: &mut Writer, f: &FreshCurrentEvidence) -> Result<()> {
    w.text(&f.account_id)?;
    w.bytes(f.character_id.as_bytes())?;
    w.bytes(f.world_id.as_bytes())?;
    w.bytes(f.channel_id.as_bytes())?;
    w.u64(f.character_lease_generation)?;
    w.text(&f.route_revision)?;
    w.text(&f.runtime_observation_revision)?;
    w.u64(f.scope_ownership_generation)?;
    for value in [
        &f.ruleset_revision,
        &f.content_revision,
        &f.map_revision,
        &f.world_policy_revision,
        &f.offer_revision,
    ] {
        w.text(value)?;
    }
    Ok(())
}
fn read_current(r: &mut Reader<'_>) -> Result<FreshCurrentEvidence> {
    Ok(FreshCurrentEvidence {
        account_id: r.text()?,
        character_id: checked(CharacterId::decode(&r.bytes::<16>()?))?,
        world_id: checked(WorldId::decode(&r.bytes::<16>()?))?,
        channel_id: checked(ChannelId::decode(&r.bytes::<16>()?))?,
        character_lease_generation: r.u64()?,
        route_revision: r.text()?,
        runtime_observation_revision: r.text()?,
        scope_ownership_generation: r.u64()?,
        ruleset_revision: r.text()?,
        content_revision: r.text()?,
        map_revision: r.text()?,
        world_policy_revision: r.text()?,
        offer_revision: r.text()?,
    })
}
fn write_operation(w: &mut Writer, operation: &FreshAdmissionOperationV1) -> Result<()> {
    let b = &operation.authorization;
    let initial = checked(b.initial_commit())?;
    w.tag(b.version)?;
    w.text(&b.account_id)?;
    w.bytes(&b.facts.replay_key().to_bytes())?;
    w.bytes(initial.character_id().as_bytes())?;
    w.bytes(initial.world_id().as_bytes())?;
    w.bytes(initial.channel_id().as_bytes())?;
    w.u64(initial.character_lease_generation())?;
    w.u64(initial.scope_ownership_generation())?;
    w.bytes(b.candidate_session.as_bytes())?;
    w.bytes(&b.transport.to_bytes())?;
    w.u64(b.connection_generation)?;
    write_current(w, &b.current_facts)?;
    w.u64(b.protocol_major)?;
    w.u64(b.transport_profile)?;
    w.u64(b.signed_security_generation)?;
    w.text(&b.signing.key_id)?;
    w.bytes(&b.signing.public_key)?;
    w.boolean(b.signing.trusted)?;
    write_provenance(w, &b.signing.provenance)?;
    write_security(w, &b.security)?;
    w.i64(b.credential_times.0)?;
    w.i64(b.credential_times.1)?;
    w.i64(b.credential_times.2)?;
    w.i64(b.verified_at)?;
    w.i64(b.accepted_deadline)?;
    write_changes(w, &b.expected_guards)?;
    write_changes(w, &operation.transition.predecessors)?;
    write_changes(w, &operation.transition.successors)?;
    w.i64(operation.transition.prepared_at)
}
fn read_operation(r: &mut Reader<'_>) -> Result<FreshAdmissionOperationV1> {
    let version = r.tag()?;
    let account_id = r.text()?;
    let replay = checked(FreshAdmissionReplayKey::decode(&r.bytes::<33>()?))?.to_bytes();
    let character = checked(CharacterId::decode(&r.bytes::<16>()?))?;
    let world = checked(WorldId::decode(&r.bytes::<16>()?))?;
    let channel = checked(ChannelId::decode(&r.bytes::<16>()?))?;
    let facts = checked(FreshAdmissionFacts::new(
        checked(replay[1..].try_into())?,
        character,
        world,
        channel,
        r.u64()?,
        r.u64()?,
    ))?;
    let candidate_session = checked(GameSessionId::decode(&r.bytes::<16>()?))?;
    let transport = checked(AuthenticatedTransportRefV1::decode(&r.bytes::<16>()?))?;
    let connection_generation = r.u64()?;
    let current_facts = read_current(r)?;
    let protocol_major = r.u64()?;
    let transport_profile = r.u64()?;
    let signed_security_generation = r.u64()?;
    let signing = FreshSigningTrustObservationV1 {
        key_id: r.text()?,
        public_key: r.bytes()?,
        trusted: r.boolean()?,
        provenance: read_provenance(r)?,
    };
    let security = read_security(r)?;
    let credential_times = (r.i64()?, r.i64()?, r.i64()?);
    let verified_at = r.i64()?;
    let accepted_deadline = r.i64()?;
    let expected_guards = read_changes(r)?;
    let authorization = FreshAdmissionAuditBindingV1 {
        version,
        account_id,
        facts,
        candidate_session,
        transport,
        connection_generation,
        current_facts,
        protocol_major,
        transport_profile,
        signed_security_generation,
        signing,
        security,
        credential_times,
        verified_at,
        accepted_deadline,
        expected_guards,
    };
    let transition = AdmissionClaimTransitionEvidenceV1 {
        predecessors: read_changes(r)?,
        successors: read_changes(r)?,
        prepared_at: r.i64()?,
    };
    Ok(FreshAdmissionOperationV1 {
        authorization,
        transition,
    })
}

/// The full immutable operation, including independently authored claim effects.
/// Each allocation is checked against the caller's finite budget before copying.
pub fn encode_operation(
    operation: &FreshAdmissionOperationV1,
    maximum_bytes: usize,
) -> Result<String> {
    encoded_operation_size(operation, maximum_bytes)?;
    let mut writer = Writer::new(maximum_bytes);
    write_operation(&mut writer, operation)?;
    // The historical predicate clones guard evidence internally: first establish
    // that the complete retained operation fits the explicit allocation budget.
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    encode_envelope(&writer.bytes, maximum_bytes)
}
/// Nonallocating checked complete operation wire-size preflight. Private request
/// copies and executor resident charges remain a separate accounting obligation.
pub fn encoded_operation_size(
    operation: &FreshAdmissionOperationV1,
    maximum_bytes: usize,
) -> Result<usize> {
    let mut counter = Writer::counter(maximum_bytes);
    write_operation(&mut counter, operation)?;
    envelope_size(counter.measured(), maximum_bytes)
}
/// Owner revalidation run inside a fresh-admission commit transaction.
pub(super) type OwnerRevalidation = Box<
    dyn for<'t, 'c> FnOnce(
            &'t mut sqlx::Transaction<'c, sqlx::Postgres>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<bool>> + Send + 't>,
        > + Send,
>;

pub(super) fn envelope_size(length: usize, maximum_bytes: usize) -> Result<usize> {
    let groups = length
        .checked_div(3)
        .and_then(|groups| groups.checked_mul(4));
    let tail = match length % 3 {
        0 => 0,
        1 => 2,
        _ => 3,
    };
    let required = groups
        .and_then(|size| size.checked_add(tail))
        .and_then(|size| size.checked_add("{\"version\":1,\"payload\":\"\"}".len()))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if required > maximum_bytes {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(required)
}
pub(super) fn encode_envelope(bytes: &[u8], maximum_bytes: usize) -> Result<String> {
    envelope_size(bytes.len(), maximum_bytes)?;
    let payload = URL_SAFE_NO_PAD.encode(bytes);
    checked(serde_json::to_string(&Envelope {
        version: 1,
        payload: &payload,
    }))
}
pub(super) fn decode_envelope(encoded: &str, maximum_bytes: usize) -> Result<Vec<u8>> {
    if encoded.len() > maximum_bytes {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Borrowed payload parsing performs no peer-sized string copy. Unknown,
    // duplicate, escaped/noncanonical members and unsupported versions reject.
    let envelope: Envelope<'_> = checked(serde_json::from_str(encoded))?;
    if envelope.version != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    const PREFIX: &str = "{\"version\":1,\"payload\":\"";
    const SUFFIX: &str = "\"}";
    if !encoded.starts_with(PREFIX)
        || !encoded.ends_with(SUFFIX)
        || encoded.len() != PREFIX.len() + envelope.payload.len() + SUFFIX.len()
        || &encoded[PREFIX.len()..encoded.len() - SUFFIX.len()] != envelope.payload
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The configured engine rejects padding and nonzero unused trailing bits.
    let bytes = checked(URL_SAFE_NO_PAD.decode(envelope.payload))?;
    Ok(bytes)
}
/// Restore historical data only. Receipt restoration additionally validates the
/// original durable decided_at; neither operation creates a current source.
pub fn decode_operation(encoded: &str, maximum_bytes: usize) -> Result<FreshAdmissionOperationV1> {
    let bytes = decode_envelope(encoded, maximum_bytes)?;
    let mut reader = Reader::new(&bytes);
    let operation = read_operation(&mut reader)?;
    reader.finish()?;
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    Ok(operation)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshReconciliation {
    Absent,
    Conflict,
    Committed(Box<FreshAdmissionDurableReconciliationSnapshotV1>),
}

/// Inert historical outcome and independently read current snapshot. This is not
/// a registered completion source or permission to activate a controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshLossReconciliation {
    Absent,
    Conflict,
    Committed {
        completion: Box<ControlLossCompletionV1>,
        current: Box<FreshAdmissionDurableReconciliationSnapshotV1>,
    },
}

/// A single-use delivery source constructed only by a validated durable read.
/// It reports persistence to the matching owning flow, never live authorization,
/// acknowledgement of consumption, or permission to release an executor slot.
/// ```compile_fail
/// use oteryn_game_server::durability::fresh_admission::DurableLossCompletionSource;
/// use oteryn_game_server::foundation::ControlLossCompletionV1;
/// fn promote_history(history: ControlLossCompletionV1) -> DurableLossCompletionSource {
///     history.into()
/// }
/// ```
#[derive(Debug)]
pub struct DurableLossCompletionSource {
    completion: Option<Box<ControlLossCompletionV1>>,
    current: Box<FreshAdmissionDurableReconciliationSnapshotV1>,
}
impl DurableLossCompletionSource {
    #[must_use]
    pub fn current_snapshot(&self) -> &FreshAdmissionDurableReconciliationSnapshotV1 {
        &self.current
    }
}
impl oteryn_game_server::foundation::fnd04_verifier::recovery_source_sealed::Sealed
    for DurableLossCompletionSource
{
}
impl ControlLossCompletionSourceV1 for DurableLossCompletionSource {
    fn take_loss_completion(
        &mut self,
        original: &ControlLossOperationV1,
    ) -> std::result::Result<Option<ControlLossCompletionV1>, ReconnectDurabilityErrorV1> {
        if self
            .completion
            .as_ref()
            .is_some_and(|completion| &completion.operation != original)
        {
            // A wrong recipient does not consume or replace the real original.
            return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
        }
        Ok(self.completion.take().map(|completion| *completion))
    }
}

/// Asynchronous storage only. Production bounded scheduling/completion remains
/// a separate adapter requirement; runtime arguments must match fixed registry caps.
#[derive(Clone)]
pub struct FreshAdmissionStore {
    guards: AdmissionGuardStore,
    maximum_operation_bytes: usize,
}
impl FreshAdmissionStore {
    #[cfg(test)]
    pub async fn connect_runtime(
        url: &str,
        maximum_operation_bytes: usize,
        maximum_guard_bytes: usize,
    ) -> Result<Self> {
        if maximum_operation_bytes != super::MAX_FRESH_OPERATION_BYTES
            || maximum_guard_bytes != super::MAX_ADMISSION_GUARD_BYTES
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let root = super::DurabilityRoot::connect_test_runtime(url)?;
        if !root.maintain_ready_once().await? {
            return Err(DurabilityError::RootUnavailable);
        }
        Ok(Self::from_root(root))
    }

    #[must_use]
    pub fn from_root(root: super::DurabilityRoot) -> Self {
        Self {
            guards: AdmissionGuardStore::from_root(root),
            maximum_operation_bytes: super::MAX_FRESH_OPERATION_BYTES,
        }
    }

    async fn receipt_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        replay: &[u8],
    ) -> Result<Option<FreshAdmissionCommitReceiptV1>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT CASE WHEN octet_length(operation_json) <= $2 AND octet_length(to_jsonb(r)::text) <= $3 THEN operation_json END AS payload, CASE WHEN octet_length(to_jsonb(r)::text) <= $3 THEN to_jsonb(r) - 'operation_json' END AS mirrors FROM game_durability_fresh_admission_receipts r WHERE replay_key = $1")
            .bind(replay).bind(checked(i64::try_from(self.maximum_operation_bytes))?).bind(super::MAX_ADMISSION_ROW_BYTES).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let payload: Option<String> = row.try_get("payload")?;
        let operation = decode_operation(
            &payload.ok_or(DurabilityError::InvalidStoredState)?,
            self.maximum_operation_bytes,
        )?;
        let mirrors: serde_json::Value = row.try_get("mirrors")?;
        let decided_at = mirrors
            .get("authorization_decided_at")
            .and_then(serde_json::Value::as_i64)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let b = &operation.authorization;
        let initial = checked(b.initial_commit())?;
        let expected = serde_json::json!({
            "replay_key": bytea_text(&b.facts.replay_key().to_bytes()),
            "game_session_id": uuid_text(b.candidate_session.as_bytes()),
            "account_id": b.account_id,
            "character_id": uuid_text(initial.character_id().as_bytes()),
            "world_id": uuid_text(initial.world_id().as_bytes()),
            "channel_id": uuid_text(initial.channel_id().as_bytes()),
            "character_lease_generation": initial.character_lease_generation(),
            "scope_ownership_generation": initial.scope_ownership_generation(),
            "connection_generation": 1,
            "transport_ref": bytea_text(&b.transport.to_bytes()),
            "semantic_version": 1,
            "authorization_decided_at": decided_at,
        });
        if mirrors != expected || b.facts.replay_key().to_bytes().as_slice() != replay {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some(checked(FreshAdmissionCommitReceiptV1::restore(
            operation, decided_at,
        ))?))
    }

    pub async fn commit(
        &self,
        request: &FreshAdmissionCommitRequestV1,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        self.commit_revalidated(request, None).await
    }

    /// Commit with an optional owner revalidation that runs inside the commit
    /// transaction, after relation locks and exact-replay reconciliation. A
    /// `false` result rejects the attempt as stale authority.
    pub(super) async fn commit_revalidated(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        revalidate: Option<OwnerRevalidation>,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        let store = (*self).clone();
        let request = (*request).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        let operation = request.operation();
        let b = &operation.authorization;
        let encoded = encode_operation(operation, store.maximum_operation_bytes)?;
        let encoded_successors: Vec<_> = operation
            .transition
            .successors
            .iter()
            .map(|change| encode_guard(change, store.guards.maximum_guard_bytes))
            .collect::<Result<_>>()?;
        let replay = b.facts.replay_key().to_bytes();
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(receipt) = store.receipt_locked(&mut tx, &replay).await? {
            let outcome = receipt.classify_retry(operation);
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(outcome);
        }
        if let Some(revalidate) = revalidate
            && !revalidate(&mut tx).await?
        {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        }
        let initial = checked(b.initial_commit())?;
        let use_state = session_use_state(&mut tx, initial.character_id()).await?;
        if !use_state.complete { return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority); }
        let used: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid)")
            .bind(b.candidate_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if used { return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(FreshAdmissionCollisionV1::CandidateSession)); }
        if use_state.revision == GAME_SESSION_USE_LEDGER_CAPACITY_V1 { return Err(DurabilityError::AdmissionGameSessionLedgerExhausted); }
        let candidate_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1, 'hex')::uuid)").bind(b.candidate_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if candidate_exists {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(
                FreshAdmissionCollisionV1::CandidateSession,
            ));
        }
        let transport_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1)").bind(b.transport.to_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if transport_exists {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(
                FreshAdmissionCollisionV1::TransportReference,
            ));
        }
        let incumbent: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE session_state IN (1,2) AND (account_id = $1::text::uuid OR character_id = encode($2, 'hex')::uuid))").bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if incumbent {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedIncumbent);
        }
        let mut current = Vec::with_capacity(b.expected_guards.len());
        for expected in &b.expected_guards {
            current.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        let previous: Vec<_> = operation
            .transition
            .successors
            .iter()
            .map(|change| {
                current
                    .iter()
                    .flatten()
                    .find(|row| row.key == change.key)
                    .cloned()
            })
            .collect();
        if !store
            .guards
            .successor_history_available(&mut tx, &operation.transition.successors, &previous)
            .await?
        {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        }
        // All relation protection and conflict/history observations precede L.
        // No SQL-generated source revision, decision or source timestamp exists.
        let decided_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let Ok(successors) = request.validate_at_decision(&current, Some(decided_at)) else {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        };
        commit_session_use(&mut tx, initial.character_id(), b.candidate_session, b.transport.to_bytes(), use_state.revision).await?;
        sqlx::query("INSERT INTO game_durability_fresh_admission_receipts (replay_key, game_session_id, account_id, character_id, world_id, channel_id, character_lease_generation, scope_ownership_generation, connection_generation, transport_ref, semantic_version, operation_json, authorization_decided_at) VALUES ($1, encode($2,'hex')::uuid, $3::text::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, $7::text::numeric(20,0), $8::text::numeric(20,0), 1, $9, 1, $10, $11)")
            .bind(replay.as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_durability_reconnect_sessions (game_session_id, account_id, character_id, world_id, runtime_scope_kind, runtime_scope_world_id, runtime_scope_channel_id, character_lease_generation, scope_ownership_generation, current_generation, current_transport_ref, session_state, fresh_replay_key) VALUES (encode($1,'hex')::uuid, $2::text::uuid, encode($3,'hex')::uuid, encode($4,'hex')::uuid, 1, encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6::text::numeric(20,0), $7::text::numeric(20,0), 1, $8, 2, $9)")
            .bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(replay.as_slice()).execute(&mut *tx).await?;
        store.guards
            .persist_locked(&mut tx, successors, &previous, &encoded_successors)
            .await?;
        sqlx::query("INSERT INTO game_durability_transport_ref_reservations (transport_ref, game_session_id, reconnect_attempt_ref, reservation_owner, fresh_replay_key) VALUES ($1, encode($2,'hex')::uuid, NULL, 2, $3)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).execute(&mut *tx).await?;
        let receipt = checked(FreshAdmissionCommitReceiptV1::restore(
            operation.clone(),
            decided_at,
        ))?;
        // An acknowledgement error is uncertain; caller reconciles this original
        // operation rather than assuming rollback or manufacturing another key.
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable);
        }
        Ok(FreshAdmissionDurableOutcomeV1::Committed(receipt))

            }))
            .await
    }

    /// Record an owning loss: a fresh-origin loss, or a loss after a same-session resume
    /// (the next epoch, retaining the resumed one). Unsupported continuity shapes remain
    /// unavailable; this API never derives loss authority from reconnect PREPARE.
    pub async fn commit_fresh_loss(
        &self,
        request: std::sync::Arc<ControlLossRequestV1>,
        source: std::sync::Arc<dyn ControlLossSourceV1 + Send + Sync>,
    ) -> Result<ControlLossOutcomeV1> {
        let store = (*self).clone();

        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let operation = request.operation();
        let encoded = encode_fresh_loss(operation)?;
        let observation = &operation.observation;
        let session_id = observation.session.commit().game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&observation.loss_epoch.get().to_be_bytes());
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? {
            let stored: Option<String> = row.try_get("operation_json")?;
            if stored.as_deref() != Some(encoded.as_str()) { return Err(DurabilityError::InvalidStoredState); }
            let decided_at: i64 = row.try_get("decided_at")?;
            if decided_at < operation.authorized_at { return Err(DurabilityError::InvalidStoredState); }
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(ControlLossOutcomeV1::Committed { decided_at });
        }
        let row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json FROM game_durability_fresh_admission_receipts r WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        let original_json: Option<String> = row.try_get("operation_json")?;
        let original = decode_operation(
            &original_json.ok_or(DurabilityError::InvalidStoredState)?,
            store.maximum_operation_bytes,
        )?;
        let FreshReconciliation::Committed(current) =
            store.reconcile_locked(&mut tx, &original).await?
        else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        let expected_claims = &original.transition.successors;
        let mut claims = Vec::with_capacity(2);
        for expected in expected_claims {
            claims.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        if original.authorization.account_id != observation.account_presence.account_id()
            || validate_claim_ownership_v1(
                &original.authorization.account_id,
                observation.session,
                current.current_session,
                expected_claims,
                &claims,
            )
            .is_err()
        {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        // The sealed observation does not supersede the independently published
        // current runtime owner. Load its complete guard under the same relation
        // fence before taking L, so concurrent ownership/readiness publication
        // cannot authorize loss against a superseded session observation.
        let runtime = store
            .guards
            .load_locked(
                &mut tx,
                &AdmissionAuthorityGuardKeyV1::Runtime(observation.session.current_runtime_scope()),
            )
            .await?;
        if !matches!(runtime.as_ref().map(|row| &row.state),
            Some(AdmissionAuthorityGuardStateV1::Runtime { ownership_generation, ready: true, .. })
            if *ownership_generation == observation.session.current_scope_generation().get())
        {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        let reservation: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1 AND game_session_id = encode($2,'hex')::uuid AND reservation_owner = 2 AND fresh_replay_key = $3 AND reconnect_attempt_ref IS NULL)")
            .bind(observation.session.commit().initial_transport().to_bytes().as_slice()).bind(session_id.as_bytes().as_slice()).bind(original.authorization.facts.replay_key().to_bytes().as_slice()).fetch_one(&mut *tx).await?;
        let epoch_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_control_loss_continuity WHERE character_id = encode($1,'hex')::uuid AND control_loss_epoch = $2::text::numeric(20,0))")
            .bind(observation.session.commit().character_id().as_bytes().as_slice()).bind(observation.loss_epoch.get().to_string()).fetch_one(&mut *tx).await?;
        if !reservation || epoch_exists {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        // A loss after a same-session resume opens the next epoch. Its retained history is
        // re-derived here from the durable receipts, never taken from the observation.
        let resumed_from = match &observation.history {
            ControlLossHistoryV1::FreshOrigin => None,
            ControlLossHistoryV1::Resumed { budget, .. } => {
                if store
                    .resumed_history_locked(&mut tx, current.current_session)
                    .await?
                    .as_ref()
                    != Some(&observation.history)
                    || budget.epoch().get().checked_add(1) != Some(observation.loss_epoch.get())
                {
                    return Ok(ControlLossOutcomeV1::Rejected);
                }
                Some(budget.epoch())
            }
        };
        // Strong common relation fencing excludes every sibling semantic writer
        // before this single final decision-time sample.
        let decided_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let Ok(effect) = request.validate_final(source.as_ref(), decided_at) else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        if effect.predecessor() != current.current_session {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        let successor = effect.successor();
        let epoch = successor.current_control_loss_epoch().ok_or(DurabilityError::InvalidStoredState)?.get().to_string();
        let grace = successor.current_original_grace_deadline().ok_or(DurabilityError::InvalidStoredState)?;
        let changed = match resumed_from {
            None => sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 1, current_transport_ref = NULL, control_loss_epoch = $2::text::numeric(20,0), original_grace_deadline = $3, predecessor_generation = current_generation WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 2 AND control_loss_epoch IS NULL AND prepared_attempt_ref IS NULL AND attempt_count = 0")
                .bind(session_id.as_bytes().as_slice()).bind(&epoch).bind(grace).execute(&mut *tx).await?,
            // The new epoch starts its own recovery budget.
            Some(resumed) => sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 1, current_transport_ref = NULL, control_loss_epoch = $2::text::numeric(20,0), original_grace_deadline = $3, predecessor_generation = current_generation, attempt_count = 0 WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 2 AND control_loss_epoch = $4::text::numeric(20,0) AND prepared_attempt_ref IS NULL")
                .bind(session_id.as_bytes().as_slice()).bind(&epoch).bind(grace).bind(resumed.get().to_string()).execute(&mut *tx).await?,
        };
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        // The complete canonical loss/protection operation is retained below.
        // Do not manufacture a legacy protection row: its connection-generation
        // namespace is not this operation's entitlement/rearm namespace. Legacy
        // prepare/replacement fail closed on this receipt until a typed bridge.
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES ($1,$2,$3)")
            .bind(&key).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(ControlLossOutcomeV1::Ambiguous);
        }
        Ok(ControlLossOutcomeV1::Committed { decided_at })

            }))
            .await
    }

    /// Bind sealed delivery to an actual validated durable original. Absence or
    /// a conflicting original supplies no definitive completion and no rejection
    /// authority. This does not clear pending custody or reconstruct live inputs.
    pub async fn loss_completion_source(
        &self,
        original: &ControlLossOperationV1,
    ) -> Result<Option<DurableLossCompletionSource>> {
        match self.reconcile_fresh_loss(original).await? {
            FreshLossReconciliation::Committed {
                completion,
                current,
            } => Ok(Some(DurableLossCompletionSource {
                completion: Some(completion),
                current,
            })),
            FreshLossReconciliation::Absent | FreshLossReconciliation::Conflict => Ok(None),
        }
    }

    /// Read the original loss receipt and current canonical session in one fenced
    /// snapshot. Restoring this report never reconstructs a live loss request.
    pub async fn reconcile_fresh_loss(
        &self,
        original: &ControlLossOperationV1,
    ) -> Result<FreshLossReconciliation> {
        let store = (*self).clone();
        let original = (*original).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let encoded = encode_fresh_loss(&original)?;
        let session_id = original.observation.session.commit().game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&original.observation.loss_epoch.get().to_be_bytes());
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? else {
                super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                return Ok(FreshLossReconciliation::Absent);
            };
        let stored: Option<String> = row.try_get("operation_json")?;
        let stored = stored.ok_or(DurabilityError::InvalidStoredState)?;
        let decided_at: i64 = row.try_get("decided_at")?;
        // The canonical fresh receipt supplies the initial commit; inventing a
        // replay key merely to deserialize the loss DTO would lose provenance.
        let row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json FROM game_durability_fresh_admission_receipts r WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let fresh_json: Option<String> = row.try_get("operation_json")?;
        let fresh = decode_operation(
            &fresh_json.ok_or(DurabilityError::InvalidStoredState)?,
            store.maximum_operation_bytes,
        )?;
        let operation = decode_fresh_loss(&stored, checked(fresh.authorization.initial_commit())?)?;
        if operation.observation.session.commit().game_session_id() != session_id
            || operation.observation.loss_epoch != original.observation.loss_epoch
            || operation.observation.account_presence.account_id() != fresh.authorization.account_id
            || decided_at < operation.authorized_at
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if stored != encoded {
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(FreshLossReconciliation::Conflict);
        }
        let FreshReconciliation::Committed(current) =
            store.reconcile_locked(&mut tx, &fresh).await?
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        // The public session snapshot does not retain the SQL predecessor
        // mirror. Read it under the same fence rather than silently dropping its
        // relationship to the immutable original loss.
        let predecessor: Option<String> = sqlx::query_scalar("SELECT predecessor_generation::text FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        let predecessor: u64 = checked(
            predecessor
                .ok_or(DurabilityError::InvalidStoredState)?
                .parse(),
        )?;
        if current.current_session.current_control_loss_epoch()
            == Some(operation.observation.loss_epoch)
            && predecessor
                != operation
                    .observation
                    .session
                    .current_connection_generation()
                    .get()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if current
            .current_session
            .current_connection_generation()
            .get()
            < operation
                .observation
                .session
                .current_connection_generation()
                .get()
            || current
                .current_session
                .current_control_loss_epoch()
                .is_none_or(|epoch| epoch.get() < operation.observation.loss_epoch.get())
            || (current.current_session.current_control_loss_epoch()
                == Some(operation.observation.loss_epoch)
                && current.current_session.current_original_grace_deadline()
                    != Some(operation.observation.original_grace_deadline))
            || (current.current_session.current_connection_generation()
                == operation
                    .observation
                    .session
                    .current_connection_generation()
                && current.current_session.current_transport().is_some())
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        super::admission_journal::commit_pass_transaction(tx, deadline).await?;
        Ok(FreshLossReconciliation::Committed {
            completion: Box::new(ControlLossCompletionV1 {
                operation,
                outcome: ControlLossOutcomeV1::Committed { decided_at },
            }),
            current,
        })

            }))
            .await
    }

    pub async fn reconcile(
        &self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        let store = (*self).clone();
        let original = (*original).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let result = store.reconcile_locked(&mut tx, &original).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    async fn reconcile_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        self.reconcile_session_locked(tx, original, original.authorization.candidate_session)
            .await
    }

    async fn reconcile_session_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: &FreshAdmissionOperationV1,
        current_id: GameSessionId,
    ) -> Result<FreshReconciliation> {
        use sqlx::Row;
        let replay = original.authorization.facts.replay_key().to_bytes();
        let Some(receipt) = self.receipt_locked(tx, &replay).await? else {
            return Ok(FreshReconciliation::Absent);
        };
        if receipt.operation() != original {
            return Ok(FreshReconciliation::Conflict);
        }
        let b = receipt.binding();
        let row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(s)::text) <= $2 THEN to_jsonb(s) END AS state FROM game_durability_reconnect_sessions s WHERE game_session_id = encode($1,'hex')::uuid").bind(current_id.as_bytes().as_slice()).bind(super::MAX_ADMISSION_ROW_BYTES).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
        let state: Option<serde_json::Value> = row.try_get("state")?;
        let state = state.ok_or(DurabilityError::InvalidStoredState)?;
        let initial = checked(b.initial_commit())?;
        if json_text(&state, "account_id")? != b.account_id
            || json_text(&state, "character_id")? != uuid_text(initial.character_id().as_bytes())
            || json_text(&state, "world_id")? != uuid_text(initial.world_id().as_bytes())
            || (state["fresh_replay_key"]
                .as_str()
                .or_else(|| state["initial_fresh_replay_key"].as_str())
                != Some(bytea_text(&replay).as_str()))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let world = checked(WorldId::decode(&json_uuid(
            &state,
            "runtime_scope_world_id",
        )?))?;
        let scope = match json_u64(&state, "runtime_scope_kind")? {
            1 if state["runtime_scope_instance_id"].is_null() => RuntimeScopeRefV1::channel(
                world,
                checked(ChannelId::decode(&json_uuid(
                    &state,
                    "runtime_scope_channel_id",
                )?))?,
            ),
            2 if state["runtime_scope_channel_id"].is_null() => checked(
                RuntimeScopeRefV1::instance(world, json_uuid(&state, "runtime_scope_instance_id")?),
            )?,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let session_state = match json_u64(&state, "session_state")? {
            1 => GameSessionState::Reconnectable,
            2 => GameSessionState::Active,
            3 => GameSessionState::Terminal,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let current_transport = if state["current_transport_ref"].is_null() {
            None
        } else {
            Some(checked(AuthenticatedTransportRefV1::decode(&json_bytea(
                &state,
                "current_transport_ref",
            )?))?)
        };
        let character = self
            .guards
            .load_locked(
                tx,
                &AdmissionAuthorityGuardKeyV1::Character(initial.character_id()),
            )
            .await?;
        let eligibility = match character.as_ref().map(|row| &row.state) {
            Some(AdmissionAuthorityGuardStateV1::Character {
                world_id,
                eligible: true,
                ..
            }) if *world_id == initial.world_id() => Some(CharacterWorldEligibilityClaimV1::new(
                initial.character_id(),
                *world_id,
            )),
            _ => None,
        };
        let mut current_session =
            checked(GameSessionAuthoritySnapshot::from_persisted_current_facts(
                current_id,
                initial,
                session_state,
                checked(ConnectionGeneration::new(json_u64(
                    &state,
                    "current_generation",
                )?))?,
                current_transport,
                checked(CharacterLease::new(
                    initial.character_id(),
                    json_u64(&state, "character_lease_generation")?,
                ))?,
                eligibility,
                scope,
                checked(ScopeOwnershipGeneration::new(json_u64(
                    &state,
                    "scope_ownership_generation",
                )?))?,
            ))?;
        match (
            state.get("control_loss_epoch"),
            state.get("original_grace_deadline"),
            state.get("predecessor_generation"),
        ) {
            (
                Some(serde_json::Value::Null),
                Some(serde_json::Value::Null),
                Some(serde_json::Value::Null),
            ) if session_state != GameSessionState::Reconnectable
                && state["prepared_attempt_ref"].is_null()
                && json_u64(&state, "attempt_count")? == 0 => {}
            (Some(epoch), Some(grace), Some(predecessor))
                if !epoch.is_null()
                    && !grace.is_null()
                    && predecessor.as_u64().is_some_and(|value| {
                        value > 0 && value <= current_session.current_connection_generation().get()
                    }) =>
            {
                current_session = checked(current_session.with_control_loss_continuity(
                    checked(ControlLossEpochRefV1::new(
                        epoch.as_u64().ok_or(DurabilityError::InvalidStoredState)?,
                    ))?,
                    grace.as_i64().ok_or(DurabilityError::InvalidStoredState)?,
                ))?;
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        }
        let reservation: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1 AND game_session_id = encode($2,'hex')::uuid AND reservation_owner = 2 AND fresh_replay_key = $3 AND reconnect_attempt_ref IS NULL)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).fetch_one(&mut **tx).await?;
        if !reservation {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(FreshReconciliation::Committed(Box::new(
            FreshAdmissionDurableReconciliationSnapshotV1 {
                receipt,
                current_session,
            },
        )))
    }
}
// Canonical complete encoding of the supported owning-loss shapes: a fresh-origin loss
// and a loss after a same-session resume, which retains the resumed epoch's history.
// Every omitted alternative is rejected, never silently projected or defaulted.
fn write_fresh_loss(w: &mut Writer, operation: &ControlLossOperationV1) -> Result<()> {
    let o = &operation.observation;
    if operation.version != 1 || o.cause != ControlLossCauseV1::AuthoritativeUnexpectedLoss {
        return Err(DurabilityError::Unavailable);
    }
    w.tag(1)?;
    w.i64(operation.authorized_at)?;
    write_scope(w, o.source_authority)?;
    w.u64(o.source_revision)?;
    w.u64(o.accepted_source_revision)?;
    w.u64(o.decision_identity.get())?;
    w.u64(o.accepted_decision_identity.get())?;
    w.i64(o.observed_at)?;
    let s = o.session;
    let c = s.commit();
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(c.character_id().as_bytes())?;
    w.bytes(c.world_id().as_bytes())?;
    w.bytes(c.channel_id().as_bytes())?;
    w.u64(c.character_lease_generation())?;
    w.u64(c.scope_ownership_generation())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.initial_transport().to_bytes())?;
    w.tag(match s.session_state() {
        GameSessionState::Active => 1,
        GameSessionState::Reconnectable => 2,
        GameSessionState::Terminal => 3,
    })?;
    w.u64(s.current_connection_generation().get())?;
    w.boolean(s.current_transport().is_some())?;
    if let Some(t) = s.current_transport() {
        w.bytes(&t.to_bytes())?;
    }
    w.bytes(s.current_character_lease().character_id().as_bytes())?;
    w.u64(s.current_character_lease().generation())?;
    w.boolean(s.current_character_world_eligibility().is_some())?;
    if let Some(e) = s.current_character_world_eligibility() {
        w.bytes(e.character_id().as_bytes())?;
        w.bytes(e.world_id().as_bytes())?;
    }
    write_scope(w, s.current_runtime_scope())?;
    w.u64(s.current_scope_generation().get())?;
    w.boolean(s.current_control_loss_epoch().is_some())?;
    if let Some(e) = s.current_control_loss_epoch() {
        w.u64(e.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(t) = s.current_original_grace_deadline() {
        w.i64(t)?;
    }
    w.text(o.account_presence.account_id())?;
    w.bytes(o.account_presence.character_id().as_bytes())?;
    w.bytes(&o.placement_identity)?;
    w.u64(o.placement_revision)?;
    w.boolean(o.actor_present)?;
    w.boolean(o.runtime_ready)?;
    w.tag(1)?;
    w.u64(o.loss_epoch.get())?;
    w.i64(o.loss_origin)?;
    w.i64(o.original_grace_deadline)?;
    match &o.history {
        ControlLossHistoryV1::FreshOrigin => w.tag(1)?,
        ControlLossHistoryV1::Resumed {
            budget,
            original_grace_deadline,
            protection,
        } => {
            w.tag(2)?;
            write_budget(w, budget)?;
            w.i64(*original_grace_deadline)?;
            write_protection(w, *protection)?;
        }
    }
    write_protection(w, o.protection)
}
pub fn encode_fresh_loss(operation: &ControlLossOperationV1) -> Result<String> {
    let maximum = super::MAX_FRESH_OPERATION_BYTES;
    let mut counter = Writer::counter(maximum);
    write_fresh_loss(&mut counter, operation)?;
    envelope_size(counter.measured(), maximum)?;
    checked(ControlLossFlowV1::restore(operation.clone()))?;
    let mut writer = Writer::new(maximum);
    write_fresh_loss(&mut writer, operation)?;
    encode_envelope(&writer.bytes, maximum)
}

fn read_protection(r: &mut Reader<'_>) -> Result<RecoveryProtectionContinuityV1> {
    let usage = match r.tag()? {
        0 => RecoveryProtectionUseV1::NotEntitled,
        1 => RecoveryProtectionUseV1::Unused {
            entitlement_generation: r.u64()?,
        },
        2 => RecoveryProtectionUseV1::Activated {
            entitlement_generation: r.u64()?,
            activated_at: r.i64()?,
            deadline: r.i64()?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let rearm = match r.tag()? {
        0 => RecoveryProtectionRearmV1::NotRearmed {
            generation: r.u64()?,
            stable_control_started_at: if r.boolean()? { Some(r.i64()?) } else { None },
            accepted_deadline: if r.boolean()? { Some(r.i64()?) } else { None },
        },
        1 => RecoveryProtectionRearmV1::Satisfied {
            generation: r.u64()?,
            established_at: r.i64()?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(RecoveryProtectionContinuityV1 { usage, rearm })
}

fn read_budget(r: &mut Reader<'_>) -> Result<RetainedRecoveryBudgetV1> {
    let epoch = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let state = match r.tag()? {
        1 => RecoveryEpochStateV1::Open,
        2 => RecoveryEpochStateV1::Restored,
        3 => RecoveryEpochStateV1::Retired,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let count = r.tag()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let attempt = checked(ReconnectAttemptRef::new(u64::from_be_bytes(
            r.bytes::<8>()?,
        )))?;
        let transport = checked(AuthenticatedTransportRefV1::decode(&r.bytes::<16>()?))?;
        let disposition = match r.tag()? {
            1 => RetainedRecoveryAttemptDispositionV1::Committed,
            2 => RetainedRecoveryAttemptDispositionV1::Prepared,
            3 => RetainedRecoveryAttemptDispositionV1::TransportCollision,
            4 => RetainedRecoveryAttemptDispositionV1::Terminal,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        entries.push(RetainedRecoveryAttemptV1 {
            attempt,
            transport,
            disposition,
        });
    }
    checked(RetainedRecoveryBudgetV1::restore(
        epoch, state, true, entries,
    ))
}

/// Restore bounded historical loss bytes against the actual durable fresh
/// receipt's initial commit. No replay key or live authority is synthesized.
pub fn decode_fresh_loss(
    encoded: &str,
    initial: FreshAdmissionCommit<AuthenticatedTransportRefV1>,
) -> Result<ControlLossOperationV1> {
    let bytes = decode_envelope(encoded, super::MAX_FRESH_OPERATION_BYTES)?;
    let mut r = Reader::new(&bytes);
    if r.tag()? != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let authorized_at = r.i64()?;
    let source_authority = read_scope(&mut r)?;
    let source_revision = r.u64()?;
    let accepted_source_revision = r.u64()?;
    let decision_identity = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let accepted_decision_identity = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let observed_at = r.i64()?;
    if r.bytes::<16>()? != *initial.game_session_id().as_bytes()
        || r.bytes::<16>()? != *initial.character_id().as_bytes()
        || r.bytes::<16>()? != *initial.world_id().as_bytes()
        || r.bytes::<16>()? != *initial.channel_id().as_bytes()
        || r.u64()? != initial.character_lease_generation()
        || r.u64()? != initial.scope_ownership_generation()
        || r.u64()? != initial.connection_generation().get()
        || r.bytes::<16>()? != initial.initial_transport().to_bytes()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let state = match r.tag()? {
        1 => GameSessionState::Active,
        2 => GameSessionState::Reconnectable,
        3 => GameSessionState::Terminal,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let generation = checked(ConnectionGeneration::new(r.u64()?))?;
    let transport = if r.boolean()? {
        Some(checked(AuthenticatedTransportRefV1::decode(
            &r.bytes::<16>()?,
        ))?)
    } else {
        None
    };
    let lease = checked(CharacterLease::new(
        checked(CharacterId::decode(&r.bytes::<16>()?))?,
        r.u64()?,
    ))?;
    let eligibility = if r.boolean()? {
        Some(CharacterWorldEligibilityClaimV1::new(
            checked(CharacterId::decode(&r.bytes::<16>()?))?,
            checked(WorldId::decode(&r.bytes::<16>()?))?,
        ))
    } else {
        None
    };
    let scope = read_scope(&mut r)?;
    let scope_generation = checked(ScopeOwnershipGeneration::new(r.u64()?))?;
    let epoch = if r.boolean()? {
        Some(checked(ControlLossEpochRefV1::new(r.u64()?))?)
    } else {
        None
    };
    let grace = if r.boolean()? { Some(r.i64()?) } else { None };
    let mut session = checked(GameSessionAuthoritySnapshot::from_current_facts(
        initial,
        state,
        generation,
        transport,
        lease,
        eligibility,
        scope,
        scope_generation,
    ))?;
    match (epoch, grace) {
        (Some(epoch), Some(grace)) => {
            session = checked(session.with_control_loss_continuity(epoch, grace))?
        }
        (None, None) => {}
        _ => return Err(DurabilityError::InvalidStoredState),
    }
    let account_presence = checked(AccountPresenceClaimV1::new(
        &r.text()?,
        checked(CharacterId::decode(&r.bytes::<16>()?))?,
    ))?;
    let placement_identity = r.bytes::<16>()?;
    let placement_revision = r.u64()?;
    let actor_present = r.boolean()?;
    let runtime_ready = r.boolean()?;
    if r.tag()? != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let loss_epoch = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let loss_origin = r.i64()?;
    let original_grace_deadline = r.i64()?;
    let history = match r.tag()? {
        1 => ControlLossHistoryV1::FreshOrigin,
        2 => ControlLossHistoryV1::Resumed {
            budget: read_budget(&mut r)?,
            original_grace_deadline: r.i64()?,
            protection: read_protection(&mut r)?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let protection = read_protection(&mut r)?;
    r.finish()?;
    let operation = ControlLossOperationV1 {
        version: 1,
        authorized_at,
        observation: ControlLossObservationV1 {
            source_authority,
            source_revision,
            accepted_source_revision,
            decision_identity,
            accepted_decision_identity,
            observed_at,
            session,
            account_presence,
            placement_identity,
            placement_revision,
            actor_present,
            runtime_ready,
            cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
            loss_epoch,
            loss_origin,
            original_grace_deadline,
            history,
            protection,
        },
    };
    // Historical Foundation validation cannot yield another live request.
    checked(ControlLossFlowV1::restore(operation.clone()))?;
    Ok(operation)
}

pub(super) async fn has_owning_loss_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &[u8],
    epoch: u64,
) -> Result<bool> {
    if session.len() != 16 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut key = b"owning-loss-v1".to_vec();
    key.extend_from_slice(session);
    key.extend_from_slice(&epoch.to_be_bytes());
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1)")
        .bind(key).fetch_one(&mut **tx).await.map_err(DurabilityError::from)
}

fn json_text<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or(DurabilityError::InvalidStoredState)
}
fn json_u64(value: &serde_json::Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or(DurabilityError::InvalidStoredState)
}
fn decode_hex(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(DurabilityError::InvalidStoredState);
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| checked(u8::from_str_radix(checked(std::str::from_utf8(pair))?, 16)))
        .collect()
}
fn json_uuid(value: &serde_json::Value, key: &str) -> Result<[u8; 16]> {
    checked(decode_hex(&json_text(value, key)?.replace('-', ""))?.try_into())
}
fn json_bytea(value: &serde_json::Value, key: &str) -> Result<Vec<u8>> {
    decode_hex(
        json_text(value, key)?
            .strip_prefix("\\x")
            .ok_or(DurabilityError::InvalidStoredState)?,
    )
}

struct SessionUseState {
    revision: u64,
    complete: bool,
}

async fn session_use_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<SessionUseState> {
    use sqlx::Row;
    let row = sqlx::query("SELECT version, complete, revision::text, revision_floor::text FROM game_durability_session_use_ledgers WHERE character_id = encode($1, 'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (revision, complete) = if let Some(row) = row {
        let revision = checked(row.try_get::<String, _>("revision")?.parse::<u64>())?;
        let floor = checked(row.try_get::<String, _>("revision_floor")?.parse::<u64>())?;
        if row.try_get::<i16, _>("version")? != 1
            || revision != floor
            || revision > GAME_SESSION_USE_LEDGER_CAPACITY_V1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        (revision, row.try_get::<bool, _>("complete")?)
    } else {
        let existing: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id = encode($1, 'hex')::uuid)")
            .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if existing {
            return Err(DurabilityError::InvalidStoredState);
        }
        (0, true)
    };
    let (count, maximum): (i64, Option<String>) = sqlx::query_as("SELECT count(*), max(membership_revision)::text FROM game_durability_session_use_memberships WHERE character_id = encode($1, 'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let maximum = maximum
        .map(|n| checked(n.parse::<u64>()))
        .transpose()?
        .unwrap_or(0);
    if checked(u64::try_from(count))? != revision || maximum != revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(SessionUseState { revision, complete })
}

/// Consumed only inside the caller's already serialized session transaction.
/// No membership can survive a rollback of the corresponding session effects.
pub(super) async fn commit_session_use(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    candidate: GameSessionId,
    operation_binding: [u8; 16],
    expected_revision: u64,
) -> Result<()> {
    let state = session_use_state(tx, character).await?;
    if !state.complete || state.revision != expected_revision || operation_binding == [0; 16] {
        return Err(DurabilityError::InvalidStoredState);
    }
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if revision > GAME_SESSION_USE_LEDGER_CAPACITY_V1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_durability_session_use_ledgers (character_id, version, complete, revision, revision_floor) VALUES (encode($1,'hex')::uuid,1,TRUE,0,0) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships (game_session_id, character_id, membership_revision, operation_binding) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4)")
        .bind(candidate.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(revision.to_string()).bind(operation_binding.as_slice()).execute(&mut **tx).await?;
    let updated = sqlx::query("UPDATE game_durability_session_use_ledgers SET revision = $2::text::numeric(20,0), revision_floor = $2::text::numeric(20,0) WHERE character_id = encode($1,'hex')::uuid AND revision = $3::text::numeric(20,0) AND revision_floor = $3::text::numeric(20,0) AND complete")
        .bind(character.as_bytes().as_slice()).bind(revision.to_string()).bind(expected_revision.to_string()).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

/// A candidate-specific owner read, not a caller-provided membership answer.
pub struct DurableSessionUseSource {
    request: GameSessionUseRequestV1,
    observation: GameSessionUseObservationV1,
}
impl recovery_source_sealed::Sealed for DurableSessionUseSource {}
impl GameSessionUseObservationSourceV1 for DurableSessionUseSource {
    fn observe_candidate_use(
        &self,
        request: &GameSessionUseRequestV1,
    ) -> std::result::Result<GameSessionUseObservationV1, GameSessionUseAuthorizationErrorV1> {
        if request != &self.request {
            return Err(GameSessionUseAuthorizationErrorV1::StaleAuthority);
        }
        Ok(self.observation.clone())
    }
}

async fn load_session_use_source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: GameSessionUseRequestV1,
) -> Result<DurableSessionUseSource> {
    use sqlx::Row;
    let state = session_use_state(tx, request.character_id()).await?;
    if let Some(fence) = request.current_fence() {
        if request.expected_current() != Some(fence.current_session()) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let current: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid AND character_id = encode($2,'hex')::uuid AND current_generation = $3::text::numeric(20,0) AND character_lease_generation = $4::text::numeric(20,0) AND scope_ownership_generation = $5::text::numeric(20,0))")
            .bind(fence.current_session().as_bytes().as_slice()).bind(request.character_id().as_bytes().as_slice()).bind(fence.connection_generation().to_string()).bind(fence.character_lease_generation().to_string()).bind(fence.scope_ownership_generation().to_string()).fetch_one(&mut **tx).await?;
        if !current {
            return Err(DurabilityError::Unavailable);
        }
    } else if request.expected_current().is_some() {
        return Err(DurabilityError::Unavailable);
    }
    let member = sqlx::query("SELECT uuid_send(character_id) AS character_id, operation_binding, membership_revision::text FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid")
        .bind(request.candidate().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (membership, binding, committed_revision) = match member {
        None => (GameSessionCandidateMembershipV1::Unused, None, None),
        Some(row) => {
            let binding = row
                .try_get::<Option<Vec<u8>>, _>("operation_binding")?
                .map(|v| checked(<[u8; 16]>::try_from(v)))
                .transpose()?;
            let owner: Vec<u8> = row.try_get("character_id")?;
            let exact = owner.as_slice() == request.character_id().as_bytes()
                && binding == Some(request.operation_binding());
            let revision = checked(
                row.try_get::<String, _>("membership_revision")?
                    .parse::<u64>(),
            )?;
            (
                if exact {
                    GameSessionCandidateMembershipV1::UsedByExactOperation
                } else {
                    GameSessionCandidateMembershipV1::UsedByDifferentOperation
                },
                binding,
                Some(revision),
            )
        }
    };
    let observation = GameSessionUseObservationV1::from_owner_results(
        &request,
        state.revision,
        if state.complete {
            GameSessionUseCompletenessV1::Complete
        } else {
            GameSessionUseCompletenessV1::Incomplete
        },
        membership,
        binding,
        committed_revision,
        state.revision,
    );
    Ok(DurableSessionUseSource {
        request,
        observation,
    })
}

impl FreshAdmissionStore {
    pub async fn session_use_source(
        &self,
        request: GameSessionUseRequestV1,
    ) -> Result<DurableSessionUseSource> {
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let source = load_session_use_source(&mut tx, request).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(source)
                })
            })
            .await
    }
}

pub(super) async fn unused_session_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    candidate: GameSessionId,
) -> Result<u64> {
    let state = session_use_state(tx, character).await?;
    if !state.complete {
        return Err(DurabilityError::Unavailable);
    }
    let used: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid)")
        .bind(candidate.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if used {
        return Err(DurabilityError::SessionUseAuthorization(
            GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed,
        ));
    }
    if state.revision == GAME_SESSION_USE_LEDGER_CAPACITY_V1 {
        return Err(DurabilityError::SessionUseAuthorization(
            GameSessionUseAuthorizationErrorV1::TerminalReplacementGameSessionLedgerExhausted,
        ));
    }
    Ok(state.revision)
}

fn write_lifecycle(w: &mut Writer, evidence: &AdmissionClaimLifecycleEvidenceV1) -> Result<()> {
    let s = evidence.operation.current_session();
    let c = s.commit();
    w.tag(match &evidence.operation {
        AdmissionClaimLifecycleOperationV1::TerminalRelease { .. } => 1,
        AdmissionClaimLifecycleOperationV1::TerminalReplacement { .. } => 2,
    })?;
    w.text(evidence.operation.account_id())?;
    for id in [
        c.game_session_id().as_bytes(),
        c.character_id().as_bytes(),
        c.world_id().as_bytes(),
        c.channel_id().as_bytes(),
        s.current_game_session_id().as_bytes(),
    ] {
        w.bytes(id)?;
    }
    w.u64(c.character_lease_generation())?;
    w.u64(c.scope_ownership_generation())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.initial_transport().to_bytes())?;
    w.tag(match s.session_state() {
        GameSessionState::Active => 1,
        GameSessionState::Reconnectable => 2,
        GameSessionState::Terminal => 3,
    })?;
    w.u64(s.current_connection_generation().get())?;
    w.boolean(s.current_transport().is_some())?;
    if let Some(t) = s.current_transport() {
        w.bytes(&t.to_bytes())?;
    }
    w.bytes(s.current_character_lease().character_id().as_bytes())?;
    w.u64(s.current_character_lease().generation())?;
    w.boolean(s.current_character_world_eligibility().is_some())?;
    if let Some(e) = s.current_character_world_eligibility() {
        w.bytes(e.character_id().as_bytes())?;
        w.bytes(e.world_id().as_bytes())?;
    }
    write_scope(w, s.current_runtime_scope())?;
    w.u64(s.current_scope_generation().get())?;
    w.boolean(s.current_control_loss_epoch().is_some())?;
    if let Some(epoch) = s.current_control_loss_epoch() {
        w.u64(epoch.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(grace) = s.current_original_grace_deadline() {
        w.i64(grace)?;
    }
    if let AdmissionClaimLifecycleOperationV1::TerminalReplacement { candidate, .. } =
        &evidence.operation
    {
        // Traverse borrowed fields in both passes: no JSON tree or candidate clone
        // may allocate ahead of the complete lifecycle envelope preflight.
        let identity = candidate.identity();
        w.u64(candidate.version().into())?;
        w.bytes(identity.game_session_id().as_bytes())?;
        w.bytes(&identity.reconnect_attempt_ref().to_be_bytes())?;
        w.text(identity.account_id())?;
        w.bytes(identity.character_id().as_bytes())?;
        w.bytes(identity.world_id().as_bytes())?;
        write_scope(w, identity.runtime_scope())?;
        w.u64(candidate.connection().predecessor().get())?;
        w.u64(candidate.connection().candidate().get())?;
        w.bytes(&candidate.connection().transport_ref().to_bytes())?;
        w.u64(candidate.authority().character_lease_generation())?;
        w.u64(candidate.authority().scope_ownership_generation().get())?;
        w.u64(candidate.continuity().control_loss_epoch().get())?;
        w.i64(candidate.continuity().original_grace_deadline())?;
        w.i64(candidate.continuity().prepared_deadline())?;
        match candidate.continuity().protection_entitlement() {
            ProtectionEntitlementV1::Unused => w.tag(1)?,
            ProtectionEntitlementV1::Fenced { generation } => {
                w.tag(2)?;
                w.u64(generation)?;
            }
        }
        match candidate.proof() {
            ReconnectProofV1::FastReconnect {
                reconnect_proof_generation,
            } => {
                w.tag(1)?;
                w.u64(*reconnect_proof_generation)?;
            }
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce,
            } => {
                w.tag(2)?;
                w.bytes(recovery_grant_nonce)?;
            }
        }
        let fnd02 = candidate.fnd02();
        w.u64(fnd02.next_command_id().get())?;
        w.u64(fnd02.pending().len() as u64)?;
        for pending in fnd02.pending() {
            w.u64(pending.command_id().get())?;
            w.tag(match pending.disposition() {
                PendingCommandDispositionV1::PendingOriginal => 1,
                PendingCommandDispositionV1::TerminalOutcomeRetained => 2,
            })?;
        }
        w.u64(fnd02.server_sequence())?;
        w.u64(fnd02.domain_revisions().len() as u64)?;
        for revision in fnd02.domain_revisions() {
            w.u64(revision.domain_id().into())?;
            w.u64(revision.revision())?;
        }
        let compatibility = candidate.compatibility();
        w.u64(compatibility.protocol_major().into())?;
        w.u64(compatibility.transport_profile().into())?;
        for value in [
            compatibility.ruleset_revision(),
            compatibility.content_revision(),
            compatibility.map_revision(),
            compatibility.world_policy_revision(),
        ] {
            w.text(value)?;
        }
        w.u64(compatibility.account_security_generation())?;
        for fence in [
            compatibility.platform_security_evidence(),
            compatibility.proof_trust_evidence(),
        ] {
            w.text(fence.authority())?;
            w.text(fence.purpose())?;
            w.text(fence.scope())?;
            w.text(fence.source_revision())?;
            w.text(fence.decision_identity())?;
            w.i64(fence.source_observed_at())?;
        }
        w.boolean(compatibility.credential_expiration().is_some())?;
        if let Some(expiration) = compatibility.credential_expiration() {
            w.i64(expiration)?;
        };
    }
    write_changes(w, &evidence.transition.predecessors)?;
    write_changes(w, &evidence.transition.successors)?;
    w.i64(evidence.transition.prepared_at)?;
    Ok(())
}

pub(super) fn encode_lifecycle(evidence: &AdmissionClaimLifecycleEvidenceV1) -> Result<String> {
    let mut size = Writer::counter(super::MAX_FRESH_OPERATION_BYTES);
    write_lifecycle(&mut size, evidence)?;
    envelope_size(size.measured(), super::MAX_FRESH_OPERATION_BYTES)?;
    checked(evidence.validate_historical(evidence.transition.prepared_at))?;
    let mut writer = Writer::new(super::MAX_FRESH_OPERATION_BYTES);
    write_lifecycle(&mut writer, evidence)?;
    encode_envelope(&writer.bytes, super::MAX_FRESH_OPERATION_BYTES)
}

fn lifecycle_key(evidence: &AdmissionClaimLifecycleEvidenceV1) -> Vec<u8> {
    let mut key = b"claim-lifecycle-v1".to_vec();
    key.extend_from_slice(
        evidence
            .operation
            .current_session()
            .current_game_session_id()
            .as_bytes(),
    );
    match &evidence.operation {
        AdmissionClaimLifecycleOperationV1::TerminalRelease { .. } => key.push(1),
        AdmissionClaimLifecycleOperationV1::TerminalReplacement { candidate, .. } => {
            key.push(2);
            key.extend_from_slice(candidate.identity().game_session_id().as_bytes());
        }
    }
    key
}

async fn lifecycle_replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    evidence: &AdmissionClaimLifecycleEvidenceV1,
    encoded: &str,
) -> Result<Option<i64>> {
    use sqlx::Row;
    let row = sqlx::query("SELECT CASE WHEN octet_length(operation_json) <= 65536 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1")
        .bind(lifecycle_key(evidence)).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row
        .try_get::<Option<String>, _>("operation_json")?
        .as_deref()
        != Some(encoded)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let decided_at = row.try_get::<i64, _>("decided_at")?;
    checked(evidence.validate_historical(decided_at))?;
    Ok(Some(decided_at))
}

impl FreshAdmissionStore {
    async fn current_session_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
        let payload: Option<String> = sqlx::query_scalar("SELECT CASE WHEN octet_length(r.operation_json) <= 65536 THEN r.operation_json END FROM game_durability_reconnect_sessions s JOIN game_durability_fresh_admission_receipts r ON r.replay_key = COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) WHERE s.game_session_id = encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.flatten();
        let original = decode_operation(
            &payload.ok_or(DurabilityError::Unavailable)?,
            self.maximum_operation_bytes,
        )?;
        match self
            .reconcile_session_locked(tx, &original, session)
            .await?
        {
            FreshReconciliation::Committed(snapshot) => Ok(snapshot.current_session),
            _ => Err(DurabilityError::InvalidStoredState),
        }
    }

    pub async fn current_session(
        &self,
        session: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let current = store.current_session_locked(&mut tx, session).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(current)
                })
            })
            .await
    }

    /// The current canonical session together with the database clock sampled
    /// in the same fenced pass. Loss observations are timed on this clock: the
    /// final decision samples it again, so a faster host clock cannot refuse
    /// a loss (`observed_at > decided_at`) or shorten its grace.
    pub async fn current_session_at(
        &self,
        session: GameSessionId,
    ) -> Result<(
        GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        i64,
    )> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let current = store.current_session_locked(&mut tx, session).await?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok((current, now))
                })
            })
            .await
    }

    pub async fn reconcile_lifecycle(
        &self,
        evidence: &AdmissionClaimLifecycleEvidenceV1,
    ) -> Result<Option<i64>> {
        let encoded = encode_lifecycle(evidence)?;
        let evidence = evidence.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let result = lifecycle_replay(&mut tx, &evidence, &encoded).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// A sealed owner transition is still revalidated against independently read
    /// SQL current state at the one decision time of the serialized transaction.
    pub async fn release(&self, transition: &TerminalReleaseClaimTransitionV1) -> Result<i64> {
        let encoded = encode_lifecycle(transition.evidence())?;
        let transition = transition.clone();
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let evidence = transition.evidence();
            let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if let Some(prior) = lifecycle_replay(&mut tx, evidence, &encoded).await? {
                super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                return Ok(prior);
            }
            let session_id = evidence.operation.current_session().current_game_session_id();
            let current = store.current_session_locked(&mut tx, session_id).await?;
            let mut rows = Vec::with_capacity(2);
            for prior in &evidence.transition.predecessors { rows.push(store.guards.load_locked(&mut tx, &prior.key).await?); }
            if !store.guards.successor_history_available(&mut tx, &evidence.transition.successors, &rows).await? { return Err(DurabilityError::Unavailable); }
            let encoded_successors = evidence.transition.successors.iter().map(|row| encode_guard(row, store.guards.maximum_guard_bytes)).collect::<Result<Vec<_>>>()?;
            let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let successors = transition.validate_locked(&rows, current, decided_at).map_err(|_| DurabilityError::Unavailable)?;
            let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3, current_transport_ref = NULL, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND current_generation = $2::text::numeric(20,0) AND character_lease_generation = $3::text::numeric(20,0) AND scope_ownership_generation = $4::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice()).bind(current.current_connection_generation().get().to_string()).bind(current.current_character_lease().generation().to_string()).bind(current.current_scope_generation().get().to_string()).execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { return Err(DurabilityError::Unavailable); }
            sqlx::query("UPDATE game_durability_reconnect_attempts SET state = 4 WHERE game_session_id = encode($1,'hex')::uuid AND state IN (1,5)")
                .bind(session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            store.guards.persist_locked(&mut tx, successors, &rows, &encoded_successors).await?;
            sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
                .bind(lifecycle_key(evidence)).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok(decided_at)
        })).await
    }
}

/// Outcome of one grace-expiry release attempt for a lost GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiredLossReleaseV1 {
    /// The session is not a reconnectable loss any more (resumed, replaced or
    /// already released); nothing is this attempt's to release.
    NotApplicable,
    /// The original grace deadline is still ahead on the durable clock.
    NotExpired { deadline: i64, now: i64 },
    /// The terminal release is durable (possibly by an earlier attempt).
    Released { decided_at: i64 },
    /// The lost session is already TERMINAL (an earlier attempt's commit whose
    /// acknowledgement was lost, or another terminal owner). The Channel actor
    /// bound to this exact session is no longer controllable.
    Terminal,
}

/// The owning release source for one expired loss: the current claim rows,
/// cleared for the terminal session at the durable decision time.
struct ExpiredLossReleaseOwner {
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    transition: AdmissionClaimTransitionEvidenceV1,
}
impl fresh_source_sealed::Sealed for ExpiredLossReleaseOwner {}
impl AdmissionClaimOwningSourceV1 for ExpiredLossReleaseOwner {
    fn prepare_lifecycle_claim(
        &self,
        operation: &AdmissionClaimLifecycleOperationV1,
        _now: i64,
    ) -> std::result::Result<
        AdmissionClaimLifecycleResolutionV1,
        AdmissionAuthorityPublicationErrorV1,
    > {
        if !matches!(
            operation,
            AdmissionClaimLifecycleOperationV1::TerminalRelease { .. }
        ) {
            return Err(AdmissionAuthorityPublicationErrorV1::Invalid);
        }
        Ok(AdmissionClaimLifecycleResolutionV1 {
            current_session: self.current,
            evidence: self.transition.clone(),
        })
    }
    fn prepare_fresh_claim(
        &self,
        _binding: &FreshAdmissionAuditBindingV1,
        _now: i64,
    ) -> std::result::Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>
    {
        Err(AdmissionAuthorityPublicationErrorV1::Unavailable)
    }
}

impl FreshAdmissionStore {
    /// FND-04B §6 grace expiry: once the original deadline of a committed loss
    /// has passed on the durable clock, terminally release the session. The
    /// release is prepared from the *current* claim rows (an owner refresh
    /// since admission is not an ownership change) and committed through the
    /// exact fenced [`Self::release`], which rejects any concurrent change of
    /// the session or its claims. A lost acknowledgement is reconciled, never
    /// decided again.
    pub async fn release_expired_loss(
        &self,
        session: GameSessionId,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        let Some(deadline) = current.current_original_grace_deadline() else {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        };
        if current.session_state() != GameSessionState::Reconnectable
            || current.current_control_loss_epoch().is_none()
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        if now < deadline {
            return Ok(ExpiredLossReleaseV1::NotExpired { deadline, now });
        }
        self.release_current_claims(current, now, account_id).await
    }

    /// Terminal release of a resumed session whose recovered connection ended again when
    /// its loss after the resume could not be recorded, so it is never stranded ACTIVE on a
    /// dead transport: the player loses grace, never the ability to enter again. Only the
    /// exact ended controller transport of a session that was already lost once is
    /// released; a newer controller keeps its session.
    pub async fn release_abandoned_session(
        &self,
        session: GameSessionId,
        account_id: &str,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<ExpiredLossReleaseV1> {
        let (current, now) = self.current_session_at(session).await?;
        if current.session_state() == GameSessionState::Terminal {
            return Ok(ExpiredLossReleaseV1::Terminal);
        }
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(transport)
            || current.current_control_loss_epoch().is_none()
        {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        }
        self.release_current_claims(current, now, account_id).await
    }

    /// Prepare the FND-04B terminal release of `current` from the current claim rows at the
    /// durable time `now` and commit it through the exact fenced [`Self::release`].
    async fn release_current_claims(
        &self,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let keys = [
            AdmissionAuthorityGuardKeyV1::Account {
                account_id: account_id.into(),
            },
            AdmissionAuthorityGuardKeyV1::Character(current.commit().character_id()),
        ];
        let predecessors = self
            .guards
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DurabilityError::Unavailable)?;
        // Successor source times never precede a predecessor's (a row last
        // published on a faster host clock): until the durable clock catches
        // up, the fenced release refuses and a later attempt retries.
        let prepared_at = predecessors
            .iter()
            .map(|row| row.source.source_observed_at)
            .fold(now, i64::max);
        let mut successors = predecessors.clone();
        for row in &mut successors {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: row.publication_revision,
            };
            row.publication_revision = row
                .publication_revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            row.source.source_revision = row
                .source
                .source_revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            row.source.decision_identity =
                format!("grace-expiry-release:{}", row.source.source_revision);
            row.source.source_observed_at = prepared_at;
            row.source.clock_uncertainty_seconds = 0;
            match &mut row.state {
                AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                    security.provenance.publication_revision = row.publication_revision;
                    *presence = None;
                }
                AdmissionAuthorityGuardStateV1::Character { holder, .. } => *holder = None,
                _ => return Err(DurabilityError::InvalidStoredState),
            }
        }
        let owner = ExpiredLossReleaseOwner {
            current,
            transition: AdmissionClaimTransitionEvidenceV1 {
                predecessors,
                successors,
                prepared_at,
            },
        };
        let transition = match TerminalReleaseClaimTransitionV1::prepare(
            &owner,
            account_id,
            current,
            prepared_at,
        ) {
            Ok(transition) => transition,
            // Ownership no longer names this session: not this release's to make.
            Err(AdmissionAuthorityPublicationErrorV1::Stale) => {
                return Ok(ExpiredLossReleaseV1::NotApplicable);
            }
            Err(_) => return Err(DurabilityError::Unavailable),
        };
        match self.release(&transition).await {
            Ok(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
            Err(error) => match self.reconcile_lifecycle(transition.evidence()).await? {
                Some(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
                None => Err(error),
            },
        }
    }
}

pub(super) struct PreparedReplacementClaims {
    rows: Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
    encoded_successors: Vec<String>,
    encoded: String,
    pub(super) decided_at: i64,
}
impl FreshAdmissionStore {
    pub(super) fn from_journal(journal: &super::AdmissionReconnectJournal) -> Self {
        Self {
            guards: AdmissionGuardStore {
                backend: journal.backend.clone(),
                maximum_guard_bytes: super::MAX_ADMISSION_GUARD_BYTES,
            },
            maximum_operation_bytes: super::MAX_FRESH_OPERATION_BYTES,
        }
    }
    pub(super) async fn replacement_claim_replay(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
    ) -> Result<Option<i64>> {
        lifecycle_replay(
            tx,
            transition.evidence(),
            &encode_lifecycle(transition.evidence())?,
        )
        .await
    }
    pub(super) async fn prepare_replacement_claims(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
        request: GameSessionUseRequestV1,
        candidate: &ReconnectDurabilityRecordV1,
    ) -> Result<PreparedReplacementClaims> {
        let evidence = transition.evidence();
        let current = self
            .current_session_locked(
                tx,
                evidence
                    .operation
                    .current_session()
                    .current_game_session_id(),
            )
            .await?;
        if request.character_id() != candidate.identity().character_id()
            || request.candidate() != candidate.identity().game_session_id()
            || request.expected_current() != Some(current.current_game_session_id())
            || request.operation_binding() != candidate.connection().transport_ref().to_bytes()
        {
            return Err(DurabilityError::Unavailable);
        }
        let source = load_session_use_source(tx, request).await?;
        match GameSessionUseAuthorityV1::from_owning_source(&source)
            .authorize_terminal_replacement(request)
            .map_err(DurabilityError::SessionUseAuthorization)?
        {
            GameSessionUseDecisionV1::NewSession { .. } => {}
            GameSessionUseDecisionV1::ExactCommittedReplay { .. } => {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        let mut rows = Vec::with_capacity(2);
        for prior in &evidence.transition.predecessors {
            rows.push(self.guards.load_locked(tx, &prior.key).await?);
        }
        if !self
            .guards
            .successor_history_available(tx, &evidence.transition.successors, &rows)
            .await?
        {
            return Err(DurabilityError::Unavailable);
        }
        let encoded_successors = evidence
            .transition
            .successors
            .iter()
            .map(|row| encode_guard(row, self.guards.maximum_guard_bytes))
            .collect::<Result<Vec<_>>>()?;
        let encoded = encode_lifecycle(evidence)?;
        let decided_at =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut **tx)
                .await?;
        transition
            .validate_locked(&rows, current, candidate, decided_at)
            .map_err(|_| DurabilityError::Unavailable)?;
        Ok(PreparedReplacementClaims {
            rows,
            encoded_successors,
            encoded,
            decided_at,
        })
    }
    pub(super) async fn persist_replacement_claims(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
        prepared: &PreparedReplacementClaims,
    ) -> Result<()> {
        let evidence = transition.evidence();
        self.guards
            .persist_locked(
                tx,
                &evidence.transition.successors,
                &prepared.rows,
                &prepared.encoded_successors,
            )
            .await?;
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key,operation_json,decided_at) VALUES ($1,$2,$3)")
            .bind(lifecycle_key(evidence)).bind(&prepared.encoded).bind(prepared.decided_at).execute(&mut **tx).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Complete reconnect (FND-04B §§11–13, §20): canonical same-session receipts.
// ---------------------------------------------------------------------------

fn write_session_snapshot(
    w: &mut Writer,
    s: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
) -> Result<()> {
    let c = s.commit();
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(c.character_id().as_bytes())?;
    w.bytes(c.world_id().as_bytes())?;
    w.bytes(c.channel_id().as_bytes())?;
    w.u64(c.character_lease_generation())?;
    w.u64(c.scope_ownership_generation())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.initial_transport().to_bytes())?;
    w.bytes(s.current_game_session_id().as_bytes())?;
    w.tag(match s.session_state() {
        GameSessionState::Active => 1,
        GameSessionState::Reconnectable => 2,
        GameSessionState::Terminal => 3,
    })?;
    w.u64(s.current_connection_generation().get())?;
    w.boolean(s.current_transport().is_some())?;
    if let Some(t) = s.current_transport() {
        w.bytes(&t.to_bytes())?;
    }
    w.bytes(s.current_character_lease().character_id().as_bytes())?;
    w.u64(s.current_character_lease().generation())?;
    w.boolean(s.current_character_world_eligibility().is_some())?;
    if let Some(e) = s.current_character_world_eligibility() {
        w.bytes(e.character_id().as_bytes())?;
        w.bytes(e.world_id().as_bytes())?;
    }
    write_scope(w, s.current_runtime_scope())?;
    w.u64(s.current_scope_generation().get())?;
    w.boolean(s.current_control_loss_epoch().is_some())?;
    if let Some(e) = s.current_control_loss_epoch() {
        w.u64(e.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(t) = s.current_original_grace_deadline() {
        w.i64(t)?;
    }
    Ok(())
}

fn write_protection(w: &mut Writer, p: RecoveryProtectionContinuityV1) -> Result<()> {
    match p.usage {
        RecoveryProtectionUseV1::NotEntitled => w.tag(0)?,
        RecoveryProtectionUseV1::Unused {
            entitlement_generation,
        } => {
            w.tag(1)?;
            w.u64(entitlement_generation)?;
        }
        RecoveryProtectionUseV1::Activated {
            entitlement_generation,
            activated_at,
            deadline,
        } => {
            w.tag(2)?;
            w.u64(entitlement_generation)?;
            w.i64(activated_at)?;
            w.i64(deadline)?;
        }
    }
    match p.rearm {
        RecoveryProtectionRearmV1::Satisfied {
            generation,
            established_at,
        } => {
            w.tag(1)?;
            w.u64(generation)?;
            w.i64(established_at)?;
        }
        RecoveryProtectionRearmV1::NotRearmed {
            generation,
            stable_control_started_at,
            accepted_deadline,
        } => {
            w.tag(0)?;
            w.u64(generation)?;
            for time in [stable_control_started_at, accepted_deadline] {
                w.boolean(time.is_some())?;
                if let Some(time) = time {
                    w.i64(time)?;
                }
            }
        }
    }
    Ok(())
}

fn write_candidate(w: &mut Writer, c: ReconnectCandidateBindingV1) -> Result<()> {
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(&c.reconnect_attempt_ref().to_be_bytes())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.transport_ref().to_bytes())?;
    w.i64(c.prepared_deadline())
}

fn write_budget(w: &mut Writer, b: &RetainedRecoveryBudgetV1) -> Result<()> {
    w.u64(b.epoch().get())?;
    w.tag(match b.state() {
        RecoveryEpochStateV1::Open => 1,
        RecoveryEpochStateV1::Restored => 2,
        RecoveryEpochStateV1::Retired => 3,
    })?;
    w.tag(checked(u8::try_from(b.entries().len()))?)?;
    for e in b.entries() {
        w.bytes(&e.attempt.to_be_bytes())?;
        w.bytes(&e.transport.to_bytes())?;
        w.tag(match e.disposition {
            RetainedRecoveryAttemptDispositionV1::Committed => 1,
            RetainedRecoveryAttemptDispositionV1::Prepared => 2,
            RetainedRecoveryAttemptDispositionV1::TransportCollision => 3,
            RetainedRecoveryAttemptDispositionV1::Terminal => 4,
        })?;
    }
    Ok(())
}

fn write_recovery_revisions(w: &mut Writer, revisions: [&str; 4]) -> Result<()> {
    for revision in revisions {
        w.text(revision)?;
    }
    Ok(())
}

fn write_complete_reconnect(
    w: &mut Writer,
    operation: &CompleteReconnectDurabilityOperationV1,
) -> Result<()> {
    let recovery = &operation.recovery;
    let original = &recovery.original;
    // Only same-session reauthenticated recovery is durable here; a replacement onto a
    // new session and fast-reconnect proof delivery have no owner yet.
    let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
        return Err(DurabilityError::Unavailable);
    };
    if recovery.version != 1
        || recovery.mode != CompleteReconnectModeV1::SameSession
        || operation.replacement.is_some()
        || original.replacement_anchor.is_some()
    {
        return Err(DurabilityError::Unavailable);
    }
    w.tag(1)?;
    let identity = &recovery.identity;
    w.bytes(identity.game_session_id().as_bytes())?;
    w.bytes(&identity.reconnect_attempt_ref().to_be_bytes())?;
    w.text(identity.account_id())?;
    w.bytes(identity.character_id().as_bytes())?;
    w.bytes(identity.world_id().as_bytes())?;
    write_scope(w, identity.runtime_scope())?;
    w.i64(recovery.prepared_at)?;
    w.tag(checked(u8::try_from(original.predecessor_attempts.len()))?)?;
    for attempt in &original.predecessor_attempts {
        w.bytes(attempt.session.as_bytes())?;
        w.bytes(&attempt.attempt.to_be_bytes())?;
        w.bytes(&attempt.transport.to_bytes())?;
    }
    write_fresh_loss(w, &original.loss)?;
    w.i64(original.loss_decided_at)?;
    write_scope(w, original.source_authority)?;
    w.u64(original.source_revision)?;
    w.u64(original.accepted_source_revision)?;
    w.i64(original.observed_at)?;
    write_session_snapshot(w, &original.session)?;
    w.text(original.account_presence.account_id())?;
    w.bytes(original.account_presence.character_id().as_bytes())?;
    w.boolean(original.actor_present)?;
    w.boolean(original.runtime_ready)?;
    w.bytes(&original.placement_identity)?;
    w.u64(original.placement_revision)?;
    write_protection(w, original.protection)?;
    write_budget(w, &original.budget)?;
    write_candidate(w, original.candidate)?;
    let proof = &original.proof_transition;
    write_scope(w, proof.owner)?;
    w.u64(proof.revision)?;
    w.u64(proof.accepted_revision)?;
    w.i64(proof.observed_at)?;
    w.bytes(proof.predecessor_session.as_bytes())?;
    w.u64(proof.predecessor_generation)?;
    w.bytes(proof.successor_session.as_bytes())?;
    w.u64(proof.successor_generation)?;
    write_candidate(w, proof.candidate)?;
    let fnd02 = &original.fnd02;
    w.u64(fnd02.next_command_id().get())?;
    w.tag(checked(u8::try_from(fnd02.pending().len()))?)?;
    for pending in fnd02.pending() {
        w.u64(pending.command_id().get())?;
        w.tag(match pending.disposition() {
            PendingCommandDispositionV1::PendingOriginal => 1,
            PendingCommandDispositionV1::TerminalOutcomeRetained => 2,
        })?;
    }
    w.u64(fnd02.server_sequence())?;
    w.tag(checked(u8::try_from(fnd02.domain_revisions().len()))?)?;
    for revision in fnd02.domain_revisions() {
        w.u64(u64::from(revision.domain_id()))?;
        w.u64(revision.revision())?;
    }
    let current = &original.recovery;
    w.text(&current.account_id)?;
    w.bytes(current.character_id.as_bytes())?;
    w.bytes(current.world_id.as_bytes())?;
    write_recovery_revisions(
        w,
        [
            &current.ruleset_revision,
            &current.content_revision,
            &current.map_revision,
            &current.world_policy_revision,
        ],
    )?;
    write_changes(w, &original.claims)?;
    w.tag(1)?;
    w.bytes(&credential.grant_nonce)?;
    w.u64(credential.account_security_generation)?;
    w.u64(credential.protocol_major)?;
    w.u64(credential.transport_profile)?;
    write_recovery_revisions(w, credential.revisions.each_ref().map(String::as_str))?;
    w.i64(credential.expires_at)?;
    w.boolean(credential.v1_trust.is_some())?;
    if let Some(trust) = &credential.v1_trust {
        w.text(&trust.signing_key_id)?;
        w.bytes(&trust.signing_public_key)?;
        w.u64(trust.minimum_generation)?;
    }
    w.boolean(credential.v2.is_some())?;
    if let Some(audit) = &credential.v2 {
        w.bytes(&audit.credential_attempt_ref)?;
        w.bytes(&audit.grant_nonce)?;
        w.text(&audit.account_id)?;
        w.bytes(audit.character_id.as_bytes())?;
        w.bytes(audit.world_id.as_bytes())?;
        w.u64(audit.account_security_generation)?;
        w.u64(audit.protocol_major)?;
        w.u64(audit.transport_profile)?;
        write_recovery_revisions(
            w,
            [
                &audit.ruleset_revision,
                &audit.content_revision,
                &audit.map_revision,
                &audit.world_policy_revision,
            ],
        )?;
        w.i64(audit.issued_at)?;
        w.i64(audit.not_before)?;
        w.i64(audit.expires_at)?;
        w.text(&audit.signing.key_id)?;
        w.bytes(&audit.signing.public_key)?;
        w.boolean(audit.signing.trusted)?;
        write_provenance(w, &audit.signing.provenance)?;
        w.text(&audit.security.account_id)?;
        w.u64(audit.security.minimum_generation)?;
        w.boolean(audit.security.allowed)?;
        write_provenance(w, &audit.security.provenance)?;
        w.i64(audit.verified_at)?;
        w.i64(audit.accepted_deadline)?;
    }
    Ok(())
}

/// Canonical bounded bytes of one same-session complete-reconnect operation.
/// Receipts compare these in full; no digest grants identity.
pub fn encode_complete_reconnect(
    operation: &CompleteReconnectDurabilityOperationV1,
) -> Result<String> {
    checked(operation.validate_historical())?;
    let maximum = super::MAX_FRESH_OPERATION_BYTES;
    let mut counter = Writer::counter(maximum);
    write_complete_reconnect(&mut counter, operation)?;
    envelope_size(counter.measured(), maximum)?;
    let mut writer = Writer::new(maximum);
    write_complete_reconnect(&mut writer, operation)?;
    encode_envelope(&writer.bytes, maximum)
}

const COMPLETE_PREPARE_KEY: &[u8] = b"complete-reconnect-prepare-v1";
const COMPLETE_COMMIT_KEY: &[u8] = b"complete-reconnect-commit-v1";

fn complete_reconnect_prefix(phase: &[u8], session: GameSessionId, epoch: u64) -> Vec<u8> {
    let mut key = phase.to_vec();
    key.extend_from_slice(session.as_bytes());
    key.extend_from_slice(&epoch.to_be_bytes());
    key
}

fn complete_reconnect_key(phase: &[u8], identity: &ReconnectIdentityV1, epoch: u64) -> Vec<u8> {
    let mut key = complete_reconnect_prefix(phase, identity.game_session_id(), epoch);
    key.extend_from_slice(&identity.reconnect_attempt_ref().to_be_bytes());
    key
}

impl FreshAdmissionStore {
    /// The complete retained recovery budget of one loss epoch, reconstructed from the
    /// immutable prepare/commit receipts and candidate transport reservations under the
    /// caller's relation locks. A prepared attempt no longer named by the session was
    /// aborted or superseded (`Terminal`).
    async fn recovery_budget_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<RetainedRecoveryBudgetV1> {
        use sqlx::Row;
        let prefix = complete_reconnect_prefix(COMPLETE_PREPARE_KEY, session, epoch.get());
        let prepared: Option<Vec<u8>> = sqlx::query_scalar("SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let rows = sqlx::query("SELECT substring(r.operation_key from $2) AS attempt, t.transport_ref, EXISTS (SELECT 1 FROM game_durability_admission_lifecycle_receipts c WHERE c.operation_key = $3 || substring(r.operation_key from $2)) AS committed FROM game_durability_admission_lifecycle_receipts r JOIN game_durability_transport_ref_reservations t ON t.game_session_id = encode($4,'hex')::uuid AND t.reservation_owner = 1 AND t.reconnect_attempt_ref = substring(r.operation_key from $2) WHERE r.operation_key > $1 AND r.operation_key < $1 || '\\xffffffffffffffffff'::bytea AND octet_length(r.operation_key) = $5 + 8 ORDER BY r.decided_at, r.operation_key")
            .bind(&prefix)
            .bind(i32::try_from(prefix.len() + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(complete_reconnect_prefix(COMPLETE_COMMIT_KEY, session, epoch.get()))
            .bind(session.as_bytes().as_slice())
            .bind(i32::try_from(prefix.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
            .fetch_all(&mut **tx).await?;
        let mut entries = Vec::with_capacity(rows.len());
        let mut restored = false;
        for row in rows {
            let attempt: Vec<u8> = row.try_get("attempt")?;
            let transport: Vec<u8> = row.try_get("transport_ref")?;
            let committed: bool = row.try_get("committed")?;
            let attempt_bytes: [u8; 8] = attempt
                .as_slice()
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            restored |= committed;
            entries.push(RetainedRecoveryAttemptV1 {
                attempt: checked(ReconnectAttemptRef::new(u64::from_be_bytes(attempt_bytes)))?,
                transport: checked(AuthenticatedTransportRefV1::decode(&transport))?,
                disposition: if committed {
                    RetainedRecoveryAttemptDispositionV1::Committed
                } else if prepared.as_deref() == Some(attempt.as_slice()) {
                    RetainedRecoveryAttemptDispositionV1::Prepared
                } else {
                    RetainedRecoveryAttemptDispositionV1::Terminal
                },
            });
        }
        let state = if restored {
            RecoveryEpochStateV1::Restored
        } else {
            RecoveryEpochStateV1::Open
        };
        checked(RetainedRecoveryBudgetV1::restore(
            epoch, state, true, entries,
        ))
    }

    /// The current retained recovery budget of one loss epoch, for the owning source.
    pub async fn recovery_budget(
        &self,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<RetainedRecoveryBudgetV1> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let budget = store
                        .recovery_budget_locked(&mut tx, session, epoch)
                        .await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(budget)
                })
            })
            .await
    }

    /// FND-04B same-session reauthenticated recovery PREPARE or COMMIT of one owning-loss
    /// session (§§11–13, §20). Under the admission relation locks the adapter binds the
    /// operation to the exact durable original loss, the exact current session, current
    /// claim ownership, the ready runtime guard and the reconstructed retained budget, then
    /// samples the single decision time and lets the sealed request validate its source.
    /// PREPARE reserves the candidate transport and names the attempt on the session;
    /// COMMIT consumes the RecoveryGrantNonce and switches the session to the candidate
    /// connection atomically. Receipts are immutable and replay exactly; a lost
    /// acknowledgement is reconciled, never decided twice.
    ///
    /// The recovery credential is revalidated against the registered Recovery V2 floors
    /// fenced in the same transaction (custody, registration, account and trust floors
    /// after the admission relation locks), never against a caller-held observation.
    pub async fn apply_registered_complete_reconnect(
        &self,
        custody: &super::runtime_scope_assignment::NodeIncarnationProof,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
    ) -> Result<CompleteReconnectOutcomeV1> {
        self.apply_complete_reconnect_with(Some(custody.clone()), request, source)
            .await
    }

    /// Test seam: the same decision against the source's own recovery evidence.
    #[cfg(test)]
    pub async fn apply_complete_reconnect(
        &self,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
    ) -> Result<CompleteReconnectOutcomeV1> {
        self.apply_complete_reconnect_with(None, request, source)
            .await
    }

    async fn apply_complete_reconnect_with(
        &self,
        custody: Option<super::runtime_scope_assignment::NodeIncarnationProof>,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
    ) -> Result<CompleteReconnectOutcomeV1> {
        let store = (*self).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let commit = match request.kind() {
            CompleteReconnectRequestKindV1::Prepare => false,
            CompleteReconnectRequestKindV1::Commit => true,
            CompleteReconnectRequestKindV1::Reconcile => return Err(DurabilityError::Unavailable),
        };
        let operation = request.operation();
        let encoded = encode_complete_reconnect(operation)?;
        let recovery = &operation.recovery;
        let original = &recovery.original;
        let identity = &recovery.identity;
        let session_id = identity.game_session_id();
        let epoch = original.loss.observation.loss_epoch;
        let key = complete_reconnect_key(
            if commit { COMPLETE_COMMIT_KEY } else { COMPLETE_PREPARE_KEY },
            identity,
            epoch.get(),
        );
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? {
            let stored: Option<String> = row.try_get("operation_json")?;
            if stored.as_deref() != Some(encoded.as_str()) { return Err(DurabilityError::InvalidStoredState); }
            let decided_at: i64 = row.try_get("decided_at")?;
            if decided_at < recovery.prepared_at { return Err(DurabilityError::InvalidStoredState); }
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(if commit {
                CompleteReconnectOutcomeV1::Committed { decided_at }
            } else {
                CompleteReconnectOutcomeV1::Prepared { decided_at }
            });
        }
        // The exact durable original loss this recovery resumes.
        let mut loss_key = b"owning-loss-v1".to_vec();
        loss_key.extend_from_slice(session_id.as_bytes());
        loss_key.extend_from_slice(&epoch.get().to_be_bytes());
        let loss_row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&loss_key).fetch_optional(&mut *tx).await?;
        let Some(loss_row) = loss_row else { return Ok(CompleteReconnectOutcomeV1::Rejected) };
        let stored_loss: Option<String> = loss_row.try_get("operation_json")?;
        let loss_decided_at: i64 = loss_row.try_get("decided_at")?;
        if stored_loss.as_deref() != Some(encode_fresh_loss(&original.loss)?.as_str())
            || loss_decided_at != original.loss_decided_at
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let fresh_row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json FROM game_durability_fresh_admission_receipts r WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let fresh_json: Option<String> = fresh_row.try_get("operation_json")?;
        let fresh = decode_operation(&fresh_json.ok_or(DurabilityError::InvalidStoredState)?, store.maximum_operation_bytes)?;
        if fresh.authorization.account_id != identity.account_id() {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        // Current session, claim ownership, runtime owner and retained budget.
        let current = store.current_session_locked(&mut tx, session_id).await?;
        // PREPARE leaves the snapshot unchanged, so both phases bind the original.
        if current != original.session {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let mut claims = Vec::with_capacity(2);
        for expected in &original.claims {
            claims.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        // Recovery grants control: the locked rows must be exactly the claims the sealed
        // request validated (security allowed and generation floor, eligibility,
        // presence and holder), not merely still owned by this session.
        if claims.len() != original.claims.len()
            || claims.iter().zip(&original.claims).any(|(row, expected)| row.as_ref() != Some(expected))
            || validate_claim_ownership_v1(identity.account_id(), original.session, current, &original.claims, &claims).is_err()
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let runtime = store.guards.load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Runtime(current.current_runtime_scope())).await?;
        // The runtime owner is ready at the session's scope generation and still serves exactly
        // the ruleset/content/map/world-policy revisions the credential was verified against.
        let revisions = &original.recovery;
        if !matches!(runtime.as_ref().map(|row| &row.state),
            Some(AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation, ready: true, ruleset_revision, content_revision, map_revision, world_policy_revision, ..
            })
            if *ownership_generation == current.current_scope_generation().get()
                && *ruleset_revision == revisions.ruleset_revision
                && *content_revision == revisions.content_revision
                && *map_revision == revisions.map_revision
                && *world_policy_revision == revisions.world_policy_revision)
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let budget = store.recovery_budget_locked(&mut tx, session_id, epoch).await?;
        let expected_budget = if commit {
            // The prepared attempt is part of the durable budget by now.
            recovery_budget_after_prepare(recovery)?
        } else {
            original.budget.clone()
        };
        if budget != expected_budget {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let candidate = original.candidate;
        let attempt = identity.reconnect_attempt_ref().to_be_bytes();
        let transport = candidate.transport_ref().to_bytes();
        let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx).await?;
        let decision = match &custody {
            Some(custody) => {
                let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                    return Ok(CompleteReconnectOutcomeV1::Rejected);
                };
                let Some(audit) = &credential.v2 else {
                    return Ok(CompleteReconnectOutcomeV1::Rejected);
                };
                let subject = super::recovery_evidence_composition::RecoveryEvidenceSubject::new(
                    identity.account_id(),
                    audit.signing.key_id.clone(),
                )?;
                super::recovery_evidence_composition::decide_with_registered_recovery(
                    &mut tx,
                    custody,
                    &subject,
                    |registered| {
                        request.validate_locked(
                            &RegisteredRecoverySource { inner: source.as_ref(), registered },
                            decided_at,
                        )
                    },
                )
                .await?
            }
            None => request.validate_locked(source.as_ref(), decided_at),
        };
        let Ok(effect) = decision else {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        };
        if commit {
            if !store.complete_prepare_exists(&mut tx, identity, epoch, &encoded).await? {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            };
            let consumed = sqlx::query("INSERT INTO game_durability_recovery_grant_consumptions (recovery_grant_nonce, game_session_id, reconnect_attempt_ref) VALUES ($1, encode($2,'hex')::uuid, $3) ON CONFLICT (recovery_grant_nonce) DO NOTHING")
                .bind(credential.grant_nonce.as_slice()).bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if consumed.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let next = effect.session();
            let switched = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 2, current_generation = $2::text::numeric(20,0), current_transport_ref = $3, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND current_transport_ref IS NULL AND prepared_attempt_ref = $4 AND current_generation = $5::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice())
                .bind(next.current_connection_generation().get().to_string())
                .bind(transport.as_slice())
                .bind(attempt.as_slice())
                .bind(original.session.current_connection_generation().get().to_string())
                .execute(&mut *tx).await?;
            if switched.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
            if store.current_session_locked(&mut tx, session_id).await? != next {
                return Err(DurabilityError::InvalidStoredState);
            }
        } else {
            // A grant nonce that was already consumed can never commit; do not let it
            // strand the session with a prepared attempt.
            let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            };
            let spent: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_recovery_grant_consumptions WHERE recovery_grant_nonce = $1)")
                .bind(credential.grant_nonce.as_slice()).fetch_one(&mut *tx).await?;
            if spent {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let reserved = sqlx::query("INSERT INTO game_durability_transport_ref_reservations (transport_ref, game_session_id, reconnect_attempt_ref, reservation_owner) VALUES ($1, encode($2,'hex')::uuid, $3, 1) ON CONFLICT (transport_ref) DO NOTHING")
                .bind(transport.as_slice()).bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if reserved.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let named = sqlx::query("UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = $2, attempt_count = attempt_count + 1 WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND current_transport_ref IS NULL AND prepared_attempt_ref IS NULL AND attempt_count < 8")
                .bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if named.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
        }
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
            .bind(&key).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        if commit && store.recovery_budget_locked(&mut tx, session_id, epoch).await? != *effect.budget() {
            return Err(DurabilityError::InvalidStoredState);
        }
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(CompleteReconnectOutcomeV1::Ambiguous);
        }
        Ok(if commit {
            CompleteReconnectOutcomeV1::Committed { decided_at }
        } else {
            CompleteReconnectOutcomeV1::Prepared { decided_at }
        })
            }))
            .await
    }

    async fn complete_prepare_exists(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        identity: &ReconnectIdentityV1,
        epoch: ControlLossEpochRefV1,
        encoded: &str,
    ) -> Result<bool> {
        let key = complete_reconnect_key(COMPLETE_PREPARE_KEY, identity, epoch.get());
        let stored: Option<Option<String>> = sqlx::query_scalar("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx).await?;
        Ok(stored.flatten().as_deref() == Some(encoded))
    }

    /// The durable original loss of `session` at `epoch` and its decision time, decoded
    /// against the session's canonical fresh receipt. `None` when no owning loss exists.
    pub async fn owning_loss(
        &self,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<Option<(ControlLossOperationV1, i64)>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let loss = store.owning_loss_locked(&mut tx, session, epoch).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(loss)
                })
            })
            .await
    }

    async fn owning_loss_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<Option<(ControlLossOperationV1, i64)>> {
        use sqlx::Row;
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session.as_bytes());
        key.extend_from_slice(&epoch.get().to_be_bytes());
        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx).await? else {
            return Ok(None);
        };
        let stored: Option<String> = row.try_get("operation_json")?;
        let decided_at: i64 = row.try_get("decided_at")?;
        let fresh_row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json FROM game_durability_fresh_admission_receipts r WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let fresh_json: Option<String> = fresh_row.try_get("operation_json")?;
        let fresh = decode_operation(
            &fresh_json.ok_or(DurabilityError::InvalidStoredState)?,
            self.maximum_operation_bytes,
        )?;
        let loss = decode_fresh_loss(
            &stored.ok_or(DurabilityError::InvalidStoredState)?,
            checked(fresh.authorization.initial_commit())?,
        )?;
        if loss.observation.loss_epoch != epoch || decided_at < loss.authorized_at {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some((loss, decided_at)))
    }

    /// The retained history of a resumed session: the restored budget of its current
    /// epoch, that epoch's original grace deadline and the protection its committed
    /// recovery left. `None` unless the session is ACTIVE on the committed recovery's
    /// transport. The owning source reads it to observe a loss after the resume.
    pub async fn resumed_history(
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
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let current = store.current_session_locked(&mut tx, session).await?;
                    let history = store.resumed_history_locked(&mut tx, current).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(history)
                })
            })
            .await
    }

    async fn resumed_history_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<ControlLossHistoryV1>> {
        let (GameSessionState::Active, Some(epoch), Some(grace), Some(transport)) = (
            current.session_state(),
            current.current_control_loss_epoch(),
            current.current_original_grace_deadline(),
            current.current_transport(),
        ) else {
            return Ok(None);
        };
        let session = current.commit().game_session_id();
        let budget = self.recovery_budget_locked(tx, session, epoch).await?;
        let Some(committed) = budget.entries().iter().find(|entry| {
            entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed
                && entry.transport == transport
        }) else {
            return Ok(None);
        };
        let mut key = complete_reconnect_prefix(COMPLETE_COMMIT_KEY, session, epoch.get());
        key.extend_from_slice(&committed.attempt.to_be_bytes());
        let committed_at: i64 = sqlx::query_scalar("SELECT decided_at FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let (loss, _) = self
            .owning_loss_locked(tx, session, epoch)
            .await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        if budget.state() != RecoveryEpochStateV1::Restored
            || loss.observation.original_grace_deadline != grace
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some(ControlLossHistoryV1::Resumed {
            budget,
            original_grace_deadline: grace,
            protection: checked(
                loss.observation
                    .protection
                    .after_complete_reconnect(committed_at),
            )?,
        }))
    }

    /// Withdraw a PREPARED same-session attempt whose COMMIT was refused or could not be
    /// proven, so a later attempt may prepare. The attempt stays in the retained budget as
    /// `Terminal`; its immutable PREPARE receipt and transport reservation remain. A
    /// committed attempt is never withdrawn (the session is no longer RECONNECTABLE).
    pub async fn abort_complete_reconnect(&self, identity: &ReconnectIdentityV1) -> Result<bool> {
        let session = identity.game_session_id();
        let attempt = identity.reconnect_attempt_ref().to_be_bytes();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let cleared = sqlx::query("UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND prepared_attempt_ref = $2")
                        .bind(session.as_bytes().as_slice()).bind(attempt.as_slice())
                        .execute(&mut *tx).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(cleared.rows_affected() == 1)
                })
            })
            .await
    }

    /// Recover the outcome of one complete-reconnect operation from its immutable
    /// receipts: `Committed`, else `Prepared`, else absent. Receipts naming a different
    /// operation for the same attempt are a stored-state conflict.
    pub async fn reconcile_complete_reconnect(
        &self,
        operation: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<Option<CompleteReconnectCompletionV1>> {
        let encoded = encode_complete_reconnect(operation)?;
        let operation = operation.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    use sqlx::Row;
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let epoch = operation.recovery.original.loss.observation.loss_epoch.get();
                    let mut outcome = None;
                    for (phase, commit) in [(COMPLETE_COMMIT_KEY, true), (COMPLETE_PREPARE_KEY, false)] {
                        let key = complete_reconnect_key(phase, &operation.recovery.identity, epoch);
                        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
                            .bind(&key).fetch_optional(&mut *tx).await? else { continue };
                        let stored: Option<String> = row.try_get("operation_json")?;
                        if stored.as_deref() != Some(encoded.as_str()) {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        let decided_at: i64 = row.try_get("decided_at")?;
                        if outcome.is_none() {
                            outcome = Some(if commit {
                                CompleteReconnectOutcomeV1::Committed { decided_at }
                            } else {
                                CompleteReconnectOutcomeV1::Prepared { decided_at }
                            });
                        }
                    }
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(outcome.map(|outcome| CompleteReconnectCompletionV1 { operation, outcome }))
                })
            })
            .await
    }
}

/// The owning reconnect source with its Recovery V2 evidence replaced by the registered
/// floors locked in the deciding transaction.
struct RegisteredRecoverySource<'a> {
    inner: &'a dyn CompleteReconnectSourceV1,
    registered: &'a dyn RecoveryDurabilityEvidenceSourceV2,
}
impl recovery_source_sealed::Sealed for RegisteredRecoverySource<'_> {}
impl CompleteReconnectSourceV1 for RegisteredRecoverySource<'_> {
    fn resolve_reconnect(
        &self,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> std::result::Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
        self.inner.resolve_reconnect(identity, now)
    }
    fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
        Some(self.registered)
    }
}

/// The retained budget as PREPARE leaves it: the historical budget with this attempt
/// added as `Prepared` (the durable reconstruction names no predecessor attempts here).
fn recovery_budget_after_prepare(
    recovery: &CompleteReconnectOperationV1,
) -> Result<RetainedRecoveryBudgetV1> {
    let original = &recovery.original.budget;
    let mut entries = original.entries().to_vec();
    if !entries
        .iter()
        .any(|entry| entry.attempt == recovery.identity.reconnect_attempt_ref())
    {
        entries.push(RetainedRecoveryAttemptV1 {
            attempt: recovery.identity.reconnect_attempt_ref(),
            transport: recovery.original.candidate.transport_ref(),
            disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
        });
    }
    checked(RetainedRecoveryBudgetV1::restore(
        original.epoch(),
        original.state(),
        true,
        entries,
    ))
}

#[cfg(test)]
mod resource_preflight_tests {
    use super::*;
    #[test]
    fn envelope_preflight_checks_exact_boundary_and_overflow() -> Result<()> {
        let exact = envelope_size(49_132, usize::MAX)?;
        assert_eq!(exact, super::super::MAX_FRESH_OPERATION_BYTES);
        assert!(envelope_size(49_133, exact).is_err());
        assert_eq!(envelope_size(49_132, exact)?, exact);
        assert!(envelope_size(49_132, exact - 1).is_err());
        assert!(envelope_size(usize::MAX, usize::MAX).is_err());
        let mut counter = Writer::counter(3);
        counter.bytes(&[1, 2, 3])?;
        assert_eq!(counter.measured(), 3);
        assert_eq!(counter.bytes.capacity(), 0);
        assert!(counter.bytes(&[4]).is_err());
        assert_eq!(counter.measured(), 3);
        assert_eq!(counter.bytes.capacity(), 0);
        Ok(())
    }
    #[test]
    fn runtime_caps_cannot_be_inflated_or_reconfigured() -> Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .block_on(async {
                for invalid in [
                    0,
                    super::super::MAX_FRESH_OPERATION_BYTES - 1,
                    super::super::MAX_FRESH_OPERATION_BYTES + 1,
                    usize::MAX,
                ] {
                    assert!(matches!(
                        FreshAdmissionStore::connect_runtime(
                            "not-a-database-url",
                            invalid,
                            super::super::MAX_ADMISSION_GUARD_BYTES
                        )
                        .await,
                        Err(DurabilityError::InvalidStoredState)
                    ));
                }
                for invalid in [
                    0,
                    super::super::MAX_ADMISSION_GUARD_BYTES - 1,
                    super::super::MAX_ADMISSION_GUARD_BYTES + 1,
                    usize::MAX,
                ] {
                    assert!(matches!(
                        AdmissionGuardStore::connect_runtime("not-a-database-url", invalid).await,
                        Err(DurabilityError::InvalidStoredState)
                    ));
                }
                Ok(())
            })
    }
}
