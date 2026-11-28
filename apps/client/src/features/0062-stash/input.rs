use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::stash::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn stash_reverse(&self) {
        let mut mailbox = lock(&self.mailbox);
        if Self::stash_enqueue_request(&mut mailbox, &self.commands, Intent::Filter) {
            mailbox.stash.reverse = !mailbox.stash.reverse;
            mailbox.stash.last_server_page = None;
        }
    }
    pub fn stash_sort(&self) {
        Self::stash_enqueue_sort(&mut lock(&self.mailbox), &self.commands);
    }

    pub(in crate::play) fn stash_enqueue_sort(
        mailbox: &mut Mailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    ) {
        use oteryn_client_ui::stash::Sort;
        if !mailbox.stash.selected || mailbox.stash.pending {
            return;
        }
        let previous = mailbox.stash.sort;
        mailbox.stash.sort = match previous {
            Sort::Name => Sort::Stock,
            Sort::Stock => Sort::Name,
        };
        if Self::stash_enqueue_request(mailbox, commands, Intent::Filter) {
            mailbox.stash.sort_edit = crate::play::feature_0017_attack::view::next_edit();
        } else {
            mailbox.stash.sort = previous;
        }
    }

    pub fn sync_stash_sort(&self, stock_order: &mut bool, edit: u64) -> Option<u64> {
        use oteryn_client_ui::stash::Sort;
        let mut mailbox = lock(&self.mailbox);
        if mailbox.stash.sort_edit > edit {
            *stock_order = mailbox.stash.sort == Sort::Stock;
            Some(mailbox.stash.sort_edit)
        } else {
            mailbox.stash.sort = if *stock_order { Sort::Stock } else { Sort::Name };
            mailbox.stash.sort_edit = edit;
            None
        }
    }

    pub fn stash_stow_selected(
        &self,
        expected: &oteryn_client_ui::stash::StashPanel,
        row: usize,
        stackable: impl FnOnce(u16) -> Option<bool>,
    ) {
        let mut mailbox = lock(&self.mailbox);
        if &mailbox.stash != expected || !mailbox.stash.open {
            return;
        }
        if let Some(Intent::Stow(handle)) =
            oteryn_client_ui::stash::carried_intent(&mailbox.stash, &expected.carried, row)
            && let Some(command) =
                oteryn_client_ui::stash::request_stow(&mut mailbox.stash, handle, stackable)
            && self.commands.send(PlayCommand::Stash(command)).is_err()
        {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some(tr!("The session has ended").into());
        }
    }

    pub fn stash_filter(&self, search: String, category: String, trader: String) {
        let mut mailbox = lock(&self.mailbox);
        Self::stash_enqueue_filter(&mut mailbox, &self.commands, search, category, trader);
    }
    pub(in crate::play) fn stash_enqueue_filter(
        mailbox: &mut Mailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
        search: String,
        category: String,
        trader: String,
    ) {
        if mailbox.stash.pending {
            return;
        }
        mailbox.stash.query = search;
        mailbox.stash.category = category;
        mailbox.stash.trader = trader;
        if let Some(command) = request(&mut mailbox.stash, Intent::Filter)
            && commands.send(PlayCommand::Stash(command)).is_err()
        {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some("The session has ended".into());
        }
    }
    pub fn stash_request(&self, intent: Intent) {
        Self::stash_enqueue_request(&mut lock(&self.mailbox), &self.commands, intent);
    }
    /// Close presentation without issuing a new gameplay request.
    pub fn stash_close(&self) {
        lock(&self.mailbox).stash.open = false;
    }

    /// Consent applies only to the exact panel that supplied the visible row.
    pub fn stash_request_bound(
        &self,
        expected: &oteryn_client_ui::stash::StashPanel,
        intent: Intent,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if &mailbox.stash != expected || !mailbox.stash.open {
            return false;
        }
        Self::stash_enqueue_request(&mut mailbox, &self.commands, intent)
    }

    fn stash_enqueue_request(
        mailbox: &mut Mailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
        intent: Intent,
    ) -> bool {
        let Some(command) = request(&mut mailbox.stash, intent) else {
            return false;
        };
        if commands.send(PlayCommand::Stash(command)).is_err() {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some(tr!("The session has ended").into());
            return false;
        }
        true
    }
    pub fn stash_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::stash::open_key(events) {
            return false;
        }
        self.open_stash();
        true
    }
    pub fn open_stash(&self) {
        Self::stash_open(&self.mailbox, &self.commands);
    }
    pub(in crate::play) fn stash_open(
        shared: &std::sync::Arc<std::sync::Mutex<Mailbox>>,
        commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    ) {
        {
            let mut mailbox = lock(shared);
            if mailbox.depot_shutdown {
                return;
            }
            if let Some(command) = request(&mut mailbox.stash, Intent::Refresh)
                && commands.send(PlayCommand::Stash(command)).is_err()
            {
                mailbox.stash.pending = false;
                mailbox.stash.feedback = Some(tr!("The session has ended").into());
            }
        }
        lock(shared).stash.open = true;
    }
}
