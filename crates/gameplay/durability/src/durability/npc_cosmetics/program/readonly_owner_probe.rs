//! Test-support hook called ONLY after the production arm and capture succeeded.
//! No fabricated private context, Node, actor witness or gameplay predecessor.
use super::*;
pub(super) async fn qualify_after_actual_capture(
    tx: &mut Type2Transaction<'_>,
) -> std::result::Result<(), DurabilityError> {
    // This exact context exists only because game_npc_arm_source accepted the
    // caller's actual Node/session/lease/actor Greet and finite source policy.
    let before: (String, String, i64, i64) = sqlx::query_as(
        "SELECT c.character_id::text,r.character_revision::text,\
        (SELECT count(*) FROM game_npc_source_readonly_greets g WHERE g.character_id=c.character_id),\
        (SELECT count(*) FROM game_npc_cosmetic_receipts p WHERE p.transaction_id=c.npc_transaction_id) \
        FROM game_npc_source_context c JOIN game_character_roots r ON r.character_id=c.character_id \
        WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()")
        .fetch_one(&mut ***tx).await?;
    if before.3 != 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut ***tx)
        .await?;
    // Exercise idempotent same-XID retry after constraints already fired. The
    // existing immutable occurrence and current attempt must not duplicate.
    sqlx::query("SELECT game_npc_source_readonly_greet_capture()")
        .execute(&mut ***tx)
        .await?;
    let after: (String, String, i64, i64) = sqlx::query_as(
        "SELECT c.character_id::text,r.character_revision::text,\
        (SELECT count(*) FROM game_npc_source_readonly_greets g WHERE g.character_id=c.character_id),\
        (SELECT count(*) FROM game_npc_cosmetic_receipts p WHERE p.transaction_id=c.npc_transaction_id) \
        FROM game_npc_source_context c JOIN game_character_roots r ON r.character_id=c.character_id \
        WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()")
        .fetch_one(&mut ***tx).await?;
    if before != after {
        return Err(DurabilityError::InvalidStoredState);
    }
    let replay: (bool, i64, i64) = sqlx::query_as(
        "SELECT g.created_xact_id=pg_current_xact_id(), \
        (SELECT count(*) FROM game_npc_source_readonly_greet_attempts a \
         WHERE a.original_observation_id=g.observation_id), \
        (SELECT count(*) FROM game_npc_cosmetic_receipts p \
         WHERE p.transaction_id IN(g.observation_id,c.npc_transaction_id)) \
        FROM game_npc_source_context c JOIN game_npc_source_readonly_greets g \
        ON g.game_session_id=c.game_session_id AND g.connection_generation=c.connection_generation \
        AND g.conversation_generation=(c.before_state->'greet'->>'conversation_generation')::numeric \
        AND g.npc_identity=decode(c.before_state->'greet'->>'npc_identity','hex') \
        AND g.npc_generation=(c.before_state->'greet'->>'npc_generation')::numeric AND g.source_key=c.source_key \
        WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()")
        .fetch_one(&mut ***tx).await?;
    // First genuine commit has one immutable occurrence/attempt; a second
    // production Root transaction must preserve it and add exactly one attempt.
    if replay.1 != if replay.0 { 1 } else { 2 } || replay.2 != 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut ***tx)
        .await?;
    Ok(())
}
