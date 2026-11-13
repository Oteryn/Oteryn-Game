use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{NpcIntentDisposition, SessionEvent, npc_service};
impl Outbox {
    pub(in crate::play) fn npc_shop_initialize<T: Stepper>(&self, session: &T) {
        let mut mailbox = lock(&self.0);
        mailbox.npc_shop.selected = session.npc_shop_selected();
        let shop = &mut mailbox.npc_shop.live;
        shop.selected = session.feature_selected(3);
        shop.resources_selected = session.feature_selected(272);
        if let Some(payload) = session.feature_snapshot(14)
            && let Ok(views) =
                oteryn_client_runtime::npc_shop::container_tree::decode_container_views_with_house_capacity(
                    &payload,
                    session.feature_selected(230),
                    session.feature_selected(446),
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
            && let Ok(window) = npc_service::decode_npc_trade_window(&payload)
        {
            shop.apply_window(window);
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
        let result = session.npc_trade(intent).await?;
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
        let response = session.npc_shop(command).await?;
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

impl Mailbox {
    pub(in crate::play) fn npc_shop_events(&mut self, events: &[SessionEvent]) {
        for event in events {
            match event {
                SessionEvent::NpcTradeWindow(delta) => {
                    // Keep a closed/refused shop visible with empty offers and inline feedback.
                    self.npc_shop.open |= delta.value.open.is_some();
                    self.npc_shop.live.apply_window(delta.value.clone());
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

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_npc_shop_supported(&self) {
        let mut mailbox = lock(&self.0);
        mailbox.npc_shop.selected = true;
        mailbox.npc_shop.live.selected = true;
    }
}
