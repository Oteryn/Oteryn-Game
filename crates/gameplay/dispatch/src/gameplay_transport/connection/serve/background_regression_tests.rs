use super::*;
use crate::gameplay_transport::container_view::ContainerPlan;
use oteryn_protocol_oteryn::item_view::{
    COMMAND_TYPE_ITEM_MOVE_INTENT, ItemMoveDestination, ItemMoveIntent, encode_item_move_intent,
};
use std::num::NonZeroU32;

#[derive(Default)]
struct MoveOwner {
    moved: Cell<bool>,
    purchases: Cell<u8>,
    inventory_busy: Cell<bool>,
    refused: Cell<bool>,
    reads: Cell<usize>,
    background_operations: Cell<usize>,
}

fn item(tag: u8) -> ViewItem {
    ViewItem {
        appearance_id: 0,
        key: ItemKey::Instance([tag; 16]),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

impl FreshAdmissionAuthority for MoveOwner {
    async fn background_operation<F: Future>(&self, future: F) -> F::Output {
        self.background_operations
            .set(self.background_operations.get() + 1);
        future.await
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(observation())
    }

    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        if self.inventory_busy.get() {
            return None;
        }
        let mut entries = vec![item(2)];
        if self.moved.get() {
            entries.push(item(3));
        }
        entries.extend((0..self.purchases.get()).map(|index| item(index + 4)));
        Some(InventoryItems {
            main_backpack: Some(item(1)),
            entries,
            equipment: Vec::new(),
        })
    }

    async fn npc_trade_with_delivery(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
        _: bool,
    ) -> NpcTradeStep {
        assert_eq!(
            intent.side,
            oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy
        );
        self.purchases.set(self.purchases.get() + 1);
        NpcTradeStep::Traded
    }

    async fn observe_character_equipment(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<Vec<(EquipmentSlot, ViewItem)>> {
        Some(Vec::new())
    }

    async fn observe_container(
        &self,
        _: ExactActorRef,
        _: Option<CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<ContainerObservation> {
        assert_eq!(target, item(2).key);
        self.reads.set(self.reads.get() + 1);
        if self.refused.get() {
            return None;
        }
        Some(ContainerObservation::Container {
            parent: Some(item(1).key),
            capacity: 20,
            entries: if self.moved.get() {
                Vec::new()
            } else {
                vec![item(3)]
            },
        })
    }

    async fn move_loop_item(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        request: crate::gameplay_transport::item_move::LoopMove,
    ) -> crate::gameplay_transport::item_move::LoopMoveResult {
        assert_eq!(request.item, item(3).key);
        self.moved.set(true);
        self.refused.set(true);
        crate::gameplay_transport::item_move::LoopMoveResult::Committed
    }
}

#[test]
fn item_move_refused_container_without_browse_retries_next_visibility_tick() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = MoveOwner::default();
                    let (mut stream, _client) = tokio::io::duplex(1 << 20);
                    let mut admitted = admitted(false)?;
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[4, 6, 12, 14],
                    )
                    .ok_or("select container capabilities")?;
                    let actor = admitted.runtime_actor.ok_or("actor")?;
                    let mut session = setup::prepare(
                        &mut stream,
                        &mut admitted,
                        &authority,
                        IDLE_LIVENESS,
                        actor,
                        observation(),
                        integrity::Integrity::default(),
                    )
                    .await
                    .map_err(|_| "setup")?;
                    assert!(session.selected_capabilities.contains(14));
                    assert!(!session.selected_capabilities.contains(230));
                    let view = session.item_view.as_mut().ok_or("item view")?;
                    let views = session.container_views.as_mut().ok_or("container views")?;
                    let handle = view.table().handle(&item(2).key).ok_or("bag handle")?;
                    let ContainerPlan::Observe(pending) = views
                        .plan(
                            view,
                            ContainerViewIntent::Open {
                                handle,
                                replace_view: None,
                            },
                        )
                        .map_err(|e| format!("{e:?}"))?
                    else {
                        return Err("open plan".into());
                    };
                    let observed = authority.observe_container(actor, None, item(2).key).await;
                    views
                        .apply(view, pending, observed)
                        .map_err(|e| format!("{e:?}"))?;
                    let revision = view.continuity().views_revision;
                    let source = view.table().handle(&item(3).key).ok_or("source handle")?;
                    let payload = encode_item_move_intent(&ItemMoveIntent {
                        source,
                        destination: ItemMoveDestination::MainBackpack,
                    })
                    .map_err(|e| format!("{e:?}"))?;
                    let frame = command_frame(1, COMMAND_TYPE_ITEM_MOVE_INTENT, &payload)?;
                    let input = receive(
                        &mut stream,
                        &authority,
                        &mut session,
                        Next::Held(frame, false, tokio::time::Instant::now()),
                    )
                    .await
                    .map_err(|_| "receive move")?
                    .ok_or("move input")?;
                    command::execute(&mut stream, &authority, &mut session, input)
                        .await
                        .map_err(|_| "execute move")?;
                    assert!(
                        authority.moved.get(),
                        "ITEM_MOVE must commit through dispatch"
                    );
                    assert_eq!(authority.background_operations.get(), 0);
                    assert_eq!(
                        session.container_views.as_ref().map(|v| v.open_views()),
                        Some(1)
                    );
                    assert!(
                        session
                            .item_view
                            .as_ref()
                            .ok_or("view")?
                            .inventory_refresh_pending,
                        "refused ITEM_MOVE follow-up must schedule a retry"
                    );
                    // A failed retry must also survive the inventory helper's completion.
                    receive(&mut stream, &authority, &mut session, Next::Visibility)
                        .await
                        .map_err(|_| "refused tick")?;
                    assert!(
                        session
                            .item_view
                            .as_ref()
                            .ok_or("view")?
                            .inventory_refresh_pending
                    );
                    let reads = authority.reads.get();
                    authority.refused.set(false);
                    receive(&mut stream, &authority, &mut session, Next::Visibility)
                        .await
                        .map_err(|_| "retry tick")?;
                    assert_eq!(authority.reads.get(), reads + 1);
                    let view = session.item_view.as_ref().ok_or("view")?;
                    assert!(!view.inventory_refresh_pending);
                    assert_eq!(authority.background_operations.get(), 2);
                    assert_eq!(
                        view.continuity().views_revision,
                        revision + 1,
                        "next tick updates the open container"
                    );
                    assert_eq!(
                        session.container_views.as_ref().map(|v| v.open_views()),
                        Some(1)
                    );
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "container retry test thread panicked")?
}

