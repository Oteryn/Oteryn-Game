use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView, UseCommand,
    UseOutcome,
};
use super::Dispatch;
use oteryn_protocol_oteryn::world_object::UseTarget;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, world_rotate as wire};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    items: &Option<SessionItemView>,
) -> Option<Dispatch> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_WORLD_ROTATE_V1)
    {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let Ok(request) = wire::decode_rotate_request(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let wire::RotateRequest {
        intent,
        observed_definition: observed,
        probe,
    } = request;
    let source = match intent.target {
        UseTarget::Item(handle) => items.as_ref().and_then(|v| v.resolve(handle)),
        _ => None,
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    if probe {
        // This is only an affordance read. An unavailable advisory answer is
        // false; no mutation can have crossed a commit boundary.
        let allowed = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.can_rotate_item(actor, command, source, observed),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);
        return Some(Dispatch::Reply(
            CommandStatus::Accepted,
            wire::encode_rotate_affordance(allowed),
        ));
    }
    if matches!(intent.target, UseTarget::Item(_))
        && !authority
            .player_trade_item_available(actor, command, source)
            .await?
    {
        return Some(Dispatch::Use(UseOutcome::rejected()));
    }
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        authority.rotate_world_item(actor, command, intent, source, observed),
    )
    .await
    .ok()?
    .ok()?
    .unwrap_or_else(UseOutcome::rejected);
    Some(Dispatch::Use(outcome))
}
