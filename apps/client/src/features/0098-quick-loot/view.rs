use crate::play::{PlayLink, mailbox::lock};

#[cfg(any(bevy_shell, test))]
use oteryn_session::{EntityDetail, EntityKind, ItemHandle, OpenContainer};

#[cfg(any(bevy_shell, test))]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CorpseLootView {
    pub contents: OpenContainer,
    pub identity: Option<[u8; 16]>,
    pub feedback: Option<&'static str>,
}

#[cfg(any(bevy_shell, test))]
pub(super) fn corpse_identity(
    mailbox: &crate::play::mailbox::Mailbox,
    handle: ItemHandle,
) -> Option<[u8; 16]> {
    if !mailbox.discovery.loot.selected {
        return None;
    }
    mailbox
        .entities_wire
        .iter()
        .find_map(|entity| match entity.detail {
            EntityDetail::Object { item_handle, .. }
            | EntityDetail::ObjectWithAppearance { item_handle, .. }
                if entity.kind == EntityKind::Corpse && item_handle == Some(handle) =>
            {
                Some(entity.entity.identity)
            }
            _ => None,
        })
}

impl PlayLink {
    #[cfg(any(bevy_shell, test))]
    pub(crate) fn corpse_loot_view(&self) -> CorpseLootView {
        let mailbox = lock(&self.mailbox);
        let contents = &mailbox.player_trade.items.corpse;
        let identity = contents
            .container_handle
            .and_then(|handle| corpse_identity(&mailbox, handle));
        let feedback = mailbox
            .discovery
            .loot
            .item_feedback
            .filter(|(corpse, _)| Some(*corpse) == identity)
            .map(|(_, message)| message);
        CorpseLootView {
            contents: contents.clone(),
            identity,
            feedback,
        }
    }

    pub fn discovery_panel(&self) -> oteryn_client_ui::quick_loot::Discovery {
        let mailbox = lock(&self.mailbox);
        let mut projection = mailbox.discovery.clone();
        if !mailbox.local_preferences.game_window.loot_messages {
            projection.loot.result = None;
        }
        projection
    }
    pub fn discovery_open(&self, panel: u32) {
        let mut mailbox = lock(&self.mailbox);
        match panel {
            98 => mailbox.discovery.loot.open = true,
            132 => mailbox.discovery.inspection.open = true,
            232 => mailbox.discovery.teams.open = true,
            285 => mailbox.discovery.history.open = true,
            _ => {}
        }
    }
}

/// The existing panel projection, including the player's loot-message visibility choice.
fn loot_panel(mailbox: &crate::play::mailbox::Mailbox) -> oteryn_client_ui::quick_loot::Panel {
    let mut panel = mailbox.discovery.loot.clone();
    if !mailbox.local_preferences.game_window.loot_messages {
        panel.result = None;
    }
    panel
}

#[cfg(bevy_shell)]
fn inbox_gold_pouch(store: &oteryn_client_ui::store::StorePanel) -> Option<[u8; 16]> {
    const GOLD_POUCH_ITEM_ID: u16 = 23721;
    store
        .selected
        .then(|| {
            store
                .inbox
                .entries
                .iter()
                .find(|entry| entry.item_id == GOLD_POUCH_ITEM_ID)
                .map(|entry| entry.fulfilment_unit_id)
        })
        .flatten()
}

#[cfg(bevy_shell)]
pub(crate) fn quick_loot_pouch_route(
    panel: &oteryn_client_ui::quick_loot::Panel,
    inbox_pouch: Option<[u8; 16]>,
    category: Option<oteryn_client_ui::quick_loot::QuickLootCategory>,
    fallback: bool,
) -> Option<oteryn_client_ui::quick_loot::Action> {
    use oteryn_client_ui::quick_loot::{Action, QuickLootRecipient};
    if !panel.destinations_selected
        && category != Some(oteryn_client_ui::quick_loot::QuickLootCategory::Gold)
    {
        return None;
    }
    let inbox_pouch = inbox_pouch.filter(|_| panel.destinations_selected);
    let id = inbox_pouch.or_else(|| {
        panel
            .destinations
            .iter()
            .rev()
            .find_map(|route| match route {
                Action::Destination {
                    recipient: QuickLootRecipient::GoldPouch,
                    id,
                    ..
                } => *id,
                _ => None,
            })
    })?;
    Some(Action::Destination {
        category,
        recipient: QuickLootRecipient::GoldPouch,
        id: Some(id),
        fallback,
    })
}

