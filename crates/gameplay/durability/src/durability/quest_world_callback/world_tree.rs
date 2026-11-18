//! The selected-phial receipt and the shared one-minute window commit together.
use super::DurabilityError;
use crate::foundation::RuntimeScopeRefV1;

pub(crate) async fn open(
    connection: &mut sqlx::PgConnection,
    scope: RuntimeScopeRefV1,
) -> Result<bool, DurabilityError> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = scope else {
        return Ok(false);
    };
    let opened = sqlx::query(
        "INSERT INTO game_forgotten_knowledge_tree_windows (world_id, expires_at)
         VALUES (encode($1,'hex')::uuid, statement_timestamp() + interval '1 minute')
         ON CONFLICT (world_id) DO UPDATE SET expires_at=EXCLUDED.expires_at
         WHERE game_forgotten_knowledge_tree_windows.expires_at <= statement_timestamp()",
    )
    .bind(world_id.as_bytes().as_slice())
    .execute(connection)
    .await?;
    Ok(opened.rows_affected() == 1)
}
