#[cfg(bevy_shell)]
#[derive(Debug, Clone)]
pub(crate) struct LookMessage {
    pub receipt: u64,
    pub text: String,
}

#[cfg(bevy_shell)]
impl crate::play::PlayLink {
    pub(crate) fn private_screen_message(
        &self,
        cursor: &mut Option<(u64, u64)>,
    ) -> (bool, Option<String>) {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        let panel = &mailbox.chat;
        let previous = cursor.replace((panel.scope, panel.received));
        let reset = previous.is_none_or(|(scope, _)| scope != panel.scope);
        let received = previous.map_or(0, |(_, n)| n);
        let count = usize::try_from(panel.received.wrapping_sub(received)).unwrap_or(usize::MAX);
        let start = panel.lines.len().saturating_sub(count);
        let latest = panel.lines[start..]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, line)| {
                if let oteryn_session::ChatLine::Private { speaker_name, text } = line
                    && panel
                        .recipients
                        .get(start + index)
                        .is_none_or(Option::is_none)
                    && panel.console.communication.visible(line)
                {
                    Some(format!("{speaker_name}: {text}"))
                } else {
                    None
                }
            });
        (reset, latest)
    }
    pub(crate) fn look_message(&self) -> Option<LookMessage> {
        crate::play::mailbox::lock(&self.mailbox)
            .screen_look
            .clone()
    }
}

#[cfg(bevy_shell)]
impl crate::play::PlayView {
    pub(crate) fn screen_message_controls(&self) -> (bool, bool, u64, u64) {
        let center =
            crate::play::feature_local_game_notifications::state::lock(&self.game_notifications.0);
        (
            center.banners_enabled,
            center.popups_enabled,
            self.game_notifications
                .0
                .screen_dismissals
                .load(std::sync::atomic::Ordering::Relaxed),
            self.message_namespace,
        )
    }
}

#[cfg(bevy_shell)]
pub(in crate::play) fn record_look(
    mailbox: &mut crate::play::mailbox::Mailbox,
    action: oteryn_client_runtime::world_interaction::Action,
    text: &str,
) {
    if action != oteryn_client_runtime::world_interaction::Action::Look {
        return;
    }
    mailbox.look_receipt = mailbox.look_receipt.saturating_add(1);
    mailbox.screen_look = Some(LookMessage {
        receipt: mailbox.look_receipt,
        text: text.into(),
    });
    let received = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    mailbox
        .chat
        .console
        .notices
        .push(oteryn_client_ui::chat::NoticeClass::Status, text, received);
}

#[cfg(all(test, bevy_shell))]
mod tests;

#[cfg(bevy_shell)]
#[derive(Debug, Default)]
pub(in crate::play) struct LootMessage {
    receipt: u64,
    items: Vec<oteryn_session::analyser_transport::AnalyserItemCount>,
    gold: u64,
    presented: bool,
    market_prices: std::collections::BTreeMap<u32, u64>,
}

#[cfg(bevy_shell)]
impl LootMessage {
    pub(in crate::play) fn record(
        &mut self,
        items: &[oteryn_session::analyser_transport::AnalyserItemCount],
        gold: u64,
    ) {
        self.receipt = self.receipt.wrapping_add(1);
        if std::mem::take(&mut self.presented) {
            self.items.clear();
            self.gold = 0;
        }
        use oteryn_session::analyser_transport::{
            MAX_ANALYSER_BATCH_FACTS, MAX_ANALYSER_LOOT_ITEMS,
        };
        let limit = MAX_ANALYSER_BATCH_FACTS * MAX_ANALYSER_LOOT_ITEMS;
        let surplus = (self.items.len() + items.len()).saturating_sub(limit);
        self.items.drain(..surplus.min(self.items.len()));
        self.items.extend_from_slice(items);
        if items.is_empty() {
            self.gold = self.gold.saturating_add(gold);
        }
    }
    pub(in crate::play) fn market_price(&mut self, id: u32, price: Option<u64>) {
        if let Some(price) = price {
            if self.market_prices.len() >= oteryn_client_ui::analyser::tracking::MAX_RECENT
                && !self.market_prices.contains_key(&id)
            {
                self.market_prices.pop_first();
            }
            self.market_prices.insert(id, price);
        } else {
            self.market_prices.remove(&id);
        }
    }
}

#[cfg(bevy_shell)]
impl crate::play::PlayLink {
    pub(crate) fn loot_receipt(&self) -> u64 {
        let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
        mailbox.screen_loot.presented = true;
        mailbox.screen_loot.receipt
    }
    pub(crate) fn loot_values(&self) -> Vec<(String, u64)> {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        let mut values = Vec::new();
        for item in &mailbox.screen_loot.items {
            let id = item.item_type.get();
            let info = mailbox.analyser.item_info.get(&id);
            let name = info.map_or_else(|| format!("#{id}"), |info| info.name.clone());
            let worth = mailbox
                .analyser
                .loot_prices
                .get(&id)
                .copied()
                .or_else(|| info.map(|i| i.coin_worth).filter(|n| *n > 0))
                .or_else(|| mailbox.screen_loot.market_prices.get(&id).copied())
                .or_else(|| {
                    mailbox
                        .npc_shop
                        .live
                        .window
                        .offers
                        .iter()
                        .filter(|offer| {
                            offer.item_definition_ref == id
                                && offer.side == oteryn_session::NpcTradeSide::Sell
                        })
                        .map(|offer| u64::from(offer.unit_price))
                        .max()
                })
                .unwrap_or(0);
            values.push((format!("{} × {name}", item.count), worth));
        }
        if mailbox.screen_loot.gold > 0 {
            values.push((format!("{} gold", mailbox.screen_loot.gold), 1));
        }
        values
    }
}
