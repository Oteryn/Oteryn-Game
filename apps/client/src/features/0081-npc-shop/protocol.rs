use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::npc_shop::{shop_command, shop_reply, trade_command, trade_reply};
use oteryn_session::{
    NpcIntentDisposition, SessionEvent, npc_service, npc_shop::CAPABILITY_NPC_SHOP_OPTIONS_V1,
};
#[path = "weights.rs"]
mod weights;
impl Outbox {
    pub(in crate::play) fn npc_shop_initialize<T: Stepper>(&self, session: &T) {
        let mut mailbox = lock(&self.0);
        mailbox.npc_shop.selected = session.npc_shop_selected();
        let shop = &mut mailbox.npc_shop.live;
        shop.selected = session.feature_selected(3);
        shop.options_selected = session.feature_selected(CAPABILITY_NPC_SHOP_OPTIONS_V1);
        shop.resources_selected = session.feature_selected(272);
        if let Some(payload) = session.feature_snapshot(14)
            && let Ok(views) =
                oteryn_client_runtime::npc_shop::container_tree::decode_container_views_with_appearances(
                    &payload,
                    session.feature_selected(230),
                    session.feature_selected(446),
                    session.feature_selected(499),
                )
        {
            shop.containers = views;
        }
        shop.inventory = session.npc_inventory();
        if let Some(payload) = session.feature_snapshot(7)
            && let Ok(conversation) = npc_service::decode_npc_conversation(&payload)
        {
            shop.conversation = conversation.open.map(|(npc, _)| npc);
        }
        if let Some(payload) = session.feature_snapshot(8)
            && let Ok(window) =
                npc_service::decode_npc_trade_window_for(&payload, session.feature_selected(501))
        {
            oteryn_client_ui::trade_details::apply_window(shop, window);
        }
    }
    pub(in crate::play) async fn npc_shop_greet<T: Stepper>(
        &self,
        session: &mut T,
        npc: oteryn_session::EntityRef,
    ) -> Result<NpcIntentDisposition, oteryn_session::SessionError> {
        if lock(&self.0).npc_shop.live.conversation == Some(npc) {
            return Ok(NpcIntentDisposition::Ok);
        }
        session
            .npc_talk(&npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: "hi".into(),
            })
            .await
    }
    pub(in crate::play) async fn npc_shop_balances<T: Stepper>(
        &self,
        session: &mut T,
    ) -> Result<(), oteryn_session::SessionError> {
        if session.feature_selected(272) {
            let balances = session.npc_resources().await?;
            lock(&self.0).npc_shop.live.balances = balances;
        }
        Ok(())
    }
    async fn npc_shop_stale<T: Stepper>(
        &self,
        session: &mut T,
        npc: oteryn_session::EntityRef,
    ) -> Result<(), oteryn_session::SessionError> {
        let refreshed = session
            .npc_talk(&npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: "trade".into(),
            })
            .await?;
        if refreshed != NpcIntentDisposition::Ok {
            lock(&self.0).npc_shop.live.feedback = lfmt!(
                "Quote refresh refused: {refreshed:?}",
                refreshed = refreshed
            );
        }
        Ok(())
    }
    pub(in crate::play) async fn npc_trade_run<T: Stepper>(
        &self,
        session: &mut T,
        intent: &npc_service::NpcTradeIntent,
    ) -> Result<(), oteryn_session::SessionError> {
        let packed = lock(&self.0).npc_shop.live.pending_pack_in_backpacks.unwrap_or(false);
        let result = if packed {
            require_shop_options(session)?;
            trade_reply(session.feature(&trade_command(intent, true)?).await?)?
        } else {
            session.npc_trade(intent).await?
        };
        lock(&self.0).npc_shop.live.result(result);
        if result == NpcIntentDisposition::Ok {
            let mut mailbox = lock(&self.0);
            mailbox.npc_shop.result = None;
            mailbox.npc_shop.feedback = None;
        } else if result == NpcIntentDisposition::Stale {
            self.npc_shop_stale(session, intent.npc_actor).await?;
        }
        self.npc_shop_balances(session).await
    }
    pub(in crate::play) async fn npc_shop_run<T: Stepper>(
        &self,
        session: &mut T,
        command: &oteryn_client_runtime::npc_shop::NpcShopCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        use oteryn_client_runtime::npc_shop::npc_shop::NpcShopAction;
        let include_equipped = command.0.action == NpcShopAction::SellAll
            && lock(&self.0).npc_shop.live.includes_equipment();
        let response = if include_equipped {
            require_shop_options(session)?;
            shop_reply(session.feature(&shop_command(command, true)?).await?)?
        } else {
            session.npc_shop(command).await?
        };
        self.npc_shop_result(&response);
        if response
            .result
            .as_ref()
            .is_some_and(|result| result.disposition == NpcIntentDisposition::Stale)
        {
            self.npc_shop_stale(session, command.0.npc_actor).await?;
        }
        if command.0.action == NpcShopAction::SellAll {
            self.npc_shop_balances(session).await?;
        }
        Ok(())
    }
    pub(in crate::play) fn npc_shop_result(
        &self,
        result: &oteryn_client_runtime::npc_shop::NpcShopResponse,
    ) {
        oteryn_client_ui::npc_shop::apply_result(&mut lock(&self.0).npc_shop, result);
    }
}

