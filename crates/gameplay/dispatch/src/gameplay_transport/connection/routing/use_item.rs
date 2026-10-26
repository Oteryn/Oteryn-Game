//! Use item command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, CAPABILITY_ITEM_USE_V1, ContainerObservation, ContainerViewOutcome,
    ContainerViewState, ExactActorRef, FreshAdmissionAuthority, ItemKey, OpenDecision, PendingOpen,
    SessionItemView, SessionVisibility, UseCommand, UseIntent, UseOutcome, UseTarget,
    decode_use_intent_target,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    container_views: &mut Option<ContainerViewState>,
    visibility: &Option<SessionVisibility>,
) -> Option<Dispatch> {
    let decoded = decode_use_intent_target(
        admitted.continuity.selected_capabilities.as_slice(),
        command.payload,
    );
    if let Ok(intent) = &decoded {
        let source = match intent.target {
            UseTarget::Item(handle) => {
                Some(item_view.as_ref().and_then(|view| view.resolve(handle)))
            }
            UseTarget::ItemByDefinition(_) => Some(None),
            _ => None,
        };
        if let Some(source) = source
            && !authority
                .player_trade_item_available(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    source,
                )
                .await?
        {
            return Some(Dispatch::Use(UseOutcome::rejected()));
        }
    }
    if let Ok(intent) = &decoded
        && intent.use_with.is_some()
    {
        let source = match intent.target {
            UseTarget::Item(handle) => item_view.as_ref().and_then(|v| v.resolve(handle)),
            _ => None,
        };
        if let Some(outcome) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.tame_mount(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent.clone(),
                source,
                intent
                    .use_with
                    .is_some_and(|target| visibility.as_ref().is_some_and(|v| v.shows(target))),
            ),
        )
        .await
        .ok()?
        .ok()?
        {
            return Some(Dispatch::Use(outcome));
        }
    }
    Some({
        match decoded {
            Ok(intent) if intent.use_with_map_item.is_some() => {
                match transport::mining::dispatch_map_use(authority, actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, intent, item_view.as_ref()).await {
                    Some(outcome) => Dispatch::Use(outcome),
                    None => return None,
                }
            }
            Ok(intent) if transport::rune_use_bridge::routes(
                &intent, &admitted.continuity.selected_capabilities,
            ) => match transport::rune_use_bridge::dispatch(
                authority, actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent, item_view.as_ref(), visibility.as_ref(),
            ).await {
                Some(outcome) => Dispatch::Use(outcome),
                None => return None,
            },
            // ITEM-USE-WIRE-1: fields 4 and 5 decode only under capability 15.
            // Creature and unrouted position targets still fail closed here.
            Ok(UseIntent {
                use_with: Some(_), ..
            })
            // WORLDINT-WIRE-1: map targets decode only under capability 11; the
            // mechanic owner above keeps unsupported tool actions closed.
            | Ok(UseIntent {
                use_with_map_item: Some(_),
                ..
            })
            // RUNE-WIRE-1: position uses without a qualified bridge route fail closed.
            | Ok(UseIntent {
                use_with_position: Some(_),
                ..
            }) => Dispatch::Use(UseOutcome::rejected()),
            Ok(UseIntent {
                target: UseTarget::ItemByDefinition(definition),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            }) => Dispatch::Use(authority.use_item_by_definition(actor, UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            }, definition).await),
            // ITEM-VIEW-1b §4.3: a handle that is not live is STALE before any authority
            // call; opening writes nothing.
            Ok(UseIntent {
                target: UseTarget::Item(handle),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            }) => {
                let key = item_view.as_ref().and_then(|view| view.resolve(handle));
                // BAGS-WIRE-1: with capability 14 a container in reach opens in a new view,
                // and one out of reach is TOO_FAR; anything else takes the corpse path.
                let container = match (key, &*container_views) {
                    (Some(key), Some(_)) => authority.observe_container(actor, admitted.item_fence, key).await,
                    _ => None,
                };
                let carried_entry = matches!(container, Some(ContainerObservation::NotAContainer));
                let container = container.filter(|observation| {
                    matches!(
                        observation,
                        ContainerObservation::Container { .. } | ContainerObservation::TooFar
                    )
                });
                // BAGS-USE-1: a carried entry of an open view can be used under capability
                // 15. The owner revalidates custody before PREPARE; equipped sources fail closed.
                let consumable = container.is_none()
                    && matches!(key, Some(ItemKey::Instance(_)))
                    && admitted
                        .continuity
                        .selected_capabilities
                        .contains(CAPABILITY_ITEM_USE_V1)
                    && item_view
                        .as_ref()
                        .zip(key)
                        .is_some_and(|(view, key)| view.is_backpack_entry(&key)
                            || (carried_entry && !view.is_main_backpack(&key)
                                && view.equipped_slot(&key).is_none()));
                let observation = match key.filter(|_| container.is_none() && !consumable) {
                    Some(key) => authority.observe_item_target(actor, key).await,
                    None => None,
                };
                match (item_view.as_mut(), key, observation) {
                    (_, Some(key), _) if consumable => {
                        let outcome = tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            authority.use_item_completion(
                                actor,
                                UseCommand {
                                    game_session_id: admitted.game_session_id,
                                    command_id: command.command_id,
                                    item_fence: admitted.item_fence,
                                },
                                key,
                            ),
                        ).await.ok().flatten()?;
                        Dispatch::Use(outcome)
                    },
                    (Some(view), Some(key), _) if container.is_some() => {
                        let views = container_views.as_mut()?;
                        let opened = views.apply(view, PendingOpen::new_view(key), container);
                        admitted.continuity.item_view = view.continuity();
                        let Ok((outcome, delta)) = opened else {
                            return None;
                        };
                        Dispatch::UseItem(
                            match outcome {
                                ContainerViewOutcome::Opened => OpenDecision::Open,
                                ContainerViewOutcome::TooFar => OpenDecision::TooFar,
                                ContainerViewOutcome::Stale | ContainerViewOutcome::Closed => {
                                    OpenDecision::StaleState
                                }
                                ContainerViewOutcome::NotAContainer
                                | ContainerViewOutcome::TooManyViews => {
                                    OpenDecision::NothingToUse
                                }
                            },
                            delta,
                        )
                    }
                    (Some(view), Some(key), Some(observation)) => {
                        let opened = view.open(key, observation);
                        admitted.continuity.item_view = view.continuity();
                        let Ok((decision, delta)) = opened else {
                            return None;
                        };
                        Dispatch::UseItem(decision, delta)
                    }
                    _ => Dispatch::UseItem(OpenDecision::StaleState, None),
                }
            }
            Ok(UseIntent {
                target: UseTarget::WorldObject(target),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            }) => Dispatch::Use(
                authority
                    .use_object(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        target,
                    )
                    .await,
            ),
            Err(_) => Dispatch::Use(UseOutcome::rejected()),
        }
    })
}
