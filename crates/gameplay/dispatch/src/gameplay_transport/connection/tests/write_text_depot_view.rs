//! Connection view qualification only; no TEXT capability or writer is activated.
use super::*;
use oteryn_protocol_oteryn::depot::{
    COMMAND_TYPE_DEPOT_INTENT, DepotAction, DepotIntent, DepotOutcome, DepotReply,
    encode_depot_intent,
};

struct DepotAuthority {
    reply: RefCell<Option<DepotObservation>>,
    calls: Cell<usize>,
    selected_item: Cell<Option<ItemKey>>,
    slots: RefCell<Vec<Option<u32>>>,
}

impl FreshAdmissionAuthority for DepotAuthority {
    fn offered_capabilities(&self) -> &'static [OfferedCapability] {
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES
    }

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
        Some(InventoryItems::default())
    }

    async fn depot(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: DepotIntent,
        _: Option<ItemKey>,
    ) -> Option<DepotObservation> {
        self.calls.set(self.calls.get() + 1);
        self.reply.borrow().clone()
    }

    async fn depot_at(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: DepotIntent,
        slot: Option<u32>,
        item: Option<ItemKey>,
    ) -> Option<DepotObservation> {
        self.selected_item.set(item);
        self.slots.borrow_mut().push(slot);
        self.depot(actor, command, intent, item).await
    }
}

fn reply(outcome: DepotOutcome) -> DepotObservation {
    DepotObservation {
        items: Vec::new(),
        reply: DepotReply {
            outcome,
            town: Some(7),
            box_number: 17,
            page: 0,
            total: 0,
            entries: Vec::new(),
        },
    }
}

#[test]
fn write_text_depot_view_records_only_current_opened_box_and_closes_on_resume()
-> Result<(), Box<dyn Error>> {
    run(async {
        let admitted = positioned()?;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let payload = encode_depot_intent(DepotIntent {
            town: None,
            box_number: 17,
            page: 0,
            action: DepotAction::Browse,
        })
        .map_err(|e| format!("{e:?}"))?;
        let command = oteryn_protocol_oteryn::ClientCommandView {
            command_id: 1,
            command_type: COMMAND_TYPE_DEPOT_INTENT,
            payload: &payload,
        };
        let authority = DepotAuthority {
            reply: RefCell::new(None),
            calls: Cell::new(0),
            selected_item: Cell::new(None),
            slots: RefCell::default(),
        };
        let mut views = Some(SessionItemView::resume(Default::default()));
        let mut replies = vec![(None, None)];
        for (outcome, expected) in [
            (DepotOutcome::Opened, Some((7, 17))),
            (DepotOutcome::Moved, Some((7, 17))),
            (DepotOutcome::NoLocker, None),
            (DepotOutcome::Stale, None),
            (DepotOutcome::Full, None),
            (DepotOutcome::NoBackpack, None),
            (DepotOutcome::Overweight, None),
            (DepotOutcome::Unsupported, None),
            (DepotOutcome::Rejected, None),
        ] {
            replies.push((Some(reply(outcome)), expected));
        }
        let mut historical = reply(DepotOutcome::Moved);
        historical.reply.town = None;
        replies.push((Some(historical), None));
        for mutation in 0..4 {
            let mut invalid = reply(DepotOutcome::Opened);
            match mutation {
                0 => invalid.reply.box_number = 16,
                1 => invalid.reply.page = 1,
                2 => invalid.reply.total = 15_001,
                _ => invalid.reply.town = None,
            }
            replies.push((Some(invalid), None));
        }
        for (response, expected) in replies {
            views.as_mut().ok_or("view")?.open_depot = Some((99, 1));
            views.as_mut().ok_or("view")?.depot_items = vec![(1, [9; 16])];
            *authority.reply.borrow_mut() = response.clone();
            let dispatched =
                routing::depot::route(&authority, actor, &admitted, command, &mut views).await;
            assert_eq!(views.as_ref().ok_or("view")?.open_depot, expected);
            assert_eq!(dispatched.is_some(), response.is_some());
            assert!(views.as_ref().ok_or("view")?.depot_items.is_empty());
            if let Some(expected) = expected {
                assert_eq!(expected, (7, 17));
                let continuity = views.as_ref().ok_or("view")?.continuity();
                for continuity in [continuity, continuity.across_channel_transfer()] {
                    assert!(SessionItemView::resume(continuity).open_depot.is_none());
                }
            }
        }
        Ok(())
    })
}

