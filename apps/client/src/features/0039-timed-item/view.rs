use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn timed_item_panel(&self) -> oteryn_client_ui::timed_item::Panel {
        lock(&self.mailbox).properties.timed_item.clone()
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: "Timed items",
        labels: &[
            "Item handle",
            "Expected revision",
            "Aldo entity UUID (repair only)",
            "Aldo generation (repair only)",
        ],
        buttons: &["Inspect", "Use", "Repair"],
        snapshot: |mailbox| {
            let panel = &mailbox.properties.timed_item;
            (
                panel.open,
                panel.pending,
                panel.fields.to_vec(),
                oteryn_client_ui::timed_item::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.properties.timed_item.open = value,
        submit: |mailbox, commands, action, values| {
            let panel = &mut mailbox.properties.timed_item;
            if panel.pending {
                return;
            }
            for (index, value) in values.into_iter().enumerate() {
                super::input::edit(panel, index, value);
            }
            let intent = match action {
                0 => oteryn_client_ui::timed_item::Intent::Inspect,
                1 => oteryn_client_ui::timed_item::Intent::Use,
                2 => oteryn_client_ui::timed_item::Intent::Repair,
                _ => return,
            };
            super::input::submit(mailbox, commands, intent);
        },
    }
}

#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn inspect_slot_expiry(&self, target: oteryn_session::ItemHandle) -> bool {
        use oteryn_client_runtime::{raw_feature::FeatureCommand, timed_item as wire};
        let mut mailbox = lock(&self.mailbox);
        let properties = &mut mailbox.properties;
        if !properties.timed_item.selected
            || properties.timed_item.pending
            || properties.expiry_pending.is_some()
        {
            return false;
        }
        let intent = wire::TimedItemIntent {
            action: wire::TimedAction::Inspect,
            target,
            expected_revision: 0,
            npc: None,
        };
        let Ok(payload) = wire::encode_timed_item_intent(intent) else {
            return false;
        };
        let command = FeatureCommand {
            capability: wire::CAPABILITY_TIMED_ITEM_V1,
            kind: wire::COMMAND_TYPE_TIMED_ITEM_INTENT,
            payload,
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(command))
            .is_err()
        {
            return false;
        }
        properties.expiry_pending = Some(target);
        true
    }

    pub(crate) fn take_slot_expiry(
        &self,
    ) -> Option<(
        oteryn_session::ItemHandle,
        Option<oteryn_client_runtime::timed_item::TimedItemResult>,
    )> {
        lock(&self.mailbox).properties.expiry_reply.take()
    }
}
