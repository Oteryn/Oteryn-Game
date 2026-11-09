use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::npc_shop::Intent;
impl PlayLink {
    pub fn npc_shop_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = oteryn_client_ui::npc_shop::request(&mut mailbox.npc_shop, intent)
            && self.commands.send(PlayCommand::NpcShop(command)).is_err()
        {
            mailbox.npc_shop.pending = false;
            mailbox.npc_shop.feedback = Some(tr!("Session ended"));
        }
    }
}
impl PlayLink {
    pub fn npc_trade_request(&self, quote: oteryn_client_ui::npc_shop::Quote, amount: u32) {
        trade_request(&self.mailbox, &self.commands, quote, amount);
    }
}
pub(in crate::play) fn trade_request(
    shared: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    quote: oteryn_client_ui::npc_shop::Quote,
    amount: u32,
) {
    let mut mailbox = lock(shared);
    if mailbox.npc_shop.pending {
        return;
    }
    if let Some(intent) =
        oteryn_client_ui::npc_dialogue::trade_request(&mut mailbox.npc_shop.live, quote, amount)
        && commands.send(PlayCommand::NpcTrade(intent)).is_err()
    {
        mailbox.npc_shop.live.pending = false;
        mailbox.npc_shop.live.feedback = tr!("Session ended").into();
    }
}

pub(in crate::play) fn offer_request(
    shared: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    quote: oteryn_client_ui::npc_shop::Quote,
    sell_all: bool,
) {
    let mut mailbox = lock(shared);
    if mailbox.npc_shop.live.pending {
        return;
    }
    if sell_all
        && (quote.side != oteryn_client_ui::npc_shop::NpcTradeSide::Sell || quote.count != 1)
    {
        return;
    }
    if let Some(look) = mailbox.npc_shop.live.look(quote)
        && let Some(command) = oteryn_client_ui::npc_shop::request(
            &mut mailbox.npc_shop,
            if sell_all {
                Intent::SellAll(look)
            } else {
                Intent::Look(look)
            },
        )
        && commands.send(PlayCommand::NpcShop(command)).is_err()
    {
        mailbox.npc_shop.pending = false;
        mailbox.npc_shop.feedback = Some(tr!("Session ended"));
    }
}

impl PlayLink {
    pub fn npc_sell_all_request(&self, quote: oteryn_client_ui::npc_shop::Quote) {
        offer_request(&self.mailbox, &self.commands, quote, true);
    }
}
