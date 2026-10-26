//! HOUSE-TRANSFER-1: atomic native handoff / reverse handoff and durable scope ack.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    LastPositionKey, LastPositionRecord, LastPositionWrite, upsert_last_position,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::fresh_admission::{
    CurrentCapabilityCheck, FreshAdmissionStore, commit_session_use, unused_session_revision,
};
use super::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseId, commit_house_entry_in_transaction,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{AuthenticatedTransportRefV1, ChannelId, GameSessionId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::house_transfer::{HouseTransferAck, encode_house_transfer_ack};
use sqlx::{Postgres, Row, Transaction};

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

impl DurabilityRoot {
    /// The existing entry PREPARE must already have succeeded. Exit has its own
    /// fully fenced direction in 0215 and never relaxes 0074's direction CHECK.
    pub async fn commit_house_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: HouseTransferRequest,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<Vec<u8>, DurabilityError> {
        let payload =
            encode_house_transfer_ack(&request.ack).map_err(|_| DurabilityError::Unavailable)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    apply(&mut tx, &root, &node, &request, &payload, &capabilities).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(payload)
                })
            })
            .await
    }
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    request: &HouseTransferRequest,
    payload: &[u8],
    capabilities: &CurrentCapabilityCheck,
) -> Result<(), DurabilityError> {
    let ack = &request.ack;
    let fence = request.fence;
    let character = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
        .map_err(|_| DurabilityError::Unavailable)?;
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
        || request.command == 0
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
        return Err(DurabilityError::Unavailable);
    }
    let session = GameSessionId::decode(&ack.session).map_err(|_| DurabilityError::Unavailable)?;
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if let Some(prior) = sqlx::query("SELECT acknowledgement,command_id::text FROM game_house_transfer_receipts WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
        // Historical replay is returned only for the exact same immutable candidate.
        return if prior.try_get::<Vec<u8>,_>("acknowledgement")? == payload
            && prior.try_get::<String,_>("command_id")? == request.command.to_string() { Ok(()) }
            else { Err(DurabilityError::Unavailable) };
    }
    if exit
        && !FreshAdmissionStore::from_root(root.clone())
            .check_current_runtime_capabilities(tx, destination, ack.scope_generation, capabilities)
            .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let fact = node.fact();
    // INSERT's 0215 trigger proves source session/transport/lease/claims and both
    // current assignments, before any source authority is terminalized.
    sqlx::query("INSERT INTO game_house_transfer_receipts(transfer_id,direction,character_id,account_id,world_id,house_key,origin_channel_id,source_game_session_id,command_id,source_connection_generation,source_character_lease_generation,source_scope_generation,source_transport,node_id,node_registration_revision,destination_game_session_id,destination_scope_generation,selection,tile_x,tile_y,tile_floor,state) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::uuid,encode($5,'hex')::uuid,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12::text::numeric,$13,encode($14,'hex')::uuid,$15::text::numeric,encode($16,'hex')::uuid,$17::text::numeric,$18,$19,$20,$21,1)")
        .bind(ack.transfer.as_slice()).bind(if exit {2_i16}else{1_i16})
        .bind(fence.character_id.as_bytes().as_slice()).bind(&account).bind(world.as_bytes().as_slice())
        .bind(request.house.house_key()).bind(request.origin.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(request.command.to_string()).bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(request.transport.to_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(ack.session.as_slice()).bind(ack.scope_generation.to_string())
        .bind(ack.selection.iter().map(|v| i32::try_from(*v)).collect::<Result<Vec<_>,_>>().map_err(|_| DurabilityError::Unavailable)?)
        .bind(ack.position[0]).bind(ack.position[1]).bind(i16::try_from(ack.position[2]).map_err(|_| DurabilityError::Unavailable)?)
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
            return Err(DurabilityError::Unavailable);
        }
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
            _ => return Err(DurabilityError::Unavailable),
        }
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$2,session_state=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(ack.session.as_slice()).bind(request.transport.to_bytes().as_slice()).execute(&mut **tx).await?;
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
    if upsert_last_position(
        tx,
        fence.character_id.as_bytes(),
        LastPositionKey {
            character_lease_generation: ack.lease_generation,
            write_sequence: 1,
        },
        &request.last_position,
    )
    .await?
        != LastPositionWrite::Written
    {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("UPDATE game_house_transfer_receipts SET state=2,acknowledgement=$2 WHERE transfer_id=encode($1,'hex')::uuid AND state=1")
        .bind(ack.transfer.as_slice()).bind(payload).execute(&mut **tx).await?;
    Ok(())
}
