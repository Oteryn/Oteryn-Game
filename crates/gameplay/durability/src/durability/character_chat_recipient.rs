//! The online recipient of a private chat message (CHAT-0 §5, CHAT-2B).
//!
//! The recipient is resolved by name key in the sender's World with the find-person query: an
//! active Character root with an eligible admission guard held by an Active session that has a
//! current transport and no control loss. The answer is a routing fact only, read without row
//! locks; delivery stays at most once.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::domain::character_name::CharacterName;
use sqlx::Row;

impl DurabilityRoot {
    /// The Character id of the online Character named `name` in World `world`; `None` when
    /// none is online under that name.
    pub async fn read_chat_recipient(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: [u8; 16],
        name: &CharacterName,
    ) -> Result<Option<[u8; 16]>, CharacterAuthorityError> {
        let key = name.comparison_key();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT uuid_send(r.character_id) AS character \
                         FROM game_character_roots r \
                         JOIN game_durability_admission_character_guards g USING (character_id) \
                         JOIN game_durability_reconnect_sessions s \
                           ON s.game_session_id = g.holder_game_session_id \
                         WHERE r.world_id = encode($1, 'hex')::uuid AND r.lifecycle = 1 \
                           AND g.eligible AND g.world_id = r.world_id \
                           AND s.character_id = r.character_id AND s.world_id = r.world_id \
                           AND s.session_state = 2 AND s.current_transport_ref IS NOT NULL \
                           AND s.control_loss_epoch IS NULL AND r.name_key = $2 \
                         LIMIT 1",
                    )
                    .bind(world.as_slice())
                    .bind(&key)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let character = match row {
                        Some(row) => {
                            <[u8; 16]>::try_from(row.try_get::<Vec<u8>, _>("character")?.as_slice())
                                .ok()
                        }
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(character))
                })
            })
            .await?
    }
}
