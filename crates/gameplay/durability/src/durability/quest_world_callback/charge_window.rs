//! The final discharge shares the existing Quest writer's PostgreSQL transaction.
use super::*;
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::assert_item_only_fence,
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::CommandRef;

#[derive(Clone, Copy)]
pub enum ChargeObservation {
    CheckTool,
    Open {
        amount: u8,
        burn_ms: u32,
        ready_ms: u32,
    },
    CheckWindow,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChargeState {
    Allowed,
    Empty,
    Waiting,
    Refused,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn observe_quest_charge(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        context: QuestWorldToolContext,
        group: String,
        tool_key: String,
        op: ChargeObservation,
    ) -> Result<ChargeState, DurabilityError> {
        let Some(tool) = context.selected_tool.as_ref() else {
            return Ok(ChargeState::Refused);
        };
        if fence.game_session_id != command.game_session_id() || permit.has_unresolved() {
            return Ok(ChargeState::Refused);
        }
        let tool_id = tool.source;
        let world = fence.runtime_scope.world_id();
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let mut borrowed = (permit, context);
        self.try_issue_semantic_pass()?.run_with_context(
            &mut borrowed,
            move |holder, deadline, (permit, context)| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err()
                    || !context_is_current(&mut tx, permit, fence, context).await?
                {
                    return Ok(ChargeState::Refused);
                }
                let held = sqlx::query_scalar::<_, bool>(
                    "SELECT true FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
                     JOIN game_item_instances i ON i.item_instance_id=c.item
                     WHERE i.item_instance_id=encode($3,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid
                       AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND i.definition_production_key=$4
                     FOR UPDATE OF i",
                )
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                .bind(tool_id.as_slice()).bind(&tool_key)
                .fetch_optional(&mut *tx).await?.unwrap_or(false);
                if !held {
                    return Ok(ChargeState::Refused);
                }
                let epoch = context.reset_epoch.to_string();
                let state = match op {
                    ChargeObservation::CheckTool => ChargeState::Allowed,
                    ChargeObservation::Open { amount, burn_ms, ready_ms } => {
                        if !(1..=64).contains(&amount) || burn_ms == 0 || ready_ms == 0 {
                            return Ok(ChargeState::Refused);
                        }
                        let opened = sqlx::query_scalar::<_, i16>(
                            "WITH clock AS (SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS now)
                             INSERT INTO game_quest_world_charge_windows AS w
                             (world_id,map_revision,reset_epoch,charge_group,opening_session,opening_command,ready_at,expires_at,remaining)
                             SELECT encode($1,'hex')::uuid,$2,$3::text::numeric,$4,encode($5,'hex')::uuid,
                                    $6::text::numeric,now+$7,now+$7+$8,$9 FROM clock
                             ON CONFLICT (world_id,map_revision,reset_epoch,charge_group) DO UPDATE
                             SET (opening_session,opening_command,ready_at,expires_at,remaining)=
                                 (EXCLUDED.opening_session,EXCLUDED.opening_command,EXCLUDED.ready_at,EXCLUDED.expires_at,EXCLUDED.remaining)
                             WHERE w.expires_at <= (SELECT now FROM clock)
                                AND (w.opening_session,w.opening_command)<>(EXCLUDED.opening_session,EXCLUDED.opening_command)
                             RETURNING remaining",
                        )
                        .bind(world.as_bytes().as_slice()).bind(&context.map_revision).bind(&epoch).bind(&group)
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .bind(i64::from(burn_ms)).bind(i64::from(ready_ms)).bind(i16::from(amount))
                        .fetch_optional(&mut *tx).await?.is_some();
                        let replay = if opened { false } else {
                            sqlx::query_scalar::<_, bool>(
                                "SELECT EXISTS(SELECT 1 FROM game_quest_world_charge_windows
                                 WHERE world_id=encode($1,'hex')::uuid AND map_revision=$2 AND reset_epoch=$3::text::numeric
                                   AND charge_group=$4 AND opening_session=encode($5,'hex')::uuid AND opening_command=$6::text::numeric
                                   AND ready_at>floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
                            )
                            .bind(world.as_bytes().as_slice()).bind(&context.map_revision).bind(&epoch).bind(&group)
                            .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                            .fetch_one(&mut *tx).await?
                        };
                        if opened || replay { ChargeState::Allowed } else { ChargeState::Waiting }
                    }
                    ChargeObservation::CheckWindow => {
                        let observed: Option<(i64, i64, i16, i64)> = sqlx::query_as(
                            "SELECT ready_at,expires_at,remaining,floor(extract(epoch FROM statement_timestamp())*1000)::bigint
                             FROM game_quest_world_charge_windows
                             WHERE world_id=encode($1,'hex')::uuid AND map_revision=$2 AND reset_epoch=$3::text::numeric
                              AND charge_group=$4",
                        )
                        .bind(world.as_bytes().as_slice()).bind(&context.map_revision).bind(&epoch).bind(&group)
                        .fetch_optional(&mut *tx).await?;
                        match observed {
                            Some((ready, _, _, now)) if now < ready => ChargeState::Waiting,
                            Some((_, expiry, remaining, now)) if now < expiry && remaining > 0 => ChargeState::Allowed,
                            _ => ChargeState::Empty,
                        }
                    }
                };
                commit_semantic_transaction(tx, deadline).await?;
                Ok(state)
            }),
        ).await
    }
}

pub(super) async fn spend(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldToolContext,
    charge_group: &str,
) -> Result<bool, DurabilityError> {
    let updated = sqlx::query(
        "WITH clock AS (SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS now)
         UPDATE game_quest_world_charge_windows SET remaining=remaining-1 FROM clock
         WHERE world_id=encode($1,'hex')::uuid AND map_revision=$2 AND reset_epoch=$3::text::numeric
           AND charge_group=$4 AND remaining>0 AND ready_at<=clock.now AND expires_at>clock.now",
    )
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(&context.map_revision).bind(context.reset_epoch.to_string()).bind(charge_group)
    .execute(&mut **tx).await?;
    Ok(updated.rows_affected() == 1)
}