#[cfg(bevy_shell)]
#[derive(Clone, PartialEq, Eq)]
pub enum QuickLootControl {
    Request(oteryn_client_ui::quick_loot::Action),
    OpenDestination(Option<oteryn_client_ui::quick_loot::QuickLootCategory>),
    RefreshInbox,
    NextInboxPage,
    Catalogue,
    Store,
    Close,
}
impl PlayLink {
    #[must_use]
    pub fn quick_loot_panel(&self) -> oteryn_client_ui::quick_loot::Panel {
        loot_panel(&lock(&self.mailbox))
    }
    #[cfg(bevy_shell)]
    pub(crate) fn quick_loot_inbox(&self) -> (Option<[u8; 16]>, bool, bool) {
        let mailbox = lock(&self.mailbox);
        let store = &mailbox.store;
        (
            inbox_gold_pouch(store),
            store.selected,
            store.inbox.next.is_some(),
        )
    }
    #[cfg(bevy_shell)]
    pub fn quick_loot_dispatch_bound(
        &self,
        expected: &oteryn_client_ui::quick_loot::Panel,
        control: QuickLootControl,
    ) -> bool {
        use oteryn_client_ui::quick_loot::{Action, QuickLootRecipient, request};
        let mut mailbox = lock(&self.mailbox);
        if loot_panel(&mailbox) != *expected || !mailbox.discovery.loot.open {
            return false;
        }
        let pouch = inbox_gold_pouch(&mailbox.store);
        let panel = &mut mailbox.discovery.loot;
        let action = match control {
            QuickLootControl::OpenDestination(category) => {
                if !panel.selected || panel.pending.is_some() || !panel.destinations.iter().any(|entry|
                    matches!(entry, Action::Destination {category: assigned, recipient: QuickLootRecipient::MainContainer, ..} if *assigned == category)) {
                    return false;
                }
                return self.quick_loot_open_main(&mut mailbox);
            }
            QuickLootControl::Close => {
                panel.open = false;
                return true;
            }
            QuickLootControl::Catalogue | QuickLootControl::Store => {
                if !panel.selected || panel.pending.is_some() {
                    return false;
                }
                panel.open = false;
                drop(mailbox);
                if control == QuickLootControl::Store {
                    self.store_open_products();
                } else {
                    self.item_catalogue_open();
                }
                return true;
            }
            QuickLootControl::RefreshInbox | QuickLootControl::NextInboxPage => {
                if !panel.selected || !panel.destinations_selected || panel.pending.is_some() {
                    return false;
                }
                drop(mailbox);
                if control == QuickLootControl::RefreshInbox {
                    self.store_inbox_refresh();
                } else {
                    self.store_inbox_next_claim_page();
                }
                return true;
            }
            QuickLootControl::Request(action) => action,
        };
        if !panel.selected || panel.pending.is_some() {
            return false;
        }
        let allowed = match &action {
            Action::LootAll | Action::Select(_) | Action::Clear(_) => true,
            Action::Destination { category, recipient, id, fallback } =>
                (*recipient == QuickLootRecipient::MainContainer && id.is_none())
                || quick_loot_pouch_route(panel, pouch, *category, *fallback).as_ref() == Some(&action)
                || panel.destinations.iter().any(|current| matches!(current,
                    Action::Destination {category: c,recipient: r,id: i,..} if c==category && r==recipient && i==id)),
            _ => false,
        };
        if !allowed {
            return false;
        }
        let Some(command) = request(panel, action) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
        {
            panel.pending = None;
            panel.feedback = tr!("Session ended").into();
            return false;
        }
        true
    }
}

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_quick_loot_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn bevy_quick_loot_selected(&self, selected: bool) {
        lock(&self.0).discovery.loot.selected = selected;
    }
    pub(crate) fn bevy_quick_loot_managed_selected(&self) {
        lock(&self.0).discovery.loot.destinations_selected = true;
    }
    pub(crate) fn bevy_quick_loot_corpse(&self, entity: oteryn_session::WorldSpatialEntity) {
        self.entities(vec![entity]);
    }
    pub(crate) fn bevy_quick_loot_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.discovery_result(98, response);
    }
}
