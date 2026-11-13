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
}

fn item(tag: u8) -> ViewItem {
    ViewItem {
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
