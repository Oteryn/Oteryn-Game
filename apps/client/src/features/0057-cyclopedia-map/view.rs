use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cyclopedia_map_panel(&self) -> oteryn_client_ui::cyclopedia_map::Panel {
        lock(&self.mailbox).properties.cyclopedia_map.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    atlas_native::open(link);
    use crate::play::feature_0115_mail::view::native::{Form, open};
    atlas_native::open(link);
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
                "Exploration status",
                "Reset POIs",
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
                "Analytics overview",
                "Hunting analyser",
                "Experience analyser",
                "Loot analyser",
                "Supplies analyser",
                "Damage analyser",
                "Healing analyser",
                "Browse creature catalogue",
                "Next creature page",
                "Creature details",
                "Next creature loot page",
                "Previous creature category",
                "Next creature category",
                "Browse items",
                "Next items",
                "Previous item",
                "Next item",
                "Market offers",
                "Loot sources",
                "Next loot sources",
                "Next market offers",
                "Load region atlas",
                "Next atlas page",
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
                if (26..=32).contains(&action) && analytics::select(mailbox, action) {
                    return;
                }
                if (33..=38).contains(&action) {
                    crate::play::feature_0067_bestiary_charm_progression::input::catalogue::submit(
                        mailbox,
                        commands,
                        action - 33,
                    );
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Bestiary);
                    return;
                }
                if action == 47 || action == 48 {
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Map);
                    super::input::submit(
                        mailbox,
                        commands,
                        if action == 47 {
                            oteryn_client_ui::cyclopedia_map::Intent::RegionAtlas
                        } else {
                            oteryn_client_ui::cyclopedia_map::Intent::AtlasNext
                        },
                    );
                    return;
                }
                if actions::navigate_section(mailbox, action, &values) {
                    return;
                }
                if (41..=46).contains(&action) {
                    match action {
                        41 | 42 => {
                            mailbox.discovery.inspection.select_item(action == 41);
                        }
                        43 | 46 => actions::item_market(mailbox, commands, action == 46),
                        44 | 45 => actions::item_sources(mailbox, commands, action == 45),
                        _ => {}
                    }
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Items);
                    return;
                }
                if action == 39 || action == 40 {
                    let panel = &mut mailbox.discovery.inspection;
                    panel.search = values.get(1).cloned().unwrap_or_default();
                    panel.category = values.get(2).cloned().unwrap_or_default();
                    actions::browse_items(mailbox, commands, action == 40);
                    mailbox
                        .properties
                        .cyclopedia_map
                        .navigate(oteryn_client_ui::cyclopedia_map::Section::Items);
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
                    if mailbox
                        .properties
                        .charm_progression
                        .catalogue
                        .page
                        .cards
                        .is_empty()
                    {
                        mailbox
                            .properties
                            .charm_progression
                            .select_race(action == 18);
                    } else {
                        crate::play::feature_0067_bestiary_charm_progression::input::catalogue::submit(mailbox,commands,if action==18 {6}else{7});
                    }
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
                    1 => oteryn_client_ui::cyclopedia_map::Intent::Query,
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
        Section::Analytics => analytics::text(mailbox),
        Section::Items => items::text(mailbox),
        Section::Houses => oteryn_client_ui::house::panel_text(&mailbox.properties.house.model),
    };
    lfmt!(
        "Cyclopedia > {}\r\n{body}",
        panel.section.title(),
        body = body
    )
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

#[cfg(any(windows, test))]
#[path = "view/items.rs"]
mod items;

#[cfg(test)]
#[path = "view/item_tests.rs"]
mod item_tests;

#[cfg(windows)]
#[path = "view/atlas_native.rs"]
mod atlas_native;

#[cfg(any(windows, test))]
#[path = "view/analytics.rs"]
mod analytics;
#[cfg(test)]
#[path = "view/analytics_tests.rs"]
mod analytics_tests;
#[cfg(windows)]
#[path = "view/atlas.rs"]
pub(in crate::play) mod atlas;

#[cfg(test)]
#[path = "view/atlas_tests.rs"]
mod atlas_tests;
