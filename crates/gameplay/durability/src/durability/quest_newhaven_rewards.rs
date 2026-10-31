//! Gustavo completion retains its item entitlement in the quest receipt transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::CurrentCharacterGameplayFence;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::quest_state::QuestTransitionRequest;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use sqlx::Row;

#[derive(Debug)]
pub struct PendingNewhavenReward {
    pub item_source_id: u32,
    pub claim_key: String,
    pub content_revision: String,
}

/// The quest receipt has been inserted in this fenced, root-locked transaction.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    content_revision: &str,
) -> Result<(), DurabilityError> {
    let spell = match request.transition_key.as_str() {
        "oteryn:quest-transition/newhaven/viola-light" => Some("candidate:spell/light"),
        "oteryn:quest-transition/newhaven/viola-wound-cleansing" => Some("candidate:spell/wound_cleansing"),
        _ => None,
    };
    if let Some(spell) = spell {
        super::character_known_spells::grant_quest_spell(tx, fence.character_id, spell).await?;
    }
    if request.transition_key != oteryn_gameplay_quests::quest::newhaven::FINISH {
        return Ok(());
    }
    let (cause, ordinal) = request.cause.key();
    sqlx::query("INSERT INTO game_character_newhaven_reward_entitlements(character_id,transition_key,cause_id,cause_ordinal,item_source_id,claim_key,content_revision) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric,3052,'oteryn:reward-claim.newhaven-gustavo',$5)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&request.transition_key)
        .bind(cause.as_slice()).bind(ordinal.to_string()).bind(content_revision)
        .execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    pub async fn pending_newhaven_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        transition: &str,
    ) -> Result<Option<PendingNewhavenReward>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let transition = transition.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT item_source_id,claim_key,content_revision FROM game_character_newhaven_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 AND state='PENDING'")
                .bind(character.as_bytes().as_slice()).bind(transition).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| -> Result<PendingNewhavenReward, DurabilityError> { Ok(PendingNewhavenReward {
                item_source_id: u32::try_from(row.try_get::<i32,_>("item_source_id")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                claim_key: row.try_get("claim_key")?, content_revision: row.try_get("content_revision")?,
            }) }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}
