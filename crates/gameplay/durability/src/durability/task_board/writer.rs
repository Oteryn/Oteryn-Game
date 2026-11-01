use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::super::db::{begin_type2_transaction, lock_admission_relations};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::spell_owner_commit::SpellLanePermit;
use super::super::{DurabilityError, DurabilityRoot};
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::{TaskBoardContext, apply_action};
use sha2::{Digest, Sha256};

pub(super) type Error = CharacterProgressionError;

impl DurabilityRoot {
    /// The generated board, quest tracks, completion XP obligations, and custody
    /// effects commit as one transaction after the existing session-generation fence.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_task_board(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: TaskBoardCommand,
        catalogue: Arc<TaskBoardCatalogue>,
        permit: Option<&SpellLanePermit>,
    ) -> Result<TaskBoardCommit, Error> {
        if request.action == TaskBoardAction::GrantExpansion {
            return Err(Error::AuthorityRejected);
        }
        self.commit_task_board_inner(authority, node, fence, request, catalogue, permit, None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn commit_task_board_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: TaskBoardCommand,
        catalogue: Arc<TaskBoardCatalogue>,
        permit: Option<&SpellLanePermit>,
        expansion_delivery: Option<String>,
    ) -> Result<TaskBoardCommit, Error> {
        if (request.action == TaskBoardAction::GrantExpansion) != expansion_delivery.is_some() {
            return Err(Error::AuthorityRejected);
        }
        if request.payload_binding.len() > MAX_TASK_BOARD_PAYLOAD_BYTES {
            return Err(Error::InvalidInput);
        }
        match (&request.action, request.cause) {
            (TaskBoardAction::CreditKill { .. }, QuestCause::CreatureDeath(_)) => {}
            (TaskBoardAction::CreditKill { .. }, _) | (_, QuestCause::CreatureDeath(_)) => {
                return Err(Error::InvalidInput);
            }
            (_, QuestCause::Command(command))
                if command.game_session_id() == fence.game_session_id => {}
            _ => return Err(Error::AuthorityRejected),
        }
        if let Some(permit) = permit {
            permit.check_scope(fence.runtime_scope)?;
        }
        let action_bytes = serde_json::to_vec(&request.action).map_err(|_| Error::InvalidInput)?;
        let mut digest = Sha256::new();
        digest.update(b"oteryn:task-board-command:v2");
        digest.update(fence.character_id.as_bytes());
        digest.update((request.payload_binding.len() as u64).to_be_bytes());
        digest.update(&request.payload_binding);
        digest.update(action_bytes);
        let binding: [u8; 32] = digest.finalize().into();
        let prepared =
            mint::prepare(self, authority, node, fence, &request, binding, &catalogue).await?;
        if let Some(replay) = prepared.replay {
            return Ok(replay);
        }
        let mint_candidate = prepared.candidate;
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let (_, cause, ordinal) = state::cause_key(request.cause);
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:taskboard:'||encode($1,'hex')||':'||encode($2,'hex')||':'||$3,0))")
                .bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
                .execute(&mut **tx).await?;
            match state::replay(&mut tx, fence, &request, &binding).await {
                Ok(Some(previous)) => { tx.commit(deadline).await?; return Ok(Ok(previous)); }
                Ok(None) => {}
                Err(error) => return Ok(Err(error)),
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(error) => return Ok(Err(error)),
            };
            if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await? {
                return Ok(Err(Error::RespawnPending));
            }
            if let Some(delivery) = &expansion_delivery {
                match expansion::replay(&mut tx, fence, delivery).await {
                    Ok(Some(previous)) => { tx.commit(deadline).await?; return Ok(Ok(previous)); }
                    Ok(None) => {}
                    Err(error) => return Ok(Err(error)),
                }
            }
            let credit_kill = matches!(request.action, TaskBoardAction::CreditKill { .. });
            let mut before = state::load(&mut tx, fence, credit_kill).await?;
            let inactive_kill = credit_kill && state::inactive(&before.data);
            let progression_row = match super::progression::qualify(&mut tx, fence, &root, &catalogue, inactive_kill).await {
                Ok(row) => row,
                Err(error) => return Ok(Err(error)),
            };
            let projected = match super::progression::project_pending(&mut tx, fence, &progression_row, &mut before, &catalogue, inactive_kill).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(Err(error)),
            };
            super::entitlements::synchronize(&mut tx, fence, &mut before.data, &catalogue).await?;
            let mut view = before.clone();
            let gold = super::fee::available_gold(&mut tx, fence, &catalogue).await?;
            let has_vocation: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid AND vocation<>$2)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(super::super::character_build::NO_VOCATION)
                .fetch_one(&mut **tx).await?;
            let ctx = TaskBoardContext { level: view.level, now_unix_seconds: view.database_now,
                period: view.period, creatures: &catalogue.creatures, available_gold: gold,
                reroll_tokens: view.reroll_tokens, has_vocation };
            let seed: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
                .fetch_one(&mut **tx).await?;
            let mut draw = Draw { seed, ordinal: 0 };
            let applied = if inactive_kill {
                Ok(oteryn_gameplay_quests::quest::task_board::full::TaskBoardEffects::default())
            } else {
                apply_action(&mut view.data, &mut view.balances, &request.action, &ctx, &mut |n| draw.below(n))
            };
            let mut effects = match applied {
                Ok(effects) => effects,
                Err(refusal) => {
                    if !credit_kill {
                        super::availability::refresh(&mut tx, fence, &catalogue, &mut before).await?;
                    }
                    let outcome = TaskBoardCommit { view: before, refusal: Some(TaskBoardRefusal::Rule(refusal)), transition: None, replayed: false };
                    state::record(&mut tx, fence, &request, &binding, &outcome, None).await?;
                    tx.commit(deadline).await?;
                    return Ok(Ok(outcome));
                }
            };
            if let Err(error) = super::progression::plan_experience(&progression_row, &mut view, &catalogue, projected, &mut effects) {
                return Ok(Err(error));
            }
            state::update_tracks(&mut view);
            let changed = view.data != before.data || view.balances != before.balances
                || effects.experience != 0 || !effects.consume_items.is_empty()
                || !effects.grant_items.is_empty() || effects.gold_cost != 0
                || effects.reroll_tokens_spent != 0 || effects.reroll_tokens_granted != 0;
            let changes = if changed {
                let changes = state::plan_changes(&before, &mut view)?;
                if !state::fits_capacity(&mut tx, fence, &before, &changes).await? {
                    if !credit_kill {
                        super::availability::refresh(&mut tx, fence, &catalogue, &mut before).await?;
                    }
                    let outcome = TaskBoardCommit { view: before, refusal: Some(TaskBoardRefusal::CapacityExceeded), transition: None, replayed: false };
                    state::record(&mut tx, fence, &request, &binding, &outcome, None).await?;
                    tx.commit(deadline).await?;
                    return Ok(Ok(outcome));
                }
                Some(changes)
            } else { None };
            // Roll back all custody writes on a refusal, while retaining the command's
            // refusal record outside the savepoint for stable retries.
            sqlx::query("SAVEPOINT task_board_effects").execute(&mut **tx).await?;
            if let Some(refusal) = custody::apply(&mut tx, *permit, fence, &request, &catalogue, &effects, mint_candidate.as_ref()).await? {
                sqlx::query("ROLLBACK TO SAVEPOINT task_board_effects").execute(&mut **tx).await?;
                if !credit_kill {
                    super::availability::refresh(&mut tx, fence, &catalogue, &mut before).await?;
                }
                let outcome = TaskBoardCommit { view: before, refusal: Some(refusal), transition: None, replayed: false };
                state::record(&mut tx, fence, &request, &binding, &outcome, None).await?;
                tx.commit(deadline).await?;
                return Ok(Ok(outcome));
            }
            view.reroll_tokens = view.reroll_tokens.checked_sub(u64::from(effects.reroll_tokens_spent))
                .and_then(|value| value.checked_add(u64::from(effects.reroll_tokens_granted)))
                .ok_or(DurabilityError::InvalidStoredState)?;
            let transition = if let Some(changes) = changes {
                Some(super::state::advance(&mut tx, fence, &request, &binding, &view, changes, effects.experience).await?)
            } else { None };
            // Client and expansion replies take one post-command item snapshot.
            // Kill settlement consumes no full reply, so its internal receipt can
            // omit these counts without adding inventory reads to party fan-out.
            if !credit_kill {
                super::availability::refresh(&mut tx, fence, &catalogue, &mut view).await?;
            }
            let outcome = TaskBoardCommit { view, refusal: None, transition, replayed: false };
            if let Some(delivery) = &expansion_delivery {
                expansion::record(&mut tx, fence, &request, delivery).await?;
            }
            state::record(&mut tx, fence, &request, &binding, &outcome, Some(&effects)).await?;
            tx.commit(deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }
}

/// Rejection sampling avoids modulo bias; the database provides the seed only
/// once, and the committed command record is the replay authority thereafter.
struct Draw {
    seed: Vec<u8>,
    ordinal: u64,
}
impl Draw {
    fn below(&mut self, upper: u32) -> u32 {
        if upper == 0 {
            return 0;
        }
        let n = u64::from(upper);
        loop {
            let hash = Sha256::new()
                .chain_update(&self.seed)
                .chain_update(self.ordinal.to_be_bytes())
                .finalize();
            self.ordinal = self.ordinal.wrapping_add(1);
            let value = u64::from_be_bytes(hash[..8].try_into().unwrap_or([0; 8]));
            if value < u64::MAX - u64::MAX % n {
                return (value % n) as u32;
            }
        }
    }
}
