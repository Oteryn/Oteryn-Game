//! Use item command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, CAPABILITY_ITEM_USE_V1, ContainerObservation, ContainerViewOutcome,
    ContainerViewState, ExactActorRef, FreshAdmissionAuthority, ItemKey, OpenDecision, PendingOpen,
    SessionItemView, SessionVisibility, UseCommand, UseIntent, UseOutcome, UseTarget,
    decode_use_intent_target,
};
use super::Dispatch;
use transport::item_view::CorpseOpenRefusal;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    container_views: &mut Option<ContainerViewState>,
    visibility: &Option<SessionVisibility>,
) -> Result<Dispatch, &'static str> {
    let decoded = decode_use_intent_target(
        admitted.continuity.selected_capabilities.as_slice(),
        command.payload,
    );
    let ledger = authority.session_commands();
    let reference = crate::foundation::CommandId::new(command.command_id)
        .ok()
        .map(|id| crate::foundation::CommandRef::new(admitted.game_session_id, id));
    if decoded.is_ok() && let (Some(ledger), Some(reference)) = (ledger, reference) {
        ledger.remember_use_wire(reference, command.payload).ok_or("use_receipt_wire")?;
    }
    // Spatial item handles name corpse projections, never a carried rune.
    // Resolve plain corpse clicks before the carried-item receipt probes.
    if let Ok(UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    }) = &decoded
        && let Some(view) = item_view.as_mut()
        && let Some(key @ ItemKey::Entity(_)) = view.resolve(*handle)
    {
        let observed = authority.observe_corpse_for_use(actor, UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: command.command_id,
            item_fence: admitted.item_fence,
        }, key).await;
        let observation = match observed {
            Ok(observation) => observation,
            Err(reason) => return Ok(refuse_corpse_open(&admitted.continuity.selected_capabilities, command.command_id, reason)),
        };
        let opened = view.open(key, observation);
        admitted.continuity.item_view = view.continuity();
        let Ok((decision, delta)) = opened else {
            return Ok(super::refuse_use(command, "corpse_view_projection"));
        };
        return Ok(Dispatch::UseItem(decision, delta));
    }
    // W09: a committed Cube may have consumed the last source unit. Its original
    // command still resolves from a receipt even when the view handle no longer exists.
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
        .map_err(|_| "world_harvest_timeout")?
        .map_err(|_| "world_harvest_unresolved")?
    {
        return Ok(Dispatch::Use(outcome));
    }
    if admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ITEM_USE_V1)
        // Cube receipts belong to Channel sessions. A named-House handoff has
        // its own session and must reach its owner after the shared trade check.
        && !admitted.item_fence.is_some_and(|fence| matches!(fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Instance { .. }))
        && let Ok(intent) = &decoded
        && intent.use_with.is_none()
        && intent.use_with_map_item.is_none()
        && intent.use_with_position.is_none()
        && matches!(
            intent.target,
            UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
        )
    {
        // A committed Cube may have consumed the last source unit. Probe its
        // receipt even if the original handle no longer resolves.
        let source = match intent.target {
            UseTarget::Item(handle) => item_view.as_ref().and_then(|view| view.resolve(handle)),
            _ => None,
        };
        let definition = source.and_then(|key| {
            item_view.as_ref().and_then(|view| view.shown_definition(key))
                .or_else(|| container_views.as_ref().and_then(|views| views.shown_definition(key)))
        });
        let receipt = ledger.zip(reference);
        if let Some((ledger, reference)) = receipt {
            ledger.set_use_receipt(reference, Some(transport::session_ingress::UseReceipt::Drome));
        }
        if let Some(outcome) = authority.use_drome_cube(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            intent.clone(),
            source,
            definition,
        )
        .await
        .map_err(|_| "drome_use_fence_or_unresolved_write")?
        {
            return Ok(Dispatch::Use(outcome));
        }
        if let Some((ledger, reference)) = receipt {
            ledger.set_use_receipt(reference, None);
        }
    }
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
                .await.ok_or("item_trade_authority")?
        {
            return Ok(Dispatch::Use(UseOutcome::rejected()));
        }
    }
    if let Ok(intent) = &decoded
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
            return Ok(Dispatch::Use(outcome));
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
        .map_err(|_| "map_item_timeout")?
    {
        return Ok(Dispatch::Use(outcome));
    }
    // The shared item-action owner also admits equipped hats and player-targeted toys.
    // Map and position uses must reach their own receipt probes before an unrelated
    // event-item inventory check can reject a historical command's stale fence.
    if let Ok(intent) = &decoded
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
        .map_err(|_| "event_item_timeout")?
        .map_err(|_| "event_item_unresolved")?
        {
            return Ok(Dispatch::Use(outcome));
        }
    }
    if let Ok(intent) = &decoded {
        let source = match intent.target {
            UseTarget::Item(handle) => item_view.as_ref().and_then(|view| view.resolve(handle)),
            _ => None,
        };
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
        .map_err(|_| "soul_war_source_timeout")?
        .map_err(|_| "soul_war_source_unresolved")?
        {
            return Ok(Dispatch::Use(outcome));
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
            .map_err(|_| "quest_tool_timeout")?
            .map_err(|_| "quest_tool_unresolved")?
        {
            return Ok(Dispatch::Use(outcome));
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
            Ok(Ok(Some(outcome))) => return Ok(Dispatch::Use(outcome)),
            Ok(Ok(None)) => {}
            Ok(Err(()))
                if matches!(
                    intent.use_with_map_item,
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(_))
                ) =>
            {
                // Base-object USE cannot reach the taming write or the two-carried-item
                // quest write. The base-object callback reports its pending write separately.
                return Ok(Dispatch::Use(UseOutcome::rejected()));
            }
            // Other target arms can lose a taming or quest commit acknowledgement,
            // and a timeout can interrupt the base-object callback while it commits.
            Ok(Err(())) => return Err("mount_taming_unresolved"),
            Err(_) => return Err("mount_taming_timeout"),
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
            Some(key) => match authority
                .observe_container_for_use(actor, admitted.item_fence, key, views.house_capacity_v2())
                .await
            {
                Ok(observation) => observation,
                Err(crate::durability::DurabilityError::SessionUseAuthorization(_)) => return Err("bag_session_fence"),
                Err(crate::durability::DurabilityError::RootUnavailable | crate::durability::DurabilityError::RootPassDeadlineExceeded) => return Ok(super::refuse_use(command, "bag_read_budget")),
                Err(_) => return Ok(super::refuse_use(command, "bag_read_failed")),
            },
            None => None,
        },
        _ => None,
    };
    let opens_container = matches!(
        container,
        Some(ContainerObservation::Container { .. } | ContainerObservation::TooFar)
    );
    Ok({
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
                        .await.map_err(|_| "dispenser_timeout")?.map_err(|_| "dispenser_unresolved")? {
                    return Ok(Dispatch::Use(outcome));
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
                ).await.map_err(|_| "soulpit_timeout")?.map_err(|_| "soulpit_unresolved")? {
                    return Ok(Dispatch::Use(outcome));
                }
                // Converter and catalyst failures can hide a sent, unacknowledged commit.
                if let (Some(source), Some(target)) = (source, target)
                    && let Some(outcome) = authority.use_coin_converter(actor, UseCommand { game_session_id: admitted.game_session_id, command_id: command.command_id, item_fence: admitted.item_fence }, source, target).await.map_err(|_| "coin_converter_unresolved")?
                { return Ok(Dispatch::Use(outcome)); }
                if let Some(outcome) = authority.use_consume_transform(
                    actor,
                    UseCommand { game_session_id: admitted.game_session_id, command_id: command.command_id, item_fence: admitted.item_fence },
                    intent.clone(), source, target,
                ).await { return Ok(Dispatch::Use(outcome)); }
                if let (Some(source), Some(target)) = (source, target)
                    && let Some(outcome) = authority.use_proficiency_catalyst(actor, UseCommand { game_session_id: admitted.game_session_id, command_id: command.command_id, item_fence: admitted.item_fence }, source, target).await.map_err(|_| "proficiency_catalyst_unresolved")?
                { return Ok(Dispatch::Use(outcome)); }
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
                    return Ok(Dispatch::Use(outcome));
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
                    return Ok(Dispatch::Use(outcome));
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
                    return Ok(Dispatch::Use(outcome));
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
                    return Ok(Dispatch::Use(outcome));
                }
                if let Some(outcome) = transport::house_destroy::dispatch_map_use(authority, actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, intent.clone(), item_view.as_ref()).await {
                    return Ok(Dispatch::Use(outcome));
                }
                match transport::mining::dispatch_map_use(authority, actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, intent, item_view.as_ref()).await {
                    Some(outcome) => Dispatch::Use(outcome),
                    None => return Err("mining_unresolved"),
                }
            }
            Ok(intent) if !opens_container
                && transport::rune_use_bridge::routes(
                    &intent, &admitted.continuity.selected_capabilities,
                ) => {
                let stale_plain_handle = intent.use_with.is_none()
                    && intent.use_with_position.is_none()
                    && matches!(intent.target, UseTarget::Item(handle)
                        if item_view.as_ref().and_then(|view| view.resolve(handle)).is_none());
                let outcome = transport::rune_use_bridge::dispatch(
                    authority, actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent, item_view.as_ref(), visibility.as_ref(),
                    &admitted.continuity.selected_capabilities,
                ).await.ok_or("rune_use_unresolved")?;
                if stale_plain_handle && outcome.disposition
                    == oteryn_protocol_oteryn::world_object::UseDisposition::StaleState
                {
                    log_corpse_open_refusal(command.command_id, &CorpseOpenRefusal::HandleStale);
                }
                Dispatch::Use(outcome)
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
                if authority.item_use_pending(actor, admitted.game_session_id).await { return Err("item_use_pending"); }
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
                        ).await.map_err(|_| "carried_item_use_timeout")?
                            .ok_or("carried_item_use_unresolved")?;
                        if authority.item_use_pending(actor, admitted.game_session_id).await { return Err("item_use_pending"); }
                        Dispatch::Use(outcome)
                    },
                    (Some(view), Some(key), _) if container.is_some() => {
                        let views = container_views.as_mut().ok_or("container_view_missing")?;
                        let opened = views.apply(view, PendingOpen::new_view(key), container);
                        admitted.continuity.item_view = view.continuity();
                        let Ok((outcome, delta)) = opened else {
                            return Ok(super::refuse_use(command, "bag_view_projection"));
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
                            return Ok(super::refuse_use(command, "corpse_view_projection"));
                        };
                        Dispatch::UseItem(decision, delta)
                    }
                    (_, None, _) => refuse_corpse_open(&admitted.continuity.selected_capabilities, command.command_id, CorpseOpenRefusal::HandleStale),
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


