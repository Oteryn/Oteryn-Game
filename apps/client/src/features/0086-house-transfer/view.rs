use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_transfer_panel(&self) -> oteryn_client_ui::house_transfer::Panel {
        lock(&self.mailbox).properties.house_transfer.clone()
    }
}

/// A presentation snapshot bound to the command stream that produced it.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub struct HouseTransferSnapshot {
    pub panel: oteryn_client_ui::house_transfer::Panel,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for HouseTransferSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.commands.same_channel(&other.commands) && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn house_transfer_snapshot(&self) -> HouseTransferSnapshot {
        HouseTransferSnapshot {
            panel: lock(&self.mailbox).properties.house_transfer.clone(),
            commands: self.commands.clone(),
        }
    }

    pub fn house_transfer_set_open_bound(
        &self,
        expected: &HouseTransferSnapshot,
        open: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.same_channel(&expected.commands)
            || self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.properties.house_transfer != expected.panel
        {
            return false;
        }
        mailbox.properties.house_transfer.open = open;
        true
    }

    /// Check the exact session, target and pending state and enqueue under the owner lock.
    /// None closes the presentation; Enter/Exit retain the native owner's reducer/transport.
    pub fn house_transfer_dispatch_bound(
        &self,
        expected: &HouseTransferSnapshot,
        intent: Option<oteryn_client_ui::house_transfer::Intent>,
        house: &str,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.same_channel(&expected.commands)
            || self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.properties.house_transfer != expected.panel
            || !mailbox.properties.house_transfer.open
        {
            return false;
        }
        let Some(intent) = intent else {
            mailbox.properties.house_transfer.open = false;
            return true;
        };
        let panel = &mut mailbox.properties.house_transfer;
        if !panel.selected || panel.pending || house.len() > 512 || house.contains('\0') {
            return false;
        }
        super::input::edit(panel, 0, house.into());
        super::input::submit(&mut mailbox, &self.commands, intent);
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_house_transfer_selected(&self, selected: bool) {
        lock(&self.0).properties.house_transfer.selected = selected;
    }
    pub(crate) fn bevy_house_transfer_response(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.properties_result(command, result);
    }
    pub(crate) fn bevy_house_transfer_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
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
            title: "House transfer",
            labels: &["House key"],
            buttons: &["Enter", "Exit"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_transfer;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::house_transfer::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_transfer.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_transfer;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::house_transfer::Intent::Enter,
                    1 => oteryn_client_ui::house_transfer::Intent::Exit,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