fn require_shop_options(session: &impl Stepper) -> Result<(), oteryn_session::SessionError> {
    if session.feature_selected(CAPABILITY_NPC_SHOP_OPTIONS_V1) {
        Ok(())
    } else {
        Err(oteryn_session::SessionError::CapabilityNotSelected {
            capability: CAPABILITY_NPC_SHOP_OPTIONS_V1,
        })
    }
}

impl Mailbox {
    pub(in crate::play) fn refresh_loot_quote(
        &mut self,
        id: u32,
        price: Option<u64>,
        source: oteryn_client_ui::analyser::PriceSource,
    ) {
        if self.analyser.loot_sources.get(&id) != Some(&source)
            || self.analyser.loot_prices.get(&id).copied() == price
        {
            return;
        }
        if let Some(price) = price {
            std::sync::Arc::make_mut(&mut self.analyser.loot_prices).insert(id, price);
        } else {
            std::sync::Arc::make_mut(&mut self.analyser.loot_prices).remove(&id);
        }
        self.analyser.party.clear();
        self.analyser.party_revision = self.analyser.party_revision.saturating_add(1);
        self.analyser.party_feedback = Some("Prices changed; refresh party accounting");
    }

    pub(in crate::play) fn npc_shop_events(&mut self, events: &[SessionEvent]) {
        for event in events {
            match event {
                SessionEvent::NpcTradeWindow(delta) => {
                    // Keep a closed/refused shop visible with empty offers and inline feedback.
                    self.npc_shop.open |= delta.value.open.is_some();
                    oteryn_client_ui::trade_details::apply_window(
                        &mut self.npc_shop.live,
                        delta.value.clone(),
                    );
                    let mut prices = std::collections::BTreeMap::<u32, u64>::new();
                    for quote in self.npc_shop.live.quotes() {
                        if quote.side == oteryn_client_ui::npc_shop::NpcTradeSide::Sell
                            && quote.count == 1
                        {
                            let price = prices.entry(quote.item).or_default();
                            *price = (*price).max(u64::from(quote.price));
                        }
                    }
                    for (id, price) in prices {
                        self.refresh_loot_quote(
                            id,
                            Some(price),
                            oteryn_client_ui::analyser::PriceSource::Merchant,
                        );
                    }
                }
                SessionEvent::NpcConversation(delta) => {
                    self.npc_shop.live.conversation = delta.value.open.map(|(npc, _)| npc);
                    if self.npc_shop.live.conversation.is_none()
                        || self
                            .npc_shop
                            .live
                            .window
                            .open
                            .is_some_and(|(npc, _)| Some(npc) != self.npc_shop.live.conversation)
                    {
                        self.npc_shop.live.window = Default::default();
                        self.npc_shop.live.pending = false;
                    }
                }
                SessionEvent::ContainerViews(delta) => {
                    self.npc_shop.live.containers = delta.value.clone();
                }
                SessionEvent::Inventory(delta) => {
                    self.npc_shop.live.inventory = Some(delta.value.clone())
                }
                _ => {}
            }
        }
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_npc_sale_accepted(&self) {
        use oteryn_client_runtime::npc_shop::{CommandStatus, NpcShopResponse, npc_shop::NpcShopResult};
        self.npc_shop_result(&NpcShopResponse {
            status: CommandStatus::Accepted,
            result: Some(NpcShopResult { disposition: NpcIntentDisposition::Ok, look_text: String::new(), sold_units: 3, total_gold_units: 21 }),
        });
    }
    pub(crate) fn bevy_npc_shop_supported(&self) {
        let mut mailbox = lock(&self.0);
        mailbox.npc_shop.selected = true;
        mailbox.npc_shop.live.selected = true;
    }
}
