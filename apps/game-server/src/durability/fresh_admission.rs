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

/// Asynchronous storage only. Production bounded scheduling/completion remains
/// a separate adapter requirement; runtime arguments must match fixed registry caps.
#[derive(Clone)]
pub struct FreshAdmissionStore {
    guards: AdmissionGuardStore,
    maximum_operation_bytes: usize,
}
impl FreshAdmissionStore {
    pub async fn connect_runtime(
        url: &str,
        maximum_operation_bytes: usize,
        maximum_guard_bytes: usize,
    ) -> Result<Self> {
        if maximum_operation_bytes != super::MAX_FRESH_OPERATION_BYTES {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Self {
            guards: AdmissionGuardStore::connect_runtime(url, maximum_guard_bytes).await?,
            maximum_operation_bytes,
        })
    }

    pub(super) fn from_backend(backend: std::sync::Arc<super::db::RuntimeBackend>) -> Self {
        Self {
            guards: AdmissionGuardStore::from_backend(backend),
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
        let operation = request.operation();
        let b = &operation.authorization;
        let encoded = encode_operation(operation, self.maximum_operation_bytes)?;
        let encoded_successors: Vec<_> = operation
            .transition
            .successors
            .iter()
            .map(|change| encode_guard(change, self.guards.maximum_guard_bytes))
            .collect::<Result<_>>()?;
        let replay = b.facts.replay_key().to_bytes();
        let mut tx = self.guards.backend.begin().await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(receipt) = self.receipt_locked(&mut tx, &replay).await? {
            let outcome = receipt.classify_retry(operation);
            tx.commit().await?;
            return Ok(outcome);
        }
        let initial = checked(b.initial_commit())?;
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
            current.push(self.guards.load_locked(&mut tx, &expected.key).await?);
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
        if !self
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
        sqlx::query("INSERT INTO game_durability_fresh_admission_receipts (replay_key, game_session_id, account_id, character_id, world_id, channel_id, character_lease_generation, scope_ownership_generation, connection_generation, transport_ref, semantic_version, operation_json, authorization_decided_at) VALUES ($1, encode($2,'hex')::uuid, $3::text::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, $7::text::numeric(20,0), $8::text::numeric(20,0), 1, $9, 1, $10, $11)")
            .bind(replay.as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_durability_reconnect_sessions (game_session_id, account_id, character_id, world_id, runtime_scope_kind, runtime_scope_world_id, runtime_scope_channel_id, character_lease_generation, scope_ownership_generation, current_generation, current_transport_ref, session_state, fresh_replay_key) VALUES (encode($1,'hex')::uuid, $2::text::uuid, encode($3,'hex')::uuid, encode($4,'hex')::uuid, 1, encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6::text::numeric(20,0), $7::text::numeric(20,0), 1, $8, 2, $9)")
            .bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(replay.as_slice()).execute(&mut *tx).await?;
        self.guards
            .persist_locked(&mut tx, successors, &previous, &encoded_successors)
            .await?;
        sqlx::query("INSERT INTO game_durability_transport_ref_reservations (transport_ref, game_session_id, reconnect_attempt_ref, reservation_owner, fresh_replay_key) VALUES ($1, encode($2,'hex')::uuid, NULL, 2, $3)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).execute(&mut *tx).await?;
        let receipt = checked(FreshAdmissionCommitReceiptV1::restore(
            operation.clone(),
            decided_at,
        ))?;
        // An acknowledgement error is uncertain; caller reconciles this original
        // operation rather than assuming rollback or manufacturing another key.
        if tx.commit().await.is_err() {
            return Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable);
        }
        Ok(FreshAdmissionDurableOutcomeV1::Committed(receipt))
    }

    /// Initial supported owning-loss slice. Unsupported continuity shapes remain
    /// unavailable; this API never derives loss authority from reconnect PREPARE.
    pub async fn commit_fresh_loss(
        &self,
        request: &ControlLossRequestV1,
        source: &dyn ControlLossSourceV1,
    ) -> Result<ControlLossOutcomeV1> {
        use sqlx::Row;
        let operation = request.operation();
        let encoded = encode_fresh_loss(operation)?;
        let observation = &operation.observation;
        let session_id = observation.session.commit().game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&observation.loss_epoch.get().to_be_bytes());
        let mut tx = self.guards.backend.begin().await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? {
            let stored: Option<String> = row.try_get("operation_json")?;
            if stored.as_deref() != Some(encoded.as_str()) { return Err(DurabilityError::InvalidStoredState); }
            let decided_at: i64 = row.try_get("decided_at")?;
            if decided_at < operation.authorized_at { return Err(DurabilityError::InvalidStoredState); }
            tx.commit().await?;
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
            self.maximum_operation_bytes,
        )?;
        let FreshReconciliation::Committed(current) =
            self.reconcile_locked(&mut tx, &original).await?
        else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        let expected_claims = &original.transition.successors;
        let mut claims = Vec::with_capacity(2);
        for expected in expected_claims {
            claims.push(self.guards.load_locked(&mut tx, &expected.key).await?);
        }
        if original.authorization.account_id != observation.account_presence.account_id()
            || validate_claim_preserving_session_v1(
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
        let runtime = self
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
        // Strong common relation fencing excludes every sibling semantic writer
        // before this single final decision-time sample.
        let decided_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let Ok(effect) = request.validate_final(source, decided_at) else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        if effect.predecessor() != current.current_session {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        let successor = effect.successor();
        let changed = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 1, current_transport_ref = NULL, control_loss_epoch = $2::text::numeric(20,0), original_grace_deadline = $3, predecessor_generation = current_generation WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 2 AND control_loss_epoch IS NULL AND prepared_attempt_ref IS NULL AND attempt_count = 0")
            .bind(session_id.as_bytes().as_slice()).bind(successor.current_control_loss_epoch().ok_or(DurabilityError::InvalidStoredState)?.get().to_string()).bind(successor.current_original_grace_deadline().ok_or(DurabilityError::InvalidStoredState)?).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        // The complete canonical loss/protection operation is retained below.
        // Do not manufacture a legacy protection row: its connection-generation
        // namespace is not this operation's entitlement/rearm namespace. Legacy
        // prepare/replacement fail closed on this receipt until a typed bridge.
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES ($1,$2,$3)")
            .bind(&key).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        if tx.commit().await.is_err() {
            return Ok(ControlLossOutcomeV1::Ambiguous);
        }
        Ok(ControlLossOutcomeV1::Committed { decided_at })
    }

    /// Read the original loss receipt and current canonical session in one fenced
    /// snapshot. Restoring this report never reconstructs a live loss request.
    pub async fn reconcile_fresh_loss(
        &self,
        original: &ControlLossOperationV1,
    ) -> Result<FreshLossReconciliation> {
        use sqlx::Row;
        let encoded = encode_fresh_loss(original)?;
        let session_id = original.observation.session.commit().game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&original.observation.loss_epoch.get().to_be_bytes());
        let mut tx = self.guards.backend.begin().await?;
        super::db::lock_admission_relations(&mut tx).await?;
        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? else {
                tx.commit().await?;
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
            self.maximum_operation_bytes,
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
            tx.commit().await?;
            return Ok(FreshLossReconciliation::Conflict);
        }
        let FreshReconciliation::Committed(current) =
            self.reconcile_locked(&mut tx, &fresh).await?
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
        tx.commit().await?;
        Ok(FreshLossReconciliation::Committed {
            completion: Box::new(ControlLossCompletionV1 {
                operation,
                outcome: ControlLossOutcomeV1::Committed { decided_at },
            }),
            current,
        })
    }

    pub async fn reconcile(
        &self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        let mut tx = self.guards.backend.begin().await?;
        super::db::lock_admission_relations(&mut tx).await?;
        let result = self.reconcile_locked(&mut tx, original).await?;
        tx.commit().await?;
        Ok(result)
    }

    async fn reconcile_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: &FreshAdmissionOperationV1,
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
        let row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(s)::text) <= $2 THEN to_jsonb(s) END AS state FROM game_durability_reconnect_sessions s WHERE game_session_id = encode($1,'hex')::uuid").bind(b.candidate_session.as_bytes().as_slice()).bind(super::MAX_ADMISSION_ROW_BYTES).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
        let state: Option<serde_json::Value> = row.try_get("state")?;
        let state = state.ok_or(DurabilityError::InvalidStoredState)?;
        let initial = checked(b.initial_commit())?;
        if json_text(&state, "account_id")? != b.account_id
            || json_text(&state, "character_id")? != uuid_text(initial.character_id().as_bytes())
            || json_text(&state, "world_id")? != uuid_text(initial.world_id().as_bytes())
            || json_text(&state, "fresh_replay_key")? != bytea_text(&replay)
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
        let mut current_session = checked(GameSessionAuthoritySnapshot::from_current_facts(
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
// Canonical complete encoding of the deliberately supported initial-loss shape.
// Every omitted alternative is rejected, never silently projected or defaulted.
fn write_fresh_loss(w: &mut Writer, operation: &ControlLossOperationV1) -> Result<()> {
    let o = &operation.observation;
    if operation.version != 1
        || !matches!(o.history, ControlLossHistoryV1::FreshOrigin)
        || o.cause != ControlLossCauseV1::AuthoritativeUnexpectedLoss
    {
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
    w.tag(1)?;
    match o.protection.usage {
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
    match o.protection.rearm {
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
    if r.tag()? != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
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
            history: ControlLossHistoryV1::FreshOrigin,
            protection: RecoveryProtectionContinuityV1 { usage, rearm },
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
