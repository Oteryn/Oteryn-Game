use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::inspection::{Request, catalogue, request};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn item_catalogue_filter(&self, search: &str, category: &str) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        if !panel.pending
            && search.len() <= 256
            && category.len() <= 128
            && !search.contains('\0')
            && !category.contains('\0')
        {
            panel.search = search.into();
            panel.category = category.into();
        }
    }
    pub fn item_catalogue_request(&self, action: usize) {
        submit(&mut lock(&self.mailbox), &self.commands, action);
    }
    pub fn item_catalogue_open(&self) {
        self.item_catalogue_request(0);
        #[cfg(windows)]
        super::super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).discovery.inspection.catalogue_open = true;
        }
    }
    pub fn item_catalogue_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        let modifiers = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        if !events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers: actual, repeat: false } if code.get() == 12 && *actual == modifiers)) { return false; }
        self.item_catalogue_open();
        true
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
) {
    let panel = &mut mailbox.discovery.inspection;
    if !panel.selected || panel.pending {
        return;
    }
    panel.search = panel.search.trim().into();
    panel.category = panel.category.trim().into();
    let selected = panel
        .catalogue_definition
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0);
    if let Some(reference) = selected {
        panel.catalogue_definition = reference.to_string();
    }
    if action == 9 {
        let Some(definition) = selected.filter(|n| panel.catalogue.contains_key(n)) else {
            panel.feedback = "Select a catalogue item before browsing offers".into();
            return;
        };
        if mailbox.market.pending.is_some() || !mailbox.market.selected {
            panel.feedback = "Market offers are unavailable or waiting for the server".into();
            return;
        }
        panel.catalogue_market = Some(definition);
        mailbox.market.definition = definition;
        mailbox.market.page = 0;
        mailbox.market.view = oteryn_client_ui::market::MarketView::Browse;
        mailbox.market.rows.clear();
        mailbox.market.selected_offer = None;
        crate::play::feature_0063_market::input::enqueue(
            mailbox,
            commands,
            oteryn_client_ui::market::Intent::Browse,
        );
        return;
    }
    if action == 4 || action == 5 {
        let keys: Vec<_> = panel.catalogue.keys().copied().collect();
        if keys.is_empty() {
            return;
        }
        let current = selected
            .and_then(|id| keys.iter().position(|key| *key == id))
            .unwrap_or(0);
        let index = if action == 4 {
            (current + keys.len() - 1) % keys.len()
        } else {
            (current + 1) % keys.len()
        };
        panel.catalogue_definition = keys[index].to_string();
        panel.catalogue_drop_next = None;
        panel.catalogue_market = None;
        return;
    }
    let query = match action {
        0..=3 => {
            let after = if action == 1 {
                let Some(next) = panel.catalogue_next else {
                    panel.feedback = "End of item catalogue".into();
                    return;
                };
                next
            } else {
                0
            };
            if action == 2 || action == 3 {
                panel.select_category(action == 2);
            }
            let category = if panel.category.trim().is_empty() {
                0
            } else {
                let Some(index) = panel
                    .categories()
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(panel.category.trim()))
                else {
                    panel.feedback =
                        "Browse first, then select one of the server's categories".into();
                    return;
                };
                (index + 1) as u8
            };
            let Ok(query) = catalogue::items(after, category, panel.search.trim()) else {
                panel.feedback = "Use a shorter item name for the search".into();
                return;
            };
            Request::CatalogueQuery(query)
        }
        6..=8 => {
            let Some(reference) = selected else {
                panel.feedback = "Select a catalogue item".into();
                return;
            };
            if action == 6 {
                Request::Catalogue(reference)
            } else {
                let after = if action == 8 {
                    let Some(next) = panel.catalogue_drop_next else {
                        panel.feedback = "End of drop sources".into();
                        return;
                    };
                    next
                } else {
                    0
                };
                Request::CatalogueQuery(catalogue::Query::Drops { reference, after })
            }
        }
        _ => return,
    };
    if let Some(command) = request(panel, query)
        && commands.send(PlayCommand::ServerMechanic(command)).is_err()
    {
        panel.pending = false;
        panel.feedback = "The session has ended".into();
    }
}