#[test]
fn write_text_depot_view_real_dispatch_requires_selected_capability() -> Result<(), Box<dyn Error>>
{
    run(async {
        for selected in [false, true] {
            let mut observed = reply(DepotOutcome::Opened);
            observed.items.push((91, [7; 16]));
            observed.reply.total = 1;
            observed
                .reply
                .entries
                .push(oteryn_protocol_oteryn::depot::DepotEntry {
                    ordinal: 91,
                    definition: 2,
                    count: 1,
                });
            let authority = DepotAuthority {
                reply: RefCell::new(Some(observed.clone())),
                calls: Cell::new(0),
                selected_item: Cell::new(None),
                slots: RefCell::default(),
            };
            let mut admitted = positioned()?;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                authority.offered_capabilities(),
                if selected { &[4, 6, 37] } else { &[4, 6] },
            )
            .ok_or("capabilities")?;
            let payload = encode_depot_intent(DepotIntent {
                town: None,
                box_number: 17,
                page: 0,
                action: DepotAction::Browse,
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut command = Vec::new();
            scalar(&mut command, 1, 1);
            scalar(&mut command, 2, u64::from(COMMAND_TYPE_DEPOT_INTENT));
            bytes(&mut command, 4, &payload);
            let (end, frames) = drive_session(
                &authority,
                admitted,
                &[envelope(7, ADMITTED_GENERATION, &command)],
            )
            .await?;
            assert_eq!(authority.calls.get(), usize::from(selected));
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            let expected = if selected {
                oteryn_protocol_oteryn::depot::encode_depot_reply(&observed.reply)
                    .map_err(|e| format!("{e:?}"))?
            } else {
                Vec::new()
            };
            let result = encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                if selected {
                    CommandStatus::Accepted
                } else {
                    CommandStatus::Rejected
                },
                &expected,
            )?;
            assert!(frames.contains(&result));
        }
        Ok(())
    })
}

