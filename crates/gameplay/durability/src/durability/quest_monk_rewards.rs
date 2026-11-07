//! Shrine reward composition. Quest owns entitlement, MINT owns item delivery.
use super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::CurrentCharacterGameplayFence;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::quest_state::QuestTransitionRequest;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use sqlx::Row;

/// Resolve immutable content before opening a transaction.
pub(super) fn prepare(
    callback: Option<&Callback>,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    if !callback
        .and_then(|c| c.monk_reward.as_ref())
        .is_some_and(|r| r.final_shrine)
    {
        return Ok(None);
    }
    let key = "oteryn:achievement/hope_of_the_merudri";
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let catalogue = if entry.retired {
        AchievementCatalogueLookup::Retired
    } else {
        AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        }
    };
    Ok(Some(AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue,
        source: AchievementSourceEvent {
            kind: "oteryn:monk-pilgrimage".into(),
            event_id: Vec::new(),
        },
    }))
}

#[derive(Debug)]
pub struct PendingMonkReward {
    pub item_source_id: u32,
    pub claim_key: String,
    pub content_revision: String,
}

/// The quest receipt has been inserted in this fenced, root-locked transaction.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    content_revision: &str,
    achievement: Option<&AchievementGrantRequest>,
) -> Result<(), DurabilityError> {
    let Some(reward) = &callback.monk_reward else {
        return Ok(());
    };
    let vocation: Option<String> = sqlx::query_scalar(
        "SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let monk = matches!(vocation.as_deref(), Some("monk" | "exalted_monk"));
    if monk {
        if let Some(key) = &reward.learn_spell {
            super::character_known_spells::grant_quest_spell(tx, fence.character_id, key).await?;
        }
        if let Some(source_id) = reward.item_source_id {
            let (cause, ordinal) = request.cause.key();
            let name = callback
                .key
                .strip_prefix("oteryn:quest-callback/monk-")
                .ok_or(DurabilityError::InvalidStoredState)?;
            let claim = format!("oteryn:reward-claim.monk-{name}");
            sqlx::query("INSERT INTO game_character_monk_reward_entitlements(character_id,transition_key,cause_id,cause_ordinal,item_source_id,claim_key,content_revision) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&callback.key)
                .bind(cause.as_slice()).bind(ordinal.to_string()).bind(i32::try_from(source_id).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(claim).bind(content_revision).execute(&mut **tx).await?;
        }
    }
    if reward.final_shrine {
        // Global grants the title and achievement at Eternity; the report grants Wheel points.
        super::character_title::grant_pensive_in_transaction(tx, fence.character_id).await?;
        let mut grant = achievement
            .ok_or(DurabilityError::InvalidStoredState)?
            .clone();
        grant.source.event_id = fence.character_id.as_bytes().to_vec();
        let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
        record_achievement_grant(tx, granter, &grant)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(())
}

impl DurabilityRoot {
    pub async fn pending_monk_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        transition: &str,
    ) -> Result<Option<PendingMonkReward>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let transition = transition.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT item_source_id,claim_key,content_revision FROM game_character_monk_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 AND state='PENDING'")
                .bind(character.as_bytes().as_slice()).bind(transition).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| -> Result<PendingMonkReward, DurabilityError> { Ok(PendingMonkReward {
                item_source_id: u32::try_from(row.try_get::<i32,_>("item_source_id")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                claim_key: row.try_get("claim_key")?, content_revision: row.try_get("content_revision")?,
            }) }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}
