//! CHAT-PM-CHANNELS-1: the recipient of a private message is the online Character of that name
//! in this World, against the production SQLx migration. An offline, unknown or foreign-World
//! name has no recipient, so the message is refused `NOT_ONLINE` and nothing is delivered.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use crate::domain::character_name::CharacterName;

/// The fixture session online (fresh admission, no control loss) or back in control loss.
async fn set_online(h: &Harness, online: bool) -> TestResult {
    if online {
        // The session continuity names its fresh admission receipt.
        sqlx::query(
            "INSERT INTO game_durability_fresh_admission_receipts SELECT \
             '\\x01'::bytea || decode(repeat('00', 32), 'hex'), game_session_id, account_id, \
             character_id, world_id, runtime_scope_channel_id, character_lease_generation, \
             scope_ownership_generation, 1, current_transport_ref, 1, '{}', 1 \
             FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid \
             ON CONFLICT DO NOTHING",
        )
        .bind(id(SESSION).as_slice())
        .execute(&h.pool)
        .await?;
    }
    let query = if online {
        "UPDATE game_durability_reconnect_sessions SET session_state=2, control_loss_epoch=NULL, \
         original_grace_deadline=NULL, predecessor_generation=NULL, \
         fresh_replay_key='\\x01'::bytea || decode(repeat('00', 32), 'hex') \
         WHERE game_session_id=encode($1,'hex')::uuid"
    } else {
        "UPDATE game_durability_reconnect_sessions SET session_state=1, control_loss_epoch=1, \
         original_grace_deadline=999999, predecessor_generation=1 \
         WHERE game_session_id=encode($1,'hex')::uuid"
    };
    sqlx::query(query)
        .bind(id(SESSION).as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

#[test]
fn chat_recipient_postgres_is_the_online_character_of_that_name_in_this_world() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "chat_recipient", true).await?;
        let result = async {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let name = |value: &str| CharacterName::parse(value).map_err(debug);
            let read = |world: u8, value: CharacterName| {
                let (root, authority) = (&h.root, &authority);
                async move {
                    root.read_chat_recipient(authority, id(world), &value)
                        .await
                        .map_err(debug)
                }
            };
            // The fixture session holds its Character but is in control loss: not online.
            assert_eq!(read(WORLD, name("Fixture Hero")?).await?, None);
            set_online(&h, true).await?;
            assert_eq!(read(WORLD, name("Fixture Hero")?).await?, Some(id(CHARACTER)));
            // The name matches by its key.
            assert_eq!(read(WORLD, name("fixture hero")?).await?, Some(id(CHARACTER)));
            assert_eq!(read(WORLD, name("Fixture Heroine")?).await?, None);
            assert_eq!(read(WORLD + 50, name("Fixture Hero")?).await?, None);
            // Control loss takes the Character offline again.
            set_online(&h, false).await?;
            assert_eq!(read(WORLD, name("Fixture Hero")?).await?, None);
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
