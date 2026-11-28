use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[cfg(any(bevy_shell, test))]
    pub(crate) fn quick_loot_corpse_item(
        &self,
        container: oteryn_session::ItemHandle,
        corpse: [u8; 16],
        item: oteryn_session::ItemHandle,
    ) -> bool {
        use oteryn_client_ui::quick_loot::{Action, request};
        let command = {
            let mut mailbox = lock(&self.mailbox);
            let view = &mailbox.player_trade.items.corpse;
            if view.container_handle != Some(container)
                || !view.entries.iter().any(|entry| entry.handle == item)
                || super::view::corpse_identity(&mailbox, container) != Some(corpse)
            {
                return false;
            }
            request(
                &mut mailbox.discovery.loot,
                Action::CorpseItem { corpse, item },
            )
        };
        command.is_some_and(|command| self.discovery_send(command))
    }

    /// Sends a Quick Loot action. Container row callers supply the selected live handle
    /// to `Action::Distribute`; the server resolves its ownership and contents.
    pub fn quick_loot_action(&self, action: oteryn_client_ui::quick_loot::Action) -> bool {
        let command =
            oteryn_client_ui::quick_loot::request(&mut lock(&self.mailbox).discovery.loot, action);
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn discovery_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::quick_loot::nearby_key(events) {
            self.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll);
            return true;
        }
        let panel = if oteryn_client_ui::quick_loot::open_key(events) {
            98
        } else if oteryn_client_ui::inspection::open_key(events) {
            132
        } else if oteryn_client_ui::team_finder::open_key(events) {
            232
        } else if oteryn_client_ui::purchase_history::open_key(events) {
            285
        } else {
            return false;
        };
        if panel == 285 {
            lock(&self.mailbox).store.page = oteryn_client_ui::store::StorePage::History;
        }
        self.discovery_open(panel);
        true
    }
}

impl PlayLink {
    pub(super) fn quick_loot_open_main(&self, mailbox: &mut crate::play::mailbox::Mailbox) -> bool {
        if mailbox.inventory_shutdown {
            return false;
        }
        let Some(backpack) = mailbox
            .inventory
            .inventory
            .as_ref()
            .and_then(|inventory| inventory.main_backpack)
        else {
            return false;
        };
        let Some(intent) = mailbox.containers.open(backpack.handle, None) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ContainerView(intent))
            .is_err()
        {
            mailbox.containers.end();
            return false;
        }
        mailbox.containers.open = true;
        true
    }
    pub fn quick_loot_control(&self, action: usize, fields: &[String], row: usize) -> bool {
        use oteryn_client_ui::quick_loot::{
            Action, QuickLootCategory as Category, QuickLootFilterMode as Mode, QuickLootRecipient,
        };
        if action == 9 {
            let mut mailbox = lock(&self.mailbox);
            let panel = &mailbox.discovery.loot;
            if !panel.managed || panel.pending.is_some() {
                return false;
            }
            let main = if panel.destinations.is_empty() {
                row == 0
            } else {
                matches!(
                    panel.destinations.get(row),
                    Some(Action::Destination {
                        recipient: QuickLootRecipient::MainContainer,
                        ..
                    })
                )
            };
            return main && self.quick_loot_open_main(&mut mailbox);
        }
        if action == 7 {
            let mut mailbox = lock(&self.mailbox);
            mailbox.discovery.loot.managed = !mailbox.discovery.loot.managed;
            return true;
        }
        if action == 8 {
            let panel = self.discovery_panel().loot;
            if !panel.managed {
                return false;
            }
            let Some(Action::Destination { category, .. }) = panel.destinations.get(row) else {
                return false;
            };
            return self.quick_loot_action(Action::Destination {
                category: *category,
                recipient: QuickLootRecipient::MainContainer,
                id: None,
                fallback: true,
            });
        }
        if (2..=5).contains(&action) && self.discovery_panel().loot.managed {
            return false;
        }
        let mut mode = match fields.get(1).map(String::as_str) {
            Some("1") => Mode::Skip,
            Some("2") => Mode::Accept,
            _ if action == 1 || action == 6 => Mode::Skip,
            _ => return false,
        };
        let request = match action {
            1 => Action::LootAll,
            2 | 3 => {
                let definition = if action == 3 && fields.first().is_none_or(String::is_empty) {
                    let panel = self.discovery_panel().loot;
                    let Some((selected_mode, definition)) = panel
                        .skipped
                        .iter()
                        .map(|id| (Mode::Skip, *id))
                        .chain(panel.accepted.iter().map(|id| (Mode::Accept, *id)))
                        .nth(row)
                    else {
                        return false;
                    };
                    mode = selected_mode;
                    definition
                } else {
                    let Some(definition) = fields.first().and_then(|v| v.parse().ok()) else {
                        return false;
                    };
                    definition
                };
                if action == 2 {
                    Action::Add { mode, definition }
                } else {
                    Action::Remove { mode, definition }
                }
            }
            4 => Action::Clear(mode),
            5 => Action::Select(mode),
            6 => {
                let category = match fields.get(2).and_then(|value| value.parse::<u32>().ok()) {
                    Some(0) => None,
                    Some(value) => {
                        let Some(category) = Category::from_wire(value) else {
                            return false;
                        };
                        if value > Category::Other as u32
                            && !lock(&self.mailbox).discovery.loot.destinations_selected
                        {
                            return false;
                        }
                        Some(category)
                    }
                    None => return false,
                };
                let recipient = match fields.get(4).map(String::as_str) {
                    Some("1") => QuickLootRecipient::Container,
                    Some("2") => QuickLootRecipient::GoldPouch,
                    Some("3") | None => QuickLootRecipient::MainContainer,
                    _ => return false,
                };
                let id = if recipient == QuickLootRecipient::MainContainer {
                    None
                } else {
                    let Some(value) = fields.get(5) else {
                        return false;
                    };
                    let value = value.replace('-', "");
                    if value.len() != 32 || !value.is_ascii() {
                        return false;
                    }
                    let mut id = [0; 16];
                    for (index, byte) in id.iter_mut().enumerate() {
                        let Ok(value) = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                        else {
                            return false;
                        };
                        *byte = value;
                    }
                    Some(id)
                };
                let fallback = match fields.get(3).map(String::as_str) {
                    Some("1") => true,
                    Some("0") => false,
                    _ => return false,
                };
                Action::Destination {
                    category,
                    recipient,
                    id,
                    fallback,
                }
            }
            _ => return false,
        };
        self.quick_loot_action(request)
    }
}
