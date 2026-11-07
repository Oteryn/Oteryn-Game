//! HOUSE-TRANSFER-1: atomic native handoff / reverse handoff and durable scope ack.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    HouseLastPosition, LastPositionKey, LastPositionRecord, LastPositionWrite, ScopedLastPosition,
    upsert_scoped_last_position,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::fresh_admission::{
    CurrentCapabilityCheck, FreshAdmissionStore, commit_session_use, unused_session_revision,
};
use super::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection, HouseId,
    HousePrepareOutcome, commit_house_entry_in_transaction, prepare_house_entry_in_transaction,
    release_house_entry_tile_in_transaction,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{AuthenticatedTransportRefV1, ChannelId, GameSessionId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::house_transfer::{HouseTransferAck, encode_house_transfer_ack};
use sqlx::{Postgres, Row, Transaction};
mod admission;
mod recovery;
pub use admission::HouseAdmissionRead;
pub use recovery::{HouseTransferRecovery, PendingHouseDeathPlacement};

/// Native destination facts selected while the source actor is frozen. All
/// durable bindings are independently checked again in the commit transaction.
#[derive(Debug, Clone)]
pub struct HouseTransferRequest {
    pub fence: CurrentCharacterItemFence,
    pub transport: AuthenticatedTransportRefV1,
    pub command: u64,
    pub house: HouseId,
    pub origin: ChannelId,
    pub ack: HouseTransferAck,
    pub last_position: LastPositionRecord,
}

#[track_caller]
fn unavailable_house_transfer() -> DurabilityError {
    #[cfg(any(test, feature = "test-support"))]
    eprintln!(
        "SRV7_HOUSE_TRANSFER_DURABILITY_UNAVAILABLE at={}",
        std::panic::Location::caller()
    );
    DurabilityError::Unavailable
}

fn trace_house_transfer_stage(stage: &str, error: DurabilityError) -> DurabilityError {
    #[cfg(any(test, feature = "test-support"))]
    eprintln!("SRV7_HOUSE_TRANSFER_STAGE_REFUSED stage={stage} error={error:?}");
    #[cfg(not(any(test, feature = "test-support")))]
    let _ = stage;
    error
}

impl DurabilityRoot {
    /// PREPARE and COMMIT share the source freeze and one transaction. Exit has
    /// its own direction in 0215 and never relaxes 0074's direction CHECK.
    pub async fn commit_house_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: HouseTransferRequest,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<(HouseTransferAck, Vec<u8>), DurabilityError> {
        self.commit_house_transfer_inner(authority, node, request, capabilities, None)
            .await
    }
    /// Transfer a genuinely settled House death while retaining its placement obligation.
    /// The pending respawn is consumed only by the existing destination placement owner.
    pub async fn commit_house_death_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: HouseTransferRequest,
        capabilities: CurrentCapabilityCheck,
        occurrence: super::character_death::PlayerDeathOccurrence,
    ) -> Result<(HouseTransferAck, Vec<u8>), DurabilityError> {
        self.commit_house_transfer_inner(authority, node, request, capabilities, Some(occurrence))
            .await
    }
    async fn commit_house_transfer_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut request: HouseTransferRequest,
        capabilities: CurrentCapabilityCheck,
        death: Option<super::character_death::PlayerDeathOccurrence>,
    ) -> Result<(HouseTransferAck, Vec<u8>), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| unavailable_house_transfer())?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()
            .map_err(|error| trace_house_transfer_stage("issue_semantic_pass", error))?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline)
                        .await
                        .map_err(|error| trace_house_transfer_stage("begin_transaction", error))?;
                    assert_recovery_fence(&mut tx, &recovery)
                        .await
                        .map_err(|error| {
                            trace_house_transfer_stage("assert_recovery_fence", error)
                        })?;
                    lock_admission_relations(&mut tx).await.map_err(|error| {
                        trace_house_transfer_stage("lock_admission_relations", error)
                    })?;
                    if !prove_current_incarnation(&mut tx, &node)
                        .await
                        .map_err(|error| trace_house_transfer_stage("prove_incarnation", error))?
                    {
                        return Err(unavailable_house_transfer());
                    }
                    prepare(&mut tx, &node, &mut request)
                        .await
                        .map_err(|error| {
                            trace_house_transfer_stage("prepare_house_entry", error)
                        })?;
                    let payload = encode_house_transfer_ack(&request.ack)
                        .map_err(|_| unavailable_house_transfer())?;
                    apply(
                        &mut tx,
                        &root,
                        &node,
                        &request,
                        &payload,
                        &capabilities,
                        None,
                        death,
                    )
                    .await
                    .map_err(|error| trace_house_transfer_stage("apply_house_transfer", error))?;
                    commit_semantic_transaction(tx, deadline)
                        .await
                        .map_err(|error| trace_house_transfer_stage("commit_transaction", error))?;
                    Ok((request.ack, payload))
                })
            })
            .await
            .map_err(|error| trace_house_transfer_stage("semantic_pass_run", error))
    }
}