#[test]
fn write_text_depot_view_preserves_only_exact_native_page_selection() -> Result<(), Box<dyn Error>>
{
    run(async {
        let admitted = positioned()?;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let payload = encode_depot_intent(DepotIntent {
            town: Some(7),
            box_number: 17,
            page: 0,
            action: DepotAction::Browse,
        })
        .map_err(|e| format!("{e:?}"))?;
        let command = oteryn_protocol_oteryn::ClientCommandView {
            command_id: 1,
            command_type: COMMAND_TYPE_DEPOT_INTENT,
            payload: &payload,
        };
        let mut observed = reply(DepotOutcome::Opened);
        observed.reply.total = 2;
        observed.reply.entries = vec![
            oteryn_protocol_oteryn::depot::DepotEntry {
                ordinal: 91,
                definition: 2,
                count: 1,
            },
            oteryn_protocol_oteryn::depot::DepotEntry {
                ordinal: 5,
                definition: 2,
                count: 1,
            },
        ];
        observed.items = vec![(91, [7; 16]), (5, [8; 16])];
        let authority = DepotAuthority {
            reply: RefCell::new(None),
            calls: Cell::new(0),
            selected_item: Cell::new(None),
            slots: RefCell::default(),
        };
        let mut views = Some(SessionItemView::resume(Default::default()));
        for mutation in 0..7 {
            let mut next = observed.clone();
            match mutation {
                0 => {}
                1 => next.items.clear(),
                2 => next.items[0].0 = 92,
                3 => next.items.swap(0, 1),
                4 => next.items[1].1 = next.items[0].1,
                5 => next.items.push((1, [9; 16])),
                _ => next.reply.outcome = DepotOutcome::Rejected,
            }
            // Each attempt must discard any previously observed selection.
            let view = views.as_mut().ok_or("view")?;
            view.open_depot = Some((99, 1));
            view.depot_items = vec![(999, [9; 16])];
            *authority.reply.borrow_mut() = Some(next);
            assert!(
                routing::depot::route(&authority, actor, &admitted, command, &mut views)
                    .await
                    .is_some()
            );
            let view = views.as_ref().ok_or("view")?;
            if mutation == 0 {
                assert_eq!(view.open_depot, Some((7, 17)));
                assert_eq!(view.depot_items, observed.items);
                assert!(
                    view.table().handle(&ItemKey::Instance([7; 16])).is_none(),
                    "depot identities do not mint an unadvertised wire handle"
                );
                for continuity in [
                    view.continuity(),
                    view.continuity().across_channel_transfer(),
                ] {
                    let resumed = SessionItemView::resume(continuity);
                    assert!(resumed.open_depot.is_none());
                    assert!(resumed.depot_items.is_empty());
                }
            } else {
                assert!(view.open_depot.is_none());
                assert!(view.depot_items.is_empty());
            }
        }
        // A deposit replaces the item at 91 before a previously queued withdrawal arrives.
        for (action, ordinal_item, expected) in [
            (DepotAction::Browse, [7; 16], None),
            (
                DepotAction::Deposit(std::num::NonZeroU64::MIN),
                [9; 16],
                None,
            ),
            (DepotAction::Withdraw(91), [9; 16], None),
            (DepotAction::Browse, [9; 16], None),
            (
                DepotAction::Withdraw(91),
                [9; 16],
                Some(ItemKey::Instance([9; 16])),
            ),
        ] {
            let payload = encode_depot_intent(DepotIntent {
                town: Some(7),
                box_number: 17,
                page: 0,
                action,
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut next = observed.clone();
            next.items[0].1 = ordinal_item;
            next.reply.outcome = if action == DepotAction::Browse {
                DepotOutcome::Opened
            } else {
                DepotOutcome::Moved
            };
            *authority.reply.borrow_mut() = Some(next);
            let command = oteryn_protocol_oteryn::ClientCommandView {
                payload: &payload,
                ..command
            };
            assert!(
                routing::depot::route(&authority, actor, &admitted, command, &mut views)
                    .await
                    .is_some()
            );
            assert_eq!(authority.selected_item.get(), expected);
        }
        Ok(())
    })
}

/// CONTAINER-MANUAL-SORT-1 (capability 510): a deposit's box slot reaches the depot owner only
/// when the session selected 510; without it the slotted payload is malformed and nothing
/// reaches the owner.
#[test]
fn a_slotted_deposit_reaches_the_depot_owner_only_with_capability_510() -> Result<(), Box<dyn Error>>
{
    run(async {
        let deposit = DepotIntent {
            town: Some(7),
            box_number: 17,
            page: 1,
            action: DepotAction::Deposit(
                oteryn_protocol_oteryn::item_view::ItemHandle::new(5).ok_or("handle")?,
            ),
        };
        let payload = oteryn_protocol_oteryn::depot::encode_depot_intent_at(deposit, Some(33))
            .map_err(|e| format!("{e:?}"))?;
        let command = oteryn_protocol_oteryn::ClientCommandView {
            command_id: 1,
            command_type: COMMAND_TYPE_DEPOT_INTENT,
            payload: &payload,
        };
        for manual_sort in [false, true] {
            let authority = DepotAuthority {
                reply: RefCell::new(Some(reply(DepotOutcome::Moved))),
                calls: Cell::new(0),
                selected_item: Cell::new(None),
                slots: RefCell::default(),
            };
            let mut admitted = positioned()?;
            let actor = admitted.runtime_actor.ok_or("actor")?;
            let requested: &[u32] = if manual_sort {
                &[4, 6, 12, 14, 37, 510]
            } else {
                &[4, 6, 12, 14, 37]
            };
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(authority.offered_capabilities(), requested)
                    .ok_or("capabilities")?;
            assert_eq!(
                admitted.continuity.selected_capabilities.contains(510),
                manual_sort
            );
            let mut views = Some(SessionItemView::resume(Default::default()));
            let dispatched =
                routing::depot::route(&authority, actor, &admitted, command, &mut views).await;
            assert_eq!(matches!(dispatched, Some(Dispatch::Depot(_))), manual_sort);
            assert_eq!(
                matches!(dispatched, Some(Dispatch::Unregistered)),
                !manual_sort
            );
            assert_eq!(authority.calls.get(), usize::from(manual_sort));
            let expected: Vec<Option<u32>> = if manual_sort {
                vec![Some(33)]
            } else {
                Vec::new()
            };
            assert_eq!(*authority.slots.borrow(), expected);
        }
        Ok(())
    })
}