fn log_corpse_open_refusal(command_id: u64, refusal: &CorpseOpenRefusal) {
    use crate::durability::{DurabilityError, SqlxFailureClass, item_transfer::ItemTransferError};

    let cause = match refusal {
        CorpseOpenRefusal::DurableRead(error) => match error {
            ItemTransferError::AuthorityRejected => "authority",
            ItemTransferError::Unavailable(DurabilityError::RootPassDeadlineExceeded) => "durable_deadline",
            ItemTransferError::Unavailable(DurabilityError::Database(SqlxFailureClass::RetryableStatement)) => {
                "durable_statement_timeout_or_lock"
            }
            ItemTransferError::Unavailable(DurabilityError::InvalidStoredState) => "durable_state",
            ItemTransferError::Unavailable(DurabilityError::Database(_)) => "durable_database",
            _ => "durable_read",
        },
        CorpseOpenRefusal::HolderBusy => "holder_busy",
        CorpseOpenRefusal::HandleStale => "handle_stale",
        CorpseOpenRefusal::CorpseAbsent => "corpse_absent",
        CorpseOpenRefusal::NotOwner => "not_owner",
    };
    transport::operator_event(&format!(
        "event=corpse_open_refused command_id={command_id} cause={cause}",
    ));
}

fn refuse_corpse_open(
    selected: &transport::capabilities::SelectedCapabilities,
    command_id: u64,
    refusal: CorpseOpenRefusal,
) -> Dispatch {
    use crate::durability::{DurabilityError, SqlxFailureClass, item_transfer::ItemTransferError};
    use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, append_refusal_detail};
    use oteryn_protocol_oteryn::world_object::{UseDisposition, encode_use_result};

    log_corpse_open_refusal(command_id, &refusal);
    match refusal {
        CorpseOpenRefusal::HolderBusy
        | CorpseOpenRefusal::DurableRead(ItemTransferError::Unavailable(
            DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::Database(SqlxFailureClass::RetryableStatement),
        )) => {
            let mut payload = encode_use_result(UseDisposition::Rejected).unwrap_or_default();
            append_refusal_detail(&mut payload, selected.as_slice(), RefusalDetail::of_use(UseDisposition::Rejected));
            Dispatch::Reply(crate::foundation::CommandStatus::Rejected, payload)
        }
        CorpseOpenRefusal::NotOwner => Dispatch::Use(UseOutcome::rejected()),
        _ => Dispatch::UseItem(OpenDecision::StaleState, None),
    }
}
