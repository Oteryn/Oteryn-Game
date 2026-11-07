use super::super::item_mint_audit::Type2Transaction;
use super::*;

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    account: sqlx::types::Uuid,
    transaction: &str,
    revision: u64,
    plan: &rules::Plan,
    grants: &[String],
) -> Result<(), DurabilityError> {
    for change in &plan.changes {
        let affected = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision WHERE game_character_quest_tracks.quest_key=EXCLUDED.quest_key AND game_character_quest_tracks.value=$6")
            .bind(character.as_bytes().as_slice()).bind(&change.key).bind(&change.quest).bind(change.after)
            .bind(revision.to_string()).bind(change.before).execute(&mut ***tx).await?.rows_affected();
        if affected != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let event = sqlx::types::Uuid::parse_str(transaction)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for key in grants {
        let occurrence: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut ***tx)
            .await?;
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,'npc',$6)")
            .bind(occurrence).bind(account).bind(key).bind(rules::PROVENANCE).bind(character.as_bytes().as_slice()).bind(event.as_bytes().as_slice()).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
            .bind(account).bind(key).bind(occurrence).execute(&mut ***tx).await?;
    }
    if let Some(achievement) = plan.achievement {
        super::super::account_achievement::grant_missing(
            tx,
            character,
            &[achievement],
            "npc",
            event.as_bytes(),
        )
        .await?;
    }
    Ok(())
}
