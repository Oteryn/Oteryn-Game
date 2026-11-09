use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn browse_field_initialize(&self, selected: bool) {
        lock(&self.0).browse_field.selected = selected;
    }
    pub(in crate::play) fn browse_field_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::browse_field::apply_result(&mut lock(&self.0).browse_field, response);
    }
}

impl Outbox {
    pub(in crate::play) fn browse_field_views(&self, payload: &[u8]) {
        let mut m = lock(&self.0);
        match oteryn_session::container_tree::decode_container_views_with_house_capacity(
            payload,
            m.browse_field.selected,
            m.containers.house_capacity_v2,
        ) {
            Ok(views) => replace_views(&mut m.browse_field, views),
            Err(_) => m.browse_field.feedback = Some(tr!("Invalid container view").into()),
        }
    }
    pub(in crate::play) fn browse_field_inventory(
        &self,
        inventory: oteryn_session::CharacterInventory,
    ) {
        lock(&self.0).browse_field.inventory = inventory;
    }
    pub(in crate::play) fn browse_field_moved(
        &self,
        outcome: Option<oteryn_client_runtime::inventory::MoveResponse>,
    ) {
        let mut m = lock(&self.0);
        m.browse_field.pending = false;
        m.browse_field.feedback = Some(oteryn_client_runtime::inventory::feedback(outcome).into());
    }
}

pub(in crate::play) fn replace_views(
    panel: &mut oteryn_client_ui::browse_field::BrowseFieldPanel,
    views: oteryn_session::container_tree::ContainerViews,
) {
    if !views.views.iter().any(|view| view.view_id == panel.view_id) {
        panel.view_id = views.views.first().map_or(0, |view| view.view_id);
    }
    panel.views = views;
}
