//! Connection view qualification only; no TEXT capability or writer is activated.
use super::*;
use oteryn_protocol_oteryn::depot::{
    COMMAND_TYPE_DEPOT_INTENT, DepotAction, DepotIntent, DepotOutcome, DepotReply,
    encode_depot_intent,
};

struct DepotAuthority {
    reply: RefCell<Option<DepotReply>>,
    calls: Cell<usize>,
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
    ) -> Option<DepotReply> {
        self.calls.set(self.calls.get() + 1);
        self.reply.borrow().clone()
    }
}

fn reply(outcome: DepotOutcome) -> DepotReply {
    DepotReply {
        outcome,
        town: Some(7),
        box_number: 17,
        page: 0,
        total: 0,
        entries: Vec::new(),
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
        historical.town = None;
        replies.push((Some(historical), None));
        for mutation in 0..4 {
            let mut invalid = reply(DepotOutcome::Opened);
            match mutation {
                0 => invalid.box_number = 16,
                1 => invalid.page = 1,
                2 => invalid.total = 15_001,
                _ => invalid.town = None,
            }
            replies.push((Some(invalid), None));
        }
        for (response, expected) in replies {
            views.as_mut().ok_or("view")?.open_depot = Some((99, 1));
            *authority.reply.borrow_mut() = response.clone();
            let dispatched =
                routing::depot::route(&authority, actor, &admitted, command, &mut views).await;
            assert_eq!(views.as_ref().ok_or("view")?.open_depot, expected);
            assert_eq!(dispatched.is_some(), response.is_some());
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
            let authority = DepotAuthority {
                reply: RefCell::new(Some(reply(DepotOutcome::Opened))),
                calls: Cell::new(0),
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
                oteryn_protocol_oteryn::depot::encode_depot_reply(&reply(DepotOutcome::Opened))
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
