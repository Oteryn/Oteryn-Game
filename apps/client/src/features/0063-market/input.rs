use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::market::{Intent, request};
impl PlayLink {
    pub fn market_filter_side(&self, side: u8) -> bool {
        if side > 2 {
            return false;
        }
        let mut m = lock(&self.mailbox);
        m.market.side_filter = side;
        m.market.selected_offer = None;
        true
    }

    pub fn market_select_offer(&self, index: usize) {
        let mut m = lock(&self.mailbox);
        m.market.selected_offer = oteryn_client_ui::market::visible_rows(&m.market)
            .get(index)
            .and_then(|index| m.market.rows.get(*index))
            .map(|r| r.offer);
    }

    pub fn market_details(
        &self,
        item: &str,
        amount: &str,
        price: &str,
        sell: bool,
        anonymous: bool,
    ) -> bool {
        oteryn_client_ui::market::edit_details(
            &mut lock(&self.mailbox).market,
            item,
            amount,
            price,
            sell,
            anonymous,
        )
    }
    pub fn market_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::market::coin_open_key(events) {self.open_coin_market();return true;}
        if !oteryn_client_ui::market::open_key(events) {
            return false;
        }
        self.open_market();
        true
    }

    pub fn market_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.market, intent)
            && self.commands.send(PlayCommand::Market(command)).is_err()
        {
            mailbox.market.pending = None;
            mailbox.market.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn market_stashback(&self) -> bool {
        stashback(&self.mailbox, &self.commands)
    }

    pub fn open_market(&self) {
        open_from_depot(&self.mailbox, &self.commands);
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.market, intent)
        && commands.send(PlayCommand::Market(command)).is_err()
    {
        mailbox.market.pending = None;
        mailbox.market.feedback = Some(tr!("The session has ended"));
    }
}

pub(in crate::play) fn open_from_depot(
    mailbox: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    {
        let mut m = lock(mailbox);
        if m.market_shutdown {
            return;
        }
        if m.stash.open {
            m.stash.return_from_market = true;
            m.stash.open = false;
        }
        enqueue(&mut m, commands, Intent::Refresh);
    }
    #[cfg(windows)]
    if !lock(mailbox).shared_frontend {
        super::view::open_window(mailbox, commands);
    }
    if cfg!(not(windows)) || lock(mailbox).shared_frontend {
        lock(mailbox).market.open = true;
    }
}

pub(in crate::play) fn stashback(
    mailbox: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) -> bool {
    {
        let mut m = lock(mailbox);
        if !m.stash.return_from_market || m.depot_shutdown || m.market_shutdown {
            return false;
        }
        m.stash.return_from_market = false;
        m.market.open = false;
    }
    PlayLink::stash_open(mailbox, commands);
    true
}

#[path="input/coins.rs"]
mod coins;
