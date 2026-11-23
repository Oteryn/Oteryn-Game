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

#[cfg(feature = "test-support")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceSceneFault {
    LoseAcknowledgment,
    ReceiptUnavailable,
    Resolved,
}

#[cfg(feature = "test-support")]
tokio::task_local! {
    static SOURCE_SCENE_FAULT: std::cell::Cell<SourceSceneFault>;
}

#[cfg(feature = "test-support")]
impl SourceSceneItemMove {
    /// Drop one real COMMIT acknowledgment and make the first recovery read unavailable.
    pub async fn with_lost_ack_and_unavailable_read<T>(
        operation: impl std::future::Future<Output = T>,
    ) -> T {
        SOURCE_SCENE_FAULT
            .scope(std::cell::Cell::new(SourceSceneFault::LoseAcknowledgment), operation)
            .await
    }
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
        let Some(last) = moves.last() else {
            return Ok(Err(LoopMoveRefusal::Rejected));
        };
        let (last_command, last_request) = (last.command, last.request.clone());
        let mut context = (permit, content, false);
        let result = self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
                let (permit, content, commit_started) = context;
                let (permit, content) = (*permit, *content);
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
                                &mut tx, permit, &record, &node, &fence, command, content,
                                &request, &intent, None, None, None, None, true,
                            ).await?;
                        }
                    }
                    Ok(())
                }.await;
                match attempt {
                    Ok(()) => {
                        *commit_started = true;
                        tx.commit(deadline).await?;
                        Ok(Ok(()))
                    }
                    Err(error) => loop_error(error).map(Err),
                }
            })).await;
        #[cfg(feature = "test-support")]
        let result = if matches!(result, Ok(Ok(())))
            && SOURCE_SCENE_FAULT.try_with(|fault| {
                fault.replace(SourceSceneFault::ReceiptUnavailable) == SourceSceneFault::LoseAcknowledgment
            }).unwrap_or(false)
        {
            Err(DurabilityError::CommitOutcomeUnknown)
        } else {
            result
        };
        let Err(error) = result else {
            return result;
        };
        let unresolved = context.2
            && !matches!(error, DurabilityError::CommitRejected | DurabilityError::CommitTransactionTimeout);
        loop {
            match self.reconcile_source_scene_move(
                recovery, fence.character_id, last_command, last_request.clone(),
            ).await {
                Ok(Some(Ok(_))) => return Ok(Ok(())),
                Ok(_) => return Err(error),
                Err(_) if unresolved => {
                    // The staged scene must stay reserved. A failed read proves
                    // neither commit nor rollback. Retry only the receipt lookup.
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(_) => return Err(error),
            }
        }
    }

    async fn reconcile_source_scene_move(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        request: LoopMoveRequest,
    ) -> Result<Option<Result<LoopMoveReceipt, LoopMoveRefusal>>, DurabilityError> {
        #[cfg(feature = "test-support")]
        if SOURCE_SCENE_FAULT.try_with(|fault| {
            fault.replace(SourceSceneFault::Resolved) == SourceSceneFault::ReceiptUnavailable
        }).unwrap_or(false) {
            return Err(DurabilityError::RootUnavailable);
        }
        self.reconcile_item_loop_move(recovery, character, command, request).await
    }
}