async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    request: &mut HouseTransferRequest,
) -> Result<(), DurabilityError> {
    if request.ack.house.is_none() {
        return Ok(());
    }
    let fact = node.fact();
    let generation: String = sqlx::query_scalar(
        "SELECT ownership_generation::text FROM game_runtime_scope_assignments \
         WHERE scope_key=$1 AND state=1 AND holder_node_id=encode($2,'hex')::uuid \
           AND holder_registration_revision=$3::text::numeric FOR SHARE",
    )
    .bind(request.house.scope_key())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_one(&mut **tx)
    .await?;
    request.ack.scope_generation = generation
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let access = sqlx::query(
        "SELECT acl_revision::text,guild_revisions FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) LIMIT 1",
    )
    .bind(request.house.world_id().as_bytes().as_slice())
    .bind(request.house.house_key())
    .bind(request.fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let acl: String = access.try_get("acl_revision")?;
    let prepared_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let reserved_tile = request
        .ack
        .position
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect();
    let entry = HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: request.ack.transfer,
        character_id: crate::foundation::CharacterId::decode(request.fence.character_id.as_bytes())
            .map_err(|_| unavailable_house_transfer())?,
        source_game_session_id: request.fence.game_session_id,
        source_connection_generation: request.fence.connection_generation.get(),
        source_character_lease_generation: request.fence.character_lease_generation,
        source_scope: request.fence.runtime_scope,
        source_scope_ownership_generation: request.fence.scope_ownership_generation.get(),
        house: request.house.clone(),
        house_scope_ownership_generation: request.ack.scope_generation,
        acl_revision: acl
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        guild_revisions: access.try_get("guild_revisions")?,
        reserved_tile,
        prepared_at,
    };
    match prepare_house_entry_in_transaction(tx, node, &entry).await {
        Ok(HousePrepareOutcome::Prepared(_) | HousePrepareOutcome::Replayed(_)) => Ok(()),
        refused => {
            #[cfg(any(test, feature = "test-support"))]
            eprintln!("SRV7_HOUSE_TRANSFER_PREPARE_REFUSED detail={refused:?}");
            let _ = &refused;
            Err(unavailable_house_transfer())
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "transaction, Node authority, immutable transfer bytes, capability proof and mutually exclusive placement/death proofs retain independent owners"
)]
async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    request: &HouseTransferRequest,
    payload: &[u8],
    capabilities: &CurrentCapabilityCheck,
    admission_restore: Option<&ScopedLastPosition>,
    death: Option<super::character_death::PlayerDeathOccurrence>,
) -> Result<(), DurabilityError> {
    let ack = &request.ack;
    let fence = request.fence;
    let character = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
        .map_err(|_| unavailable_house_transfer())?;
    let world = request.house.world_id();
    let exit = ack.house.is_none();
    let expected_source = if exit {
        request.house.runtime_scope()?
    } else {
        RuntimeScopeRefV1::channel(world, request.origin)
    };
    let destination = if exit {
        RuntimeScopeRefV1::channel(world, request.origin)
    } else {
        request.house.runtime_scope()?
    };
    if fence.runtime_scope != expected_source
        || ack.world != *world.as_bytes()
        || ack.channel != *request.origin.as_bytes()
        || ack
            .house
            .as_deref()
            .is_some_and(|key| key != request.house.house_key())
        || ack.connection_generation != 1
        || (request.command == 0) != admission_restore.is_some()
        || admission_restore.is_some() && exit
        || fence.character_lease_generation.checked_add(1) != Some(ack.lease_generation)
        || ack.session == *fence.game_session_id.as_bytes()
        || request.last_position.world_id != world
        || exit
            && [
                request.last_position.cell.x,
                request.last_position.cell.y,
                i32::from(request.last_position.cell.floor),
            ] != ack.position
    {
        return Err(unavailable_house_transfer());
    }
    let session = GameSessionId::decode(&ack.session).map_err(|_| unavailable_house_transfer())?;
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if let Some(prior) = sqlx::query("SELECT acknowledgement,command_id::text FROM game_house_transfer_receipts WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
        // Historical replay is returned only for the exact same immutable candidate.
        return if prior.try_get::<Vec<u8>,_>("acknowledgement")? == payload
            && prior.try_get::<String,_>("command_id")? == request.command.to_string() { Ok(()) }
            else { Err(unavailable_house_transfer()) };
    }
    if let Some(occurrence) = death {
        let RuntimeScopeRefV1::Instance { instance_id, .. } = expected_source else {
            return Err(unavailable_house_transfer());
        };
        let mut position = Vec::with_capacity(10);
        position.extend_from_slice(&ack.position[0].to_be_bytes());
        position.extend_from_slice(&ack.position[1].to_be_bytes());
        position.extend_from_slice(
            &i16::try_from(ack.position[2])
                .map_err(|_| unavailable_house_transfer())?
                .to_be_bytes(),
        );
        let exact: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns p JOIN game_character_death_receipts d USING(death_occurrence_id) JOIN game_character_roots r ON r.character_id=d.character_id WHERE p.character_id=encode($1,'hex')::uuid AND d.character_id=p.character_id AND p.death_occurrence_id=encode($2,'hex')::uuid AND d.death_world_id=encode($3,'hex')::uuid AND d.death_scope_kind=2 AND d.death_instance_id=encode($4,'hex')::uuid AND d.death_scope_generation=$5::text::numeric AND p.respawn_position=$6 AND d.respawn_position=$6 AND d.committed_character_revision=r.character_revision FOR UPDATE OF p,r) AND NOT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice())
            .bind(world.as_bytes().as_slice()).bind(instance_id.as_slice())
            .bind(fence.scope_ownership_generation.get().to_string()).bind(position)
            .fetch_one(&mut **tx).await?;
        if !exit || admission_restore.is_some() || !exact {
            return Err(unavailable_house_transfer());
        }
    }

    if exit
        && !FreshAdmissionStore::from_root(root.clone())
            .check_current_runtime_capabilities(tx, destination, ack.scope_generation, capabilities)
            .await?
    {
        return Err(unavailable_house_transfer());
    }
    let fact = node.fact();
    // INSERT's 0215 trigger proves source session/transport/lease/claims and both
    // current assignments, before any source authority is terminalized.
    sqlx::query("INSERT INTO game_house_transfer_receipts(transfer_id,direction,character_id,account_id,world_id,house_key,origin_channel_id,source_game_session_id,command_id,source_connection_generation,source_character_lease_generation,source_scope_generation,source_transport,node_id,node_registration_revision,destination_game_session_id,destination_scope_generation,selection,tile_x,tile_y,tile_floor,state,admission_restore,restored_position_lease,restored_position_sequence) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::uuid,encode($5,'hex')::uuid,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12::text::numeric,$13,encode($14,'hex')::uuid,$15::text::numeric,encode($16,'hex')::uuid,$17::text::numeric,$18,$19,$20,$21,1,$22,$23::text::numeric,$24::text::numeric)")
        .bind(ack.transfer.as_slice()).bind(if exit {2_i16}else{1_i16})
        .bind(fence.character_id.as_bytes().as_slice()).bind(&account).bind(world.as_bytes().as_slice())
        .bind(request.house.house_key()).bind(request.origin.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(request.command.to_string()).bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(request.transport.to_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(ack.session.as_slice()).bind(ack.scope_generation.to_string())
        .bind(ack.selection.iter().map(|v| i32::try_from(*v)).collect::<Result<Vec<_>,_>>().map_err(|_| unavailable_house_transfer())?)
        .bind(ack.position[0]).bind(ack.position[1]).bind(i16::try_from(ack.position[2]).map_err(|_| unavailable_house_transfer())?)
        .bind(admission_restore.is_some())
        .bind(admission_restore.map(|s| s.position.key.character_lease_generation.to_string()))
        .bind(admission_restore.map(|s| s.position.key.write_sequence.to_string()))
        .execute(&mut **tx).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    if exit {
        let revision = unused_session_revision(tx, character, session).await?;
        sqlx::query("UPDATE game_durability_reconnect_attempts SET state=4 WHERE game_session_id=encode($1,'hex')::uuid AND state IN (1,5)")
            .bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
        let terminal = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN (1,2)")
            .bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
        if terminal.rows_affected() != 1 {
            return Err(unavailable_house_transfer());
        }
        let entry: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(handoff_id) FROM game_house_scope_handoffs WHERE destination_game_session_id=encode($1,'hex')::uuid AND state IN(2,3) FOR UPDATE",
        ).bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        release_house_entry_tile_in_transaction(
            tx,
            node,
            entry
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            now,
        )
        .await
        .map_err(|_| unavailable_house_transfer())?;
        commit_session_use(tx, character, session, ack.transfer, revision).await?;
        sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,$6,1,$7::text::numeric,$8::text::numeric,1,$9,2)")
            .bind(ack.session.as_slice()).bind(&account).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(request.origin.as_bytes().as_slice())
            .bind(now).bind(ack.lease_generation.to_string()).bind(ack.scope_generation.to_string()).bind(request.transport.to_bytes().as_slice()).execute(&mut **tx).await?;
    } else {
        match commit_house_entry_in_transaction(
            tx,
            node,
            &HouseEntryCommit {
                handoff_id: ack.transfer,
                destination_game_session_id: session,
                committed_at: now,
            },
        )
        .await
        {
            Ok(HouseCommitOutcome::Committed(_)) => {}
            refused => {
                #[cfg(any(test, feature = "test-support"))]
                eprintln!("SRV7_HOUSE_TRANSFER_COMMIT_REFUSED detail={refused:?}");
                let _ = &refused;
                return Err(unavailable_house_transfer());
            }
        }
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$2,session_state=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(ack.session.as_slice()).bind(request.transport.to_bytes().as_slice()).execute(&mut **tx).await?;
    }
    // Retain the original authenticated admission as history, while the current
    // session, scope, lease and transport are read from the destination row.
    // `fresh_replay_key` stays absent: this is not another use of the grant.
    let lineage = sqlx::query("UPDATE game_durability_reconnect_sessions d SET initial_fresh_replay_key=COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) FROM game_durability_reconnect_sessions s WHERE d.game_session_id=encode($1,'hex')::uuid AND s.game_session_id=encode($2,'hex')::uuid AND COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) IS NOT NULL")
        .bind(ack.session.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    if lineage.rows_affected() != 1 {
        return Err(unavailable_house_transfer());
    }
    super::fresh_admission_composition::transfer_claims_in_transaction(
        tx,
        root,
        &account,
        character,
        (fence.game_session_id, session),
        ack.lease_generation,
        now,
    )
    .await?;
    // The native checkpoint/exit location shares the session commit point. A
    // superseded or pending-respawn position cannot be silently called placed.
    let mut placed_record = request.last_position;
    let house_position = if exit {
        None
    } else {
        placed_record.cell = super::character_last_position::LastPositionCell {
            x: ack.position[0],
            y: ack.position[1],
            floor: ack.position[2]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        Some(HouseLastPosition {
            house: request.house.clone(),
            origin_channel: request.origin,
            origin: request.last_position.cell,
        })
    };
    if death.is_none()
        && upsert_scoped_last_position(
            tx,
            fence.character_id.as_bytes(),
            LastPositionKey {
                character_lease_generation: ack.lease_generation,
                write_sequence: 1,
            },
            &placed_record,
            house_position.as_ref(),
        )
        .await?
            != LastPositionWrite::Written
    {
        return Err(unavailable_house_transfer());
    }
    sqlx::query("UPDATE game_house_transfer_receipts SET state=2,acknowledgement=$2 WHERE transfer_id=encode($1,'hex')::uuid AND state=1")
        .bind(ack.transfer.as_slice()).bind(payload).execute(&mut **tx).await?;
    Ok(())
}
