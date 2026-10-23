//! Lost acknowledgement replay through real dispatch after rebuilding the native DB owner.
use super::*;

pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    change: &appearance::Change,
) -> TestResult {
    use crate::gameplay_transport::connection::appearance_change::dispatch;
    let payload = change.encode().map_err(debug)?;
    assert_eq!(
        dispatch(owner, actor, command, &payload).await,
        Some(appearance::ChangeResult::Committed(2))
    );
    let mut successor = command.item_fence.ok_or("fence")?;
    successor.connection_generation =
        crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
    // Historical settlement remains available without current connection authority.
    assert_eq!(
        dispatch(
            owner,
            actor,
            UseCommand {
                item_fence: Some(successor),
                ..command
            },
            &payload
        )
        .await,
        Some(appearance::ChangeResult::Committed(2))
    );
    for case in 0..5 {
        let mut rebound = change.clone();
        match case {
            0 => rebound.selection.colours[0] += 1,
            1 => rebound.expected_root_revision += 1,
            2 => rebound.context.content_generation[0] ^= 1,
            3 => rebound.context.cosmetics_digest[0] ^= 1,
            _ => rebound.content_revision = "content-rebound".into(),
        }
        assert_eq!(
            dispatch(owner, actor, command, &rebound.encode().map_err(debug)?).await,
            Some(appearance::ChangeResult::Rejected)
        );
    }
    Ok(())
}
