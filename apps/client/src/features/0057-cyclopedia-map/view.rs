use crate::play::{PlayLink, mailbox::lock};
impl crate::play::PlayView {
    pub(in crate::play) fn present_cyclopedia_discovery(&self, link: &PlayLink) {
        let notice = lock(&link.mailbox)
            .properties
            .cyclopedia_map
            .discovery_banner
            .take();
        if let Some(sequence) = notice {
            self.game_notifications.show(
                &format!("cyclopedia-discovery:{sequence}"),
                tr!("Subarea discovered! All seven points of interest found."),
                false,
            );
        }
    }
}

impl PlayLink {
    pub fn cyclopedia_atlas_panel(&self) -> Option<oteryn_client_ui::cyclopedia_map::Panel> {
        let mailbox = lock(&self.mailbox);
        let panel = &mailbox.properties.cyclopedia_map;
        panel.geometry.open.then(|| panel.clone())
    }
    pub fn cyclopedia_atlas_close(&self) {
        lock(&self.mailbox).properties.cyclopedia_map.geometry.open = false;
    }
    pub fn cyclopedia_map_panel(&self) -> oteryn_client_ui::cyclopedia_map::Panel {
        lock(&self.mailbox).properties.cyclopedia_map.clone()
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
#[path = "view/actions.rs"]
mod actions;

#[cfg(test)]
#[path = "view/tests.rs"]
mod navigation_tests;

#[cfg(test)]
#[path = "view/items.rs"]
mod items;

#[cfg(test)]
#[path = "view/item_tests.rs"]
mod item_tests;


#[cfg(test)]
#[path = "view/analytics.rs"]
mod analytics;
#[cfg(test)]
#[path = "view/analytics_tests.rs"]
mod analytics_tests;

#[cfg(test)]
#[path = "view/atlas_tests.rs"]
mod atlas_tests;

#[cfg(test)]
#[path = "view/discovery_tests.rs"]
mod discovery_tests;

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_cyclopedia_map_selected(&self) {
        lock(&self.0).properties.cyclopedia_map.selected = true;
    }
    pub(crate) fn bevy_cyclopedia_map_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.properties_result(command, result);
    }
}
