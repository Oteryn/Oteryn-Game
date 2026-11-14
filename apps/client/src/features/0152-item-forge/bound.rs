//! Atomic shared-frontend controls over the retained Forge owner.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::cui3::item_forge::Operation;
use oteryn_client_ui::item_forge::ItemForgePanel;
use std::sync::{Arc, Weak};
#[derive(Clone)]
pub(crate) struct ForgeSnapshot {
    pub panel: ItemForgePanel,
    session: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
impl PartialEq for ForgeSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.panel == other.panel && Weak::ptr_eq(&self.session, &other.session)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForgeControl {
    Close,
    Refresh,
    Next,
    Operation(Operation),
    First(u64),
    Second(u64),
    Cores { boost: bool, protect: bool },
    Quote,
    Review,
    Forge,
    Retry,
    History,
    Resource(Operation),
    ConfirmResource(u64),
    DismissResource(u64),
}
impl PlayLink {
    pub(crate) fn item_forge_snapshot(&self) -> ForgeSnapshot {
        ForgeSnapshot {
            panel: lock(&self.mailbox).item_forge.clone(),
            session: Arc::downgrade(&self.mailbox),
        }
    }
    /// Re-proves the session, current operands and exact displayed quote under one lock.
    pub(crate) fn item_forge_dispatch_bound(
        &self,
        expected: &ForgeSnapshot,
        control: ForgeControl,
    ) -> bool {
        use super::super::input::controller::{Action, activate};
        let mut mailbox = lock(&self.mailbox);
        if !Weak::ptr_eq(&expected.session, &Arc::downgrade(&self.mailbox))
            || mailbox.quest_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.item_forge != expected.panel
            || !expected.panel.open
        {
            return false;
        }
        if control == ForgeControl::Close {
            mailbox.item_forge.open = false;
            mailbox.item_forge.resource_dialog.reset();
            return true;
        }
        let panel = &mailbox.item_forge;
        if !panel.selected || panel.pending || panel.preview.pending.is_some() {
            return false;
        }
        let current_pair = || {
            [panel.first_item, panel.second_item]
                .into_iter()
                .all(|handle| {
                    panel.preview.item(handle).is_some_and(|item| {
                        !item.container
                            && panel.preview.live.iter().any(|entry| {
                                entry.handle == item.handle
                                    && entry.item_definition_ref.get() == item.definition_ref
                            })
                    })
                })
                && panel
                    .selection()
                    .is_some_and(|selection| selection.canonical([0; 16]).is_some())
        };
        let action = match control {
            ForgeControl::Close => return false,
            ForgeControl::Refresh if panel.editing() && panel.preview.selected => Action::Refresh,
            ForgeControl::Next
                if panel.editing()
                    && panel.preview.selected
                    && panel
                        .preview
                        .page
                        .as_ref()
                        .is_some_and(|p| p.next.is_some()) =>
            {
                Action::Next
            }
            ForgeControl::Operation(op) if panel.editing() && op.requires_pair() => {
                Action::Operation(op)
            }
            ForgeControl::First(handle) | ForgeControl::Second(handle)
                if panel.editing()
                    && panel
                        .preview
                        .item(handle)
                        .is_some_and(|item| !item.container) =>
            {
                if matches!(control, ForgeControl::First(_)) {
                    Action::First(handle)
                } else {
                    Action::Second(handle)
                }
            }
            ForgeControl::Cores { boost, protect }
                if panel.editing() && panel.operation == Operation::Fusion =>
            {
                Action::Cores { boost, protect }
            }
            ForgeControl::Quote if panel.editing() && panel.preview.selected && current_pair() => {
                Action::Quote
            }
            ForgeControl::Review => {
                return panel.editing()
                    && panel.preview.selected
                    && panel.verified_quote()
                    && current_pair();
            }
            ForgeControl::Forge
                if panel.editing()
                    && panel.preview.selected
                    && panel.verified_quote()
                    && current_pair() =>
            {
                Action::Forge
            }
            ForgeControl::Retry if panel.retry.is_some() => Action::Retry,
            ForgeControl::History if !panel.resource_dialog.confirming() => {
                Action::Legacy(Operation::QueryHistory)
            }
            ForgeControl::Resource(op)
                if panel.editing()
                    && panel.preview.selected
                    && oteryn_client_runtime::forge_preview::resource_operation(op) =>
            {
                Action::Legacy(op)
            }
            ForgeControl::ConfirmResource(serial)
                if panel.resource_dialog.serial == serial
                    && panel
                        .resource_dialog
                        .expires
                        .is_some_and(|at| at > std::time::Instant::now())
                    && matches!(
                        panel.resource_dialog.content,
                        Some(
                            oteryn_client_ui::item_forge::resource_dialog::Content::Confirm {
                                quote: Some(_),
                                ..
                            }
                        )
                    ) =>
            {
                Action::ConfirmResource(serial)
            }
            ForgeControl::DismissResource(serial) if panel.resource_dialog.serial == serial => {
                Action::DismissResource(serial)
            }
            _ => return false,
        };
        let before = mailbox.item_forge.clone();
        activate(&mut mailbox, &self.commands, action);
        mailbox.item_forge != before
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_forge_setup(&self) {
        self.item_forge_initialize(true);
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.item_forge;
        panel.open = true;
        panel.preview.selected = true;
        panel.preview.live = [2, 3]
            .into_iter()
            .filter_map(|handle| {
                Some(oteryn_client_runtime::depot::ItemEntry {
                    handle: std::num::NonZeroU64::new(handle)?,
                    item_definition_ref: std::num::NonZeroU32::new(7)?,
                    count: std::num::NonZeroU32::MIN,
                    sub_type: 0,
                })
            })
            .collect();
    }
    pub(crate) fn bevy_forge_preview(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        reply: oteryn_client_runtime::forge_preview::Reply,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.properties_result(
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
                payload: reply.encode().ok_or("forge preview reply")?,
            },
        );
        Ok(())
    }
    pub(crate) fn bevy_forge_result(&self, payload: Vec<u8>) {
        self.item_forge_result(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 2,
                server_sequence: 2,
                status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
                payload,
            },
        );
    }
    pub(crate) fn bevy_forge_item_changed(&self) {
        lock(&self.0).item_forge.observe(vec![]);
    }
    pub(crate) fn bevy_forge_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::play_channel;
    #[test]
    fn forge_bound_owner_rejects_every_control_atomically() -> Result<(), Box<dyn std::error::Error>>
    {
        for failure in 0..7 {
            let (link, mut commands, outbox) = play_channel();
            link.use_shared_frontend();
            outbox.bevy_forge_setup();
            let expected = link.item_forge_snapshot();
            let _modal = if failure == 0 {
                Some(link.bevy_server_modal_fixture())
            } else {
                None
            };
            match failure {
                0 => assert!(link.server_modal_active()),
                1 => outbox.bevy_forge_item_changed(),
                2 => outbox.bevy_forge_end(),
                3 => lock(&link.mailbox).item_forge.open = false,
                4 => lock(&link.mailbox).item_forge.pending = true,
                5 => lock(&link.mailbox).item_forge.preview.selected = false,
                _ => commands.close(),
            }
            let before = link.item_forge_panel();
            for control in [
                ForgeControl::Close,
                ForgeControl::Refresh,
                ForgeControl::Next,
                ForgeControl::Operation(Operation::Fusion),
                ForgeControl::First(2),
                ForgeControl::Second(3),
                ForgeControl::Cores {
                    boost: true,
                    protect: true,
                },
                ForgeControl::Quote,
                ForgeControl::Review,
                ForgeControl::Forge,
                ForgeControl::Retry,
                ForgeControl::History,
                ForgeControl::Resource(Operation::DustToSlivers),
                ForgeControl::Resource(Operation::SliversToCores),
                ForgeControl::Resource(Operation::DustLimit),
                ForgeControl::ConfirmResource(0),
                ForgeControl::DismissResource(0),
            ] {
                assert!(
                    !link.item_forge_dispatch_bound(&expected, control),
                    "{control:?}"
                );
                assert_eq!(link.item_forge_panel(), before);
                assert!(commands.try_recv().is_err());
            }
        }
        Ok(())
    }
    #[test]
    fn forge_bound_owner_rejects_identical_snapshot_from_another_session() {
        let (first, _first_commands, first_outbox) = play_channel();
        let (second, mut second_commands, second_outbox) = play_channel();
        first_outbox.bevy_forge_setup();
        second_outbox.bevy_forge_setup();
        let expected = first.item_forge_snapshot();
        let before = second.item_forge_panel();
        assert_eq!(expected.panel, before);
        assert!(!second.item_forge_dispatch_bound(&expected, ForgeControl::Refresh));
        assert_eq!(second.item_forge_panel(), before);
        assert!(second_commands.try_recv().is_err());
    }
}