#[test]
fn committed_mail_and_trade_results_invalidate_capacity() -> TestResult {
    use oteryn_protocol_oteryn::{mail, player_trade};
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = MoveOwner::default();
                    let (mut stream, _client) = tokio::io::duplex(1 << 20);
                    let mut admitted = admitted(false)?;
                    let actor = admitted.runtime_actor.ok_or("actor")?;
                    let mut session = setup::prepare(
                        &mut stream,
                        &mut admitted,
                        &authority,
                        IDLE_LIVENESS,
                        actor,
                        observation(),
                        integrity::Integrity::default(),
                    )
                    .await
                    .map_err(|_| "setup")?;
                    for (command_type, payload, invalidates) in [
                        (
                            u32::from(mail::COMMAND_TYPE_MAIL),
                            vec![mail::MailOutcome::Posted as u8],
                            true,
                        ),
                        (
                            u32::from(mail::COMMAND_TYPE_MAIL),
                            vec![mail::MailOutcome::TextWritten as u8],
                            false,
                        ),
                        (
                            player_trade::COMMAND_TYPE_PLAYER_TRADE_INTENT,
                            player_trade::ResultCode::Completed.payload(),
                            true,
                        ),
                        (
                            player_trade::COMMAND_TYPE_PLAYER_TRADE_INTENT,
                            player_trade::ResultCode::Accepted.payload(),
                            false,
                        ),
                    ] {
                        let deadline =
                            tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                        session.inventory_capacity_refresh_at = Some(deadline);
                        session.sequence += 1;
                        let mut dispatch = Dispatch::Reply(CommandStatus::Accepted, payload);
                        result::encode(
                            &mut session,
                            oteryn_protocol_oteryn::ClientCommandView {
                                command_id: 1,
                                command_type,
                                payload: &[],
                            },
                            &mut dispatch,
                        )
                        .map_err(|_| "command result")?;
                        assert_eq!(
                            session.inventory_capacity_refresh_at,
                            (!invalidates).then_some(deadline)
                        );
                    }
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "capacity result thread")?
}

