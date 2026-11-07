//! The current durable name of one Character, for the speaker name of a chat line (CHAT-WIRE-1).
//!
//! Read once at fresh admission under the existing Character recovery authority. The name is a
//! display fact only: it is neither presence, nor a position, nor a permission.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::domain::CharacterId;
use crate::domain::character_name::CharacterName;
use sqlx::{Postgres, Row, Transaction};

impl DurabilityRoot {
    /// The name of the active Character `character_id`; `None` when it has no active root or
    /// the stored name does not parse.
    pub async fn read_character_chat_name(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<Option<CharacterName>, CharacterAuthorityError> {
        let character = *character_id.as_bytes();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT name FROM game_character_roots \
                         WHERE character_id = encode($1, 'hex')::uuid AND lifecycle = 1",
                    )
                    .bind(character.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let name = match row {
                        Some(row) => CharacterName::parse(&row.try_get::<String, _>("name")?).ok(),
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(name))
                })
            })
            .await?
    }
}

/// Read the caster's display name while retaining the caller's Character root share lock.
/// The caller supplies its already fenced Character identity and keeps the transaction open.
pub async fn read_character_name_for_share_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
) -> Result<String, sqlx::Error> {
    sqlx::query(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE",
    )
    .bind(character.as_slice())
    .fetch_one(&mut **tx)
    .await?
    .try_get("name")
}
