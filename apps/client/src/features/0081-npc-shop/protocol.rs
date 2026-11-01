use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn npc_shop_initialize(&self, selected: bool) {
        lock(&self.0).npc_shop.selected = selected;
    }
    pub(in crate::play) fn npc_shop_result(
        &self,
        result: &oteryn_client_runtime::npc_shop::NpcShopResponse,
    ) {
        oteryn_client_ui::npc_shop::apply_result(&mut lock(&self.0).npc_shop, result);
    }
}
