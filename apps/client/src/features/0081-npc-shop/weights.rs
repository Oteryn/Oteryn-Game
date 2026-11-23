//! Read catalogue template weights independently of the open Cyclopedia page.
use crate::play::{Outbox, mailbox::lock};
use oteryn_client_ui::inspection;
#[cfg(bevy_shell)]
impl crate::play::PlayLink {
    pub(crate) fn npc_request_weight(&self, quote: oteryn_client_ui::npc_shop::Quote) {
        use oteryn_client_runtime::raw_feature::FeatureCommand;
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.discovery.inspection.selected
            || mailbox.npc_dialog_dismissed
            || !mailbox.npc_shop.open
        {
            return;
        }
        let shop = &mut mailbox.npc_shop.live;
        if shop.weight_pending.is_some()
            || shop.weights.contains_key(&quote.item)
            || shop.look(quote).is_none()
        {
            return;
        }
        let Ok(payload) = inspection::encode_request(inspection::Request::Catalogue(quote.item))
        else {
            return;
        };
        let query = FeatureCommand {
            capability: 132,
            kind: 153,
            payload,
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(query))
            .is_ok()
        {
            shop.weight_pending = Some(quote);
        }
    }
}
impl Outbox {
    pub(crate) fn npc_weight_result(
        &self,
        definition: u32,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let shop = &mut mailbox.npc_shop.live;
        if !shop.weight_pending.is_some_and(|q| q.item == definition) {
            return;
        }
        let Some(quote) = shop.weight_pending.take() else {
            return;
        };
        if shop.look(quote).is_none() {
            return;
        }
        let weight = (result.status == oteryn_session::CommandStatus::Accepted)
            .then(|| inspection::decode_reply(&result.payload).ok())
            .flatten()
            .and_then(|reply| {
                let [item] = reply.items.as_slice() else {
                    return None;
                };
                if reply.status != 1 || item.count != 1 || item.definition_ref != quote.item {
                    return None;
                }
                let (_, value) = item
                    .attributes
                    .iter()
                    .find(|(label, _)| label == "Weight")?;
                let (whole, fraction) = value.strip_suffix(" oz")?.split_once('.')?;
                if fraction.len() != 2 {
                    return None;
                }
                whole
                    .parse::<u32>()
                    .ok()?
                    .checked_mul(100)?
                    .checked_add(fraction.parse().ok()?)
            });
        shop.weights.insert(quote.item, weight);
    }
}
