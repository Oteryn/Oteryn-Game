//! The online recipient of a private chat message (CHAT-0 §5, CHAT-2B).
//!
//! The recipient is resolved by name key in the sender's World with the find-person query: an
//! active Character root with an eligible admission guard held by an Active session that has a
//! current transport and no control loss. The answer is a routing fact only, read without row
//! locks; delivery stays at most once.

use crate::durability::db::commit_semantic_read;
use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction};
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
                    commit_semantic_read(tx, deadline).await?;
                    Ok(Ok(character))
                })
            })
            .await?
    }
}

impl DurabilityRoot {
    /// Public current levels, resolved by indexed names within the requester's World.
    pub async fn read_console_levels(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: [u8; 16],
        names: Vec<CharacterName>,
    ) -> Result<Vec<Option<u32>>, CharacterAuthorityError> {
        if names.len() > 32 { return Err(CharacterAuthorityError::Rejected); }
        let keys: Vec<String> = names.iter().map(CharacterName::comparison_key).collect();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let mut levels = Vec::with_capacity(keys.len());
            for key in keys {
                let level: Option<i64> = sqlx::query_scalar("SELECT p.level FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.name_key=$2 AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id) LIMIT 1")
                    .bind(world.as_slice()).bind(key).fetch_optional(&mut *tx).await?;
                levels.push(level.and_then(|value| u32::try_from(value).ok()).filter(|value| *value > 0));
            }
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(levels))
        })).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness::{
        Harness, TestResult, WORLD, configured_admin, debug, id, runtime,
    };

    #[test]
    fn console_levels_excludes_detached_and_retired_closure_members() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            runtime().map_err(debug)?.block_on(async move {
                for (step, state, decision) in [(1_i16, "CLOSED_DETACHED", None), (3, "FINALIZED", Some("FINALIZE"))] {
                    let h = Harness::create(admin.clone(), "console_closure_members", true).await?;
                    let result = async {
                        let seal = h.recovery.seal_current().map_err(debug)?;
                        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                        let names = || CharacterName::parse("Fixture Hero").map(|name| vec![name]).map_err(debug);
                        assert_eq!(h.root.read_console_levels(&authority, id(WORLD), names()?).await.map_err(debug)?, vec![Some(50)]);
                        // Private retained closure state: public lifecycle deliberately stays active.
                        sqlx::query("INSERT INTO game_account_closures(account_id,termination_id,close_operation_id,lifecycle_generation,close_generation,state,decision,recovery_generation,closed_at) SELECT account_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,1,$4,$5,1,0 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).bind(id(80).as_slice()).bind(id(81).as_slice()).bind(state).bind(decision).execute(&h.pool).await?;
                        sqlx::query("INSERT INTO game_account_closure_members(close_operation_id,character_id,account_id,root_revision,step) SELECT encode($2,'hex')::uuid,character_id,account_id,character_revision,$3 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).bind(id(81).as_slice()).bind(step).execute(&h.pool).await?;
                        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?, 1);
                        assert_eq!(h.root.read_console_levels(&authority, id(WORLD), names()?).await.map_err(debug)?, vec![None], "closure state {state}");
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }.await;
                    h.cleanup().await?;
                    result?;
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?.join().map_err(|_| "console closure test thread failed")??;
        Ok(())
    }
}
