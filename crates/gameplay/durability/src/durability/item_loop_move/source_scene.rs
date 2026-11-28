//! A source-scene close moves every selected ground root in one custody transaction.
use super::*;
use crate::durability::map_item_mint::{
    MapItemMintCandidate, MapItemMintError, commit_reserved_map_item_mint,
};

const MAX_SOURCE_SCENE_ITEM_MOVES: usize = 8
    * (oteryn_protocol_oteryn::world_map::MAX_BASE_ORDINAL as usize + 1
        + super::super::ground_cell_limit::GROUND_CELL_ROOT_LIMIT);

pub struct SourceSceneItemMove {
    pub command: CommandRef,
    pub request: LoopMoveRequest,
    pub mint: Option<MapItemMintCandidate>,
}

/// The final exact receipt proves the result of the complete atomic batch.
#[derive(Debug, PartialEq, Eq)]
pub enum SourceSceneReceipt {
    Committed,
    Absent,
    Conflict,
    RecoverySuperseded,
}

#[cfg(feature = "test-support")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum SceneCommitFault {
    LoseCommitAck,
    RefuseReceiptRead,
    PauseReceiptRead,
    Completed,
}

#[cfg(feature = "test-support")]
tokio::task_local! {
    static LOST_SCENE_COMMIT_ACK: std::cell::Cell<SceneCommitFault>;
    static SCENE_RECEIPT_PAUSE: (std::sync::Arc<tokio::sync::Notify>, std::sync::Arc<tokio::sync::Notify>);
}

#[cfg(feature = "test-support")]
pub async fn with_lost_source_scene_commit_ack<T>(
    operation: impl std::future::Future<Output = T>,
    receipt_started: std::sync::Arc<tokio::sync::Notify>,
    allow_receipt: std::sync::Arc<tokio::sync::Notify>,
) -> T {
    SCENE_RECEIPT_PAUSE.scope((receipt_started, allow_receipt), LOST_SCENE_COMMIT_ACK
        .scope(std::cell::Cell::new(SceneCommitFault::LoseCommitAck), operation))
        .await
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "the batch uses the existing fenced item loop and map MINT owners")]
    pub async fn commit_source_scene_item_moves<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        mut moves: Vec<SourceSceneItemMove>,
    ) -> Result<Result<(), LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        if moves.is_empty() || moves.len() > MAX_SOURCE_SCENE_ITEM_MOVES || moves.iter().any(|movement| {
            movement.request.pickup_cell.is_none()
                || !matches!(movement.request.destination, LoopDestination::Ground(_))
                || movement.request.count.is_some()
                || movement.mint.as_ref().is_some_and(|mint| mint.request().command != movement.command
                    || *mint.item_instance_id() != movement.request.item)
        }) {
            return Ok(Err(LoopMoveRefusal::Rejected));
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        for movement in &mut moves {
            if let Some(mint) = &mut movement.mint
                && let Err(error) = self.charge_map_item_mint_work_unit(record.clone(), mint).await
            {
                return match error {
                    MapItemMintError::Unavailable(error) => Err(error),
                    _ => Ok(Err(LoopMoveRefusal::Rejected)),
                };
            }
        }
        let node = node.clone();
        let mut commit_started = false;
        let mut context = (permit, content, &mut commit_started);
        let result = self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
                let (permit, content, commit_started) = context;
                let mut tx = super::super::db::begin_type2_transaction(holder, deadline).await?;
                let attempt: LoopResult<()> = async {
                    for SourceSceneItemMove { command, mut request, mint } in moves {
                        // Each MINT must prove its initial Ground state before
                        // the loop moves it. Each loop MOVE likewise proves its
                        // after-state before another stack can merge into it.
                        sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                        if let Some(mint) = mint {
                            request.item = match commit_reserved_map_item_mint(
                                &mut tx, permit, &record, &node, &fence, &mint,
                            ).await {
                                Ok(minted) => minted.item_instance_id,
                                Err(MapItemMintError::Unavailable(error)) => return Err(error.into()),
                                Err(_) => return refuse(LoopMoveRefusal::Rejected),
                            };
                            sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut **tx).await?;
                            sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                        }
                        let intent = intent_bytes(fence.character_id, &request)?;
                        lock_loop_command(&mut tx, command).await?;
                        if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                            replay.map_err(LoopError::Refused)?;
                        } else {
                            apply_loop_move_with_source_policy(
                                &mut tx, permit, &record, &node, &fence, command, *content,
                                &request, &intent, None, None, None, None, true,
                            ).await?;
                        }
                    }
                    Ok(())
                }.await;
                match attempt {
                    Ok(()) => {
                        **commit_started = true;
                        tx.commit(deadline).await?;
                        #[cfg(feature = "test-support")]
                        if LOST_SCENE_COMMIT_ACK.try_with(|state|
                            state.replace(SceneCommitFault::RefuseReceiptRead) == SceneCommitFault::LoseCommitAck
                        ) == Ok(true) {
                            return Err(DurabilityError::CommitOutcomeUnknown);
                        }
                        Ok(Ok(()))
                    }
                    Err(error) => loop_error(error).map(Err),
                }
            })).await;
        match result {
            Err(error) if commit_started && !matches!(
                error, DurabilityError::CommitRejected | DurabilityError::CommitTransactionTimeout
            ) => Err(DurabilityError::CommitOutcomeUnknown),
            result => result,
        }
    }

    /// Wait for the exact batch's command lock before concluding absence.
    /// This is historical custody evidence. It grants no scene publication authority.
    pub async fn reconcile_source_scene_move(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        request: &LoopMoveRequest,
    ) -> Result<SourceSceneReceipt, DurabilityError> {
        #[cfg(feature = "test-support")]
        match LOST_SCENE_COMMIT_ACK.try_with(|state| state.get()) {
            Ok(SceneCommitFault::RefuseReceiptRead) => {
                LOST_SCENE_COMMIT_ACK.with(|state| state.set(SceneCommitFault::PauseReceiptRead));
                return Err(DurabilityError::RootUnavailable);
            }
            Ok(SceneCommitFault::PauseReceiptRead) => {
                LOST_SCENE_COMMIT_ACK.with(|state| state.set(SceneCommitFault::Completed));
                let (started, allow) = SCENE_RECEIPT_PAUSE.with(Clone::clone);
                started.notify_one();
                allow.notified().await;
            }
            _ => {}
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let mut request = request;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut request, move |holder, deadline, request| Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                if !super::super::character_authority::recovery_fence_is_current(&mut tx, &record, true).await? {
                    return Ok(SourceSceneReceipt::RecoverySuperseded);
                }
                let read: LoopResult<_> = async {
                    let intent = intent_bytes(character, request)?;
                    lock_loop_command(&mut tx, command).await?;
                    committed_loop_move(&mut tx, command, character, &intent).await
                }.await;
                let receipt = match read {
                    Ok(Some(Ok(_))) => SourceSceneReceipt::Committed,
                    Ok(None) => SourceSceneReceipt::Absent,
                    Ok(Some(Err(_))) => SourceSceneReceipt::Conflict,
                    Err(error) => return loop_error(error).map(|_| SourceSceneReceipt::Conflict),
                };
                super::super::db::commit_semantic_read(tx, deadline).await?;
                Ok(receipt)
            }))
            .await
    }
}
