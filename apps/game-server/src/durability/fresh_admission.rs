//! Strict historical fresh-operation storage. No decoded value is a live capability.
//! Budgets are explicit caller allocations; this module selects no production ceiling.
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
    let mut writer = Writer::new(maximum_bytes);
    write_operation(&mut writer, operation)?;
    // The historical predicate clones guard evidence internally: first establish
    // that the complete retained operation fits the explicit allocation budget.
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    encode_envelope(&writer.bytes, maximum_bytes)
}
pub(super) fn encode_envelope(bytes: &[u8], maximum_bytes: usize) -> Result<String> {
    let groups = bytes
        .len()
        .checked_div(3)
        .and_then(|groups| groups.checked_mul(4));
    let tail = match bytes.len() % 3 {
        0 => 0,
        1 => 2,
        _ => 3,
    };
    let required = groups
        .and_then(|size| size.checked_add(tail))
        .and_then(|size| size.checked_add("{\"version\":1,\"payload\":\"\"}".len()));
    if required.is_none_or(|required| required > maximum_bytes) {
        return Err(DurabilityError::InvalidStoredState);
    }
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

/// Asynchronous storage only. Production bounded scheduling/completion remains
/// a separate adapter requirement; these arguments select no policy defaults.
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
        if maximum_operation_bytes == 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Self {
            guards: AdmissionGuardStore::connect_runtime(url, maximum_guard_bytes).await?,
            maximum_operation_bytes,
        })
    }

    async fn receipt_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        replay: &[u8],
    ) -> Result<Option<FreshAdmissionCommitReceiptV1>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT CASE WHEN octet_length(operation_json) <= $2 THEN operation_json END AS payload, to_jsonb(r) - 'operation_json' AS mirrors FROM game_durability_fresh_admission_receipts r WHERE replay_key = $1")
            .bind(replay).bind(checked(i64::try_from(self.maximum_operation_bytes))?).fetch_optional(&mut **tx).await?;
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
        let mut tx = self.guards.pool.begin().await?;
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

    pub async fn reconcile(
        &self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        use sqlx::Row;
        let mut tx = self.guards.pool.begin().await?;
        super::db::lock_admission_relations(&mut tx).await?;
        let replay = original.authorization.facts.replay_key().to_bytes();
        let Some(receipt) = self.receipt_locked(&mut tx, &replay).await? else {
            return Ok(FreshReconciliation::Absent);
        };
        if receipt.operation() != original {
            return Ok(FreshReconciliation::Conflict);
        }
        let b = receipt.binding();
        let row = sqlx::query("SELECT to_jsonb(s) AS state FROM game_durability_reconnect_sessions s WHERE game_session_id = encode($1,'hex')::uuid").bind(b.candidate_session.as_bytes().as_slice()).fetch_optional(&mut *tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
        let state: serde_json::Value = row.try_get("state")?;
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
                &mut tx,
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
                    && predecessor.as_u64().is_some_and(|value| value > 0) =>
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
        let reservation: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1 AND game_session_id = encode($2,'hex')::uuid AND reservation_owner = 2 AND fresh_replay_key = $3 AND reconnect_attempt_ref IS NULL)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).fetch_one(&mut *tx).await?;
        if !reservation {
            return Err(DurabilityError::InvalidStoredState);
        }
        tx.commit().await?;
        Ok(FreshReconciliation::Committed(Box::new(
            FreshAdmissionDurableReconciliationSnapshotV1 {
                receipt,
                current_session,
            },
        )))
    }
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
