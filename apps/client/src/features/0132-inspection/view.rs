use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
impl PlayLink {
    /// Read the retained inspection owner without cloning unrelated discovery panels.
    pub fn inspection_panel(&self) -> oteryn_client_ui::inspection::Panel {
        lock(&self.mailbox).discovery.inspection.clone()
    }
    #[cfg(bevy_shell)]
    pub(crate) fn with_inspected_items<T>(
        &self,
        read: impl FnOnce(&oteryn_client_ui::inspection::Panel) -> T,
    ) -> T {
        read(&lock(&self.mailbox).discovery.inspection)
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
    #[cfg(bevy_shell)]
    pub(crate) fn item_catalogue_projected_text(
        &self,
        panel: &oteryn_client_ui::inspection::Panel,
    ) -> String {
        projected_text(&lock(&self.mailbox), panel)
    }
    pub fn item_catalogue_text(&self) -> String {
        text(&lock(&self.mailbox))
    }
}
fn text(mailbox: &Mailbox) -> String {
    projected_text(mailbox, &mailbox.discovery.inspection)
}
fn projected_text(mailbox: &Mailbox, panel: &oteryn_client_ui::inspection::Panel) -> String {
    let mut text = oteryn_client_ui::inspection::catalogue_text(panel);
    if let Ok(definition) = panel.catalogue_definition.parse::<u32>()
        && panel.catalogue.contains_key(&definition)
    {
        use std::fmt::Write;
        for quote in mailbox
            .npc_shop
            .live
            .quotes()
            .into_iter()
            .filter(|q| q.item == definition)
        {
            let verb = match quote.side {
                oteryn_client_ui::npc_shop::NpcTradeSide::Sell => "buys",
                oteryn_client_ui::npc_shop::NpcTradeSide::Buy => "sells",
            };
            let _ = write!(
                text,
                "\r\nCurrent merchant {verb} {} for {} gold",
                quote.count, quote.price
            );
        }
    }
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
    ) -> Option<oteryn_client_ui::inspection::OutfitPortrait> {
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
        let tint = mailbox
            .properties
            .appearance
            .catalogue
            .as_ref()
            .and_then(|catalogue| {
                let [head, body, legs, feet] =
                    colours.map(|index| catalogue.palette.get(usize::from(index)).copied());
                Some([head?, body?, legs?, feet?])
            });
        Some(oteryn_client_ui::inspection::OutfitPortrait {
            look_type: *look_type,
            addons: *addons,
            colours: tint,
        })
    }
}
