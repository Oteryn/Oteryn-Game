//! Recover an unacknowledged activity handoff through the existing S2 verifier.
//! Discovery and retained placement are data, never authority to attach a controller.
//! A real first command closes this path before its effect. Ordinary reconnect
//! remains the owner of any session with an independently recorded control loss.
use super::super::fresh_admission::{CurrentCapabilityCheck, has_owning_loss_receipt};
use super::super::recovery_evidence_composition::{
    RecoveryEvidenceSubject, decide_with_registered_recovery,
};
use super::*;
use crate::foundation::fnd04_verifier::{
    RecoveryCurrentEvidence, RecoveryDurabilityTrustContextV2, VerifiedRecoveryDurabilityFactsV2,
};
use crate::foundation::{AuthenticatedTransportRefV1, ConnectionGeneration, GameSessionState};

/// An inert, current routing projection. Callers must retain the actual native
/// actor/spell state; the authored destination cell is not permission to reset it.
#[derive(Clone)]
pub struct ActivityTransferRecovery {
    pub handoff_id: [u8; 16],
    pub return_id: Option<[u8; 16]>,
    pub source_session: GameSessionId,
    pub current_session: GameSessionId,
    pub runtime_scope: RuntimeScopeRefV1,
    pub scope_generation: ScopeOwnershipGeneration,
    pub connection_generation: u64,
    pub lease_generation: u64,
    pub account: [u8; 16],
    pub current: RecoveryCurrentEvidence,
    pub selected_capabilities: Vec<u32>,
    pub started: bool,
    pub has_owning_loss: bool,
    pub destination_cell: LastPositionCell,
    pub destination_content: ActivityContentPin,
}

impl ActivityTransferRecovery {
    fn same_handoff(&self, other: &Self) -> bool {
        self.handoff_id == other.handoff_id
            && self.return_id == other.return_id
            && self.source_session == other.source_session
            && self.current_session == other.current_session
            && self.runtime_scope == other.runtime_scope
            && self.scope_generation == other.scope_generation
            && self.lease_generation == other.lease_generation
            && self.account == other.account
            && self.current == other.current
            && self.selected_capabilities == other.selected_capabilities
            && self.destination_cell == other.destination_cell
            && self.destination_content == other.destination_content
    }

    fn fence(&self) -> Result<CurrentCharacterItemFence> {
        Ok(CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(
                *self.current.character_id.as_bytes(),
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?,
            game_session_id: self.current_session,
            connection_generation: ConnectionGeneration::new(self.connection_generation)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            character_lease_generation: self.lease_generation,
            runtime_scope: self.runtime_scope,
            scope_ownership_generation: self.scope_generation,
        })
    }
}

