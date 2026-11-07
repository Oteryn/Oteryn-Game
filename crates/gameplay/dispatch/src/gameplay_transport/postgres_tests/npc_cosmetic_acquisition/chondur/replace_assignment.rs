//! Existing ordinary assignment replacement after actual logout, before frame cutover.
use super::*;
use crate::durability::runtime_scope_assignment::*;
use crate::foundation::{ChannelId, WorldId};
pub(super) async fn replace(h: &Harness, world: WorldId, channel: ChannelId) -> TestResult<u64> {
    let sessions:i64=sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE world_id=encode($1,'hex')::uuid AND session_state<>3")
  .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        sessions, 0,
        "genuine terminal source session before frame lifecycle transition"
    );
    assert_eq!(
        h.root.active_world_reset_epoch(world).await?,
        None,
        "no WorldReset bypass or reset-frame cutover exemption"
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2) ON CONFLICT DO NOTHING")
  .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("current source assignment")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([0xc9; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: h.node.fact()
                }
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    let generation:String=sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
  .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    Ok(generation.parse()?)
}
