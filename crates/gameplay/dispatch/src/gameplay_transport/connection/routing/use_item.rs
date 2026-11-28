//! Use item command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, CAPABILITY_ITEM_USE_V1, ContainerObservation, ContainerViewOutcome,
    ContainerViewState, ExactActorRef, FreshAdmissionAuthority, ItemKey, ItemUseKind, OpenDecision, PendingOpen,
    SessionItemView, SessionVisibility, UseCommand, UseIntent, UseOutcome, UseOwnerError, UseTarget,
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
    let ledger = authority.session_commands();
    let reference = crate::foundation::CommandId::new(command.command_id)
        .ok()
        .map(|id| crate::foundation::CommandRef::new(admitted.game_session_id, id));
    if decoded.is_ok() && let (Some(ledger), Some(reference)) = (ledger, reference) {
        ledger.remember_use_wire(reference, command.payload)?;
    }
    // A harvest can have sent its durable write before an opaque error or timeout.
    let use_command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    let selected = admitted.continuity.selected_capabilities;
    let item_use = selected.contains(CAPABILITY_ITEM_USE_V1);
    let plain = decoded.as_ref().is_ok_and(|intent| {
        intent.use_with.is_none()
            && intent.use_with_map_item.is_none()
            && intent.use_with_position.is_none()
    });
    let source = match decoded.as_ref().ok().map(|intent| &intent.target) {
        Some(UseTarget::Item(handle)) => item_view.as_ref().and_then(|view| view.resolve(*handle)),
        _ => None,
    };
    let definition = match decoded.as_ref().ok().map(|intent| &intent.target) {
        Some(UseTarget::ItemByDefinition(definition)) => Some(*definition),
        _ => source.and_then(|key| {
            item_view.as_ref()?.backpack_entry_definition(key).or_else(|| {
                container_views.as_ref()?.carried_entry_definition(key)
            })
        }),
    };
    let kind = match definition {
        Some(definition) => authority.item_use_kind(definition).await,
        None => None,
    };
    let opens_item = plain && (
        matches!(source, Some(ItemKey::Entity(_) | ItemKey::BrowseField { .. }))
        || source.is_some_and(|key| {
            item_view.as_ref().is_some_and(|view| view.is_main_backpack(&key))
        })
        || kind == Some(ItemUseKind::Container) && source.is_some()
    );

    if item_use && plain && kind == Some(ItemUseKind::Food)
        && let Some(definition) = definition
    {
        let outcome = authority.use_known_food(actor, use_command, source, definition).await;
        if authority.item_use_pending(actor, admitted.game_session_id).await {
            return None;
        }
        return owner_result(
            command.command_id, "food", outcome.map(|value| value.unwrap_or_else(UseOutcome::rejected)),
        );
    }
    let rune_owner = if let Ok(intent) = &decoded
        && transport::rune_use_bridge::accepts_intent_shape(intent, &selected)
        && !opens_item
    {
        kind == Some(ItemUseKind::Rune)
            || (kind.is_none() && authority.retained_rune_use(actor, use_command, intent).await)
    } else {
        false
    };
    if let Ok(UseIntent {
        target: UseTarget::WorldObject(target),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    }) = &decoded
        && let Some(outcome) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_event_world_object(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                target.clone(),
            ),
        )
        .await
        .ok()?
        .ok()?
    {
        return Some(Dispatch::Use(outcome));
    }
    // Cube receipts belong to the Channel. Replay also admits a retired handle,
    // after excluding any exact retained rune USE and other known owners.
    if item_use && plain && !rune_owner
        && !admitted.item_fence.is_some_and(|fence| matches!(fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Instance { .. }))
        && !opens_item
        && matches!(kind, None | Some(ItemUseKind::Cube | ItemUseKind::KooldownAid))
        && let Ok(intent) = &decoded
        && matches!(intent.target, UseTarget::Item(_) | UseTarget::ItemByDefinition(_))
        && !matches!(source, Some(ItemKey::MapBase { .. } | ItemKey::MapAdded(_)))
    {
        let receipt = ledger.zip(reference);
        if let Some((ledger, reference)) = receipt {
            ledger.set_use_receipt(reference, Some(transport::session_ingress::UseReceipt::Drome));
        }
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_drome_cube(actor, use_command, intent.clone(), source),
        )
        .await
        {
            Ok(Ok(Some(outcome))) => return Some(Dispatch::Use(outcome)),
            Ok(Ok(None)) => {
                if let Some((ledger, reference)) = receipt {
                    ledger.set_use_receipt(reference, None);
                }
            }
            Ok(Err(())) | Err(_) => {
                transport::operator_event("drome_cube_use failed: owner_unavailable_or_deadline");
                // The receipt read may have failed before identifying its owner.
                return None;
            }
        }
    }
    if let Ok(UseIntent { target, .. }) = &decoded
        && matches!(target, UseTarget::Item(_) | UseTarget::ItemByDefinition(_))
        && !authority.player_trade_item_available(actor, use_command, source).await?
    {
        return Some(Dispatch::Use(UseOutcome::rejected()));
    }

    if item_use && plain && kind == Some(ItemUseKind::Stone)
        && let Some(definition) = definition
    {
        let outcome = authority.use_stone(actor, use_command, source, definition).await;
        return owner_result(command.command_id, "stone", outcome);
    }
    if item_use
        && kind == Some(ItemUseKind::Catalyst)
        && let Some(source) = source
        && let Ok(intent) = &decoded
        && let Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(handle)) =
            intent.use_with_map_item
        && let Some(target) = item_view.as_ref().and_then(|view| view.resolve(handle))
    {
        return owner_result(
            command.command_id,
            "proficiency_catalyst",
            authority
                .use_proficiency_catalyst(actor, use_command, source, target)
                .await
                .map(|outcome| outcome.unwrap_or_else(UseOutcome::rejected)),
        );
    }
    if rune_owner && let Ok(intent) = &decoded
    {
        let outcome = transport::rune_use_bridge::dispatch(
            authority, actor, use_command, intent.clone(),
            item_view.as_ref(), visibility.as_ref(), &selected,
        )
        .await;
        return owner_result(command.command_id, "rune", outcome);
    }
    if !opens_item && let Ok(intent) = &decoded
        && intent.use_with.is_none()
        && intent.use_with_map_item.is_none()
        && intent.use_with_position.is_none()
        && matches!(intent.target, UseTarget::Item(_) | UseTarget::WorldObject(_))
        && admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1,
        )
        && admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
        )
    {
        let resolved = match intent.target {
            UseTarget::Item(handle) => item_view.as_ref().and_then(|view| view.resolve(handle)),
            _ => None,
        };
        if let Some(outcome) = authority.server_modal_offer_bed(*admitted, intent.target.clone(), resolved).await {
            return Some(Dispatch::Use(outcome));
        }
    }
    // A map handle reaches only its explicit world owner. It never widens the
    // carried-item or container admissions below; stale handles keep their refusal.
    // A ladder timeout can arrive after a movement-triggered quest write was sent.
    if let Ok(UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    }) = &decoded
        && admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_USE_V1)
        && let Some(key @ (ItemKey::MapBase { .. } | ItemKey::MapAdded(_))) =
            item_view.as_ref().and_then(|view| view.resolve(*handle))
        && let Some(outcome) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_map_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                key,
            ),
        )
        .await
        .ok()?
    {
        return Some(Dispatch::Use(outcome));
    }
    // The shared item-action owner also admits equipped hats and player-targeted toys.
    // Map and position uses must reach their own receipt probes before an unrelated
    // event-item inventory check can reject a historical command's stale fence.
    if !opens_item && kind != Some(ItemUseKind::SoulWar)
        && let Ok(intent) = &decoded
        && (intent.use_with_map_item.is_none()
            || (admitted.item_fence.is_some_and(|fence| {
                matches!(
                    fence.runtime_scope,
                    crate::foundation::RuntimeScopeRefV1::Instance { .. }
                )
            }) && matches!(
                intent.use_with_map_item.as_ref(),
                Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
                    _
                ))
            )))
        && intent.use_with_position.is_none()
        && admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_USE_V1)
        && let UseTarget::Item(handle) = intent.target
    {
        let source = item_view.as_ref().and_then(|view| view.resolve(handle));
        // Event-item commits may have been sent before the owner returns an opaque error.
        if let Some(outcome) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_event_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent.clone(),
                source,
                intent.use_with.is_none_or(|target| {
                    visibility.as_ref().is_some_and(|view| view.shows(target))
                }),
            ),
        )
        .await
        .ok()?
        .ok()?
        {
            return Some(Dispatch::Use(outcome));
        }
    }
    if !opens_item
        && let Ok(intent) = &decoded
        && matches!(intent.target, UseTarget::Item(_) | UseTarget::ItemByDefinition(_))
        && (kind.is_none() || kind == Some(ItemUseKind::SoulWar)
            || intent.use_with.is_some())
    {
        let target = match intent.use_with_map_item {
            Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(handle)) => {
                item_view.as_ref().and_then(|view| view.resolve(handle))
            }
            _ => None,
        };
        // Soul War source consumption may already be committing on an opaque failure.
        if let Some(outcome) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.use_soul_war_source_items(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent.clone(),
                source,
                target,
            ),
        )
        .await
        .ok()?
        .ok()?
        {
            return Some(Dispatch::Use(outcome));
        }
    }
    if let Ok(intent) = &decoded
        && (intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || (intent.use_with_position.is_none()
                && matches!(intent.target, UseTarget::ItemByDefinition(_))))
    {
        let source = match intent.target {
            UseTarget::Item(handle) => item_view.as_ref().and_then(|v| v.resolve(handle)),
            _ => None,
        };
        let item_target = match intent.use_with_map_item {
            Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(handle)) => {
                item_view.as_ref().and_then(|v| v.resolve(handle))
            }
            _ => None,
        };
        // Probe the quest receipt before fresh handle/custody admission. A
        // committed sample remains replayable after either handle disappears.
        // An opaque failure can follow a sent quest-tool commit.
        if transport::cosmetic_item_use::quest_targets::probes(intent)
            && let Some(outcome) = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                authority.use_quest_tool(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent.clone(),
                    source,
                    item_target,
                ),
            )
            .await
            .ok()?
            .ok()?
        {
            return Some(Dispatch::Use(outcome));
        }
        match tokio::time::timeout(
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
                item_target,
                intent
                    .use_with
                    .is_some_and(|target| visibility.as_ref().is_some_and(|v| v.shows(target))),
            ),
        )
        .await
        {
            Ok(Ok(Some(outcome))) => return Some(Dispatch::Use(outcome)),
            Ok(Ok(None)) => {}
            Ok(Err(()))
                if matches!(
                    intent.use_with_map_item,
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(_))
                ) =>
            {
                // Base-object USE cannot reach the taming write or the two-carried-item
                // quest write. The base-object callback reports its pending write separately.
                return Some(Dispatch::Use(UseOutcome::rejected()));
            }
            // Other target arms can lose a taming or quest commit acknowledgement,
            // and a timeout can interrupt the base-object callback while it commits.
            Ok(Err(())) | Err(_) => return None,
        }
    }
    // Direct USE can open a reachable container even with rune USE selected.
    // Only probe plain handles, leaving targeted actions and their receipts alone.
    let container = match (&decoded, item_view.as_ref(), container_views.as_ref()) {
        (
            Ok(UseIntent {
                target: UseTarget::Item(handle),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            }),
            Some(view),
            Some(views),
        ) => match view.resolve(*handle) {
            Some(key) => authority
                .observe_container_with_house_capacity(
                    actor,
                    admitted.item_fence,
                    key,
                    views.house_capacity_v2(),
                )
                .await,
            None => None,
        },
        _ => None,
    };
    let opens_container = matches!(
        container,
        Some(ContainerObservation::Container { .. } | ContainerObservation::TooFar)
    );
    Some({
        match decoded {
            Ok(intent) if intent.use_with_map_item.is_some() => {
                let source = match intent.target {
                    UseTarget::Item(handle) => item_view.as_ref().and_then(|v| v.resolve(handle)),
                    _ => None,
                };
                let target = match intent.use_with_map_item {
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(handle)) => {
                        item_view.as_ref().and_then(|v| v.resolve(handle))
                    }
                    _ => None,
                };
                // The dispenser mutation can have been sent before an opaque failure.
                if admitted.item_fence.is_some_and(|fence| matches!(fence.runtime_scope,
                    crate::foundation::RuntimeScopeRefV1::Instance{..}))
                    && let Some(outcome)=tokio::time::timeout(std::time::Duration::from_secs(10),
                        authority.use_dispenser_action(actor,UseCommand{game_session_id:admitted.game_session_id,
                            command_id:command.command_id,item_fence:admitted.item_fence},intent.clone(),source,target))
                        .await.ok()?.ok()? {
                    return Some(Dispatch::Use(outcome));
                }
                // Carried targets otherwise enter the pick owner directly. Probe W07
                // here, including historical receipts whose source handle was retired.
                // An opaque failure can follow a sent Soulpit entry or soul-core commit.
                if let Some(outcome) = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    authority.use_soulpit_items(
                        actor,
                        UseCommand { game_session_id: admitted.game_session_id,
                            command_id: command.command_id, item_fence: admitted.item_fence },
                        intent.clone(), source, target,
                    ),
                ).await.ok()?.ok()? {
                    return Some(Dispatch::Use(outcome));
                }
                // Converter and catalyst failures can hide a sent, unacknowledged commit.
                if let (Some(source), Some(target)) = (source, target)
                    && let Some(outcome) = authority.use_coin_converter(actor, UseCommand { game_session_id: admitted.game_session_id, command_id: command.command_id, item_fence: admitted.item_fence }, source, target).await.ok()?
                { return Some(Dispatch::Use(outcome)); }
                if let Some(outcome) = authority.use_consume_transform(
                    actor,
                    UseCommand { game_session_id: admitted.game_session_id, command_id: command.command_id, item_fence: admitted.item_fence },
                    intent.clone(), source, target,
                ).await { return Some(Dispatch::Use(outcome)); }
                if let (Some(source), Some(target)) = (source, target)
                    && let Some(outcome) = authority
                        .use_fluid_container(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            source,
                            target,
                        )
                        .await
                {
                    return Some(Dispatch::Use(outcome));
                }
                if let Some(outcome) = authority
                    .use_imbuement_item(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        intent.clone(),
                        source,
                        target,
                    )
                    .await
                {
                    return Some(Dispatch::Use(outcome));
                }
                if let (
                    Some(source),
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(door)),
                ) = (source, intent.use_with_map_item.clone())
                    && let Some(outcome) = authority
                        .use_key_on_door(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            source,
                            door,
                        )
                        .await
                {
                    return Some(Dispatch::Use(outcome));
                }
                if let (
                    Some(source),
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target)),
                ) = (source, intent.use_with_map_item.clone())
                    && let Some(outcome) = authority
                        .use_fluid_on_map(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            source,
                            target,
                        )
                        .await
                {
                    return Some(Dispatch::Use(outcome));
                }
                if let Some(outcome) = transport::house_destroy::dispatch_map_use(authority, actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, intent.clone(), item_view.as_ref()).await {
                    return Some(Dispatch::Use(outcome));
                }
                match transport::mining::dispatch_map_use(authority, actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, intent, item_view.as_ref()).await {
                    Some(outcome) => Dispatch::Use(outcome),
                    // A map-tool or craft owner can lose the acknowledgement after sending COMMIT.
                    None => return None,
                }
            }
            // Preserve the USE disposition for a targeted handle with no specific owner.
            // The rune adapter cannot execute or diagnose a non-rune source.
            Ok(intent) if transport::rune_use_bridge::accepts_intent_shape(&intent, &selected)
                && (intent.use_with.is_some() || intent.use_with_position.is_some()) => {
                Dispatch::Use(UseOutcome {
                    disposition: if source.is_some() {
                        oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
                    } else {
                        oteryn_protocol_oteryn::world_object::UseDisposition::StaleState
                    },
                    ..UseOutcome::rejected()
                })
            }
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
            }) => {
                let outcome = authority.use_item_by_definition(actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, definition).await;
                // A retained native item write must reconcile before acknowledging USE.
                if authority.item_use_pending(actor, admitted.game_session_id).await { return None; }
                Dispatch::Use(outcome)
            },
            // ITEM-VIEW-1b §4.3: a handle that is not live is STALE before any authority
            // call; opening writes nothing.
            Ok(UseIntent {
                target: UseTarget::Item(handle),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            }) => {
                let key = item_view.as_ref().and_then(|view| view.resolve(handle));
                let carried_entry = matches!(container, Some(ContainerObservation::NotAContainer));
                let container = container.filter(|_| opens_container);
                // BAGS-USE-1: a carried entry of an open view can be used under capability
                // 15. The owner revalidates custody before PREPARE; equipped sources fail closed.
                let consumable = !opens_item && container.is_none()
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
                        // The consumable writer may have sent a commit before its timeout or lost ACK.
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
                        if authority.item_use_pending(actor, admitted.game_session_id).await { return None; }
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

#[cfg(test)]
mod fluid_routing_tests {
    #[test]
    fn donor_generic_fluid_handler_runs_after_specific_map_handlers() {
        let route = include_str!("use_item.rs");
        let fluid = route.find(".use_fluid_on_map(");
        let imbuement = route.find(".use_imbuement_item(");
        let key = route.find(".use_key_on_door(");
        assert!(
            fluid.is_some() && imbuement.is_some() && key.is_some(),
            "handlers present"
        );
        assert!(imbuement < fluid, "imbuement handler before fluid");
        assert!(key < fluid, "key handler before fluid");
    }
}

#[cfg(test)]
#[path = "use_item/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "use_item/owner_tests.rs"]
mod owner_tests;

fn owner_result(
    command_id: u64,
    owner: &'static str,
    outcome: Result<UseOutcome, UseOwnerError>,
) -> Option<Dispatch> {
    match outcome {
        Ok(outcome) => Some(Dispatch::Use(outcome)),
        Err(error) => {
            let cause = match error {
                UseOwnerError::HolderBusy => "holder_busy",
                UseOwnerError::OutcomeUnknown => "outcome_unknown",
            };
            transport::operator_event(&format!(
                "event=use_owner_unavailable command_id={command_id} owner={owner} cause={cause}"
            ));
            match error {
                UseOwnerError::HolderBusy => Some(Dispatch::UseUnavailable),
                UseOwnerError::OutcomeUnknown => None,
            }
        }
    }
}
