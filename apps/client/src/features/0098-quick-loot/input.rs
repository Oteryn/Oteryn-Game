use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn quick_loot_action(&self, action: oteryn_client_ui::quick_loot::Action) -> bool {
        let command =
            oteryn_client_ui::quick_loot::request(&mut lock(&self.mailbox).discovery.loot, action);
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn discovery_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
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
    pub fn quick_loot_control(&self, action: usize, fields: &[String], row: usize) -> bool {
        use oteryn_client_ui::quick_loot::{
            Action, QuickLootCategory as Category, QuickLootFilterMode as Mode, QuickLootRecipient,
        };
        if action == 9 {
            return self.discovery_panel().loot.managed && self.inventory_open_container();
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
                category: *category, recipient: QuickLootRecipient::MainContainer,
                id: None, fallback: true,
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
                let category = match fields.get(2).map(String::as_str) {
                    Some("0") => None,
                    Some("1") => Some(Category::Gold),
                    Some("2") => Some(Category::CreatureProduct),
                    Some("3") => Some(Category::Equipment),
                    Some("4") => Some(Category::Consumable),
                    Some("5") => Some(Category::Other),
                    _ => return false,
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