impl ActivityHandoffOwner {
    /// Resolve the independently current descendant of a retained activity
    /// source/destination. This remains readable after first input for domain 99;
    /// it never resets command continuity or grants a transport.
    pub async fn transfer_recovery(
        &self,
        session: GameSessionId,
    ) -> Result<Option<ActivityTransferRecovery>> {
        let root = self.root.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |pass, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(pass, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    let candidate = load(&mut tx, &root, session).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(candidate)
                })
            })
            .await
    }

    /// Journal first input while holding the connection's existing input gate,
    /// before dispatching a command. The SQL lock also serializes fresh transport
    /// replacement, so an old connection cannot win a native effect afterward.
    /// Non-activity sessions are deliberately a no-op, as with House transfers.
    pub async fn note_transfer_input(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<()> {
        let root = self.root.clone();
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |pass, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(pass, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let activity: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND (activity_handoff_id IS NOT NULL OR activity_return_id IS NOT NULL))")
                .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if activity {
                sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let current = FreshAdmissionStore::from_root(root.clone())
                    .current_session_in_transaction(&mut tx, fence.game_session_id).await?;
                if !prove_current_incarnation(&mut tx, &node).await?
                    || !assert_runtime_scope_fence(&mut tx, fence.runtime_scope, fence.scope_ownership_generation, &node).await?
                    || assert_item_only_fence(&mut tx, &fence, &node).await?.is_err()
                    || current.session_state() != GameSessionState::Active
                    || current.current_transport() != Some(transport)
                    || current.current_connection_generation() != fence.connection_generation
                    || current.current_character_lease().character_id().as_bytes() != fence.character_id.as_bytes()
                    || current.current_character_lease().generation() != fence.character_lease_generation
                    || current.current_runtime_scope() != fence.runtime_scope
                    || current.current_scope_generation() != fence.scope_ownership_generation {
                    return Err(DurabilityError::Unavailable);
                }
                sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES($1,'{}',floor(extract(epoch FROM clock_timestamp()))::bigint) ON CONFLICT DO NOTHING")
                    .bind(input_key(fence.game_session_id)).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// Adopt only an unused committed destination through current registered S2
    /// evidence, exact current claims/assignment and the original selection.
    /// The same candidate, nonce and transport can be retried after an uncertain
    /// commit: its immutable recovery receipt returns the same generation.
    pub async fn recover_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: ActivityTransferRecovery,
        verified: VerifiedRecoveryDurabilityFactsV2,
        transport: AuthenticatedTransportRefV1,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<ActivityTransferRecovery> {
        if candidate.started || candidate.has_owning_loss {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |pass, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(pass, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? { return Err(DurabilityError::Unavailable); }
            sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(candidate.current_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(candidate.current.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let mut current = load(&mut tx, &root, candidate.current_session).await?
                .ok_or(DurabilityError::Unavailable)?;
            if current.started || current.has_owning_loss || !candidate.same_handoff(&current) {
                return Err(DurabilityError::Unavailable);
            }
            let audit = verified.audit();
            let subject = RecoveryEvidenceSubject::new(&audit.account_id, &audit.signing.key_id)?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx).await?;
            decide_with_registered_recovery(&mut tx, &node, &subject, |source| {
                verified.revalidate(now, &RecoveryDurabilityTrustContextV2::from_owning_source(source), &current.current)
            }).await?.map_err(|_| DurabilityError::Unavailable)?;
            let store = FreshAdmissionStore::from_root(root.clone());
            if !assert_runtime_scope_fence(&mut tx, current.runtime_scope, current.scope_generation, &node).await?
                || !store.check_current_runtime_capabilities(&mut tx, current.runtime_scope, current.scope_generation.get(), &capabilities).await?
                || assert_item_only_fence(&mut tx, &current.fence()?, &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            // Death/arrival and actual later command continuity keep their own
            // placement/reconnect owners. A lost acknowledgement cannot consume them.
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
                .bind(current.current.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if pending { return Err(DurabilityError::Unavailable); }
            let snapshot = store.current_session_in_transaction(&mut tx, current.current_session).await?;
            let row = sqlx::query("SELECT attempt_count,prepared_attempt_ref,predecessor_generation::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(current.current_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if row.try_get::<i16, _>("attempt_count")? != 0 || row.try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?.is_some() {
                return Err(DurabilityError::Unavailable);
            }
            let prior = sqlx::query("SELECT uuid_send(game_session_id) AS session,transport_ref,connection_generation::text FROM game_activity_transfer_recoveries WHERE recovery_grant_nonce=$1 FOR SHARE")
                .bind(audit.grant_nonce.as_slice()).fetch_optional(&mut *tx).await?;
            if let Some(prior) = prior {
                if bytes16(prior.try_get("session")?)? != *current.current_session.as_bytes()
                    || prior.try_get::<Vec<u8>, _>("transport_ref")? != transport.to_bytes()
                    || parse_u64(prior.try_get("connection_generation")?)? != current.connection_generation
                    || snapshot.current_transport() != Some(transport)
                    || snapshot.session_state() != GameSessionState::Active {
                    return Err(DurabilityError::Unavailable);
                }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(current);
            }
            if candidate.connection_generation != current.connection_generation || snapshot.current_transport() == Some(transport) {
                return Err(DurabilityError::Unavailable);
            }
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_transfer_recoveries WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(current.current_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            // Reuse the existing recovery attempt ceiling. This separate ledger
            // never spends/resets the ordinary control-loss epoch's budget.
            if count >= i64::from(super::super::admission_journal::MAX_ATTEMPTS_PER_EPOCH)
                || u64::try_from(count).ok().and_then(|v| v.checked_add(1)) != Some(current.connection_generation) {
                return Err(DurabilityError::Unavailable);
            }
            match snapshot.session_state() {
                GameSessionState::Active if snapshot.current_control_loss_epoch().is_none()
                    && snapshot.current_original_grace_deadline().is_none()
                    && row.try_get::<Option<String>, _>("predecessor_generation")?.is_none() => {}
                GameSessionState::Reconnectable if count == 0 && snapshot.current_transport().is_none()
                    && snapshot.current_control_loss_epoch().is_some_and(|epoch| epoch.get() == 1)
                    && snapshot.current_original_grace_deadline().is_some_and(|deadline| deadline > now)
                    && row.try_get::<Option<String>, _>("predecessor_generation")?.as_deref() == Some("1") => {}
                _ => return Err(DurabilityError::Unavailable),
            }
            let next = current.connection_generation.checked_add(1).ok_or(DurabilityError::Unavailable)?;
            // A server-issued transport and verified grant determine this private
            // attempt identity. Explicit collision checks cover both owner journals.
            let mut hash = Sha256::new();
            hash_field(&mut hash, b"activity-transfer-recovery-attempt-v1");
            hash_field(&mut hash, &transport.to_bytes());
            hash_field(&mut hash, &audit.grant_nonce);
            let digest: [u8; 32] = hash.finalize().into();
            let attempt: [u8; 8] = digest[..8].try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
            if attempt == [0; 8] { return Err(DurabilityError::Unavailable); }
            let collision: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_attempts WHERE game_session_id=encode($1,'hex')::uuid AND reconnect_attempt_ref=$2) OR EXISTS(SELECT 1 FROM game_activity_transfer_recoveries WHERE game_session_id=encode($1,'hex')::uuid AND reconnect_attempt_ref=$2)")
                .bind(current.current_session.as_bytes().as_slice()).bind(attempt.as_slice()).fetch_one(&mut *tx).await?;
            if collision { return Err(DurabilityError::Unavailable); }
            let reserved = sqlx::query("INSERT INTO game_durability_transport_ref_reservations(transport_ref,game_session_id,reconnect_attempt_ref,reservation_owner) VALUES($1,encode($2,'hex')::uuid,$3,1) ON CONFLICT DO NOTHING")
                .bind(transport.to_bytes().as_slice()).bind(current.current_session.as_bytes().as_slice()).bind(attempt.as_slice()).execute(&mut *tx).await?;
            let consumed = sqlx::query("INSERT INTO game_durability_recovery_grant_consumptions(recovery_grant_nonce,game_session_id,reconnect_attempt_ref) VALUES($1,encode($2,'hex')::uuid,$3) ON CONFLICT DO NOTHING")
                .bind(audit.grant_nonce.as_slice()).bind(current.current_session.as_bytes().as_slice()).bind(attempt.as_slice()).execute(&mut *tx).await?;
            if reserved.rows_affected() != 1 || consumed.rows_affected() != 1 { return Err(DurabilityError::Unavailable); }
            let fact = node.fact();
            sqlx::query("INSERT INTO game_activity_transfer_recoveries(recovery_grant_nonce,game_session_id,handoff_id,return_id,recovery_ordinal,reconnect_attempt_ref,predecessor_generation,connection_generation,predecessor_transport_ref,transport_ref,predecessor_session_state,retained_grace_deadline,node_id,node_registration_revision,decided_at) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7::text::numeric,$8::text::numeric,$9,$10,$11,$12,encode($13,'hex')::uuid,$14::text::numeric,$15)")
                .bind(audit.grant_nonce.as_slice()).bind(current.current_session.as_bytes().as_slice()).bind(current.handoff_id.as_slice())
                .bind(current.return_id.as_ref().map(|id| id.as_slice())).bind(i16::try_from(count + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(attempt.as_slice()).bind(current.connection_generation.to_string()).bind(next.to_string())
                .bind(snapshot.current_transport().map(|t| t.to_bytes().to_vec())).bind(transport.to_bytes().as_slice())
                .bind(if snapshot.session_state() == GameSessionState::Active { 2_i16 } else { 1_i16 })
                .bind(snapshot.current_original_grace_deadline()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(now).execute(&mut *tx).await?;
            // An offline successor inherited an acknowledgement deadline from its
            // predecessor, not an owning-loss receipt for this new session. Its
            // explicit recovery receipt now closes that awaiting-controller state.
            // A later real loss starts through the ordinary loss owner.
            let changed = sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=$2::text::numeric,current_transport_ref=$3,session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL WHERE game_session_id=encode($1,'hex')::uuid AND current_generation=$4::text::numeric AND prepared_attempt_ref IS NULL AND attempt_count=0 AND session_state IN(1,2)")
                .bind(current.current_session.as_bytes().as_slice()).bind(next.to_string()).bind(transport.to_bytes().as_slice())
                .bind(current.connection_generation.to_string()).execute(&mut *tx).await?;
            if changed.rows_affected() != 1 { return Err(DurabilityError::Unavailable); }
            current.connection_generation = next;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(current)
        })).await
    }
}

fn input_key(session: GameSessionId) -> Vec<u8> {
    let mut key = b"activity-transfer-input-v1".to_vec();
    key.extend_from_slice(session.as_bytes());
    key
}

async fn load(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    requested: GameSessionId,
) -> Result<Option<ActivityTransferRecovery>> {
    let source = sqlx::query("SELECT uuid_send(s.character_id) AS character,COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) AS initial_fresh FROM game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid AND (s.activity_handoff_id IS NOT NULL OR s.activity_return_id IS NOT NULL OR EXISTS(SELECT 1 FROM game_activity_handoff_participants p JOIN game_activity_handoff_groups g USING(handoff_id) WHERE p.source_game_session_id=s.game_session_id AND g.state=2))")
        .bind(requested.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(source) = source else {
        return Ok(None);
    };
    let character = CharacterId::decode(&bytes16(source.try_get("character")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let initial: Vec<u8> = source
        .try_get::<Option<Vec<u8>>, _>("initial_fresh")?
        .ok_or(DurabilityError::Unavailable)?;
    let guards = AdmissionGuardStore::from_root(root.clone());
    let guard = guards
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Character(character))
        .await?;
    let Some(AdmissionAuthorityGuardStateV1::Character {
        account_id,
        world_id,
        eligible: true,
        lease_generation,
        holder: Some(session),
    }) = guard.map(|row| row.state)
    else {
        return Err(DurabilityError::Unavailable);
    };
    let account = guards
        .load_locked(
            tx,
            &AdmissionAuthorityGuardKeyV1::Account {
                account_id: account_id.clone(),
            },
        )
        .await?;
    if !matches!(account.map(|row| row.state), Some(AdmissionAuthorityGuardStateV1::Account { security, presence: Some((held_character, holder)) })
        if security.allowed && held_character == character && holder == session)
    {
        return Err(DurabilityError::Unavailable);
    }
    if !unacknowledged_lineage(tx, root, character, requested, session).await? {
        return Err(DurabilityError::Unavailable);
    }
    let row = sqlx::query(r#"
        SELECT uuid_send(g.handoff_id) AS handoff,uuid_send(r.return_id) AS return_id,
          uuid_send(CASE WHEN r.return_id IS NULL THEN p.source_game_session_id ELSE r.source_game_session_id END) AS source,
          uuid_send(p.account_id) AS account,uuid_send(g.instance_id) AS instance,uuid_send(p.origin_channel_id) AS origin,
          CASE WHEN r.return_id IS NULL THEN g.ownership_generation ELSE r.destination_ownership_generation END::text AS generation,
          CASE WHEN r.return_id IS NULL THEN p.source_lease_generation ELSE r.source_lease_generation END::text AS source_lease,
          CASE WHEN r.return_id IS NULL THEN q.destination_x ELSE r.return_x END AS x,
          CASE WHEN r.return_id IS NULL THEN q.destination_y ELSE r.return_y END AS y,
          CASE WHEN r.return_id IS NULL THEN q.destination_floor ELSE r.return_floor END AS floor,
          CASE WHEN r.return_id IS NULL THEN e.destination_coordinate_frame_digest ELSE r.return_coordinate_frame_digest END AS frame_digest,
          CASE WHEN r.return_id IS NULL THEN e.destination_map_revision_digest ELSE r.return_map_revision_digest END AS map_digest,
          CASE WHEN r.return_id IS NULL THEN e.destination_server_artifact_digest ELSE r.return_server_artifact_digest END AS server_digest,
          q.selected_capabilities
        FROM game_activity_handoff_participants p JOIN game_activity_handoff_groups g USING(handoff_id)
          JOIN game_activity_runtime_entries e USING(handoff_id) JOIN game_activity_runtime_placements q USING(handoff_id,character_id)
          JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($1,'hex')::uuid
          LEFT JOIN game_activity_runtime_returns r ON r.handoff_id=p.handoff_id AND r.character_id=p.character_id AND r.destination_game_session_id=s.game_session_id
        WHERE p.character_id=encode($2,'hex')::uuid AND g.state=2 AND p.account_id=s.account_id
          AND (p.destination_game_session_id=s.game_session_id OR r.destination_game_session_id=s.game_session_id)
          AND s.initial_fresh_replay_key=$3 AND s.session_state IN(1,2)
        LIMIT 2
    "#).bind(session.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(initial)
        .fetch_all(&mut **tx).await?;
    let [row] = row.as_slice() else {
        return Err(DurabilityError::Unavailable);
    };
    let return_id = row
        .try_get::<Option<Vec<u8>>, _>("return_id")?
        .map(bytes16)
        .transpose()?;
    let scope = if return_id.is_some() {
        RuntimeScopeRefV1::channel(
            world_id,
            crate::foundation::ChannelId::decode(&bytes16(row.try_get("origin")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    } else {
        RuntimeScopeRefV1::instance(world_id, bytes16(row.try_get("instance")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?
    };
    let generation = ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let expected_lease = parse_u64(row.try_get("source_lease")?)?
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let store = FreshAdmissionStore::from_root(root.clone());
    if !store
        .committed_activity_origin_in_transaction(tx, character, session)
        .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let snapshot = store.current_session_in_transaction(tx, session).await?;
    if snapshot.current_character_lease().character_id() != character
        || lease_generation != expected_lease
        || snapshot.current_character_lease().generation() != expected_lease
        || snapshot.current_runtime_scope() != scope
        || snapshot.current_scope_generation() != generation
    {
        return Err(DurabilityError::Unavailable);
    }
    let runtime = guards
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
        .await?;
    let Some(AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready: true,
        ruleset_revision,
        content_revision,
        map_revision,
        world_policy_revision,
        ..
    }) = runtime.map(|row| row.state)
    else {
        return Err(DurabilityError::Unavailable);
    };
    if ownership_generation != generation.get() {
        return Err(DurabilityError::Unavailable);
    }
    let selected_capabilities = row
        .try_get::<Vec<i64>, _>("selected_capabilities")?
        .into_iter()
        .map(|id| u32::try_from(id).map_err(|_| DurabilityError::InvalidStoredState))
        .collect::<Result<Vec<_>>>()?;
    if !entry::valid_selection(&selected_capabilities) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let started: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1)")
        .bind(input_key(session)).fetch_one(&mut **tx).await?;
    let has_owning_loss = match snapshot.current_control_loss_epoch() {
        Some(epoch) => has_owning_loss_receipt(tx, session.as_bytes(), epoch.get()).await?,
        None => false,
    };
    Ok(Some(ActivityTransferRecovery {
        handoff_id: bytes16(row.try_get("handoff")?)?,
        return_id,
        source_session: GameSessionId::decode(&bytes16(row.try_get("source")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        current_session: session,
        runtime_scope: scope,
        scope_generation: generation,
        connection_generation: snapshot.current_connection_generation().get(),
        lease_generation: expected_lease,
        account: bytes16(row.try_get("account")?)?,
        current: RecoveryCurrentEvidence {
            account_id,
            character_id: character,
            world_id,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
        },
        selected_capabilities,
        started,
        has_owning_loss,
        destination_cell: LastPositionCell {
            x: row.try_get("x")?,
            y: row.try_get("y")?,
            floor: row.try_get("floor")?,
        },
        destination_content: ActivityContentPin {
            coordinate_frame_digest: bytes32(row.try_get("frame_digest")?)?,
            map_revision_digest: bytes32(row.try_get("map_digest")?)?,
            server_artifact_digest: bytes32(row.try_get("server_digest")?)?,
        },
    }))
}

/// Before input, the Channel -> Instance -> origin Channel owner can have only
/// an entry and its server return. A further entry requires real input and may
/// not be skipped by replaying an older session's acknowledgement.
async fn unacknowledged_lineage(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    character: CharacterId,
    requested: GameSessionId,
    current: GameSessionId,
) -> Result<bool> {
    if requested == current {
        return Ok(true);
    }
    let store = FreshAdmissionStore::from_root(root.clone());
    let mut cursor = requested;
    for _ in 0..2 {
        let Some(next) = store
            .activity_successor_in_transaction(tx, character, cursor)
            .await?
        else {
            return Ok(false);
        };
        if next == current {
            return Ok(true);
        }
        let unused: bool = sqlx::query_scalar("SELECT current_generation=1 AND NOT EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$2) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(next.as_bytes().as_slice()).bind(input_key(next)).fetch_optional(&mut **tx).await?.unwrap_or(false);
        if !unused {
            return Ok(false);
        }
        cursor = next;
    }
    Ok(false)
}
