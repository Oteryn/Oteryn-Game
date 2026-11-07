//! Source rank filtering for the existing NPC window. The window retains exactly the
//! offers it displayed; later rank changes cannot reinterpret an old offer index.
use super::*;

/// The Item index is a wire identity, not inventory admission. Keep a rejected or
/// absent bounded source profile out of the shop even when its canonical key exists.
pub(super) fn admitted_items(
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> std::collections::BTreeSet<String> {
    let Some(native) = native else {
        return Default::default();
    };
    crate::content::npc_catalogue::grizzly_shop::offers()
        .into_iter()
        .flatten()
        .filter(|offer| {
            super::super::item_ref_admission::generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: offer.item.key.clone(),
                    revision_ref: offer.item.revision.clone(),
                },
            )
            .is_ok_and(|facts| {
                facts.weight.is_some()
                    && !matches!(
                        facts.stack,
                        crate::durability::item_transfer::ItemStackClass::Unknown
                    )
            })
        })
        .map(|offer| offer.item.key.clone())
        .collect()
}

impl NpcConversations {
    pub(super) fn refresh_grizzly_shop_context(
        &self,
        session: GameSessionId,
        points: Option<i64>,
        items: std::collections::BTreeSet<String>,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            let state = sessions.entry(session).or_default();
            state.grizzly_shop_points = points;
            state.grizzly_shop_items = items;
        }
    }

    pub(super) fn open_window(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
        npc: EntityRef,
        items: Option<&ItemDefinitionIndex>,
    ) -> Option<NpcTradeWindow> {
        let (book, items) = (chat.npc_talk()?, items?);
        let visible = runtime.visible_entities();
        let placed = visible.npcs.iter().find(|placed| {
            placed.actor.placement_identity() == npc.identity && placed.generation == npc.generation
        })?;
        let key = runtime.npc_key(placed.actor)?;
        let entry = book.get(key)?;
        let mut sessions = self.sessions.lock().ok()?;
        let state = sessions.get_mut(&session)?;
        let eligible: Vec<_> = entry
            .offers()
            .iter()
            .filter(|offer| {
                crate::content::npc_catalogue::grizzly_shop::visible(
                    key,
                    offer,
                    state.grizzly_shop_points,
                ) && (key != crate::content::npc_catalogue::grizzly_shop::NPC
                    || state.grizzly_shop_items.contains(&offer.item.key))
            })
            .cloned()
            .collect();
        let (shown, sources): (Vec<_>, Vec<_>) = projected(&eligible, items)
            .map(|(shown, source)| (shown, source.clone()))
            .unzip();
        if shown.is_empty() {
            let previous = state.window.take();
            state.window_offers.clear();
            return previous.map(|_| NpcTradeWindow::default());
        }
        state.window = Some(npc);
        state.window_offers = sources;
        Some(NpcTradeWindow {
            open: Some((npc, book.catalogue_revision())),
            offers: shown,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "closed native NPC fixtures")]
mod tests {
    use super::*;
    use crate::content::{
        ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOffer,
        ProjectV2ServiceOfferDirection,
    };
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use std::sync::Arc;

    #[test]
    fn grizzly_rank_window_freezes_offer_indices_and_closes_after_rank_loss() {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(0x72, 2);
        runtime
            .initialize_first_entry_position(actor)
            .expect("position");
        let at = runtime
            .read_actor_position(actor)
            .expect("position")
            .position();
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.grizzly_adams".into(),
                        placements: vec![NpcPlacement {
                            x: u16::try_from(at.x + 1).expect("x"),
                            y: u16::try_from(at.y).expect("y"),
                            floor: -i8::try_from(at.floor).expect("floor"),
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .expect("NPC");
        let placed = runtime.visible_entities().npcs[0];
        let npc = EntityRef {
            identity: placed.actor.placement_identity(),
            generation: placed.generation,
        };
        let row = |id, price| ProjectV2ServiceOffer {
            item: ProjectV2DefinitionRef {
                family: ProjectV2Family::Item,
                key: format!("oteryn:item.tibia.i{id}"),
                revision: "definition-r1".into(),
            },
            direction: ProjectV2ServiceOfferDirection::SellToPlayer,
            unit_price: price,
            currency: None,
            count: Some(1),
            sub_type: None,
            parity_pending: false,
        };
        // Deliberately put the locked item first; unlocking it changes the next window's index.
        // i10454 is one of Q12's actual physical-conflict rejections. Its canonical
        // index entry alone must never qualify the donor's 60-gold sell offer.
        let mut rejected_profile = row(10454, 60);
        rejected_profile.direction = ProjectV2ServiceOfferDirection::BuyFromPlayer;
        let book =
            NpcTalkBook::default().with(
                "oteryn:npc.grizzly_adams",
                NpcTalkEntry::new("Grizzly Adams", Some("Trade hunting rewards."))
                    .with_offers(vec![row(9601, 1_000), row(5907, 35_000), rejected_profile]),
            );
        let revision = book.catalogue_revision();
        let chat = ChatRuntime::default();
        chat.admit(session, "Hunter", SemanticTimeMicros::from_micros(1));
        chat.set_npc_service(session);
        chat.install_npc_talk(Arc::new(book));
        let index = ItemDefinitionIndex::decode(&serde_json::to_vec(&serde_json::json!({
            "schema": crate::content::item_ref::ITEM_KEYS_SCHEMA,
            // Native compact references follow bytewise key order, including the
            // canonical key whose rejected profile is deliberately in this fixture.
            "records": [["oteryn:item.tibia.i10454", "definition-r1"], ["oteryn:item.tibia.i5907", "definition-r1"],
                ["oteryn:item.tibia.i9601", "definition-r1"]],
        })).expect("json")).expect("item index");
        let conversations = NpcConversations::default();
        let admitted_items = || {
            [
                "oteryn:item.tibia.i9601".into(),
                "oteryn:item.tibia.i5907".into(),
            ]
            .into()
        };
        conversations.refresh_grizzly_shop_context(session, Some(20), admitted_items());
        let say = |text: &str, time| {
            conversations.talk_in(
                &chat,
                &runtime,
                actor,
                session,
                &NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
                SemanticTimeMicros::from_micros(time),
                Some(&index),
            )
        };
        assert_eq!(say("hi", 1).disposition, NpcIntentDisposition::Ok);
        let shown = say("trade", 2).window.expect("window");
        assert_eq!(shown.offers.len(), 1);
        let request = NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: 0,
            side: oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: 35_000,
        };
        let rejected_offer = NpcTradeIntent {
            offer_index: 1,
            side: oteryn_protocol_oteryn::npc_service::NpcTradeSide::Sell,
            expected_unit_price: 60,
            ..request
        };
        assert_eq!(
            conversations
                .trade_in(
                    &chat,
                    &runtime,
                    actor,
                    session,
                    &rejected_offer,
                    Some(&index)
                )
                .expect_err("canonical i10454 with rejected physical profile cannot be traded")
                .disposition,
            NpcIntentDisposition::Stale
        );
        conversations.refresh_grizzly_shop_context(session, Some(70), admitted_items());
        let frozen =
            conversations.trade_in(&chat, &runtime, actor, session, &request, Some(&index));
        assert_eq!(
            frozen.ok().expect("original visible offer").item_key,
            "oteryn:item.tibia.i5907"
        );
        assert_eq!(
            say("trade", 2_000_000)
                .window
                .expect("new window")
                .offers
                .len(),
            2
        );
        let stale = conversations.trade_in(&chat, &runtime, actor, session, &request, Some(&index));
        assert_eq!(
            stale
                .expect_err("new index has a different source price")
                .disposition,
            NpcIntentDisposition::Stale
        );
        // Q12: the canonical index still names i9601, but a source-profile rejection
        // excludes it. Neither its previous index nor a rewritten price can buy it.
        conversations.refresh_grizzly_shop_context(
            session,
            Some(70),
            ["oteryn:item.tibia.i5907".into()].into(),
        );
        let missing = say("trade", 4_000_000)
            .window
            .expect("eligible native offer");
        assert_eq!(missing.offers.len(), 1);
        assert_eq!(missing.offers[0].unit_price, 35_000);
        let mut refused = request;
        refused.expected_unit_price = 1_000;
        assert_eq!(
            conversations
                .trade_in(&chat, &runtime, actor, session, &refused, Some(&index))
                .expect_err("rejected profile cannot reinterpret the surviving index")
                .disposition,
            NpcIntentDisposition::Stale
        );
        conversations.refresh_grizzly_shop_context(session, Some(70), Default::default());
        assert_eq!(
            say("trade", 6_000_000).window,
            Some(NpcTradeWindow::default())
        );
        assert_eq!(
            conversations
                .trade_in(&chat, &runtime, actor, session, &request, Some(&index))
                .expect_err("no admitted native profile")
                .disposition,
            NpcIntentDisposition::Rejected
        );
        conversations.refresh_grizzly_shop_context(session, Some(70), admitted_items());
        assert_eq!(
            say("trade", 8_000_000)
                .window
                .expect("reopened native window")
                .offers
                .len(),
            2
        );
        conversations.refresh_grizzly_shop_context(session, Some(0), admitted_items());
        assert_eq!(
            say("trade", 10_000_000).window,
            Some(NpcTradeWindow::default())
        );
        assert_eq!(
            conversations
                .trade_in(&chat, &runtime, actor, session, &request, Some(&index))
                .expect_err("closed")
                .disposition,
            NpcIntentDisposition::Rejected
        );
    }
}
