//! CHARINFO durable title facts. Writes are fenced by the existing reconciled
//! Character authority and never copy the native facts used for qualification.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use oteryn_protocol_oteryn::character_info::CharacterTitle;
use crate::domain::CharacterId;

impl DurabilityRoot {
    pub async fn unlock_character_title(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        title: CharacterTitle,
        earned_at_unix_ms: i64,
        expires_at_unix_ms: Option<i64>,
    ) -> Result<bool, CharacterAuthorityError> {
        if earned_at_unix_ms < 0
            || expires_at_unix_ms.is_some_and(|expiry| expiry <= earned_at_unix_ms)
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let changed = sqlx::query(
                    "INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms,expires_at_unix_ms) \
                     VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING",
                )
                .bind(character.as_bytes().as_slice())
                .bind(title as i16)
                .bind(earned_at_unix_ms)
                .bind(expires_at_unix_ms)
                .execute(&mut *tx)
                .await?
                .rows_affected() == 1;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(changed))
            }))
            .await?
    }

    pub async fn select_character_title(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        title: Option<CharacterTitle>,
    ) -> Result<u64, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let revision: i64 = sqlx::query_scalar(
                    "INSERT INTO game_character_title_selection(character_id,title,revision) \
                     VALUES(encode($1,'hex')::uuid,$2,1) ON CONFLICT(character_id) DO UPDATE \
                     SET title=EXCLUDED.title,revision=game_character_title_selection.revision+1 \
                     RETURNING revision",
                )
                .bind(character.as_bytes().as_slice())
                .bind(title.map(|value| value as i16))
                .fetch_one(&mut *tx)
                .await?;
                commit_semantic_transaction(tx, deadline).await?;
                let revision = match u64::try_from(revision) {
                    Ok(revision) => revision,
                    Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)),
                };
                Ok(Ok(revision))
            }))
            .await?
    }
}
