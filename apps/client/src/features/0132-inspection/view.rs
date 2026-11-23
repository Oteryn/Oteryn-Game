use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
impl PlayLink {
    /// Read the retained inspection owner without cloning unrelated discovery panels.
    pub fn inspection_panel(&self) -> oteryn_client_ui::inspection::Panel {
        lock(&self.mailbox).discovery.inspection.clone()
    }
    /// Close/reopen only the existing presentation flag; preserve in-flight work and replies.
    pub fn inspection_set_open(&self, open: bool) {
        lock(&self.mailbox).discovery.inspection.open = open;
    }
    #[cfg(all(test, bevy_shell))]
    pub(crate) fn bevy_item_details_selected(&self, selected: bool) {
        lock(&self.mailbox).discovery.inspection.selected = selected;
    }
    #[cfg(all(test, bevy_shell))]
    pub(crate) fn bevy_item_details_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        crate::play::Outbox(self.mailbox.clone()).discovery_result(132, response);
    }
    #[cfg(all(test, bevy_shell))]
    pub(crate) fn bevy_item_details_link(&self) -> Self {
        Self {
            owns_session: false,
            commands: self.commands.clone(),
            mailbox: self.mailbox.clone(),
        }
    }
    pub fn item_catalogue_text(&self) -> String {
        text(&lock(&self.mailbox))
    }
}
fn text(mailbox: &Mailbox) -> String {
    let panel = &mailbox.discovery.inspection;
    let mut text = oteryn_client_ui::inspection::catalogue_text(panel);
    if panel.catalogue_market == Some(mailbox.market.definition)
        && panel.catalogue_definition == mailbox.market.definition.to_string()
    {
        text.push_str("\r\nRelated market offers:\r\n");
        text.push_str(&oteryn_client_ui::market::panel_text(&mailbox.market));
    }
    text
}

impl crate::play::PlayView {
    #[cfg(bevy_shell)]
    pub(crate) fn inspection_preview(
        &self,
        link: &crate::play::PlayLink,
        entity: oteryn_session::EntityRef,
    ) -> Option<oteryn_client_ui::appearance::Preview> {
        let visible = self
            .appearances
            .iter()
            .find(|entry| entry.entity.entity == entity)?;
        let oteryn_session::Look::Outfit {
            look_type,
            addons,
            colours,
        } = visible.look.as_ref()?
        else {
            return None;
        };
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        let palette = &mailbox.properties.appearance.catalogue.as_ref()?.palette;
        let [Some(head), Some(body), Some(legs), Some(feet)] =
            colours.map(|index| palette.get(usize::from(index)).copied())
        else {
            return None;
        };
        Some(oteryn_client_ui::appearance::Preview {
            look_type: *look_type,
            addons: *addons,
            colours: [head, body, legs, feet],
        })
    }
}
