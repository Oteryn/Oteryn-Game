use super::*;
use crate::gameplay_transport::item_view::ViewItem;
use oteryn_protocol_oteryn::{
    container_tree as bags, quick_loot as loot, world_object as use_wire,
};
use std::num::NonZeroU32;

struct InventoryAuthority {
    looted: Cell<bool>,
    trade_read_failed: bool,
    backpack: ViewItem,
    sliver: ViewItem,
}

impl InventoryAuthority {
    fn new(trade_read_failed: bool) -> Result<Self, Box<dyn Error>> {
        let item = |id, definition| -> Result<ViewItem, Box<dyn Error>> {
            Ok(ViewItem {
                key: ItemKey::Instance(uuid_v7(id)),
                item_definition_ref: NonZeroU32::new(definition).ok_or("definition")?,
                count: NonZeroU32::new(1).ok_or("quantity")?,
                appearance_id: u16::try_from(definition)?,
                sub_type: 0,
            })
        };
        Ok(Self {
            looted: Cell::new(false),
            trade_read_failed,
            backpack: item(60, 1988)?,
            sliver: item(61, 37109)?,
        })
    }
}

impl FreshAdmissionAuthority for InventoryAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems {
            main_backpack: Some(self.backpack),
            entries: self
                .looted
                .get()
                .then_some(self.sliver)
                .into_iter()
                .collect(),
            equipment: Vec::new(),
        })
    }

    async fn observe_container(
        &self,
        _: ExactActorRef,
        _: Option<CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<ContainerObservation> {
        assert_eq!(target, self.backpack.key);
        Some(ContainerObservation::Container {
            parent: None,
            capacity: 20,
            entries: self
                .looted
                .get()
                .then_some(self.sliver)
                .into_iter()
                .collect(),
        })
    }

    async fn player_trade_item_available(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: Option<ItemKey>,
    ) -> Option<bool> {
        (!self.trade_read_failed).then_some(true)
    }

    async fn quick_loot(
        &self,
        _: AdmittedSession,
        _: UseCommand,
        request: loot::QuickLootRequest,
    ) -> Option<(crate::foundation::CommandStatus, loot::QuickLootResult)> {
        assert_eq!(request.action, loot::QuickLootAction::Click);
        let ItemKey::Instance(item) = self.sliver.key else {
            return None;
        };
        assert!(!self.looted.replace(true));
        Some((
            crate::foundation::CommandStatus::Accepted,
            loot::QuickLootResult {
                items: vec![loot::QuickLootItemResult {
                    item,
                    accepted: true,
                }],
            },
        ))
    }
}

fn connection_command(
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(oteryn_protocol_oteryn::encode_client_command(
        ADMITTED_GENERATION,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )?)
}

fn capabilities(ids: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
    let mut admitted = positioned()?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        ids,
    )
    .ok_or("capabilities")?;
    Ok(admitted)
}

fn on_connection_stack(test: fn() -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || test().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "connection test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn quick_loot_click_publishes_the_open_backpack_after_its_command_result()
-> Result<(), Box<dyn Error>> {
    on_connection_stack(|| {
        run(async {
            let authority = InventoryAuthority::new(false)?;
            let mut view = SessionItemView::resume(Default::default()).with_container_tree();
            let admitted = capabilities(&[4, 6, 14, loot::CAPABILITY_QUICK_LOOT_V1])?;
            let inventory = authority
                .observe_character_inventory(
                    admitted.runtime_actor.ok_or("actor")?,
                    admitted.game_session_id,
                )
                .await
                .ok_or("inventory")?;
            view.snapshot(inventory, None)
                .map_err(|err| format!("{err:?}"))?;
            let backpack = view
                .table()
                .handle(&authority.backpack.key)
                .ok_or("backpack handle")?;
            let open = bags::encode_container_view_intent(&bags::ContainerViewIntent::Open {
                handle: backpack,
                replace_view: None,
            })
            .map_err(|error| format!("{error:?}"))?;
            let click = loot::encode_quick_loot_request(&loot::QuickLootRequest {
                action: loot::QuickLootAction::Click,
                corpse: Some(uuid_v7(62)),
                category: None,
                recipient: None,
                recipient_id: None,
                fallback_to_main: false,
                filter_mode: None,
                filter_definition_ref: None,
            })
            .map_err(|error| format!("{error:?}"))?;
            let (_, frames) = drive_session(
                &authority,
                admitted,
                &[
                    connection_command(1, bags::COMMAND_TYPE_CONTAINER_VIEW_INTENT, &open)?,
                    connection_command(2, loot::COMMAND_TYPE_QUICK_LOOT, &click)?,
                ],
            )
            .await?;
            let mut opened = false;
            let mut clicked = false;
            let mut published = false;
            for frame in &frames {
                let frame = decode_wire_envelope(frame)?;
                match frame.message_type() {
                    MessageType::CommandResult => {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                        if result.command_id == 2 {
                            assert_eq!(result.status, crate::foundation::CommandStatus::Accepted);
                            assert!(
                                loot::decode_quick_loot_result(result.payload)
                                    .map_err(|error| format!("{error:?}"))?
                                    .items[0]
                                    .accepted
                            );
                            clicked = true;
                        }
                    }
                    MessageType::StateDelta => {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload())?;
                        if delta.domain_id == bags::STATE_DOMAIN_CONTAINER_VIEWS {
                            let views = bags::decode_container_views(delta.payload)
                                .map_err(|error| format!("{error:?}"))?;
                            if let Some(open) = views.views.first() {
                                if clicked {
                                    assert!(opened);
                                    assert_eq!(open.entries.len(), 1);
                                    assert_eq!(
                                        open.entries[0].item_definition_ref,
                                        authority.sliver.item_definition_ref
                                    );
                                    published = true;
                                } else {
                                    assert!(open.entries.is_empty());
                                    opened = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            assert!(
                clicked && published,
                "Quick Loot must refresh an already open backpack"
            );
            Ok(())
        })
    })
}

#[test]
fn rune_use_read_failure_returns_correlated_refusals_without_disconnect()
-> Result<(), Box<dyn Error>> {
    on_connection_stack(|| {
        run(async {
            let authority = InventoryAuthority::new(true)?;
            let admitted = capabilities(&[4, 6, 11, 15, 18, 22, 30])?;
            let payload = use_wire::encode_item_use_intent(&UseIntent {
                target: UseTarget::Item(oteryn_protocol_oteryn::item_view::ItemHandle::MIN),
                use_with: None,
                use_with_position: Some(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                }),
                use_with_map_item: None,
            })
            .map_err(|error| format!("{error:?}"))?;
            let (_, frames) = drive_session(
                &authority,
                admitted,
                &[
                    connection_command(1, use_wire::COMMAND_TYPE_USE_INTENT, &payload)?,
                    connection_command(2, use_wire::COMMAND_TYPE_USE_INTENT, &payload)?,
                ],
            )
            .await?;
            let mut refused = Vec::new();
            for frame in &frames {
                let frame = decode_wire_envelope(frame)?;
                if frame.message_type() == MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                    assert_eq!(result.status, crate::foundation::CommandStatus::Rejected);
                    assert_eq!(
                        use_wire::decode_use_result(result.payload)
                            .map_err(|error| format!("{error:?}"))?,
                        UseDisposition::Rejected
                    );
                    refused.push(result.command_id);
                }
            }
            assert_eq!(refused, [1, 2], "each command must receive a typed refusal");
            Ok(())
        })
    })
}
