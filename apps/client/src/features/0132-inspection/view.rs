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
    #[cfg(all(test, feature = "bevy-shell"))]
    pub(crate) fn bevy_item_details_selected(&self, selected: bool) {
        lock(&self.mailbox).discovery.inspection.selected = selected;
    }
    #[cfg(all(test, feature = "bevy-shell"))]
    pub(crate) fn bevy_item_details_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        crate::play::Outbox(self.mailbox.clone()).discovery_result(132, response);
    }
    #[cfg(all(test, feature = "bevy-shell"))]
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
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Item catalogue — Ctrl+Alt+Shift+I",
            labels: &[
                "Item name search",
                "Category (empty means all)",
                "Selected definition reference",
            ],
            buttons: &[
                "Browse",
                "Next page",
                "Previous category",
                "Next category",
                "Previous item",
                "Next item",
                "Item details",
                "Drop sources",
                "More drop sources",
                "Market offers",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.discovery.inspection;
                (
                    panel.catalogue_open,
                    panel.pending,
                    vec![
                        panel.search.clone(),
                        panel.category.clone(),
                        panel.catalogue_definition.clone(),
                    ],
                    text(mailbox),
                )
            },
            set_open: |mailbox, open| mailbox.discovery.inspection.catalogue_open = open,
            submit: |mailbox, commands, action, fields| {
                let panel = &mut mailbox.discovery.inspection;
                if panel.pending {
                    return;
                }
                panel.search = fields.first().cloned().unwrap_or_default();
                panel.category = fields.get(1).cloned().unwrap_or_default();
                panel.catalogue_definition = fields.get(2).cloned().unwrap_or_default();
                super::input::catalogue::submit(mailbox, commands, action);
            },
        },
    );
}

#[cfg(feature = "bevy-shell")]
#[path = "catalogue_bound.rs"]
mod catalogue_bound;
#[cfg(feature = "bevy-shell")]
pub use catalogue_bound::ItemCatalogueSnapshot;
