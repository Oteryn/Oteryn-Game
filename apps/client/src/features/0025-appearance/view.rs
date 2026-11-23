use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn appearance_panel(&self) -> oteryn_client_ui::appearance::Panel {
        lock(&self.mailbox).properties.appearance.clone()
    }
}

#[cfg(test)]
#[path = "view/preview.rs"]
pub(in crate::play) mod gallery_preview;


#[path = "view/memorial.rs"]
pub(in crate::play) mod memorial;

#[cfg(all(test, not(bevy_shell)))]
#[path = "preview.rs"]
pub(in crate::play) mod preview;

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_outfit_test_panel(&self, panel: oteryn_client_ui::appearance::Panel) {
        lock(&self.0).properties.appearance = panel;
    }
}

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn outfit_preview_fixture(&self, title: oteryn_client_runtime::character_info_panel::CharacterTitle, page: oteryn_client_runtime::cyclopedia_character::gallery::Page) {
        let mut mailbox = lock(&self.0);
        mailbox.properties.character_info.own_titles = Some(oteryn_client_ui::character_info::OwnTitles { earned: vec![title], selected: Some(title) });
        mailbox.cyclopedia_character.gallery.page = Some(page);
    }
}
