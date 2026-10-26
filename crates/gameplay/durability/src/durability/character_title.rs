//! CHARINFO durable title facts. Writes are fenced by the existing reconciled
//! Character authority and never copy the native facts used for qualification.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use oteryn_protocol_oteryn::character_info::{CharacterTitle, TitleFacts, qualified_selection};
use crate::domain::CharacterId;
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTitleState {
    pub earned: Vec<(CharacterTitle, Option<i64>)>,
    pub selected: Option<CharacterTitle>,
    pub revision: u64,
}

fn decode_title(value: i16) -> Result<CharacterTitle, CharacterAuthorityError> {
    match value { 1 => Ok(CharacterTitle::AdeptArmorer), 2 => Ok(CharacterTitle::AdeptEnhancer),
        3 => Ok(CharacterTitle::Cartographer), 4 => Ok(CharacterTitle::RotationChampion),
        _ => Err(CharacterAuthorityError::Rejected) }
}

impl DurabilityRoot {
    /// Reads earned state and atomically clears a selection which no longer
    /// qualifies. The supplied facts are observations from the native owners,
    /// not persisted copies of their truth.
    pub async fn read_and_requalify_character_titles(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, character: CharacterId,
        facts: TitleFacts, now_unix_ms: i64,
    ) -> Result<CharacterTitleState, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let rows = sqlx::query("SELECT title,expires_at_unix_ms FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND (expires_at_unix_ms IS NULL OR expires_at_unix_ms>$2) ORDER BY title FOR SHARE")
                .bind(character.as_bytes().as_slice()).bind(now_unix_ms).fetch_all(&mut *tx).await?;
            let mut earned = Vec::with_capacity(rows.len());
            for row in rows {
                let title = match decode_title(row.try_get("title")?) { Ok(title) => title, Err(error) => return Ok(Err(error)) };
                earned.push((title, row.try_get("expires_at_unix_ms")?));
            }
            let selection = sqlx::query("SELECT title,revision FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let (selected, mut revision) = match selection {
                Some(row) => {
                    let selected = match row.try_get::<Option<i16>, _>("title")? { Some(raw) => match decode_title(raw) { Ok(title) => Some(title), Err(error) => return Ok(Err(error)) }, None => None };
                    let revision = match u64::try_from(row.try_get::<i64, _>("revision")?) { Ok(value) => value, Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)) };
                    (selected, revision)
                }
                None => (None, 0),
            };
            let qualified = qualified_selection(selected, facts, now_unix_ms)
                .filter(|title| earned.iter().any(|(earned, _)| earned == title));
            if selected.is_some() && qualified.is_none() {
                let next: i64 = sqlx::query_scalar("UPDATE game_character_title_selection SET title=NULL,revision=revision+1 WHERE character_id=encode($1,'hex')::uuid RETURNING revision")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                revision = match u64::try_from(next) { Ok(value) => value, Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)) };
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterTitleState { earned, selected: qualified, revision }))
        })).await?
    }

    pub async fn revoke_character_title(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, character: CharacterId,
        title: CharacterTitle,
    ) -> Result<bool, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("UPDATE game_character_title_selection SET title=NULL,revision=revision+1 WHERE character_id=encode($1,'hex')::uuid AND title=$2")
                .bind(character.as_bytes().as_slice()).bind(title as i16).execute(&mut *tx).await?;
            let changed = sqlx::query("DELETE FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=$2")
                .bind(character.as_bytes().as_slice()).bind(title as i16).execute(&mut *tx).await?.rows_affected() == 1;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(changed))
        })).await?
    }
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
