//! Crystal 201189f8 data-global/lib/quests/svargrond_arena.lua:207-232.
use super::*;
use crate::durability::arena_match::{MatchState, pit_reset::lock_reset_custody};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::spell_items_abi::decode_ground_cell;
use oteryn_gameplay_quests::quest::barbarian_arena::{Pit, ResetTopThing};

/// One top Ground item as observed under the Channel's physical Item lane.
#[derive(Debug, Clone)]
pub struct ArenaGroundObservation {
    pub item: [u8; 16],
    pub appearance: u16,
    pub spatial_position: Vec<u8>,
    pub stack_ordinal: u64,
}

impl DurabilityRoot {
    pub async fn retire_arena_pit_ground(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        state: &MatchState,
        observations: Vec<ArenaGroundObservation>,
    ) -> Result<Vec<[u8; 16]>> {
        permit.check_scope(fence.runtime_scope)?;
        if permit.has_unresolved() || observations.len() > 100 {
            return Err(DecayRetireError::AuthorityRejected);
        }
        let Some(pit) = Pit::new(state.pit) else {
            return Err(DecayRetireError::InvalidInput);
        };
        let mut cells = std::collections::BTreeSet::new();
        for observation in &observations {
            check_uuid_v7(&observation.item)?;
            let cell = decode_ground_cell(&observation.spatial_position)
                .map_err(|_| DecayRetireError::InvalidInput)?;
            let at = [cell.x, cell.y, -cell.z.abs()];
            if observation.stack_ordinal == 0
                || !cells.insert(at)
                || pit.layout().item_to_reset(
                    at,
                    ResetTopThing::Item {
                        identity: observation.item,
                        appearance: observation.appearance,
                        movable: true,
                    },
                ) != Some(observation.item)
            {
                return Err(DecayRetireError::InvalidInput);
            }
        }
        if observations.is_empty() {
            return Ok(Vec::new());
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let node = node.clone();
        let state = state.clone();
        let channel = fence
            .runtime_scope
            .channel_id()
            .ok_or(DecayRetireError::AuthorityRejected)?;
        let decay_fence = CorpseDecayFence {
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| {
            Box::pin(async move {
                let permit: &SpellLanePermit = permit;
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err()
                    || !lock_reset_custody(&mut tx, fence, &state).await?
                {
                    return Ok(Err(DecayRetireError::AuthorityRejected));
                }
                let mut retired = Vec::with_capacity(observations.len());
                for observation in observations {
                    let item = observation.item;
                    lock_cause(&mut tx, item).await?;
                    if load_receipt(&mut tx, item).await?.is_some()
                        || load_reservation(&mut tx, item, &state.scope_generation.to_string()).await?.is_some()
                    {
                        continue;
                    }
                    let step = DecayRetireStep {
                        corpse_item_instance_id: item,
                        item_instance_id: item,
                        cause: RetireCause::ArenaReset,
                    };
                    let admitted = match world_reset_cause::admit_ground(&mut tx, &decay_fence, step).await? {
                        Ok(admitted) => admitted,
                        Err(DecayRetireError::Refused(_)) => continue,
                        Err(error) => return Ok(Err(error)),
                    };
                    if admitted.corpse_ground.spatial_position != observation.spatial_position
                        || admitted.corpse_ground.runtime_scope_ownership_generation != state.scope_generation
                        || admitted.definition.production_key != format!("oteryn:item.tibia.i{}", observation.appearance)
                    {
                        continue;
                    }
                    let claimed: Option<(String, String)> = sqlx::query_as(
                        "INSERT INTO game_svargrond_ground_retirements
                           (item_instance_id,world_id,channel_id,character_id,game_session_id,run_id,
                            pit,rank,scope_generation,expires_at_ms,spatial_position,source_stack_ordinal)
                         SELECT g.item_instance_id,g.world_id,g.channel_id,encode($4,'hex')::uuid,
                            encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,$9::text::numeric,$10,
                            g.spatial_position,g.stack_ordinal
                         FROM game_item_ground_locations g WHERE g.item_instance_id=encode($1,'hex')::uuid
                            AND g.spatial_position=$2 AND g.stack_ordinal=$3
                         ON CONFLICT DO NOTHING
                         RETURNING game_character_uuid_v7()::text,game_character_uuid_v7()::text",
                    )
                    .bind(item.as_slice())
                    .bind(&observation.spatial_position)
                    .bind(i64::try_from(observation.stack_ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(state.run.as_slice())
                    .bind(i16::from(state.pit))
                    .bind(i16::from(state.rank))
                    .bind(state.scope_generation.to_string())
                    .bind(state.deadline_ms)
                    .fetch_optional(&mut **tx).await?;
                    let Some((transaction, event)) = claimed else { continue };
                    let transaction_id = uuid_text(&transaction)?;
                    let event_id = uuid_text(&event)?;
                    let tuple = tx.tuple();
                    let mut message = decay_message(step, &decay_fence, &admitted);
                    message.cause = None;
                    message.arena_reset = Some(audit::OneItemArenaResetV1 {
                        world_id: admitted.world_id.to_vec(),
                        channel_id: channel.as_bytes().to_vec(),
                        character_id: fence.character_id.as_bytes().to_vec(),
                        game_session_id: fence.game_session_id.as_bytes().to_vec(),
                        run_id: state.run.to_vec(),
                        item_instance_id: item.to_vec(),
                        rank: u32::from(state.rank),
                        pit: u32::from(state.pit),
                        deadline_unix_ms: u64::try_from(state.deadline_ms).map_err(|_| DurabilityError::InvalidStoredState)?,
                    });
                    let envelope = match audit::encode_decay_retire_event(
                        DecayRetireEventIdentity {
                            event_id,
                            transaction_id,
                            occurred_at_unix_ms: admitted.database_now_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        message, tuple.tuple(),
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let fact = node.fact();
                    let reservation = Reservation {
                        cause: step.cause,
                        corpse_item_instance_id: item,
                        fence_generation: state.scope_generation,
                        transaction_id,
                        event_id,
                        occurred_at_unix_ms: admitted.database_now_unix_ms,
                        deadline_unix_ms: admitted.deadline_unix_ms,
                        quantity_before: admitted.quantity_before,
                        placement_ordinal: None,
                        envelope,
                        fence_node_id: *fact.node_id().as_bytes(),
                        fence_registration_revision: fact.registration_revision(),
                        work_units_used: 0,
                    };
                    retirement_workflow::insert_reservation(&mut tx, step, &admitted, &reservation, tuple).await?;
                    apply_retire(&mut tx, permit, step, &admitted, &reservation, tuple).await?;
                    retired.push(item);
                }
                tx.commit(deadline).await?;
                Ok(Ok(retired))
            })
        }).await?
    }
}
