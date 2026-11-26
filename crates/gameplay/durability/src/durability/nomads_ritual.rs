//! Pending native Nomads gestures are written by the physical item transaction.
use super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::assert_item_only_fence,
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    item_transfer::CurrentCharacterItemFence,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::world_callbacks::nomads_ritual;
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone, Copy, Debug)]
pub struct NomadsRitualTarget {
    pub map_digest: [u8; 32],
    pub reset_epoch: u64,
    pub object_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NomadsRitualGesture {
    Drop,
    Pour,
}

#[derive(Debug)]
pub struct NomadsRitualReceipt {
    pub target: NomadsRitualTarget,
    pub settled: bool,
}

pub(super) fn crack_cell() -> [u8; 10] {
    let pos = nomads_ritual::CRACK;
    let mut bytes = [0; 10];
    bytes[..4].copy_from_slice(&i32::from(pos.x).to_be_bytes());
    bytes[4..8].copy_from_slice(&i32::from(pos.y).to_be_bytes());
    bytes[8..].copy_from_slice(&i16::from(pos.floor).to_be_bytes());
    bytes
}

pub fn is_water_placement(placement: &[u8]) -> bool {
    let water = nomads_ritual::WATER;
    oteryn_world_bundle::bundle::placement_key(water.floor, water.x, water.y, 0)
        .is_some_and(|key| placement.len() == 40 && placement[32..] == key.to_be_bytes())
}

pub(super) async fn record_drop(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
    moved: [u8; 16],
    target: NomadsRitualTarget,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_nomads_ritual_gestures(
            game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,
            move_transaction_id,map_digest,reset_epoch,object_revision)
         SELECT r.game_session_id,r.command_id,r.character_id,r.world_id,r.channel_id,l.item_instance_id,
                r.transaction_id,$2,$3::text::numeric,$4::text::numeric
           FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id)
          WHERE r.transaction_id=encode($1,'hex')::uuid AND l.item_instance_id=encode($5,'hex')::uuid
            AND r.created_xact_id=pg_current_xact_id()
            AND l.to_kind=3 AND l.to_spatial_position=$6
            AND l.definition_family='Item' AND l.definition_production_key=$7",
    )
    .bind(transaction_id.as_slice())
    .bind(target.map_digest.as_slice())
    .bind(target.reset_epoch.to_string())
    .bind(target.object_revision.to_string())
    .bind(moved.as_slice())
    .bind(crack_cell().as_slice())
    .bind(nomads_ritual::SCIMITAR)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn record_pour(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
    target: NomadsRitualTarget,
) -> Result<(), DurabilityError> {
    let inserted = sqlx::query(
        "INSERT INTO game_nomads_ritual_gestures(
            game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,
            fluid_transaction_id,map_digest,reset_epoch,object_revision)
         SELECT t.game_session_id,t.command_id,t.character_id,s.world_id,s.channel_id,t.item_instance_id,
                t.transaction_id,$2,$3::text::numeric,$4::text::numeric
           FROM game_item_fluid_splashes s
           JOIN game_item_fluid_transitions t USING(transaction_id,item_instance_id)
          WHERE s.transaction_id=encode($1,'hex')::uuid AND t.created_xact_id=pg_current_xact_id()
            AND t.fluid_before=$5 AND t.fluid_after=0 AND s.fluid_subtype=t.fluid_before",
    )
    .bind(transaction_id.as_slice())
    .bind(target.map_digest.as_slice())
    .bind(target.reset_epoch.to_string())
    .bind(target.object_revision.to_string())
    .bind(nomads_ritual::WATER_SUBTYPE)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact receipt and current session are independently fenced"
    )]
    pub async fn nomads_ritual_gesture(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        gesture: NomadsRitualGesture,
        settle: bool,
    ) -> Result<Option<NomadsRitualReceipt>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(None);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let query = if settle {
                        "UPDATE game_nomads_ritual_gestures SET settled=true
                      WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric
                        AND character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid
                        AND channel_id=encode($5,'hex')::uuid
                        AND (move_transaction_id IS NOT NULL)=$6
                      RETURNING map_digest,reset_epoch::text,object_revision::text,settled"
                    } else {
                        "SELECT map_digest,reset_epoch::text,object_revision::text,settled
                       FROM game_nomads_ritual_gestures
                      WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric
                        AND character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid
                        AND channel_id=encode($5,'hex')::uuid
                        AND (move_transaction_id IS NOT NULL)=$6"
                    };
                    let row = sqlx::query(query)
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(world_id.as_bytes().as_slice())
                        .bind(channel_id.as_bytes().as_slice())
                        .bind(gesture == NomadsRitualGesture::Drop)
                        .fetch_optional(&mut *tx)
                        .await?;
                    let receipt = row
                        .map(|row| -> Result<_, DurabilityError> {
                            Ok(NomadsRitualReceipt {
                                target: NomadsRitualTarget {
                                    map_digest: row
                                        .try_get::<Vec<u8>, _>("map_digest")?
                                        .try_into()
                                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                                    reset_epoch: row
                                        .try_get::<String, _>("reset_epoch")?
                                        .parse()
                                        .map_err(|_| {
                                        DurabilityError::InvalidStoredState
                                    })?,
                                    object_revision: row
                                        .try_get::<String, _>("object_revision")?
                                        .parse()
                                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                                },
                                settled: row.try_get("settled")?,
                            })
                        })
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(receipt)
                })
            })
            .await
    }
}
