use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cyclopedia_map_panel(&self) -> oteryn_client_ui::cyclopedia_map::Panel {
        lock(&self.mailbox).properties.cyclopedia_map.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Cyclopedia",
            labels: &[
                "Donation gold (1-10000000)",
                "Item name search",
                "Item category (empty = all)",
            ],
            buttons: &[
                "Character",
                "Items",
                "Map",
                "Bestiary",
                "Charms",
                "Bosses",
                "Houses",
                "Analytics",
                "Query map",
                "Start",
                "Shuffle",
                "Find",
                "Donate",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.cyclopedia_map;
                (
                    panel.open,
                    false,
                    vec![
                        panel.fields[0].clone(),
                        mailbox.discovery.inspection.search.clone(),
                        mailbox.discovery.inspection.category.clone(),
                    ],
                    section_text(mailbox),
                )
            },
            set_open: |mailbox, value| mailbox.properties.cyclopedia_map.open = value,
            submit: |mailbox, commands, action, values| {
                if let Some(section) = oteryn_client_ui::cyclopedia_map::Section::ALL.get(action) {
                    mailbox.properties.cyclopedia_map.navigate(*section);
                    if *section == oteryn_client_ui::cyclopedia_map::Section::Items {
                        mailbox.discovery.inspection.search =
                            values.get(1).cloned().unwrap_or_default();
                        mailbox.discovery.inspection.category =
                            values.get(2).cloned().unwrap_or_default();
                    }
                    return;
                }
                let panel = &mut mailbox.properties.cyclopedia_map;
                panel.navigate(oteryn_client_ui::cyclopedia_map::Section::Map);
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action.saturating_sub(8) {
                    0 => oteryn_client_ui::cyclopedia_map::Intent::Query,
                    1 => oteryn_client_ui::cyclopedia_map::Intent::Start,
                    2 => oteryn_client_ui::cyclopedia_map::Intent::Shuffle,
                    3 => oteryn_client_ui::cyclopedia_map::Intent::Find,
                    4 => oteryn_client_ui::cyclopedia_map::Intent::Donate,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(any(windows, test))]
fn section_text(mailbox: &crate::play::mailbox::Mailbox) -> String {
    use oteryn_client_ui::cyclopedia_map::Section;
    let panel = &mailbox.properties.cyclopedia_map;
    let body = match panel.section {
        Section::Character => {
            oteryn_client_ui::cyclopedia_character::panel_text(&mailbox.cyclopedia_character)
        }
        Section::Map => oteryn_client_ui::cyclopedia_map::panel_text(panel),
        Section::Bestiary => oteryn_client_ui::charm_progression::bestiary_text(
            &mailbox.properties.charm_progression,
        ),
        Section::Charms => {
            oteryn_client_ui::charm_progression::charms_text(&mailbox.properties.charm_progression)
        }
        Section::Bosses => oteryn_client_ui::boss_encounter::panel_text(&mailbox.boss_encounter),
        Section::Analytics => oteryn_client_ui::analyser::panel_text(&mailbox.analyser),
        Section::Items => {
            oteryn_client_ui::inspection::catalogue_text(&mailbox.discovery.inspection)
        }
        Section::Houses => "House catalogue is not loaded".into(),
    };
    format!("Cyclopedia > {}\r\n{body}", panel.section.title())
}

#[cfg(test)]
mod navigation_tests {
    #[test]
    fn host_reads_selected_section_without_losing_map_state() {
        let mut mailbox = crate::play::mailbox::Mailbox::default();
        mailbox.properties.cyclopedia_map.summary = "retained map".into();
        mailbox
            .properties
            .cyclopedia_map
            .navigate(oteryn_client_ui::cyclopedia_map::Section::Charms);
        assert!(super::section_text(&mailbox).starts_with("Cyclopedia > Charms"));
        mailbox
            .properties
            .cyclopedia_map
            .navigate(oteryn_client_ui::cyclopedia_map::Section::Map);
        assert!(super::section_text(&mailbox).contains("retained map"));
    }
}
