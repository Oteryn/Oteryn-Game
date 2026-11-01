//! Authenticate and adopt only the independently current committed destination.
use super::super::admission_authority_guards::AdmissionGuardStore;
use super::super::recovery_evidence_composition::{
    RecoveryEvidenceSubject, decide_with_registered_recovery,
};
use super::*;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::fnd04_verifier::{
    RecoveryCurrentEvidence, RecoveryDurabilityTrustContextV2, VerifiedRecoveryDurabilityFactsV2,
};

#[derive(Clone)]
pub struct HouseTransferRecovery {
    pub ack: HouseTransferAck,
    pub command: u64,
    pub current: RecoveryCurrentEvidence,
    pub source: GameSessionId,
    pub account: [u8; 16],
    /// A client command proves it crossed the scope barrier. Later recovery must
    /// use ordinary retained command continuity, never reset this namespace.
    pub started: bool,
    pub placement: super::super::character_last_position::StoredLastPosition,
    pub house_position: Option<super::super::character_last_position::HouseLastPosition>,
    /// True only for the closed native fresh-admission restore source (command 0).
    pub admission_restore: bool,
}

impl DurabilityRoot {
    /// Routing evidence only; no controller is installed by this read.
    pub async fn house_transfer_recovery(
        &self,
        session: GameSessionId,
    ) -> Result<Option<HouseTransferRecovery>, DurabilityError> {
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |pass, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(pass, deadline).await?;
                    let candidate = load(&mut tx, &root, session).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(candidate)
                })
            })
            .await
    }

    /// The first destination command acknowledges the scope barrier implicitly.
    /// Journal it before any effect, so restart recovery cannot reset used command
    /// IDs. Also refuse an old connection after transport replacement.
    pub async fn note_house_transfer_input(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<(), DurabilityError> {
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |pass, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(pass, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                if !prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let transferred: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_transfer_receipts WHERE destination_game_session_id=encode($1,'hex')::uuid AND state=2)")
                    .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if transferred {
                    let current = FreshAdmissionStore::from_root(root)
                        .current_session_in_transaction(&mut tx, fence.game_session_id).await?;
                    if current.session_state() != crate::foundation::GameSessionState::Active
                        || current.current_transport() != Some(transport)
                        || current.current_connection_generation() != fence.connection_generation
                        || current.current_character_lease().character_id().as_bytes() != fence.character_id.as_bytes()
                        || current.current_character_lease().generation() != fence.character_lease_generation
                        || current.current_runtime_scope() != fence.runtime_scope
                        || current.current_scope_generation() != fence.scope_ownership_generation {
                        return Err(DurabilityError::Unavailable);
                    }
                    let mut key = b"house-transfer-input-v1".to_vec();
                    key.extend_from_slice(fence.game_session_id.as_bytes());
                    sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES($1,'{}',floor(extract(epoch FROM clock_timestamp()))::bigint) ON CONFLICT DO NOTHING")
                        .bind(key).execute(&mut *tx).await?;
                }
                commit_semantic_transaction(tx, deadline).await
            })
        }).await
    }

    /// Reverify current S2 floors and every destination fence under the deciding locks.
    pub async fn recover_house_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: HouseTransferRecovery,
        verified: VerifiedRecoveryDurabilityFactsV2,
        transport: AuthenticatedTransportRefV1,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<HouseTransferRecovery, DurabilityError> {
        if candidate.started {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |pass, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(pass, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let session = GameSessionId::decode(&candidate.ack.session)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let mut current = load(&mut tx, &root, session).await?
                        .ok_or(DurabilityError::Unavailable)?;
                    if current.started || current.ack != candidate.ack || current.current != candidate.current {
                        return Err(DurabilityError::Unavailable);
                    }
                    let audit = verified.audit();
                    let subject = RecoveryEvidenceSubject::new(&audit.account_id, &audit.signing.key_id)?;
                    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                        .fetch_one(&mut *tx).await?;
                    decide_with_registered_recovery(&mut tx, &node, &subject, |source| {
                        verified.revalidate(now, &RecoveryDurabilityTrustContextV2::from_owning_source(source), &current.current)
                    }).await?.map_err(|_| DurabilityError::Unavailable)?;
                    let world = crate::foundation::WorldId::decode(&current.ack.world)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let channel = ChannelId::decode(&current.ack.channel)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let (scope, key) = if let Some(key) = &current.ack.house {
                        let house = HouseId::new(world, key).ok_or(DurabilityError::InvalidStoredState)?;
                        let access = sqlx::query("SELECT role,content_fenced FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) LIMIT 2")
                            .bind(world.as_bytes().as_slice()).bind(key).bind(audit.character_id.as_bytes().as_slice())
                            .fetch_all(&mut *tx).await?;
                        if !matches!(access.as_slice(), [row] if row.try_get::<Option<String>,_>("role")?.as_deref().is_some_and(|r| ["OWNER","SUBOWNER","GUEST"].contains(&r)) && row.try_get::<Option<bool>,_>("content_fenced")? == Some(false)) {
                            return Err(DurabilityError::Unavailable);
                        }
                        (house.runtime_scope()?, house.scope_key())
                    } else {
                        let mut key = vec![1];
                        key.extend_from_slice(world.as_bytes());
                        key.extend_from_slice(channel.as_bytes());
                        (RuntimeScopeRefV1::channel(world, channel), key)
                    };
                    let fact = node.fact();
                    let valid: bool = sqlx::query_scalar("SELECT state=1 AND ownership_generation=$2::text::numeric AND holder_node_id=encode($3,'hex')::uuid AND holder_registration_revision=$4::text::numeric FROM game_runtime_scope_assignments WHERE scope_key=$1 FOR SHARE")
                        .bind(key).bind(current.ack.scope_generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                        .fetch_optional(&mut *tx).await?.unwrap_or(false);
                    let store = FreshAdmissionStore::from_root(root.clone());
                    if !valid || current.ack.house.is_none() && !store.check_current_runtime_capabilities(&mut tx, scope, current.ack.scope_generation, &capabilities).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let snapshot = store.current_session_in_transaction(&mut tx, session).await?;
                    if snapshot.current_runtime_scope() != scope
                        || snapshot.commit().character_id() != audit.character_id
                        || snapshot.current_character_lease().generation() != current.ack.lease_generation
                        || snapshot.current_scope_generation().get() != current.ack.scope_generation
                        || !matches!(snapshot.session_state(), crate::foundation::GameSessionState::Active | crate::foundation::GameSessionState::Reconnectable)
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    // A pending death/arrival or a newer checkpoint belongs to
                    // ordinary placement recovery, not this committed transfer.
                    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
                        .bind(audit.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    if pending || current.placement != candidate.placement
                        || current.house_position != candidate.house_position
                        || current.placement.key.character_lease_generation != current.ack.lease_generation
                        || current.placement.key.write_sequence == 0 {
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = sqlx::query("SELECT attempt_count FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND account_id=$2::uuid AND prepared_attempt_ref IS NULL FOR UPDATE")
                        .bind(session.as_bytes().as_slice()).bind(&audit.account_id).fetch_one(&mut *tx).await?;
                    let guards = AdmissionGuardStore::from_root(root.clone());
                    let account = guards.load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Account { account_id: audit.account_id.clone() }).await?;
                    let character = guards.load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Character(audit.character_id)).await?;
                    let claims = matches!(account.map(|r| r.state), Some(AdmissionAuthorityGuardStateV1::Account { security, presence: Some((character, holder)) }) if security.allowed && character == audit.character_id && holder == session)
                        && matches!(character.map(|r| r.state), Some(AdmissionAuthorityGuardStateV1::Character { account_id, world_id, eligible: true, lease_generation, holder: Some(holder) }) if account_id == audit.account_id && world_id == world && lease_generation == current.ack.lease_generation && holder == session);
                    if !claims { return Err(DurabilityError::Unavailable); }
                    let generation = snapshot.current_connection_generation().get();
                    let mut replay_key = b"house-transfer-recovery-v1".to_vec();
                    replay_key.extend_from_slice(&audit.grant_nonce);
                    let operation = |generation: u64| serde_json::json!({
                        "transfer": current.ack.transfer, "session": current.ack.session,
                        "transport": transport.to_bytes(), "generation": generation.to_string()
                    }).to_string();
                    let prior: Option<Option<String>> = sqlx::query_scalar("SELECT CASE WHEN octet_length(operation_json)<=2048 THEN operation_json END FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1 FOR SHARE")
                        .bind(&replay_key).fetch_optional(&mut *tx).await?;
                    if let Some(prior) = prior {
                        if prior.as_deref() != Some(operation(generation).as_str()) || snapshot.current_transport() != Some(transport) {
                            return Err(DurabilityError::Unavailable);
                        }
                        current.ack.connection_generation = generation;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(current);
                    }
                    // A nonce and a transport each name one recovery, including across
                    // process loss. They cannot be reused to acquire a later generation.
                    let attempts: i16 = row.try_get("attempt_count")?;
                    if attempts >= 8 || snapshot.current_transport() == Some(transport) {
                        return Err(DurabilityError::Unavailable);
                    }
                    let next = generation.checked_add(1).ok_or(DurabilityError::Unavailable)?;
                    let attempt = u64::try_from(attempts + 1).map_err(|_| DurabilityError::InvalidStoredState)?.to_be_bytes();
                    let reserved = sqlx::query("INSERT INTO game_durability_transport_ref_reservations(transport_ref,game_session_id,reconnect_attempt_ref,reservation_owner) VALUES($1,encode($2,'hex')::uuid,$3,1) ON CONFLICT DO NOTHING")
                        .bind(transport.to_bytes().as_slice()).bind(session.as_bytes().as_slice()).bind(attempt.as_slice()).execute(&mut *tx).await?;
                    let consumed = sqlx::query("INSERT INTO game_durability_recovery_grant_consumptions(recovery_grant_nonce,game_session_id,reconnect_attempt_ref) VALUES($1,encode($2,'hex')::uuid,$3) ON CONFLICT DO NOTHING")
                        .bind(audit.grant_nonce.as_slice()).bind(session.as_bytes().as_slice()).bind(attempt.as_slice()).execute(&mut *tx).await?;
                    if reserved.rows_affected() != 1 || consumed.rows_affected() != 1 {
                        return Err(DurabilityError::Unavailable);
                    }
                    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=$2::text::numeric,predecessor_generation=$3::text::numeric,current_transport_ref=$4,session_state=2,attempt_count=attempt_count+1 WHERE game_session_id=encode($1,'hex')::uuid")
                        .bind(session.as_bytes().as_slice()).bind(next.to_string()).bind(generation.to_string()).bind(transport.to_bytes().as_slice()).execute(&mut *tx).await?;
                    sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES($1,$2,$3)")
                        .bind(replay_key).bind(operation(next)).bind(now).execute(&mut *tx).await?;
                    current.ack.connection_generation = next;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(current)
                })
            }).await
    }
}

