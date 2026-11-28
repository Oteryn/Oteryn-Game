use super::*;
use crate::gameplay_transport::container_view::ContainerPlan;
use oteryn_protocol_oteryn::item_view::{
    COMMAND_TYPE_ITEM_MOVE_INTENT, ItemMoveDestination, ItemMoveIntent, encode_item_move_intent,
};
use std::num::NonZeroU32;

#[derive(Default)]
struct MoveOwner {
    moved: Cell<bool>,
    refused: Cell<bool>,
    reads: Cell<usize>,
    background_operations: Cell<usize>,
    quick_loot_commands: Cell<usize>,
    trade_revision: Cell<Option<u64>>,
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
        Some(InventoryItems {
            main_backpack: Some(item(1)),
            entries: if self.moved.get() {
                vec![item(2), item(3)]
            } else {
                vec![item(2)]
            },
            equipment: Vec::new(),
        })
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

    async fn quick_loot_wire(
        &self,
        _: AdmittedSession,
        _: UseCommand,
        _: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
    ) -> Option<(CommandStatus, Vec<u8>)> {
        use oteryn_protocol_oteryn::quick_loot::{QuickLootRefusal, encode_quick_loot_refusal};
        self.quick_loot_commands
            .set(self.quick_loot_commands.get() + 1);
        Some((
            CommandStatus::Rejected,
            encode_quick_loot_refusal(QuickLootRefusal::Stale),
        ))
    }

    async fn player_trade_open(&self, _: AdmittedSession) -> Option<()> {
        Some(())
    }

    async fn player_trade_observation(
        &self,
        _: GameSessionId,
    ) -> Option<(u64, oteryn_protocol_oteryn::player_trade::State, bool)> {
        self.trade_revision
            .get()
            .map(|revision| (revision, Default::default(), false))
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
fn stale_quick_loot_finishes_before_unrelated_trade_observation() -> TestResult {
    use crate::gameplay_transport::capabilities::OfferedCapability;
    use oteryn_protocol_oteryn::{player_trade, quick_loot};
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = MoveOwner::default();
                    authority.trade_revision.set(Some(1));
                    let (mut stream, _client) = tokio::io::duplex(1 << 20);
                    let mut admitted = admitted(false)?;
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[
                            OfferedCapability {
                                id: player_trade::CAPABILITY_PLAYER_TRADE_V1,
                                requires: &[],
                            },
                            OfferedCapability {
                                id: quick_loot::CAPABILITY_QUICK_LOOT_V1,
                                requires: &[],
                            },
                        ],
                        &[
                            player_trade::CAPABILITY_PLAYER_TRADE_V1,
                            quick_loot::CAPABILITY_QUICK_LOOT_V1,
                        ],
                    )
                    .ok_or("select loot and trade capabilities")?;
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
                    assert_eq!(session.admitted.continuity.player_trade_revision, 1);
                    authority.trade_revision.set(Some(0));
                    for id in 1..=2 {
                        let frame =
                            command_frame(id, quick_loot::COMMAND_TYPE_QUICK_LOOT, &[8, 2])?;
                        let input = receive(
                            &mut stream,
                            &authority,
                            &mut session,
                            Next::Held(frame, false, tokio::time::Instant::now()),
                        )
                        .await
                        .map_err(|_| "receive loot")?
                        .ok_or("loot input")?;
                        command::execute(&mut stream, &authority, &mut session, input)
                            .await
                            .map_err(|_| "stale loot ended the session")?;
                    }
                    assert_eq!(authority.quick_loot_commands.get(), 2);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "loot refusal test thread panicked")?
}
