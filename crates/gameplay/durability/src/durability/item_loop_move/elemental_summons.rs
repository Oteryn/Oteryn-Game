//! Recover the bounded, still-leased M49 summon set under independently current authority.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

/// The current native map and reset observed by the runtime owner.
#[derive(Debug, Clone, Copy)]
pub struct ElementalSummonScope {
    pub map: [u8; 32],
    pub reset_epoch: u64,
}

/// Immutable expected spawn details, never a current actor/controller authority grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementalSummonIntent {
    pub transaction_id: [u8; 16],
    pub altar: u16,
    pub creature_key: String,
    pub position: Position,
}

#[derive(Debug, Clone, Copy)]
pub struct ElementalSummonClaim {
    pub scope: ElementalSummonScope,
    pub transaction_id: [u8; 16],
}

/// Committed clock metadata. Native projection must still prove its current runtime
/// binding and deduplicate this spawn key under that owner before installing an actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementalSummonClock {
    pub intent: ElementalSummonIntent,
    pub spawn_key: String,
    pub cycle: u64,
    pub owner_generation: crate::foundation::ScopeOwnershipGeneration,
}

impl DurabilityRoot {
    /// Read immutable spawn details, then independently re-prove current authority,
    /// Content/frame, reset and the live lease in the clock's actual claiming transaction.
    pub async fn claim_elemental_summon<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        request: ElementalSummonClaim,
    ) -> Result<Option<ElementalSummonClock>, DurabilityError> {
        let Some(intent) = self.read_elemental_summons(permit, recovery, node, fence, content, request.scope)
            .await?.into_iter().find(|intent| intent.transaction_id == request.transaction_id)
        else { return Ok(None); };
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, content| {
            Box::pin(async move {
                let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
                let attempt: LoopResult<Option<ElementalSummonClock>> = async {
                    assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence,
                        None, content.loop_content_digest()).await?;
                    let (world,channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                    let source = crate::durability::boss_encounter::ElementalClockSource {
                        world, channel, generation: fence.scope_ownership_generation,
                        transaction: request.transaction_id, map: request.scope.map, reset_epoch: request.scope.reset_epoch,
                    };
                    let claimed = crate::durability::boss_encounter::claim_elemental_clock(&mut tx, &record, &node, source)
                        .await.map_err(|error| match error {
                            crate::durability::boss_encounter::BossClockError::Unavailable(error) => LoopError::Durability(error),
                            crate::durability::boss_encounter::BossClockError::AuthorityRejected => LoopError::Refused(LoopMoveRefusal::Rejected),
                            _ => LoopError::Durability(DurabilityError::InvalidStoredState),
                        })?;
                    Ok(claimed.map(|(key, clock)| ElementalSummonClock {
                        intent, spawn_key: key.spawn, cycle: clock.cycle, owner_generation: fence.scope_ownership_generation,
                    }))
                }.await;
                match attempt {
                    Ok(clock) => { tx.commit(deadline).await?; Ok(clock) },
                    Err(error) => loop_error(error).map(|_| None),
                }
            })
        }).await
    }

    /// Resolve at most one active intent per source altar. Recovery accepts the original
    /// intent's old ownership generation only after independently proving the current one.
    /// The caller still owns native actor admission, deduplication and projection.
    pub async fn read_elemental_summons<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        current: ElementalSummonScope,
    ) -> Result<Vec<ElementalSummonIntent>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, content| {
            Box::pin(async move {
                let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
                let attempt: LoopResult<Vec<ElementalSummonIntent>> = async {
                    assert_recovery_fence(&mut tx, &record).await?;
                    assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence,
                        None, content.loop_content_digest()).await?;
                    let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                    let reset: i64 = current.reset_epoch.try_into().map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                    let rows = sqlx::query(
                        "SELECT b.transaction_id::text,b.altar_id,b.creature_key,b.summon_position \
                         FROM game_elemental_boss_leases l JOIN game_elemental_summon_intents b USING(transaction_id) \
                         JOIN LATERAL (SELECT server_artifact_digest,frame_binding_digest FROM game_content_activations \
                           WHERE world_id=l.world_id AND channel_id=l.channel_id ORDER BY activation_sequence DESC LIMIT 1) a ON true \
                         WHERE l.world_id=encode($1,'hex')::uuid AND l.channel_id=encode($2,'hex')::uuid AND l.reset_epoch=$3 \
                           AND game_world_reset_map_epoch_current(l.world_id,$3::text::numeric) \
                           AND b.map_revision='sha256:'||encode($4,'hex') \
                           AND b.content_revision='sha256:'||encode(a.server_artifact_digest,'hex') \
                           AND b.placement_context=a.frame_binding_digest \
                         ORDER BY l.altar_id LIMIT 4"
                    ).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(reset)
                        .bind(current.map.as_slice()).fetch_all(&mut **tx).await?;
                    rows.into_iter().map(|row| {
                        let transaction: String = row.try_get("transaction_id")?;
                        let altar: i32 = row.try_get("altar_id")?;
                        let (x,y,floor): (u16,u16,i8) = serde_json::from_value(row.try_get("summon_position")?)
                            .map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))?;
                        Ok(ElementalSummonIntent {
                            transaction_id: uuid_text(&transaction)?,
                            altar: altar.try_into().map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))?,
                            creature_key: row.try_get("creature_key")?, position: Position { x,y,floor },
                        })
                    }).collect()
                }.await;
                match attempt {
                    Ok(intents) => { tx.commit(deadline).await?; Ok(intents) },
                    Err(error) => loop_error(error).map(|_| Vec::new()),
                }
            })
        }).await
    }
}
