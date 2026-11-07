//! The existing House USE_WITH boundary admits only its closed paid statue owner.
use super::*;
pub(super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    items: &Option<SessionItemView>,
) -> Option<Dispatch> {
    use oteryn_protocol_oteryn::world_object::MapUseTarget;
    let rejected = || Some(Dispatch::Use(UseOutcome::rejected()));
    let Ok(intent) = decode_use_intent_target(
        admitted.continuity.selected_capabilities.as_slice(),
        command.payload,
    ) else {
        return rejected();
    };
    if matches!(intent.target, UseTarget::WorldObject(_)) {
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_house_static(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent,
            ),
        )
        .await
        .ok()?
        .ok()?;
        return Some(Dispatch::Use(outcome.unwrap_or_else(UseOutcome::rejected)));
    }
    let (UseTarget::Item(source), Some(MapUseTarget::Item(target))) =
        (&intent.target, &intent.use_with_map_item)
    else {
        return rejected();
    };
    if intent.use_with.is_some() || intent.use_with_position.is_some() {
        return rejected();
    }
    let source = items.as_ref().and_then(|view| view.resolve(*source));
    let target = items.as_ref().and_then(|view| view.resolve(*target));
    // The concrete owner independently rechecks actualcarried677 and exactinactive
    //9035/9040 under the current House fence. None is unrelated; Err is uncertain.
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        authority.use_house_dracoyle(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            intent,
            source,
            target,
        ),
    )
    .await
    .ok()?
    .ok()?;
    Some(Dispatch::Use(outcome.unwrap_or_else(UseOutcome::rejected)))
}
