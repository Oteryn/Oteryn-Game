//! Project the authoritative catalogue inspection into the loaded market page.
use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::inspection;

#[cfg(bevy_shell)]
impl crate::play::PlayLink {
    pub(crate) fn market_toggle_hands(&self, hand: u8) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.discovery.inspection.selected || !mailbox.market.open {
            return false;
        }
        let panel = &mut mailbox.market;
        panel.hand_filter = (panel.hand_filter != Some(hand)).then_some(hand);
        true
    }

    pub(crate) fn market_request_hand(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.market;
        if !panel.open
            || panel.hand_filter.is_none()
            || panel.hand_pending.is_some()
            || panel.details.pending.is_some()
        {
            return;
        }
        panel
            .hand_positions
            .retain(|id, _| panel.details.items.iter().any(|w| w.definition == *id));
        let Some(ware) = panel
            .details
            .items
            .iter()
            .find(|w| !panel.hand_positions.contains_key(&w.definition))
        else {
            return;
        };
        let Ok(payload) =
            inspection::encode_request(inspection::Request::Catalogue(ware.definition))
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
            panel.hand_pending = Some(ware.definition);
        }
    }
}

impl Outbox {
    pub(crate) fn trade_inspection_result(
        &self,
        command: &FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let Ok(inspection::Request::Catalogue(definition)) =
            inspection::decode_request(&command.payload)
        else {
            return;
        };
        self.npc_weight_result(definition, result);
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.market;
        if panel.hand_pending != Some(definition) {
            return;
        }
        panel.hand_pending = None;
        let position = (result.status == oteryn_session::CommandStatus::Accepted)
            .then(|| inspection::decode_reply(&result.payload).ok())
            .flatten()
            .and_then(|reply| {
                let [item] = reply.items.as_slice() else {
                    return None;
                };
                if reply.status != 1 || item.definition_ref != definition {
                    return None;
                }
                let (_, position) = item
                    .attributes
                    .iter()
                    .find(|(label, _)| label == "Body Position")?;
                match position.as_str() {
                    "weapon hand" => Some(1),
                    "both hands" => Some(2),
                    _ => None,
                }
            });
        panel.hand_positions.insert(definition, position);
    }
}