async fn load(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    session: GameSessionId,
) -> Result<Option<HouseTransferRecovery>, DurabilityError> {
    let Some(row)=sqlx::query("SELECT uuid_send(transfer_id) AS transfer,uuid_send(source_game_session_id) AS source,uuid_send(destination_game_session_id) AS session,uuid_send(world_id) AS world,uuid_send(origin_channel_id) AS channel,house_key,direction,destination_scope_generation::text AS generation,source_character_lease_generation::text AS lease,selection,tile_x,tile_y,tile_floor,command_id::text AS command,admission_restore,account_id::text AS account,uuid_send(account_id) AS account_bytes,uuid_send(character_id) AS character FROM game_house_transfer_receipts WHERE state=2 AND destination_game_session_id=(SELECT c.holder_game_session_id FROM game_durability_admission_character_guards c WHERE c.character_id=(SELECT character_id FROM game_house_transfer_receipts WHERE (source_game_session_id=encode($1,'hex')::uuid OR destination_game_session_id=encode($1,'hex')::uuid) AND state=2 LIMIT 1)) LIMIT 1")
        .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else {
            let historical: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_transfer_receipts WHERE source_game_session_id=encode($1,'hex')::uuid OR destination_game_session_id=encode($1,'hex')::uuid)")
                .bind(session.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            return if historical { Err(DurabilityError::Unavailable) } else { Ok(None) };
        };
    let id = |name| -> Result<[u8; 16], DurabilityError> {
        row.try_get::<Vec<u8>, _>(name)?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let world = crate::foundation::WorldId::decode(&id("world")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let channel =
        ChannelId::decode(&id("channel")?).map_err(|_| DurabilityError::InvalidStoredState)?;
    let character = crate::foundation::CharacterId::decode(&id("character")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let guard = AdmissionGuardStore::from_root(root.clone())
        .load_locked(
            tx,
            &AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(world, channel)),
        )
        .await?;
    let Some(AdmissionAuthorityGuardStateV1::Runtime {
        ruleset_revision,
        content_revision,
        map_revision,
        world_policy_revision,
        ready: true,
        ..
    }) = guard.map(|r| r.state)
    else {
        return Err(DurabilityError::Unavailable);
    };
    let number = |name| -> Result<u64, DurabilityError> {
        row.try_get::<String, _>(name)?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let mut started_key = b"house-transfer-input-v1".to_vec();
    started_key.extend_from_slice(&id("session")?);
    let started: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1)")
        .bind(started_key).fetch_one(&mut **tx).await?;
    let placement = super::super::character_last_position::load_scoped_last_position(
        tx,
        crate::domain::CharacterId::from_bytes(*character.as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    Ok(Some(HouseTransferRecovery {
        started,
        placement: placement.position,
        house_position: placement.house,
        admission_restore: row.try_get("admission_restore")?,
        ack: HouseTransferAck {
            transfer: id("transfer")?,
            session: id("session")?,
            world: *world.as_bytes(),
            channel: *channel.as_bytes(),
            house: if row.try_get::<i16, _>("direction")? == 1 {
                Some(row.try_get("house_key")?)
            } else {
                None
            },
            scope_generation: number("generation")?,
            connection_generation: 1,
            lease_generation: number("lease")?
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?,
            selection: row
                .try_get::<Vec<i32>, _>("selection")?
                .into_iter()
                .map(|v| u32::try_from(v).map_err(|_| DurabilityError::InvalidStoredState))
                .collect::<Result<_, _>>()?,
            position: [
                row.try_get("tile_x")?,
                row.try_get("tile_y")?,
                i32::from(row.try_get::<i16, _>("tile_floor")?),
            ],
        },
        account: id("account_bytes")?,
        command: number("command")?,
        source: GameSessionId::decode(&id("source")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        current: RecoveryCurrentEvidence {
            account_id: row.try_get("account")?,
            character_id: character,
            world_id: world,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
        },
    }))
}
