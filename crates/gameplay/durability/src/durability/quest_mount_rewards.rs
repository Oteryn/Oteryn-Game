//! A world-object quest receipt, consumed input and earned mount commit together.
//! Crystal 201189f: quests/targuna/actions_reliable_ram.lua:20-50.
use super::DurabilityError;
use super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};

pub(super) fn prepare(
    callback: Option<&Callback>,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    let Some(key) = callback
        .and_then(|c| c.effects.mount_reward.as_ref())
        .and_then(|r| r.achievement.as_ref())
    else {
        return Ok(None);
    };
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(key)
        .filter(|entry| !entry.retired)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(AchievementGrantRequest {
        achievement_key: key.clone(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:quest-mount".into(),
            event_id: Vec::new(),
        },
    }))
}

use super::character_progression::CurrentCharacterGameplayFence;
use super::quest_state::{QuestCause, QuestTransitionRequest};
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, EarnedCosmetics, UnlockFact,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::{reward_callbacks::PROVENANCE, world_callbacks::Callback};
use sha2::{Digest, Sha256};

/// The Quest writer has locked the Character, validated all progress predicates,
/// and admitted the exact world occurrence. Refusal must roll back this transaction.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    native: &NativeGameplayState,
    achievement: Option<&AchievementGrantRequest>,
) -> Result<bool, DurabilityError> {
    let Some(reward) = &callback.effects.mount_reward else {
        return Ok(false);
    };
    if !matches!(request.cause, QuestCause::Use(_)) {
        return Ok(false);
    }
    let key = reward.grant_key();
    let Some(cosmetics) = native.cosmetics() else {
        return Ok(false);
    };
    if !cosmetics.mount_exists(&reward.mount)
        || cosmetics.earned_unlock(&key, PROVENANCE, EarnedCosmetic::Mount(&reward.mount))
            != UnlockFact::Compatible
    {
        return Ok(false);
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    super::character_appearance::lock_account_grants(tx, account).await?;
    let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2)")
        .bind(account).bind(&key).fetch_one(&mut **tx).await?;
    if owned {
        return Ok(false);
    }
    let account_id = crate::domain::AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts = super::character_appearance::read_account_appearance_unlocks(
        tx,
        account_id,
        &cosmetics.earned_grant_keys(),
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|fact| {
            matches!(
                fact.source_kind.as_str(),
                "quest" | "npc" | "mount_taming" | "item_action" | "task_board"
            )
        })
        .map(|fact| (fact.unlock_key, fact.content_revision))
        .collect();
    if (EarnedCosmetics {
        content: cosmetics,
        facts: &facts,
    })
    .mount(&reward.mount)
        == UnlockFact::Compatible
    {
        return Ok(false);
    }
    let (cause, ordinal) = request.cause.key();
    let event = serde_json::to_vec(&(
        fence.character_id.as_bytes(),
        cause,
        ordinal,
        &callback.key,
        &key,
    ))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-mount:v1\0")
        .chain_update(&event)
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x80;
    id[8] = (id[8] & 63) | 0x80;
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(encode($1,'hex')::uuid,$2,$3,$4,encode($5,'hex')::uuid,'quest',$6)")
        .bind(id.as_slice()).bind(account).bind(&key).bind(PROVENANCE).bind(fence.character_id.as_bytes().as_slice()).bind(event.clone()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,encode($3,'hex')::uuid)")
        .bind(account).bind(key).bind(id.as_slice()).execute(&mut **tx).await?;
    if let Some(achievement) = achievement {
        let mut grant = achievement.clone();
        grant.source.event_id = digest.to_vec();
        let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
        record_achievement_grant(tx, granter, &grant)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(true)
}
