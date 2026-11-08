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
                "House town key",
                "Item definition reference",
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
                "Query Houses",
                "Next Houses",
                "Previous analyser",
                "Next analyser",
                "Refresh section",
                "Previous creature",
                "Next creature",
                "Load item definition",
                "Previous item category",
                "Next item category",
                "Previous map floor",
                "Next map floor",
                "Toggle POI layer",
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
                        mailbox.properties.house.fields[1].clone(),
                        mailbox.discovery.inspection.catalogue_definition.clone(),
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
                if (23..=25).contains(&action) {
                    let panel = &mut mailbox.properties.cyclopedia_map;
                    if action == 25 {
                        panel.hide_pois = !panel.hide_pois;
                    } else {
                        panel.select_floor(action == 23);
                    }
                    panel.navigate(oteryn_client_ui::cyclopedia_map::Section::Map);
                    return;
                }
                if (20..=22).contains(&action) {
                    let panel = &mut mailbox.discovery.inspection;
                    panel.search = values.get(1).cloned().unwrap_or_default();
                    panel.category = values.get(2).cloned().unwrap_or_default();
                    if action == 20 {
                        if !panel.pending {
                            panel.catalogue_definition = values.get(4).cloned().unwrap_or_default();
                            actions::load_item_definition(mailbox, commands);
                        }
                    } else {
                        panel.select_category(action == 21);
                    }
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Items);
                    return;
                }
                if action == 18 || action == 19 {
                    mailbox
                        .properties
                        .charm_progression
                        .select_race(action == 18);
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Bestiary);
                    return;
                }
                if action == 17 {
                    actions::refresh_section(mailbox, commands);
                    return;
                }
                if action == 15 || action == 16 {
                    select_analytics(mailbox, action == 15);
                    return;
                }
                if action == 13 || action == 14 {
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Houses);
                    if mailbox.properties.house.model.pending.is_none() {
                        mailbox.properties.house.fields[1] =
                            values.get(3).cloned().unwrap_or_default();
                        crate::play::feature_0049_house::input::submit(
                            mailbox,
                            commands,
                            usize::from(action == 14),
                        );
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
        Section::Houses => oteryn_client_ui::house::panel_text(&mailbox.properties.house.model),
    };
    format!("Cyclopedia > {}\r\n{body}", panel.section.title())
}

#[cfg(any(windows, test))]
fn select_analytics(mailbox: &mut crate::play::mailbox::Mailbox, previous: bool) {
    use oteryn_client_ui::{analyser::Tool, cyclopedia_map::Section};
    let index = Tool::CYCLOPEDIA
        .iter()
        .position(|tool| *tool == mailbox.analyser.tool)
        .unwrap_or(0);
    let index = if previous {
        (index + Tool::CYCLOPEDIA.len() - 1) % Tool::CYCLOPEDIA.len()
    } else {
        (index + 1) % Tool::CYCLOPEDIA.len()
    };
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, index);
    mailbox
        .properties
        .cyclopedia_map
        .navigate(Section::Analytics);
}

#[cfg(any(windows, test))]
#[path = "view/actions.rs"]
mod actions;

#[cfg(test)]
#[path = "view/tests.rs"]
mod navigation_tests;