#[test]
fn committed_npc_purchases_keep_command_attribution_through_inventory_read_retry() -> TestResult {
    use crate::gameplay_transport::tcp_tls::read_frame;
    use oteryn_protocol_oteryn::{
        decode_command_result, decode_state_delta, npc_service as wire,
        world_spatial_entities::EntityRef,
    };
    use std::time::Duration;

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    for caps in [
                        &[3, 4, 6][..],
                        &[3, 4, 6, wire::CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1],
                    ] {
                        let selected = caps.contains(&wire::CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1);
                        let authority = MoveOwner::default();
                        let (mut stream, mut client) = tokio::io::duplex(1 << 20);
                        let mut admitted = admitted(false)?;
                        admitted.continuity.selected_capabilities =
                            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, caps)
                                .ok_or("select NPC purchase capabilities")?;
                        assert_eq!(admitted.continuity.selected_capabilities.contains(
                            wire::CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1
                        ), selected);
                        let actor = admitted.runtime_actor.ok_or("actor")?;
                        let mut session = setup::prepare(
                            &mut stream, &mut admitted, &authority, IDLE_LIVENESS,
                            actor, observation(), integrity::Integrity::default(),
                        ).await.map_err(|_| "setup")?;
                        let payload = wire::encode_npc_trade_intent(&wire::NpcTradeIntent {
                            npc_actor: EntityRef { identity: [1; oteryn_protocol_oteryn::world_spatial_entities::ENTITY_IDENTITY_BYTES], generation: 1 },
                            catalogue_revision: 1,
                            offer_index: 0,
                            side: wire::NpcTradeSide::Buy,
                            quantity: 1,
                            expected_unit_price: 10,
                        }).map_err(|error| format!("{error:?}"))?;
                        let mut ids = Vec::new();
                        authority.inventory_busy.set(true);
                        for id in 1..=2 {
                            let frame = command_frame(id, wire::COMMAND_TYPE_NPC_TRADE_INTENT, &payload)?;
                            let input = receive(
                                &mut stream, &authority, &mut session,
                                Next::Held(frame, false, tokio::time::Instant::now()),
                            ).await.map_err(|_| "receive NPC purchase")?.ok_or("purchase input")?;
                            assert!(
                                command::execute(&mut stream, &authority, &mut session, input).await.is_ok(),
                                "committed NPC purchase must answer OK when the inventory read fails",
                            );
                            loop {
                                let bytes = tokio::time::timeout(Duration::from_secs(2), read_frame(&mut client)).await??;
                                let envelope = decode_wire_envelope(&bytes)?;
                                if envelope.message_type() == MessageType::CommandResult {
                                    let result = decode_command_result(envelope.payload())?;
                                    assert_eq!((result.command_id, result.status), (id, CommandStatus::Accepted));
                                    assert_eq!(
                                        wire::decode_npc_intent_result(result.payload),
                                        Ok(wire::NpcIntentDisposition::Ok),
                                    );
                                    break;
                                }
                                assert_ne!(envelope.message_type(), MessageType::StateDelta);
                            }
                            ids.push(id);
                        }
                        assert_eq!(authority.purchases.get(), 2);
                        assert!(session.item_view.as_ref().ok_or("item view")?.inventory_refresh_pending);
                        authority.inventory_busy.set(false);
                        receive(&mut stream, &authority, &mut session, Next::Visibility)
                            .await.map_err(|_| "inventory retry tick")?;
                        let bytes = tokio::time::timeout(Duration::from_secs(2), read_frame(&mut client)).await??;
                        let envelope = decode_wire_envelope(&bytes)?;
                        assert_eq!(envelope.message_type(), MessageType::StateDelta);
                        let delta = decode_state_delta(envelope.payload())?;
                        assert_eq!(delta.domain_id, oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY);
                        let attributed = delta.npc_purchase_attribution.map(|field| field.command_ids().collect::<Vec<_>>());
                        assert_eq!(attributed, selected.then_some(ids));
                        if !selected {
                            assert_eq!(bytes, encode_state_delta(
                                envelope.connection_generation(), envelope.server_sequence(),
                                delta.domain_id, delta.base_revision, delta.new_revision, delta.delta_type, delta.payload,
                            )?);
                        }
                        assert!(!session.item_view.as_ref().ok_or("item view")?.inventory_refresh_pending);
                    }
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "NPC purchase retry test thread panicked")?
}
