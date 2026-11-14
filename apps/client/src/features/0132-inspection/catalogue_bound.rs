//! Atomic presentation admission for the shared frontend; native submission is unchanged.
use super::*;
use oteryn_client_ui::{inspection::Panel, market::MarketPanel};
use std::sync::{Arc, Mutex, Weak};
#[derive(Clone, Debug)]
pub struct ItemCatalogueSnapshot {
    pub panel: Panel,
    pub text: String,
    market: MarketPanel,
    session: Weak<Mutex<Mailbox>>,
}
impl PartialEq for ItemCatalogueSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.panel == other.panel
            && self.market == other.market
            && self.text == other.text
    }
}
impl Eq for ItemCatalogueSnapshot {}
impl ItemCatalogueSnapshot {
    pub fn fields(&self) -> [String; 3] {
        [
            self.panel.search.clone(),
            self.panel.category.clone(),
            self.panel.catalogue_definition.clone(),
        ]
    }
}
impl PlayLink {
    pub fn item_catalogue_snapshot(&self) -> ItemCatalogueSnapshot {
        let m = lock(&self.mailbox);
        ItemCatalogueSnapshot {
            panel: m.discovery.inspection.clone(),
            market: m.market.clone(),
            text: text(&m),
            session: Arc::downgrade(&self.mailbox),
        }
    }
    /// Reopening retains pending settlement; opening an idle window uses the native Browse owner.
    pub fn item_catalogue_open_bound(&self) -> bool {
        let mut m = lock(&self.mailbox);
        if self.commands.is_closed()
            || m.quest_shutdown
            || m.custom_hotkeys_ended
            || m.world_input_blocked()
            || m.server_modal.is_active()
        {
            return false;
        }
        if m.discovery.inspection.catalogue_open {
            return false;
        }
        m.discovery.inspection.catalogue_open = true;
        super::super::input::catalogue::submit(&mut m, &self.commands, 0);
        true
    }
    /// Close is session-only and preserves requests. All other controls compare under the owner lock.
    pub fn item_catalogue_dispatch_bound(
        &self,
        expected: &ItemCatalogueSnapshot,
        fields: [String; 3],
        action: Option<usize>,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || self.commands.is_closed()
            || m.quest_shutdown
            || m.custom_hotkeys_ended
        {
            return false;
        }
        if action.is_none() {
            let was_open = m.discovery.inspection.catalogue_open;
            m.discovery.inspection.catalogue_open = false;
            return was_open;
        }
        if m.world_input_blocked()
            || m.server_modal.is_active()
            || m.discovery.inspection != expected.panel
            || m.market != expected.market
            || !m.discovery.inspection.catalogue_open
            || !m.discovery.inspection.selected
            || m.discovery.inspection.pending
        {
            return false;
        }
        let Some(action) = action.filter(|action| *action < 10) else {
            return false;
        };
        // A repeated market browse must not alter even feedback while its reply is pending.
        if action == 9 && (m.market.pending.is_some() || m.market_shutdown) {
            return false;
        }
        // Same field limits as the existing search owner; definitions are unsigned wire references.
        if fields[0].len() > 256
            || fields[1].len() > 128
            || fields.iter().any(|field| field.contains('\0'))
        {
            return false;
        }
        let before = m.discovery.inspection.clone();
        let market = m.market.clone();
        let panel = &mut m.discovery.inspection;
        panel.search.clone_from(&fields[0]);
        panel.category.clone_from(&fields[1]);
        panel.catalogue_definition.clone_from(&fields[2]);
        super::super::input::catalogue::submit(&mut m, &self.commands, action);
        m.discovery.inspection != before || m.market != market
    }
}
