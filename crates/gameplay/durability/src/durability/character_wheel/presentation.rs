//! Advisory coherent read. The exact point-budget reader is shared with the unchanged writer.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_gameplay_progression::atelier::AtelierState;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WheelPointBudget {
    pub available: u32,
    pub unlock_points: u32,
}
fn compose_budget(
    level: u32,
    task: u32,
    pilgrimage: bool,
    pilgrimage_points: u32,
    grade_iv: u32,
) -> std::result::Result<WheelPointBudget, DurabilityError> {
    let unlock_points = available_points(level)
        .checked_add(task)
        .and_then(|v| v.checked_add(if pilgrimage { pilgrimage_points } else { 0 }))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let available = unlock_points
        .checked_add(grade_iv)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(WheelPointBudget {
        available,
        unlock_points,
    })
}
pub(super) async fn read_budget(
    tx: &mut Transaction<'_>,
    character: CharacterId,
    level: u32,
    atelier: Option<&AtelierState>,
    revision: &str,
    pilgrimage_points: u32,
) -> std::result::Result<WheelPointBudget, DurabilityError> {
    let grade_iv = atelier.map_or(0, |state| {
        if state.grade_ruleset_revision.as_deref() == Some(revision) {
            state.grades.values().filter(|g| **g == 3).count() as u32
        } else {
            0
        }
    });
    let pilgrimage:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value=1)").bind(character.as_bytes().as_slice()).bind(oteryn_gameplay_quests::quest::monk_shrines::FINAL_TRACK).fetch_one(&mut **tx).await?;
    let task =
        super::super::task_board::read_promotion_points_in_transaction(tx, character).await?;
    compose_budget(level, task, pilgrimage, pilgrimage_points, grade_iv)
}
#[derive(Clone, Debug)]
pub struct WheelPresentationRead {
    pub allocation: WheelAllocation,
    pub atelier: Option<AtelierState>,
    pub vessels: WheelVessels,
    pub level: u32,
    pub vocation: String,
    pub promoted: bool,
    pub premium_permitted: bool,
    pub budget: WheelPointBudget,
}
impl DurabilityRoot {
    pub async fn read_wheel_presentation<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        ruleset: std::sync::Arc<WheelRuleset>,
        gate: &G,
    ) -> Result<WheelPresentationRead> {
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let pilgrimage_points = oteryn_gameplay_quests::quest::monk_shrines::final_points()
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = gate;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                let gate = *context;
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // Preserve the writer's Premium-latch-before-root lock order.
                    let account: Option<String> = sqlx::query_scalar(
                        "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
                    ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                    let Some(account) = account else {
                        return Ok(Err(CharacterProgressionError::AuthorityRejected));
                    };
                    let premium_permitted = gate.allocation_permitted(&mut tx, uuid_text(&account)?).await?;
                    let root = match super::super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let current_account: String = sqlx::query_scalar(
                        "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
                    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    if account != current_account {
                        return Ok(Err(CharacterProgressionError::AuthorityRejected));
                    }
                    let Some(state) = lock_progression_state(&mut tx, fence.character_id).await? else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision || !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let level = u32::try_from(state.try_get::<i64, _>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                    let stored = read_wheel_state(&mut tx, fence.character_id, false).await?;
                    let allocation = allocation_from_stored(&mut tx, stored, &ruleset).await?;
                    let atelier = super::super::character_atelier::load_in_transaction(&mut tx, fence.character_id).await?;
                    let vessels = read_vessels(&mut tx, fence.character_id).await?;
                    let vocation = read_vocation(&mut tx, fence.character_id).await?;
                    let promoted = super::super::character_promotion::read_durable_promotion(&mut tx, fence.character_id).await?;
                    let budget = read_budget(&mut tx, fence.character_id, level, atelier.as_ref(), ruleset.revision(), pilgrimage_points).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(WheelPresentationRead { allocation, atelier, vessels, level, vocation, promoted, premium_permitted, budget }))
                })
            }).await?
    }
}
impl WheelPresentationRead {
    pub fn validate_draft(
        &self,
        ruleset: &WheelRuleset,
        slots: &WheelSlots,
        vessels: &WheelVessels,
        removal: bool,
    ) -> std::result::Result<(), WheelRefusal> {
        if !self.allocation.current
            || !valid_vessels(
                &self.vessels,
                vessels,
                self.atelier.as_ref(),
                ruleset,
                (slots, &self.vocation, self.level, self.promoted),
            )
        {
            return Err(WheelRefusal::RulesetNotCurrent);
        }
        eligibility(
            ruleset,
            &self.vocation,
            self.level,
            self.promoted,
            self.premium_permitted,
        )
        .map_err(WheelRefusal::NotEligible)?;
        ruleset.validate_with_unlock_points(
            &self.allocation.slots,
            slots,
            self.budget.available,
            self.budget.unlock_points,
            removal,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheel_presentation_budget_keeps_task_pilgrimage_and_grade_iv_distinct()
    -> std::result::Result<(), DurabilityError> {
        assert_eq!(
            compose_budget(100, 9, true, 10, 4)?,
            WheelPointBudget {
                unlock_points: 69,
                available: 73
            }
        );
        assert_eq!(compose_budget(100, 9, false, 10, 4)?.unlock_points, 59);
        assert!(compose_budget(u32::MAX, u32::MAX, true, 10, 4).is_err());
        Ok(())
    }
}
