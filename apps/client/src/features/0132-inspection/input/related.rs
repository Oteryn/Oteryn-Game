//! Navigation is bound to the displayed inspection, never the current inventory selection.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::inspection::{Panel, Request};
impl PlayLink {
    pub fn inspected_item_related(
        &self,
        expected: &Panel,
        index: usize,
        proficiency: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.discovery.inspection != *expected || expected.pending || !expected.open {
            return false;
        }
        let Some(item) = expected
            .reply
            .as_ref()
            .filter(|reply| reply.status == 1)
            .and_then(|reply| reply.items.get(index))
        else {
            return false;
        };
        if item.definition_ref == 0 {
            return false;
        }
        if proficiency {
            let panel = &mut mailbox.weapon_proficiency;
            if !panel.selected || panel.pending {
                return false;
            }
            let row = panel.catalogue.as_ref().and_then(|page| {
                page.entries
                    .iter()
                    .position(|entry| entry.definition == item.definition_ref)
            });
            if let Some(row) = row {
                if !panel.select_weapon(row) {
                    return false;
                }
            } else {
                if !item.attributes.iter().any(|(label, value)| {
                    label == "Weapon Type"
                        && matches!(
                            value.split(',').next().map(str::trim),
                            Some("axe" | "club" | "distance" | "fist" | "sword" | "wand")
                        )
                }) {
                    return false;
                }
                panel.weapon = item.definition_ref;
                panel.progress_weapon = item.definition_ref;
                panel.shaping = None;
                panel.revision = 0;
                panel.state_refresh_needed = panel.state_selected;
            }
            panel.open = true;
            mailbox.discovery.inspection.open = false;
            drop(mailbox);
            self.weapon_proficiency_tick(std::time::Instant::now());
            true
        } else {
            let panel = &mut mailbox.discovery.inspection;
            let Some(command) = oteryn_client_ui::inspection::request(
                panel,
                Request::Catalogue(item.definition_ref),
            ) else {
                return false;
            };
            panel.catalogue_definition = item.definition_ref.to_string();
            panel.catalogue_open = true;
            panel.open = false;
            drop(mailbox);
            self.discovery_send(command)
        }
    }
}
